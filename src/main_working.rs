use std::path::Path;

use prost::Message;
use tokio::io::AsyncBufReadExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
        // info!("Setting up Windows Named Pipe server at {}", PIPE_NAME);
        // if let Err(e) = start_windows_server(connection_semaphore, &mut rx).await {
        //     tracing::error!("Error starting windows server: {}", e)
        // }
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
async fn start_windows_server(// semaphore: Arc<Mutex<mpsc::Sender<()>>>,
    // rx: &mut mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    // use tokio::time::Instant;

    let mut should_be_first_pipe_instance = true;

    info!("start_windows_server()...");
    loop {
        // if let Err(e) = cleanup_existing_pipe() {
        //     tracing::error!("Error cleaning up existing pipe: {}", e);
        //     return Err(Box::new(e));
        // }

        info!("Opening pipe...");
        let pipe = match ServerOptions::new()
            .reject_remote_clients(true)
            .first_pipe_instance(should_be_first_pipe_instance)
            .pipe_mode(PipeMode::Message)
            .in_buffer_size(104857600)
            .out_buffer_size(104857600)
            .create(PIPE_NAME)
        {
            Err(err) => {
                tracing::error!("Error creating pipe: {}", err);
                return Err(Box::new(err));
            }
            Ok(pipe) => pipe,
        };

        should_be_first_pipe_instance = false;

        if let Err(e) = pipe.connect().await {
            tracing::error!("Error pip.connect(), error waiting for connection: {}", e);
            return Err(Box::new(e));
        }
        let named_pipe_server = pipe;

        info!(
            "Spawning task to handle_connection()... {}",
            should_be_first_pipe_instance
        );
        // tokio::spawn(handle_connection(named_pipe_server));
        tokio::spawn(async move {
            if let Err(e) = handle_connection(named_pipe_server).await {
                tracing::error!("Error handle Windows connection: {}", e)
            }
        });
    }
}

async fn handle_connection<T>(stream: T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
{
    let start = Instant::now();

    let (reader, mut writer) = tokio::io::split(stream);
    info!("handle_connection() about to read from connection...");

    let mut reader = tokio::io::BufReader::new(reader);

    let mut requested_file_name = String::new();
    let n = reader.read_line(&mut requested_file_name).await?;
    requested_file_name = requested_file_name.trim().to_string();

    // let image_request = match ImageRequest::decode(&buffer[..]) {
    //     Ok(req) => req,
    //     Err(err) => return Err(Box::new(err)),
    // };

    // let n = reader.read_exact(&mut buffer).await?;
    // requested_file_name =

    // WARN Lossy conversion
    // let requested_file_name = String::from_utf8_lossy(&buffer[..n]).trim().to_string();

    // let image_path = Path::new(s)

    println!(
        "handle_connection(), number of bytes read: {}, requested file :{}",
        n, requested_file_name,
    );

    // let image_reader = ImageReader::open(requested_file_name)?;
    // let image = image_reader.decode()?;
    // let image_bytes = image.as_bytes();

    // let mut file = File::open(&requested_file_name)?;
    let mut file = tokio::fs::File::open(&requested_file_name).await?;
    let mut image_bytes = Vec::new();
    file.read_to_end(&mut image_bytes).await?;

    // let mime_type = mime_guess::from_path(requested_file_name)
    //     .first_or(mime::IMAGE_JPEG)
    //     .to_string();

    let content_length = image_bytes.len() as u32;

    let image_response = ImageResponse {
        response_code: ResponseCode::Success.into(),
        image_type: ImageType::Jpeg.into(),
        content_length: content_length,
        data: image_bytes,
    };

    // writer.write_all(b"Hello the server has received the request, here's a response until the server is implemented").await?;

    let mut response_buffer = Vec::new();
    image_response.encode(&mut response_buffer)?;

    // let test_response = ImageResponse::decode(&response_buffer[..])?;
    // println!("Test sreponse: {:?}", test_response);

    writer.write_all(&response_buffer).await?;
    writer.flush().await?;

    let duration = start.elapsed();
    println!(
        "Execution done! Path: {}, time: {:?}",
        requested_file_name, duration
    );

    Ok(())
}

// async fn handle_connection_new<T>(stream: T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
// where
//     T: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
// {
//     let (reader, mut writer) = tokio::io::split(stream);
//     let mut reader = tokio::io::BufReader::new(reader);

//     loop {
//         let mut requested_file_name = String::new();

//         // Read request
//         let bytes_read = reader.read_line(&mut requested_file_name).await?;
//         if bytes_read == 0 {
//             println!("Client disconnected.");
//             break;
//         }

//         requested_file_name = requested_file_name.trim().to_string();
//         println!("Received request: {}", requested_file_name);

//         let mut file = match tokio::fs::File::open(&requested_file_name).await {
//             Ok(f) => f,
//             Err(e) => {
//                 eprintln!("Error opening file {}: {}", requested_file_name, e);
//                 continue;
//             }
//         };

//         let mut image_bytes = Vec::new();
//         tokio::io::copy(&mut file, &mut image_bytes).await?;

//         let content_length = image_bytes.len() as u32;

//         let image_response = ImageResponse {
//             response_code: ResponseCode::Success.into(),
//             image_type: ImageType::Jpeg.into(),
//             content_length,
//             data: image_bytes,
//         };

//         let mut response_buffer = Vec::new();
//         image_response.encode(&mut response_buffer)?;

//         // Write response
//         writer.write_all(&response_buffer).await?;
//         writer.flush().await?;

//         println!("Response sent for: {}", requested_file_name);
//     }

//     Ok(())
// }

// image_name_image_id?thumb_size=mid&force_libraw=false

// flip=1&thumb_size=Low&force_libraw=false&jpeg_sidecar=true
// &global_id=95af5d9c-c0ec-4da6-a6cf-52a55823a895&path=QzpcVXNlcnNcZGFuaWVcd29ya1xBbGJ1bVwyOTgtMjk4IER1cGxpY2F0ZVwxMiBqcGVncyAyXERTQzA0NzQ3LmpwZ
