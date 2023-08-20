use std::collections::HashMap;

use tower_lsp::lsp_types::{DidChangeTextDocumentParams, DidOpenTextDocumentParams};
use tree_sitter::{Parser, Tree};

use crate::declarations::Declaration;

pub struct Document {
    pub content: String,
    pub tree: Tree,
    pub parser: Parser,
    pub declarations: HashMap<String, Declaration>,
}

impl Document {
    pub fn from_params(params: DidOpenTextDocumentParams) -> Option<Self> {
        let content = params.text_document.text;
        let mut parser = Parser::new();

        parser
            .set_language(tree_sitter_icelang::language())
            .expect("Error loading icelang grammar");

        let tree = parser.parse(&content, None)?;
        let declarations = HashMap::new();

        Some(Self {
            content,
            tree,
            parser,
            declarations,
        })
    }

    pub fn did_change(&mut self, params: DidChangeTextDocumentParams) {
        let changes = &params.content_changes[0];
        let text = changes.text.clone();

        // FIXME: edit old tree
        self.tree = self.parser.parse(&text, None).unwrap();
        self.content = text;
    }
}
