use tower_lsp::lsp_types::{Diagnostic, Range};
use tree_sitter::{Node, Tree};

use crate::{
    ast::NodeType,
    declarations::{Declaration, DeclarationKind, DeclarationMap},
    diagnostic::{error, ErrorKind},
    utils::*,
};

pub fn analyze(source: &[u8], tree: &Tree) -> AnalyzeResult {
    Analyzer::new(source, tree).analyze()
}

pub struct AnalyzeResult {
    pub diagnostics: Vec<Diagnostic>,
    pub declarations: DeclarationMap,
}

struct Analyzer<'a> {
    source: &'a [u8],
    tree: &'a Tree,
    diagnostics: Vec<Diagnostic>,
    declarations: DeclarationMap,
    identifiers: Vec<(String, Range)>,
}

impl<'a> Analyzer<'a> {
    fn new(source: &'a [u8], tree: &'a Tree) -> Self {
        Self {
            source,
            tree,
            diagnostics: Vec::new(),
            declarations: DeclarationMap::new(),
            identifiers: Vec::new(),
        }
    }

    fn analyze(&mut self) -> AnalyzeResult {
        let tree = self.tree.clone();
        let root_node = tree.root_node();
        let mut cursor = Node::walk(&root_node);

        for child in root_node.children(&mut cursor) {
            self.eval_node(&child);
        }

        self.resolve_identifiers();

        AnalyzeResult {
            diagnostics: self.diagnostics.clone(),
            declarations: self.declarations.clone(),
        }
    }

    fn eval_node(&mut self, node: &Node) {
        self.handle_syntax_error(node);

        match NodeType::from(node) {
            NodeType::StmtVarDecl => self.eval_var_decl(node),
            NodeType::StmtFuncDecl => self.eval_func_decl(node),
            NodeType::ExprIdentifier => self.eval_identifier(node),
            _ => {}
        }

        let mut cursor = Node::walk(node);

        for child in node.children(&mut cursor) {
            self.eval_node(&child);
        }
    }

    fn handle_syntax_error(&mut self, node: &Node) {
        if node.is_error() {
            let range = get_node_range(&node);
            let child = node.named_child(0);
            let error = match child {
                Some(child) => {
                    let child_range = get_node_range(&child);
                    match NodeType::from(&child) {
                        NodeType::ExprIdentifier => error(ErrorKind::SyntaxError, child_range),
                        _ => error(ErrorKind::Unexpected, child_range),
                    }
                }
                None => error(ErrorKind::SyntaxError, range),
            };

            self.diagnostics.push(error);
        }

        if node.is_missing() {
            let range = get_node_range(&node);
            let error = match NodeType::from(node) {
                NodeType::ExprIdentifier => error(ErrorKind::ExpectedExpr, range),
                _ => error(ErrorKind::Missing(node.kind().to_owned()), range),
            };

            self.diagnostics.push(error);
        }
    }

    // TODO: handle variable type
    fn eval_var_decl(&mut self, node: &Node) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = name_node.utf8_text(&self.source).unwrap();
            let kind = DeclarationKind::Variable(None);
            let range = tsrange_to_lsprange(node.range());
            let parent = node.parent();
            let mut scope = None;

            if let Some(parent) = parent {
                if NodeType::from(&parent) == NodeType::StmtBlock {
                    scope = Some(get_node_range(&parent));
                }
            }

            let declaration = Declaration {
                name: name.to_owned(),
                kind,
                range,
                scope,
            };
            let inserted = self.declarations.insert(declaration);

            if !inserted {
                let name_range = get_node_range(&name_node);

                self.diagnostics
                    .push(error(ErrorKind::Redeclaration(name.to_owned()), name_range));
            }
        }
    }

    // TODO: handle function params
    fn eval_func_decl(&mut self, node: &Node) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = name_node.utf8_text(&self.source).unwrap();
            let kind = DeclarationKind::Function(vec![]);
            let block = node.child_by_field_name("body").unwrap();

            let start = point_to_position(node.start_position());
            let end = point_to_position(block.start_position());
            let range = Range::new(start, end);

            let parent = node.parent();
            let mut scope = None;

            if let Some(parent) = parent {
                if NodeType::from(&parent) == NodeType::StmtBlock {
                    scope = Some(get_node_range(&parent));
                }
            }

            let declaration = Declaration {
                name: name.to_owned(),
                kind,
                range,
                scope,
            };
            let inserted = self.declarations.insert(declaration);

            if !inserted {
                let range = get_node_range(&name_node);

                self.diagnostics
                    .push(error(ErrorKind::Redeclaration(name.to_owned()), range));
            }
        }
    }

    fn eval_identifier(&mut self, node: &Node) {
        let name = node.utf8_text(&self.source).unwrap();
        let range = get_node_range(&node);

        if !skip_identifer(node) {
            self.identifiers.push((name.to_owned(), range));
        }
    }

    fn resolve_identifiers(&mut self) {
        for (name, range) in &self.identifiers {
            if name == "" {
                self.diagnostics
                    .push(error(ErrorKind::ExpectedField, *range));
                continue;
            }

            let is_declared = self.declarations.is_declared_at(name, range.end);

            if !is_declared {
                self.diagnostics
                    .push(error(ErrorKind::Undeclared(name.to_owned()), *range));
            }
        }
    }
}

fn skip_identifer(node: &Node) -> bool {
    if let Some(parent) = node.parent() {
        let name_node = parent.child_by_field_name("name");

        match NodeType::from(&parent) {
            NodeType::StmtFuncDecl | NodeType::Args | NodeType::ExprField => true,
            NodeType::StmtVarDecl | NodeType::Prop => name_node == Some(*node),
            _ => false,
        }
    } else {
        false
    }
}
