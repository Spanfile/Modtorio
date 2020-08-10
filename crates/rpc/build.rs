fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/mod_rpc.proto");

    tonic_build::compile_protos("proto/mod_rpc.proto")?;
    Ok(())
}
