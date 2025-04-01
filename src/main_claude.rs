// src/main.rs
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

// Platform-specific imports
#[cfg(windows)]
use tokio::net::windows::named_pipe::{ClientOptions, ServerOptions};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

// Protocol module
mod protocol;
use protocol::{Message, MessageError, MessageType};

// Configuration
const PIPE_NAME: &str = if cfg!(windows) {
    r"\\.\pipe\photomanager_pipe"
} else {
    "/tmp/photomanager.sock"
};
const MAX_CONCURRENT_CONNECTIONS: usize = 10;
const CONNECTION_TIMEOUT_SECS: u64 = 30;

// Image processing module (would be your actual business logic)
mod image_processor {
    use image::{ImageBuffer, Rgb};
    
    // This would be replaced with your actual image processing logic
    pub async fn process_image(data: &[u8]) -> Result<Vec<u8>, String> {
        // Placeholder for actual image processing
        // In a real app, you'd use your actual processing logic here
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Return processed data (simplified example)
        Ok(data.to_vec())
    }
    
    pub async fn get_metadata(data: &[u8]) -> Result<serde_json::Value, String> {
        // Simulate metadata extraction
        let metadata = serde_json::json!({
            "width": 1920,
            "height": 1080,
            "format": "JPEG",
            "size_bytes": data.len(),
        });
        
        Ok(metadata)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting PhotoManager IPC server");
    
    // Clean up any existing socket file on Unix systems
    #[cfg(unix)]
    if Path::new(PIPE_NAME).exists() {
        std::fs::remove_file(PIPE_NAME)?;
        info!("Removed existing Unix socket file");
    }
    
    // Use a channel to limit concurrent connections
    let (tx, mut rx) = mpsc::channel::<()>(MAX_CONCURRENT_CONNECTIONS);
    let connection_semaphore = Arc::new(Mutex::new(tx));
    
    // Platform-specific server setup
    #[cfg(windows)]
    {
        info!("Setting up Windows named pipe server at {}", PIPE_NAME);
        windows_server_loop(connection_semaphore, &mut rx).await?;
    }
    
    #[cfg(unix)]
    {
        info!("Setting up Unix domain socket server at {}", PIPE_NAME);
        unix_server_loop(connection_semaphore, &mut rx).await?;
    }
    
    Ok(())
}

#[cfg(windows)]
async fn windows_server_loop(
    semaphore: Arc<Mutex<mpsc::Sender<()>>>,
    rx: &mut mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let pipe = ServerOptions::new()
            .first_pipe_instance(true)
            .create(PIPE_NAME)?;
        
        info!("Waiting for client connection on Windows named pipe");
        
        // Wait for client connection
        match time::timeout(Duration::from_secs(5), pipe.connect()).await {
            Ok(result) => {
                result?;
                info!("Client connected to Windows named pipe");
                
                // Get a permit from the semaphore
                let permit_tx = semaphore.lock().await.clone();
                let permit = permit_tx.send(()).await.map_err(|e| {
                    format!("Failed to acquire connection permit: {}", e)
                })?;
                
                // Spawn a task to handle this connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(pipe).await {
                        error!("Error handling Windows client: {}", e);
                    }
                    
                    // Permit is automatically dropped when task ends
                    drop(permit);
                });
            }
            Err(_) => {
                // No connection within timeout, continue the loop
                debug!("Connection timeout, recreating pipe");
                continue;
            }
        }
        
        // Wait for a connection slot to become available
        rx.recv().await;
    }
}

#[cfg(unix)]
async fn unix_server_loop(
    semaphore: Arc<Mutex<mpsc::Sender<()>>>,
    rx: &mut mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = UnixListener::bind(PIPE_NAME)?;
    info!("Unix domain socket server listening at {}", PIPE_NAME);
    
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                info!("Client connected to Unix domain socket");
                
                // Get a permit from the semaphore
                let permit_tx = semaphore.lock().await.clone();
                let permit = permit_tx.send(()).await.map_err(|e| {
                    format!("Failed to acquire connection permit: {}", e)
                })?;
                
                // Spawn a task to handle this connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream).await {
                        error!("Error handling Unix client: {}", e);
                    }
                    
                    // Permit is automatically dropped when task ends
                    drop(permit);
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
                // Brief pause to avoid CPU spinning on repeated errors
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        
        // Wait for a connection slot to become available
        rx.recv().await;
    }
}

