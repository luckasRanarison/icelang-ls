use tower_lsp::lsp_types::{Diagnostic, Range};
use tree_sitter::{Node, Tree};

use crate::{
    ast::NodeType,
    declarations::{Declaration, DeclarationKind, DeclarationMap},
    diagnostic::{error, ErrorKind},
    utils::{get_node_range, point_to_position},
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
        self.handle_declaration(node);
        self.handle_runtime_exception(node);

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

    fn handle_declaration(&mut self, node: &Node) {
        let node_type = NodeType::from(node);

        // TODO: handle scope & function params
        match node_type {
            NodeType::StmtVarDecl | NodeType::StmtFuncDecl => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(&self.source).unwrap();
                    let range = get_node_range(&name_node);
                    let start = point_to_position(node.end_position());
                    let parent = node.parent();
                    let mut scope = None;

                    if let Some(parent) = parent {
                        if NodeType::from(&parent) == NodeType::StmtBlock {
                            scope = Some(get_node_range(&parent));
                        }
                    }

                    let kind = match node_type {
                        NodeType::StmtVarDecl => DeclarationKind::Variable(None),
                        NodeType::StmtFuncDecl => DeclarationKind::Function(vec![]),
                        _ => unreachable!(),
                    };
                    let declaration = Declaration {
                        name: name.to_owned(),
                        kind,
                        start,
                        scope,
                    };
                    let inserted = self.declarations.insert(declaration);

                    if !inserted {
                        self.diagnostics
                            .push(error(ErrorKind::Redeclaration(name.to_owned()), range));
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_runtime_exception(&mut self, node: &Node) {
        match NodeType::from(node) {
            NodeType::ExprIdentifier => {
                let name = node.utf8_text(&self.source).unwrap();
                let range = get_node_range(&node);
                let parent = node.parent();

                if let Some(parent) = parent {
                    match NodeType::from(&parent) {
                        NodeType::StmtVarDecl | NodeType::StmtFuncDecl | NodeType::Args => {}
                        _ => self.identifiers.push((name.to_owned(), range)),
                    }
                } else {
                    self.identifiers.push((name.to_owned(), range));
                }
            }
            _ => {}
        }
    }

    fn resolve_identifiers(&mut self) {
        for (name, range) in &self.identifiers {
            let is_declared = self.declarations.is_declared(name, range);

            if !is_declared {
                self.diagnostics
                    .push(error(ErrorKind::Undeclared(name.to_owned()), *range));
            }
        }
    }
}
