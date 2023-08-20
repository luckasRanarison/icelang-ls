use std::collections::HashMap;

use tower_lsp::lsp_types::{Diagnostic, Range};
use tree_sitter::{Node, Tree};

use crate::{
    declarations::{Declaration, DeclarationKind},
    diagnostic::{error, ErrorKind},
    utils::{get_node_range, point_to_position},
};

pub fn analyze(source: &[u8], tree: &Tree) -> (Vec<Diagnostic>, HashMap<String, Declaration>) {
    Analyzer::new(source, tree).analyze()
}

struct Analyzer<'a> {
    source: &'a [u8],
    tree: &'a Tree,
    diagnostics: Vec<Diagnostic>,
    declarations: HashMap<String, Declaration>,
    undeclared: HashMap<String, Range>,
}

impl<'a> Analyzer<'a> {
    fn new(source: &'a [u8], tree: &'a Tree) -> Self {
        Self {
            source,
            tree,
            diagnostics: vec![],
            declarations: HashMap::new(),
            undeclared: HashMap::new(),
        }
    }

    fn analyze(&mut self) -> (Vec<Diagnostic>, HashMap<String, Declaration>) {
        let tree = self.tree.clone();
        let root_node = tree.root_node();
        let mut cursor = Node::walk(&root_node);

        for child in root_node.children(&mut cursor) {
            self.eval_node(&child);
        }

        self.resolve_identifiers();

        (self.diagnostics.clone(), self.declarations.clone())
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
                    match child.kind() {
                        "expr_identifier" => error(ErrorKind::SyntaxError, child_range),
                        _ => error(ErrorKind::Unexpected, child_range),
                    }
                }
                None => error(ErrorKind::SyntaxError, range),
            };

            self.diagnostics.push(error);
        }

        if node.is_missing() {
            let range = get_node_range(&node);
            let error = match node.kind() {
                "expr_identifier" => error(ErrorKind::ExpectedExpr, range),
                _ => error(ErrorKind::Missing(node.kind().to_owned()), range),
            };

            self.diagnostics.push(error);
        }
    }

    fn handle_declaration(&mut self, node: &Node) {
        // TODO: handle scope & function params
        match node.kind() {
            "stmt_var_decl" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(&self.source).unwrap();
                    let range = get_node_range(&node);
                    let end_pos = point_to_position(node.end_position());
                    let declaration = Declaration {
                        kind: DeclarationKind::Variable(None),
                        end_pos,
                        scope: None,
                    };

                    if self.declarations.contains_key(name) {
                        self.diagnostics
                            .push(error(ErrorKind::Redeclaration(name.to_owned()), range));
                    } else {
                        self.declarations.insert(name.to_owned(), declaration);
                    }
                }
            }
            "stmt_func_decl" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = name_node.utf8_text(&self.source).unwrap();
                    let range = get_node_range(&node);
                    let end_pos = point_to_position(node.end_position());
                    let declaration = Declaration {
                        kind: DeclarationKind::Function(vec![]),
                        end_pos,
                        scope: None,
                    };

                    if self.declarations.contains_key(name) {
                        self.diagnostics
                            .push(error(ErrorKind::Redeclaration(name.to_owned()), range));
                    } else {
                        self.undeclared.remove(name); // hoisting
                        self.declarations.insert(name.to_owned(), declaration);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_runtime_exception(&mut self, node: &Node) {
        match node.kind() {
            "expr_identifier" => {
                let name = node.utf8_text(&self.source).unwrap();
                let range = get_node_range(&node);

                if !self.declarations.contains_key(name) {
                    self.undeclared.insert(name.to_owned(), range);
                }
            }
            _ => {}
        }
    }

    fn resolve_identifiers(&mut self) {
        for (name, range) in &self.undeclared {
            self.diagnostics
                .push(error(ErrorKind::Undeclared(name.to_owned()), *range));
        }
    }
}
