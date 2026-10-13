use std::path::PathBuf;

use tower_lsp_server::{
    Client, LanguageServer, LspService, Server, jsonrpc,
    ls_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DidSaveTextDocumentParams, InitializeParams, InitializeResult, InitializedParams,
        MessageType, ServerCapabilities,
    },
};
use tracing::info;
mod ast;
mod lexer;
mod parser;
mod span;

pub use lexer::*;
pub use span::*;

#[derive(Debug, Clone)]
struct Backend {
    client: Client,
}

impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "Initialize Requested")
            .await;
        info!("Initialized Requested");
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Initialized!")
            .await;
        info!("Initialized");
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.client
            .log_message(MessageType::INFO, "Shutdown Requested")
            .await;
        info!("Shutdown Requested");
        Ok(())
    }

    async fn did_open(&self, _params: DidOpenTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "Did open").await;
        info!("Did Open");
    }

    async fn did_change(&self, _params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "Did change")
            .await;
        info!("Did Change");
    }

    async fn did_save(&self, _params: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "Did save").await;
        info!("Did Save");
    }

    async fn did_close(&self, _params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "Did close")
            .await;
        info!("Did Close");
    }
}

fn log_dir() -> PathBuf {
    let base = {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir().map(|home| home.join("Library/Logs"))
        }

        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir()
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            dirs::state_dir()
        }
    };
    let app = "vl_lsp";
    base.map_or_else(|| std::env::temp_dir().join(app), |d| d.join(app))
}

#[tokio::main]
pub async fn main() {
    let writer = tracing_appender::rolling::never(log_dir(), "log.log");
    // let (non_blocking_writer, _guard) = tracing_appender::non_blocking(writer);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_max_level(tracing::Level::ERROR)
        .init();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    info!("Started");
    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
    info!("Stopped");
}
