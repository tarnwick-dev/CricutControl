use std::path::PathBuf;

// Compileert transport.proto met de pure-Rust protox-compiler, zodat er geen
// losse protoc-binary op het systeem nodig is.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto = "transport.proto";
    println!("cargo:rerun-if-changed={proto}");

    let file_descriptors = protox::compile([proto], ["."])?;

    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .file_descriptor_set_path(out_dir.join("transport.fds"))
        .compile_fds(file_descriptors)?;

    Ok(())
}
