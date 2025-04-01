use std::{env, io::Result, path::PathBuf};

fn main() -> Result<()> {
    // let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    prost_build::Config::new()
        .out_dir("src/schema")
        .compile_protos(&["src/proto/image.proto"], &["src/"])?;
    // prost_build::compile_protos(&["src/proto/image.proto"], &["src/"])?;
    Ok(())
}
