use ipc_poc_backend::image_schema::{ImageResponse, ImageType, ResponseCode};
use prost::Message;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{PipeMode, ServerOptions};
use tokio::sync::{Mutex, mpsc};
use tokio::task;
use tracing::{error, info};

const PIPE_NAME: &str = r"\\.\pipe\as_ipc_poc_pipe";
const MAX_CONCURRENT_CONNECTIONS: usize = 10;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();
    info!("Starting IPC POC backend...");

    // Request queue with a semaphore-like mechanism
    let (tx, mut rx) = mpsc::channel::<()>(MAX_CONCURRENT_CONNECTIONS);

    let pipe = Arc::new(Mutex::new(
        ServerOptions::new()
            .reject_remote_clients(true)
            .first_pipe_instance(true)
            .pipe_mode(PipeMode::Message)
            .in_buffer_size(104857600)
            .out_buffer_size(104857600)
            .create(PIPE_NAME)?,
    ));

    loop {
        // Limit concurrent requests
        if tx.send(()).await.is_err() {
            error!("Error sending to request queue");
            break;
        }

        let pipe = Arc::clone(&pipe);
        let tx = tx.clone(); // Clone the sender, so each task can release its own slot

        task::spawn(async move {
            if let Err(e) = handle_connection(pipe).await {
                error!("Error handling connection: {}", e);
            }
            // Release the slot after handling the request
            let _ = tx.send(()).await;
        });
    }

    Ok(())
}

async fn handle_connection(
    pipe: Arc<Mutex<tokio::net::windows::named_pipe::NamedPipeServer>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut pipe = pipe.lock().await;

    pipe.connect().await?;
    info!("Client connected!");

    let (reader, mut writer) = tokio::io::split(&mut *pipe);
    let mut reader = BufReader::new(reader);

    let mut requested_file_name = String::new();
    let n = reader.read_line(&mut requested_file_name).await?;
    if n == 0 {
        return Ok(());
    }

    requested_file_name = requested_file_name.trim().to_string();
    info!("Received request for file: {}", requested_file_name);

    let mut file = tokio::fs::File::open(&requested_file_name).await?;
    let mut image_bytes = Vec::new();
    file.read_to_end(&mut image_bytes).await?;

    let response = ImageResponse {
        response_code: ResponseCode::Success.into(),
        image_type: ImageType::Jpeg.into(),
        content_length: image_bytes.len() as u32,
        data: image_bytes,
    };

    let mut response_buffer = Vec::new();
    response.encode(&mut response_buffer)?;

    writer.write_all(&response_buffer).await?;
    writer.flush().await?;

    info!("Response sent for: {}", requested_file_name);

    Ok(())
}
