use bytes::BytesMut;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::NamedPipeServer;
#[cfg(windows)]
use tokio::net::windows::named_pipe::{PipeMode, ServerOptions};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use tokio::time::Instant;
use tracing::info;

use ipc_poc_backend::image_schema::{ImageRequest, ImageResponse, ImageType, ResponseCode};

// protocol

const PIPE_NAME: &str = if cfg!(windows) {
    r"\\.\pipe\as_ipc_poc_pipe"
} else {
    "/tmp/as_ipc_poc.sock"
};
// const MAX_CONCURRENT_CONNECTIONS: usize = 10;
// const CONNECTION_TIMEOUT_SECS: u64 = 30;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     println!("HIIII");

//     info!("Setting up tracing...");
//     let subscriber = FmtSubscriber::builder()
//         .with_max_level(Level::ERROR)
//         .finish();
//     tracing::subscriber::set_global_default(subscriber)?;

//     info!("Starting IPC POC's backend...");

//     Ok(())
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up tracing...");
    // let subscriber = FmtSubscriber::builder()
    //     .fmt()
    // .with_max_level(Level::ERROR)
    // .finish();
    // tracing::subscriber::set_global_default(subscriber)?;
    // tracing::subscriber
    tracing_subscriber::fmt().init();

    info!("Starting IPC POC's backend...");

    #[cfg(unix)]
    if Path::new(PIPE_NAME).exists() {
        std::fs::remove_file(PIPE_NAME)?;
        info!("Remove existing Unix socket file during startup")
    }

    // let (tx, mut rx) = mpsc::channel::<()>(MAX_CONCURRENT_CONNECTIONS);
    // let connection_semaphore = Arc::new(Mutex::new(tx));

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

// fn cleanup_existing_pipe() -> std::io::Result<()> {
//     if Path::new(PIPE_NAME).exists() {
//         // Try deleting the pipe file (if it exists)
//         std::fs::remove_file(PIPE_NAME)?;
//         info!("Successfully cleaned up existing pipe.");
//     }
//     Ok(())
// }

#[cfg(windows)]
async fn start_windows_server() -> Result<(), Box<dyn std::error::Error>> {
    info!("start_windows_server()...");
    info!("Opening pipe...");

    loop {
        let mut pipe = match ServerOptions::new()
            .reject_remote_clients(true)
            .first_pipe_instance(true)
            .pipe_mode(PipeMode::Message)
            .create(PIPE_NAME)
            // .in_buffer_size(104857600)
            // .out_buffer_size(104857600)
        {
            Err(err) => {
                tracing::error!("Error creating pipe: {}", err);
                return Err(Box::new(err));
            }
            Ok(pipe) => pipe,
        };

        if let Err(e) = pipe.connect().await {
            tracing::error!("Error pip.connect(), error waiting for connection: {}", e);
            return Err(Box::new(e));
        }

        if let Err(e) = handle_connection(&mut pipe).await {
            tracing::error!("Error handle Windows connection: {}", e)
        }

        pipe.disconnect()?;
    }

    Ok(())
}

async fn handle_connection(
    stream: &mut NamedPipeServer,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // let (reader, mut writer) = tokio::io::split(stream);
    info!("handle_connection() about to read from connection...");

    let mut buffer = BytesMut::with_capacity(2048);

    loop {
        info!("New wait loop!");

        let bytes_read = stream.read_buf(&mut buffer).await?;
        if bytes_read == 0 {
            info!("handle_connection(): Client disconnected (read 0 bytes)");
            break;
        }
        info!("handle_connection(): Bytes read: {}", bytes_read);

        loop {
            let mut buf_cursor = std::io::Cursor::new(&buffer);

            match ImageRequest::decode_length_delimited(&mut buf_cursor) {
                Err(err) => {
                    // Maybe be case of insufficient data
                    // or a corrupted message. Prost's DecodeError doesn't have different types of errors.
                    // Just assume more data is needed and break the inner loop \
                    // (Never has a pipe failed on me upto now except when queueing a lot of data into the socket's buffer, in which case: SOCKET_STACK_BUFFER_OVERFLOW something something).
                    info!("Decoding failed (need more data or malformed): {}", err);
                    break;
                }
                Ok(request) => {
                    let start_time = Instant::now();

                    let mut file = tokio::fs::File::open(request.path).await?;
                    info!("Opened the file successfully!");

                    let mut image_bytes = Vec::new();
                    file.read_to_end(&mut image_bytes).await?;

                    let content_length = image_bytes.len() as u32;

                    let image_response = ImageResponse {
                        request_id: request.request_id,
                        response_code: ResponseCode::Success.into(),
                        image_type: ImageType::Jpeg.into(),
                        content_length,
                        data: image_bytes,
                    };

                    let mut response_buffer = Vec::new();
                    ImageResponse::encode_length_delimited(&image_response, &mut response_buffer)?;
                    stream.write(&response_buffer).await?;
                    stream.flush().await?;

                    let duration = start_time.elapsed();
                    println!(
                        "Execution done! Request ID: {}, time: {:?}",
                        request.request_id, duration
                    );

                    buffer.clear();
                }
            }

            if buffer.is_empty() {
                break;
            }
        }
    }

    Ok(())
}
