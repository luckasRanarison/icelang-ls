use tower_lsp::lsp_types::{Diagnostic, Position, Range};
use tree_sitter::{Node, Tree};

use crate::{
    ast::{NodeType, FUNCTION_NODE, LOOP_NODE},
    declarations::{Declaration, DeclarationKind, DeclarationMap, VariableType},
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
            NodeType::StmtFor => self.eval_for_loop(node),
            NodeType::StmtContinue => self.eval_continue(node),
            NodeType::StmtBreak => self.eval_break(node),
            NodeType::StmtReturn => self.eval_return(node),
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

    fn eval_var_decl(&mut self, node: &Node) {
        let name_node = node.child_by_field_name("name").unwrap();
        let name = name_node.utf8_text(&self.source).unwrap();
        let value_node = node.child_by_field_name("value").unwrap();
        let mut scope = None;

        if let Some(parent) = node.parent() {
            if NodeType::from(&parent) == NodeType::StmtBlock {
                scope = Some(get_node_range(&parent));
            }
        }

        let declaration = match NodeType::from(&value_node) {
            NodeType::ExprLambda => {
                let body = value_node.child_by_field_name("body").unwrap();
                let (names, args_decl) = self.get_function_args(&value_node);
                let kind = DeclarationKind::Function(names);

                let range = Range::new(
                    point_to_position(node.start_position()),
                    point_to_position(body.start_position()),
                );

                for decl in args_decl {
                    self.declarations.insert(decl);
                }

                Declaration::new(name.to_owned(), kind, range, scope)
            }
            _ => {
                let kind = DeclarationKind::Variable(self.get_var_type(&value_node));
                let range = tsrange_to_lsprange(node.range());

                Declaration::new(name.to_owned(), kind, range, scope)
            }
        };

        if !self.declarations.insert(declaration) {
            self.diagnostics.push(error(
                ErrorKind::Redeclaration(name.to_owned()),
                get_node_range(&name_node),
            ));
        }
    }

    fn eval_func_decl(&mut self, node: &Node) {
        let name_node = node.child_by_field_name("name").unwrap();
        let block = node.child_by_field_name("body").unwrap();

        let (names, args_decl) = self.get_function_args(&node);
        let name = name_node.utf8_text(&self.source).unwrap();
        let kind = DeclarationKind::Function(names);

        let range = Range::new(
            point_to_position(node.start_position()),
            point_to_position(block.start_position()),
        );

        let scope = node
            .parent()
            .filter(|parent| NodeType::from(parent) == NodeType::StmtBlock)
            .map(|parent| get_node_range(&parent));

        for decl in args_decl {
            self.declarations.insert(decl);
        }

        let declaration = Declaration::new(name.to_owned(), kind, range, scope);

        if !self.declarations.insert(declaration) {
            self.diagnostics.push(error(
                ErrorKind::Redeclaration(name.to_owned()),
                get_node_range(&name_node),
            ));
        }
    }

    fn eval_identifier(&mut self, node: &Node) {
        let name = node.utf8_text(&self.source).unwrap();
        let range = get_node_range(&node);

        if !skip_identifer(node) {
            self.identifiers.push((name.to_owned(), range));
        }
    }

    fn eval_continue(&mut self, node: &Node) {
        if has_parent(node, &LOOP_NODE) {
            self.next_unreachable(node);
        } else {
            self.diagnostics
                .push(error(ErrorKind::ContinueOutside, get_node_range(node)));
        }
    }

    fn eval_break(&mut self, node: &Node) {
        if has_parent(node, &LOOP_NODE) {
            self.next_unreachable(node);
        } else {
            self.diagnostics
                .push(error(ErrorKind::BreakOutside, get_node_range(node)));
        }
    }

    fn eval_return(&mut self, node: &Node) {
        if has_parent(node, &FUNCTION_NODE) {
            self.next_unreachable(node);
        } else {
            self.diagnostics
                .push(error(ErrorKind::ReturnOutside, get_node_range(node)));
        }
    }

    fn eval_for_loop(&mut self, node: &Node) {
        let iterator = node.child_by_field_name("iterator").unwrap();
        let body = node.child_by_field_name("body").unwrap();
        let mut cursor = Node::walk(&iterator);

        for child in iterator.children(&mut cursor) {
            let name = child.utf8_text(&self.source).unwrap();
            let kind = DeclarationKind::Variable(VariableType::Any);
            let range = get_node_range(&iterator);
            let scope = Some(get_node_range(&body));
            let declaration = Declaration::new(name.to_owned(), kind, range, scope);

            self.declarations.insert(declaration);
        }
    }

    // TODO: dynamic type resolution
    fn get_var_type(&mut self, node: &Node) -> VariableType {
        match NodeType::from(node) {
            NodeType::ExprLiteral => {
                let literal = node.child(0).unwrap();

                match literal.kind() {
                    "number" => VariableType::Number,
                    "string" => VariableType::String,
                    "boolean" => VariableType::Boolean,
                    "null" => VariableType::Null,
                    _ => VariableType::Any,
                }
            }
            NodeType::ExprObject => {
                let mut props = Vec::new();
                let mut cursor = Node::walk(&node);

                for prop in node.children(&mut cursor) {
                    if NodeType::from(&prop) == NodeType::Prop {
                        let name_node = prop.child_by_field_name("name").unwrap();
                        let name = name_node.utf8_text(&self.source).unwrap();

                        props.push(name.to_owned())
                    }
                }

                VariableType::Object(props)
            }
            NodeType::ExprArray => VariableType::Array,
            _ => VariableType::Any,
        }
    }

    fn get_function_args(&self, node: &Node) -> (Vec<String>, Vec<Declaration>) {
        let mut names = Vec::new();
        let mut declarations = Vec::new();
        let body = node.child_by_field_name("body").unwrap();
        let args = node.child_by_field_name("args").unwrap();
        let mut cursor = Node::walk(&args);

        for arg in args.named_children(&mut cursor) {
            let name = arg.utf8_text(&self.source).unwrap();
            let kind = DeclarationKind::Variable(VariableType::Any);
            let range = get_node_range(&args);
            let scope = Some(get_node_range(&body));
            let declaration = Declaration::new(name.to_string(), kind, range, scope);

            names.push(name.to_owned());
            declarations.push(declaration)
        }

        (names, declarations)
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
        return match NodeType::from(&parent) {
            NodeType::StmtFuncDecl | NodeType::ExprField | NodeType::Iterator => true,
            NodeType::StmtVarDecl | NodeType::Prop => {
                parent.child_by_field_name("name") == Some(*node)
            }
            NodeType::Args => match parent.parent() {
                Some(value) => NodeType::from(&value) != NodeType::ExprCall,
                None => false,
            },
            _ => false,
        };
    }

    false
}

fn has_parent(node: &Node, parent_types: &[NodeType]) -> bool {
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
