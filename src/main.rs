use prost::Message;
use tokio::io::AsyncBufReadExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::time::Instant;
use tracing::info;

use ipc_poc_backend::image_schema::{ImageResponse, ImageType, ResponseCode};

// protocol

const PIPE_NAME: &str = if cfg!(windows) {
    r"\\.\pipe\as_ipc_poc_pipe"
} else {
    "/tmp/as_ipc_poc.sock"
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up tracing...");
    tracing_subscriber::fmt().init();

    info!("Starting IPC POC's backend...");

    #[cfg(unix)]
    if Path::new(PIPE_NAME).exists() {
        std::fs::remove_file(PIPE_NAME)?;
        info!("Remove existing Unix socket file during startup")
    }

    #[cfg(windows)]
    {
        info!("Setting up Windows Named Pipe server at {}", PIPE_NAME);
        if let Err(e) = start_windows_server().await {
            tracing::error!("Error starting windows server: {}", e)
        }
    }

    #[cfg(unix)]
    {
        info!("Setting up Unix domain socket server at {}", PIPE_NAME);
        // unix_server_loop()
    }

    Ok(())
}

#[cfg(windows)]
async fn start_windows_server() -> Result<(), Box<dyn std::error::Error>> {
    use ipc_poc_backend::grpc::{self, named_pipe_stream::TonicNamedPipeServer};
    use tonic::transport::Server;

    info!("start_windows_server()...");

    let image_service = grpc::server::get_image_service_server();

    Server::builder()
        .add_service(image_service)
        .serve_with_incoming(TonicNamedPipeServer::get_named_pipe_server_stream(
            "as_ipc_poc_pipe",
        ))
        .await?;

    Ok(())
}