async fn handle_connection<T>(stream: T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
{
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    
    // Set up heartbeat for connection monitoring
    let (heartbeat_tx, mut heartbeat_rx) = mpsc::channel::<()>(1);
    let heartbeat_handle = tokio::spawn(async move {
        loop {
            match time::timeout(Duration::from_secs(CONNECTION_TIMEOUT_SECS), heartbeat_rx.recv()).await {
                // Got heartbeat, continue
                Ok(Some(())) => {},
                // Channel closed, client disconnected normally
                Ok(None) => break,
                // Timeout reached, client is unresponsive
                Err(_) => {
                    warn!("Client connection timed out");
                    break;
                }
            }
        }
    });
    
    // Process messages
    loop {
        // Send heartbeat to reset timeout
        let _ = heartbeat_tx.send(()).await;
        
        // Read message
        match time::timeout(Duration::from_secs(5), protocol::read_message(&mut reader)).await {
            Ok(Ok(message)) => {
                // Process the message
                match process_message(message).await {
                    Ok(response) => {
                        // Send response
                        if let Err(e) = protocol::write_message(&mut writer, response).await {
                            error!("Failed to write response: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        // Send error response
                        error!("Error processing message: {}", e);
                        let error_msg = Message {
                            msg_type: MessageType::Error,
                            payload: e.to_string().into_bytes(),
                        };
                        
                        if let Err(e) = protocol::write_message(&mut writer, error_msg).await {
                            error!("Failed to write error response: {}", e);
                            break;
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                // Protocol error
                error!("Protocol error: {}", e);
                
                // Try to send error message back
                let error_msg = Message {
                    msg_type: MessageType::Error,
                    payload: format!("Protocol error: {}", e).into_bytes(),
                };
                
                let _ = protocol::write_message(&mut writer, error_msg).await;
                break;
            }
            Err(_) => {
                // Read timeout
                debug!("Read timeout, client may be idle");
                continue;
            }
        }
    }
    
    // Cancel heartbeat monitoring
    heartbeat_handle.abort();
    info!("Client disconnected");
    
    Ok(())
}

async fn process_message(message: Message) -> Result<Message, String> {
    match message.msg_type {
        MessageType::Command => {
            // Parse command JSON
            let command_str = String::from_utf8(message.payload)
                .map_err(|e| format!("Invalid UTF-8 in command: {}", e))?;
            
            let command: serde_json::Value = serde_json::from_str(&command_str)
                .map_err(|e| format!("Invalid JSON in command: {}", e))?;
            
            // Dispatch command
            process_command(command).await
        }
        
        MessageType::ImageData => {
            // Process image data
            info!("Processing image data of {} bytes", message.payload.len());
            let processed_data = image_processor::process_image(&message.payload).await?;
            
            // Get metadata
            let metadata = image_processor::get_metadata(&message.payload).await?;
            
            // Create response with metadata
            let response = serde_json::json!({
                "status": "success",
                "processingTime": "10ms", // In a real app, measure actual time
                "metadata": metadata,
                "previewAvailable": true
            });
            
            Ok(Message {
                msg_type: MessageType::Response,
                payload: serde_json::to_vec(&response).map_err(|e| e.to_string())?,
            })
        }
        
        MessageType::Heartbeat => {
            // Respond to heartbeat
            Ok(Message {
                msg_type: MessageType::Heartbeat,
                payload: vec![],
            })
        }
        
        _ => {
            // Unsupported message type
            Err(format!("Unsupported message type: {:?}", message.msg_type))
        }
    }
}

async fn process_command(command: serde_json::Value) -> Result<Message, String> {
    // Extract command name
    let cmd_name = command["command"].as_str()
        .ok_or("Missing 'command' field in command message")?;
    
    info!("Processing command: {}", cmd_name);
    
    // Dispatch based on command name
    match cmd_name {
        "listImages" => {
            // Example command implementation
            let directory = command["params"]["directory"].as_str()
                .ok_or("Missing directory parameter")?;
            
            info!("Listing images in directory: {}", directory);
            
            // Simulate directory listing
            let result = serde_json::json!({
                "images": [
                    {"filename": "img1.jpg", "size": 1024000},
                    {"filename": "img2.jpg", "size": 2048000},
                ]
            });
            
            Ok(Message {
                msg_type: MessageType::Response,
                payload: serde_json::to_vec(&result).map_err(|e| e.to_string())?,
            })
        }
        
        "getImageMetadata" => {
            // Example metadata command
            let filename = command["params"]["filename"].as_str()
                .ok_or("Missing filename parameter")?;
            
            info!("Getting metadata for image: {}", filename);
            
            // Simulate metadata retrieval
            let metadata = serde_json::json!({
                "filename": filename,
                "width": 1920,
                "height": 1080,
                "format": "JPEG",
                "created": "2023-03-15T10:30:00Z"
            });
            
            Ok(Message {
                msg_type: MessageType::Response,
                payload: serde_json::to_vec(&metadata).map_err(|e| e.to_string())?,
            })
        }
        
        "prepareImageProcessing" => {
            // Example command to prepare for image processing
            let filename = command["params"]["filename"].as_str()
                .ok_or("Missing filename parameter")?;
            let file_type = command["params"]["type"].as_str()
                .ok_or("Missing type parameter")?;
            
            info!("Preparing to process image: {} ({})", filename, file_type);
            
            // Respond with ready status
            let response = serde_json::json!({
                "status": "ready",
                "message": format!("Ready to receive image data for {}", filename)
            });
            
            Ok(Message {
                msg_type: MessageType::Response,
                payload: serde_json::to_vec(&response).map_err(|e| e.to_string())?,
            })
        }
        
        _ => {
            // Unknown command
            Err(format!("Unknown command: {}", cmd_name))
        }
    }
}