mod mcp;
mod state;
mod telnet;

use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio::sync::mpsc;

use mcp::HumanBridge;
use state::HumanRequest;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let http_port: u16 = std::env::var("SLAVE_MCP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let telnet_port: u16 = std::env::var("SLAVE_MCP_TELNET_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    let (request_tx, request_rx) = mpsc::channel::<HumanRequest>(1);

    let telnet_handle = tokio::spawn(async move {
        if let Err(e) = telnet::run_telnet_listener(telnet_port, request_rx).await {
            tracing::error!("Telnet listener error: {}", e);
        }
    });

    let tx = request_tx;
    let service = StreamableHttpService::new(
        move || Ok(HumanBridge::new(tx.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

    let router = axum::Router::new().nest_service("/mcp", service);
    let bind_addr = format!("0.0.0.0:{}", http_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("MCP server on http://{}/mcp", bind_addr);
    tracing::info!("Telnet interface on port {}", telnet_port);

    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Shutting down...");
        })
        .await?;

    telnet_handle.abort();
    Ok(())
}
