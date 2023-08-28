use dashmap::DashMap;
use tower_lsp::{jsonrpc::Result, lsp_types::*, Client, LanguageServer};

use crate::{
    analyzer::analyze, builtins::KEYWORDS, declarations::DeclarationKind, document::Document,
};

pub struct Backend {
    client: Client,
    document_map: DashMap<String, Document>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            document_map: DashMap::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            offset_encoding: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;

        if let Some(mut document) = Document::from_params(params) {
            self.client
                .log_message(MessageType::INFO, "file opened!")
                .await;

            let content = &document.content.as_bytes();
            let tree = &document.tree;
            let result = analyze(content, tree);

            document.declarations = result.declarations;

            self.document_map.insert(uri.to_string(), document);
            self.client
                .publish_diagnostics(uri, result.diagnostics, Some(version))
                .await;
        } else {
            self.client
                .log_message(MessageType::ERROR, "'textDocument/didOpen' failed")
                .await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;

        if let Some(mut document) = self.document_map.get_mut(&uri.to_string()) {
            document.did_change(params);

            let content = &document.content.as_bytes();
            let tree = &document.tree;
            let result = analyze(content, tree);

            document.declarations = result.declarations;

            self.client
                .publish_diagnostics(uri, result.diagnostics, Some(version))
                .await;
        }
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file saved!")
            .await;
    }
    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file closed!")
            .await;
    }

    // TODO: handle variables state & field completion
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut completions = vec![];
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        for keyword in KEYWORDS {
            completions.push(CompletionItem {
                label: keyword.to_owned(),
                insert_text: Some(keyword.to_owned()),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }
        
        // FIXME: use symbol table
        if let Some(document) = self.document_map.get(&uri.to_string()) {
            for decl in document.declarations.get_declared_at(position) {
                let kind = match decl.kind {
                    DeclarationKind::Variable => CompletionItemKind::VARIABLE,
                    DeclarationKind::Function(_) => CompletionItemKind::FUNCTION,
                };
                let detail = Some(decl.get_details());

                completions.push(CompletionItem {
                    label: decl.name.clone(),
                    insert_text: Some(decl.name),
                    kind: Some(kind),
                    detail,
                    documentation: decl.doc,
                    ..Default::default()
                });
            }
        }

        Ok(Some(completions).map(CompletionResponse::Array))
    }

    async fn goto_definition(&self, _: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        todo!()
    }

    async fn references(&self, _: ReferenceParams) -> Result<Option<Vec<Location>>> {
        todo!()
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        todo!()
    }

    async fn rename(&self, _: RenameParams) -> Result<Option<WorkspaceEdit>> {
        todo!()
    }
}
