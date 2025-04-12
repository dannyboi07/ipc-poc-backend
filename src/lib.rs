pub mod image_schema {
    include!(concat!(env!("OUT_DIR"), "/image.rs"));
}

pub mod grpc;
