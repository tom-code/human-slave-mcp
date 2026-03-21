use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::mpsc,
};

use crate::state::HumanRequest;

pub async fn run_telnet_listener(
    port: u16,
    mut request_rx: mpsc::Receiver<HumanRequest>,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Telnet listener on port {}", port);

    loop {
        let (stream, addr) = listener.accept().await?;
        tracing::info!("Human connected from {}", addr);

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        writer
            .write_all(b"=== slave-mcp: connected ===\r\nWaiting for agent requests...\r\n\r\n")
            .await?;

        loop {
            let request = match request_rx.recv().await {
                Some(req) => req,
                None => {
                    tracing::info!("Request channel closed, shutting down");
                    return Ok(());
                }
            };

            writer.write_all(b"--- Agent Request ---\r\n").await?;
            writer.write_all(request.message.as_bytes()).await?;
            writer
                .write_all(b"\r\n---------------------\r\n> ")
                .await?;
            writer.flush().await?;

            let mut response = String::new();
            match reader.read_line(&mut response).await {
                Ok(0) => {
                    tracing::info!("Human disconnected from {}", addr);
                    drop(request.response_tx);
                    break;
                }
                Ok(_) => {
                    let trimmed = response.trim().to_string();
                    writer.write_all(b"\r\n").await?;
                    let _ = request.response_tx.send(trimmed);
                }
                Err(e) => {
                    tracing::error!("Read error from {}: {}", addr, e);
                    drop(request.response_tx);
                    break;
                }
            }
        }
    }
}
