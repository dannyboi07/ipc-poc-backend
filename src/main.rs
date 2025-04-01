use std::path::Path;

use image::EncodableLayout;
use prost::Message;
use tokio::io::AsyncBufReadExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::NamedPipeServer;
#[cfg(windows)]
use tokio::net::windows::named_pipe::{PipeMode, ServerOptions};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
use tokio::time::Instant;
use tracing::info;

use ipc_poc_backend::image_schema::{ImageResponse, ImageType, ResponseCode};

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
    // use tokio::time::Instant;

    // let mut should_be_first_pipe_instance = true;

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

// where
//     T: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
async fn handle_connection(
    stream: &mut NamedPipeServer,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = tokio::io::split(stream);
    info!("handle_connection() about to read from connection...");

    let mut reader = tokio::io::BufReader::new(reader);

    loop {
        info!("New wait loop!");
        let mut requested_file_name = String::new();
        let n = reader.read_line(&mut requested_file_name).await?;
        if n == 0 {
            println!("Oh no! Anyways...");
            break;
        }

        // let collected: Vec<String> = requested_file_name.split(":").collect();
        // requested_file_name = requested_file_name.trim().to_string();
        let start = Instant::now();
        let (request_id, file_name) = requested_file_name.split_once(":").unwrap();

        info!(
            "handle_connection(), number of bytes read: {}, request id: {}, requested file: {}",
            n, request_id, file_name,
        );
        let mut file = tokio::fs::File::open(file_name.trim().to_string()).await?;
        info!("Opened the file successfully!");

        let mut image_bytes = Vec::new();
        file.read_to_end(&mut image_bytes).await?;

        let content_length = image_bytes.len() as u32;

        let image_response = ImageResponse {
            request_id: request_id.parse::<u32>().unwrap(),
            response_code: ResponseCode::Success.into(),
            image_type: ImageType::Jpeg.into(),
            content_length: content_length,
            data: image_bytes,
        };

        let mut response_buffer = Vec::new();
        // image_response.encode(&mut response_buffer)?;

        // stream.write_all(&response_buffer).await?;
        // stream.flush().await?;
        ImageResponse::encode_length_delimited(&image_response, &mut response_buffer)?;
        // info!("Image buffer: {:?}", response_buffer,);
        // info!(
        //     "Image buffer: {:?}, \n\n\n\n\n\n\n\n\n{:?}",
        //     response_buffer,
        // ImageResponse::decode_length_delimited(response_buffer.as_bytes())?.len
        // );
        writer.write(&response_buffer).await?;
        writer.flush().await?;

        let duration = start.elapsed();
        println!(
            "Execution done! Path: {}, time: {:?}",
            requested_file_name, duration
        );
    }

    Ok(())
}
