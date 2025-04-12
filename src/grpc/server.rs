use crate::image_schema::image_service_server::{ImageService, ImageServiceServer};
use crate::image_schema::{ImageRequest, ImageResponse, ImageType, ResponseCode};
use tokio::io::AsyncReadExt;
use tokio::time::Instant;
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct ImageServer {}

#[tonic::async_trait]
impl ImageService for ImageServer {
    async fn get_image(
        &self,
        request: Request<ImageRequest>,
    ) -> Result<Response<ImageResponse>, Status> {
        let start = Instant::now();

        let actual_request = request.into_inner();

        println!(
            "ImageServer.get_image(): Received a request: {}",
            actual_request.path
        );

        let mut file = tokio::fs::File::open(actual_request.path.trim().to_string()).await?;
        let mut image_bytes = Vec::new();
        file.read_to_end(&mut image_bytes).await?;

        let content_length = image_bytes.len();

        let image_response = ImageResponse {
            response_code: ResponseCode::Success.into(),
            image_type: ImageType::Jpeg.into(),
            data: image_bytes,
            content_length: content_length as u32,
            ..Default::default()
        };

        let response = Response::new(image_response);

        println!(
            "Execution done! Path: {}, time: {:?}",
            actual_request.path,
            start.elapsed()
        );
        Ok(response)
    }
}

pub fn get_image_service_server() -> ImageServiceServer<ImageServer> {
    let image_server = ImageServer::default();

    ImageServiceServer::new(image_server)
}
