use tower_lsp_server::{LspService, Server};
use vl_lsp::{log_dir, server};

#[tokio::main]
pub async fn main() {
    let log_dir = log_dir();

    // Clear log on startup. Makes testing each iteration of the app easier.
    #[cfg(debug_assertions)]
    {
        _ = std::fs::File::create(log_dir.join("log.log"));
    }
    let writer = tracing_appender::rolling::never(log_dir, "log.log");
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(writer);
    tracing_subscriber::fmt()
        .with_writer(non_blocking_writer)
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(server::Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
