//! Provides several utilities related filesystem files.

use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
};

/// The world rwx permission bits (007: `------rwx`).
const W_RWX: u32 = 0o7;
/// The group rwx permission bits (070: `---rwx---`).
const G_RWX: u32 = 0o70;
/// The user rwx permission bits (007: `rwx------`).
const U_RWX: u32 = 0o700;

/// Returns whether two given paths point to the same file or directory.
pub fn are_same<P1, P2>(first: P1, second: P2) -> anyhow::Result<bool>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let meta1 = fs::metadata(first)?;
    let meta2 = fs::metadata(second)?;

    Ok((meta1.dev(), meta1.ino()) == (meta2.dev(), meta2.ino()))
}

/// Returns a given file's Unix permission mode.
pub fn get_permissions<P>(path: P) -> anyhow::Result<u32>
where
    P: AsRef<Path>,
{
    let permissions = fs::metadata(path)?.permissions();
    Ok(permissions.mode())
}

/// Sets a given file's Unix permission mode.
pub fn set_permissions<P>(path: P, mode: u32) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_mode(mode);
    fs::set_permissions(&path, permissions)?;
    Ok(())
}

/// Returns whether a given file's Unix permission mode is more-or-equally restrictive as a given
/// maximum permission mode.
pub fn ensure_permission<P>(path: P, max: u32) -> anyhow::Result<bool>
where
    P: AsRef<Path>,
{
    let permissions = fs::metadata(path)?.permissions();
    let mode = permissions.mode();

    Ok(is_higher_or_equal_permission(mode, max))
}

/// Returns whether a given permission mode is more-or-equally restrictive as a given
/// maximum permission mode.
fn is_higher_or_equal_permission(smaller: u32, higher: u32) -> bool {
    higher & W_RWX >= smaller & W_RWX && higher & G_RWX >= smaller & G_RWX && higher & U_RWX >= smaller & U_RWX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_min_file_permissions() {
        assert!(is_higher_or_equal_permission(0o666, 0o666));
        assert!(is_higher_or_equal_permission(0o666, 0o667));
        assert!(is_higher_or_equal_permission(0o666, 0o676));
        assert!(is_higher_or_equal_permission(0o666, 0o766));
    }
}
