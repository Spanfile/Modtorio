use blake2::Blake2b;
use digest::Digest;
use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();

    let dest_path = Path::new(&out_dir).join("store_consts.rs");
    let schema_path = Path::new(&manifest_dir).join("schema.sql");

    let schema = fs::read_to_string(&schema_path).expect("failed to read schema");
    let checksum = blake2b_string(&schema);

    fs::write(
        &dest_path,
        format!(
            r##"/// The default schema.
const SCHEMA: &str = r#"{}"#;
/// The default schema's BLAKE2b checksum.
const SCHEMA_CHECKSUM: &str = "{}";"##,
            schema, checksum
        ),
    )
    .unwrap();

    println!("cargo:rerun-if-changed={}", schema_path.display());
}

fn blake2b_string(value: &str) -> String {
    let mut hasher = Blake2b::new();
    hasher.update(value);
    let result = hasher.finalize();
    hex::encode(&result[..])
}
