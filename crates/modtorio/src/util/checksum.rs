use blake2::Blake2b;
use digest::Digest;
use sha1::Sha1;
use std::path::Path;

pub fn blake2b_file<P>(path: P) -> anyhow::Result<String>
where
    P: AsRef<Path>,
{
    let mut hasher = Blake2b::new();
    let mut zip = std::fs::File::open(path)?;

    std::io::copy(&mut zip, &mut hasher)?;

    let result = hasher.finalize();
    Ok(hex::encode(&result[..]))
}

pub fn blake2b_string(value: &str) -> String {
    let mut hasher = Blake2b::new();
    hasher.update(value);
    let result = hasher.finalize();
    hex::encode(&result[..])
}

pub fn sha1_file<P>(path: P) -> anyhow::Result<String>
where
    P: AsRef<Path>,
{
    let mut hasher = Sha1::new();
    let mut zip = std::fs::File::open(path)?;

    std::io::copy(&mut zip, &mut hasher)?;

    let result = hasher.finalize();
    Ok(hex::encode(&result[..]))
}
