use std::path::PathBuf;
// use tokio
// use interprocess::local_socket::{Sock}
// #[cfg(windows)]
// use interprocess::os::windows::named_pipe::{PipeListenerOptions, PipeStream};
// // #[cfg(unix)]
// use interprocess::os::{
//     unix::local_socket::{LocalSocketListener, LocalSocketStream},
//     windows::named_pipe::PipeMode,
// };

enum ImageFormat {
    Jpeg,
    Png,
}

// fn get_ipc_path() -> PathBuf {
//     if cfg!(windows) {
//         PathBuf::from(r"\\.\pipe\as_ipc_poc")
//     } else {
//         PathBuf::from("/tmp/as_ipc_poc.sock")
//     }
// }

// async fn create_ipc_listener() {
//     if cfg!(windows) {
//         let pipe_name = r"\\.\pip\as_ipc_poc";
//         let listener = PipeListenerOptions::new()
//             .mode(PipeMode::Bytes)
//             .create()?;
//         listener.
//     }
// }

// async fn handle_request(stream: LocalSocketStream) -> Result<()> {}

fn main() {
    println!("Hello, world!");
}

async fn handle_connection<T>(stream: T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: AsyncReadExt + AsyncWriteExt + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    // let mut string_result = String::new();
    // reader.read_to_string(&mut string_result).await?;

    // let mut buffer = vec![0; 1024];
    // let n = stream.read(&mut buffer).await?;
    // let mut requested_file_params = String::new();
    info!("handle_connection() about to read from connection...");

    // ipc_poc_backend::image_schema::ImageRequest::decode(buf)

    let mut reader = tokio::io::BufReader::new(reader);

    // let mut buffer = Vec::new();
    let mut requested_file_params = String::new();
    let n = reader.read_line(&mut requested_file_params).await?;

    // let image_request = match ImageRequest::decode(&buffer[..]) {
    //     Ok(req) => req,
    //     Err(err) => return Err(Box::new(err)),
    // };

    // let n = reader.read_exact(&mut buffer).await?;
    // requested_file_params =

    // WARN Lossy conversion
    // let requested_file_params = String::from_utf8_lossy(&buffer[..n]).trim().to_string();

    // let image_path = Path::new(s)

    println!(
        "handle_connection(), number of bytes read: {}, requested file :{}",
        n, requested_file_params,
    );
    // println!("handle_connection(), requeste: {:?}", image_request);

    writer.write_all(b"Hello the server has received the request, here's a response until the server is implemented").await?;
    writer.flush().await?;

    Ok(())
}
