use tower_lsp::lsp_types::{Diagnostic, Position, Range};
use tree_sitter::{Node, Tree};

use crate::{
    ast::NodeType,
    declarations::{Declaration, DeclarationKind, DeclarationMap},
    diagnostic::{error, hint, ErrorKind, HintKind},
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
            NodeType::StmtContinue | NodeType::StmtBreak | NodeType::StmtReturn => {
                self.eval_control_flow(node)
            }
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
                        NodeType::ExprIdentifier => {
                            error(ErrorKind::SyntaxError, child_range)
                        }
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

    fn eval_control_flow(&mut self, node: &Node) {
        let node_type = NodeType::from(node);
        let parent_types = match node_type {
            NodeType::StmtContinue | NodeType::StmtBreak => {
                vec![NodeType::StmtFor, NodeType::StmtWhile, NodeType::StmtLoop]
            }
            NodeType::StmtReturn => vec![NodeType::StmtFuncDecl, NodeType::ExprLambda],
            _ => unreachable!(),
        };

        if has_parent(node, parent_types) {
            self.next_unreachable(node);
        } else {
            let kind = match NodeType::from(node) {
                NodeType::StmtContinue => ErrorKind::ContinueOutside,
                NodeType::StmtBreak => ErrorKind::BreakOutside,
                NodeType::StmtReturn => ErrorKind::ReturnOutside,
                _ => unreachable!(),
            };

            self.diagnostics.push(error(kind, get_node_range(node)))
        }
    }

    fn next_unreachable(&mut self, node: &Node) {
        let mut start: Option<Position> = None;
        let mut end: Option<Position> = None;
        let mut sibling = node.next_named_sibling();

        while let Some(value) = sibling {
            if start.is_none() {
                let node_start = point_to_position(value.start_position());

                start = Some(node_start);
            }

            let node_end = point_to_position(value.end_position());

            if let Some(value) = end {
                if node_end > value {
                    end = Some(node_end)
                }
            } else {
                end = Some(node_end);
            }

            sibling = value.next_named_sibling();
        }

        if let (Some(start), Some(end)) = (start, end) {
            self.diagnostics
                .push(hint(HintKind::Unreachable, Range::new(start, end)));
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

        return match NodeType::from(&parent) {
            NodeType::StmtFuncDecl
            | NodeType::Args
            | NodeType::ExprField
            | NodeType::Iterator => true,
            NodeType::StmtVarDecl | NodeType::Prop => name_node == Some(*node),
            _ => false,
        };
    } else {
        false
    }
}

fn has_parent(node: &Node, parent_types: Vec<NodeType>) -> bool {
    let mut parent = node.parent();

    while let Some(value) = parent {
        let parent_type = NodeType::from(&value);

        if parent_types.contains(&parent_type) {
            return true;
        }

        parent = value.parent();
    }

    false
}
