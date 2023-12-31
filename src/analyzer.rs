use tower_lsp::lsp_types::{Diagnostic, Position, Range};
use tree_sitter::{Node, Tree};

use crate::{
    ast::{NodeType, FUNCTION_NODE, LOOP_NODE},
    builtins::KEYWORDS,
    declarations::{Declaration, DeclarationKind, DeclarationMap},
    diagnostic::{error, hint, warn, ErrorKind, HintKind, WarnKind},
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
    identifiers: Vec<Identifier>, // FIXME: use symbol table
}

pub struct Identifier {
    pub name: String,
    pub range: Range,
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
        self.report_unused();

        AnalyzeResult {
            diagnostics: self.diagnostics.clone(),
            declarations: self.declarations.clone(),
        }
    }

    fn eval_node(&mut self, node: &Node) {
        self.handle_syntax_error(node);

        match NodeType::from(node) {
            NodeType::StmtExpression => self.eval_expression(node),
            NodeType::StmtVarDecl => self.eval_var_decl(node),
            NodeType::StmtFuncDecl => self.eval_func_decl(node),
            NodeType::StmtFor => self.eval_for_loop(node),
            NodeType::StmtContinue => self.eval_continue(node),
            NodeType::StmtBreak => self.eval_break(node),
            NodeType::StmtReturn => self.eval_return(node),
            NodeType::ExprMatch => self.eval_match(node),
            NodeType::ExprLambda => self.eval_lambda(node),
            NodeType::ExprIdentifier => self.eval_identifier(node),
            NodeType::ExprLiteral => self.eval_literal(node),
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
                NodeType::ExprIdentifier => {
                    let parent = node.parent();
                    let text = parent.as_ref().map(|p| p.utf8_text(&self.source).unwrap());
                    let kind = match text {
                        Some(".") => ErrorKind::ExpectedField,
                        _ => ErrorKind::ExpectedExpr,
                    };

                    error(kind, range)
                }
                _ => error(ErrorKind::Missing(node.kind().to_owned()), range),
            };

            self.diagnostics.push(error);
        }
    }

    fn eval_expression(&mut self, node: &Node) {
        let child = node.named_child(0);

        if let Some(child) = child {
            let node_type = NodeType::from(&child);
            let return_value = match node.parent() {
                Some(parent) => {
                    NodeType::from(&parent) == NodeType::StmtBlock
                        && parent.named_child(parent.named_child_count() - 1) == Some(*node)
                }
                None => false,
            };
            let unused = match node_type {
                NodeType::ExprBinary => {
                    let operator_node = child.child_by_field_name("operator").unwrap();
                    let operator = operator_node.utf8_text(&self.source).unwrap();

                    operator != "="
                }
                NodeType::ExprUnary | NodeType::ExprLiteral | NodeType::ExprIdentifier => true,
                _ => false,
            };

            if !return_value && unused {
                self.diagnostics
                    .push(warn(WarnKind::UnusedResult, get_node_range(&node)));
                self.diagnostics
                    .push(hint(HintKind::Assign, get_node_range(&node)));
            }
        }
    }

    fn eval_literal(&mut self, node: &Node) {
        let child = node.named_child(0);

        if let Some(child) = child {
            if child.kind() == "string" {
                let start = point_to_position(child.start_position());
                let end = point_to_position(child.end_position());

                if start.line != end.line {
                    self.diagnostics
                        .push(error(ErrorKind::UndelimitedStr, Range::new(start, start)));
                    self.diagnostics
                        .push(error(ErrorKind::UndelimitedStr, Range::new(end, end)))
                }
            }
        }
    }

    fn eval_lambda(&mut self, node: &Node) {
        let (_, args_decl) = self.get_function_args(node);

        for decl in args_decl {
            self.declarations.insert(decl);
        }
    }

    fn eval_var_decl(&mut self, node: &Node) {
        let name_node = node.child_by_field_name("name").unwrap();
        let name = name_node.utf8_text(&self.source).unwrap();
        let name_range = get_node_range(&name_node);
        let kind = DeclarationKind::Variable;

        if KEYWORDS.contains(&name) {
            self.diagnostics
                .push(error(ErrorKind::InvalidName, name_range));
        }

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
                let range = Range::new(
                    point_to_position(node.start_position()),
                    point_to_position(body.start_position()),
                );

                Declaration::new(name.to_owned(), kind, range, name_range, scope, false)
            }
            _ => {
                let range = tsrange_to_lsprange(node.range());

                Declaration::new(name.to_owned(), kind, range, name_range, scope, false)
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
        let name = name_node.utf8_text(&self.source).unwrap();
        let name_range = get_node_range(&name_node);

        if KEYWORDS.contains(&name) {
            self.diagnostics
                .push(error(ErrorKind::InvalidName, name_range));
        }

        let block = node.child_by_field_name("body").unwrap();
        let (names, args_decl) = self.get_function_args(&node);
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

        let decl = Declaration::new(name.to_owned(), kind, range, name_range, scope, false);

        if !self.declarations.insert(decl) {
            self.diagnostics.push(error(
                ErrorKind::Redeclaration(name.to_owned()),
                get_node_range(&name_node),
            ));
        }
    }

    fn eval_identifier(&mut self, node: &Node) {
        if !skip_identifer(node) {
            let name = node.utf8_text(&self.source).unwrap().to_owned();
            let range = get_node_range(node);
            let data = Identifier { name, range };

            self.identifiers.push(data);
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

        for child in iterator.named_children(&mut cursor) {
            let name = child.utf8_text(&self.source).unwrap();
            let kind = DeclarationKind::Variable;
            let range = get_node_range(&iterator);
            let name_range = get_node_range(&child);
            let scope = Some(get_node_range(&body));
            let decl = Declaration::new(name.to_owned(), kind, range, name_range, scope, false);

            self.declarations.insert(decl);
        }
    }

    fn eval_match(&mut self, node: &Node) {
        let body = node.child_by_field_name("body").unwrap();

        if body.named_child_count() == 0 {
            self.diagnostics
                .push(hint(HintKind::EmptyMatch, get_node_range(&node)));
        }
    }

    fn get_function_args(&self, node: &Node) -> (Vec<String>, Vec<Declaration>) {
        let mut names = Vec::new();
        let mut declarations = Vec::new();
        let body = node.child_by_field_name("body").unwrap();
        let args = node.child_by_field_name("args").unwrap();
        let range = get_node_range(&args);
        let mut cursor = Node::walk(&args);

        for arg in args.named_children(&mut cursor) {
            if arg.is_error() {
                continue;
            }

            let name = arg.utf8_text(&self.source).unwrap();
            let name_range = get_node_range(&arg);
            let kind = DeclarationKind::Variable;
            let scope = Some(get_node_range(&body));
            let decl = Declaration::new(name.to_string(), kind, range, name_range, scope, true);

            names.push(name.to_owned());
            declarations.push(decl)
        }

        let kind = DeclarationKind::Variable;
        let scope = Some(get_node_range(&body));
        let decl = Declaration::new("self".to_owned(), kind, *NIL_RANGE, *NIL_RANGE, scope, true);
        declarations.push(decl);

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
        for ident in &self.identifiers {
            let decl = self.declarations.get_mut(&ident);

            if let Some(decl) = decl {
                decl.used = true;
            } else {
                self.diagnostics.push(error(
                    ErrorKind::Undeclared(ident.name.to_owned()),
                    ident.range,
                ));
            }
        }
    }

    fn report_unused(&mut self) {
        for unused in self.declarations.get_unused() {
            if unused.name != "_" && unused.name != "self" {
                self.diagnostics
                    .push(hint(HintKind::Unused(unused.name), unused.name_range))
            }
        }
    }
}

fn skip_identifer(node: &Node) -> bool {
    if node.start_position() == node.end_position() {
        return true;
    }

    if let Some(parent) = node.parent() {
        return match NodeType::from(&parent) {
            NodeType::StmtFuncDecl | NodeType::Iterator => true,
            NodeType::StmtVarDecl | NodeType::Prop => {
                parent.child_by_field_name("name") == Some(*node)
            }
            NodeType::Args => match parent.parent() {
                Some(value) => NodeType::from(&value) != NodeType::ExprCall,
                None => false,
            },
            NodeType::ExprField => parent.child_by_field_name("field") == Some(*node),
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
