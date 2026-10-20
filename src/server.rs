use dashmap::DashMap;
use tower_lsp_server::{
    Client, LanguageServer, jsonrpc,
    ls_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DidSaveTextDocumentParams, InitializeParams, InitializeResult, InitializedParams,
        MessageType, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
    },
};

use crate::document::Document;

#[derive(Debug, Clone)]
pub struct Backend {
    client: Client,
    documents: DashMap<Uri, Document>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    async fn log_debug<S: Into<String>>(&self, message: S, path: Option<&str>) {
        #[cfg(debug_assertions)]
        {
            let mut message = message.into();
            if let Some(tail) = path {
                message.push(' ');
                message.push_str(tail);
            }
            tracing::debug!(message);
        }
    }

    async fn log_info<S: AsRef<str>>(&self, message: S) {
        let message = message.as_ref();
        tracing::info!(message);
        self.client.log_message(MessageType::INFO, message).await;
    }

    async fn log_warn<S: AsRef<str>>(&self, message: S) {
        let message = message.as_ref();
        tracing::warn!(message);
        self.client.log_message(MessageType::WARNING, message).await;
    }

    async fn log_err<S: AsRef<str>>(&self, message: S) {
        let message = message.as_ref();
        tracing::error!(message);
        self.client.log_message(MessageType::ERROR, message).await;
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        self.log_debug("Initialize Requested", None).await;
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.log_info("Server Initialized").await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.log_debug("Shutdown Requested", None).await;
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.log_info(format!("Opened {:?}", params.text_document.uri.path()))
            .await;
        self.documents.insert(
            params.text_document.uri,
            Document::new(params.text_document.text),
        );
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        {
            self.log_debug("Changed", Some(params.text_document.uri.as_str()))
                .await;
        }
        let Some(mut doc) = self.documents.get_mut(&params.text_document.uri) else {
            self.log_err(format!(
                "{:?} changed without opening",
                params.text_document.uri.path()
            ))
            .await;
            return;
        };
        if let Some(text) = params.content_changes.last() {
            doc.update(text.text.clone());
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        #[cfg(debug_assertions)]
        {
            self.log_debug(format!("Saved: {params:?}"), None).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.log_info(format!("Closed {:?}", params.text_document.uri.path()))
            .await;
        self.documents.remove(&params.text_document.uri);
    }
}
