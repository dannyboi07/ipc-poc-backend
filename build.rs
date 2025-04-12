use std::{env, path::Path};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let builder = tonic_build::configure();

    builder
        .build_client(false)
        .build_server(true)
        .build_transport(false)
        .out_dir(Path::new(&out_dir));

    Ok(())
}
