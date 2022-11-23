fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("http_callback.proto")?;
    Ok(())
}
