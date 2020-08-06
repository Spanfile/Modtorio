use std::{fs, os::unix::fs::PermissionsExt, path::Path};

const W_RWX: u32 = 0o7;
const G_RWX: u32 = 0o70;
const U_RWX: u32 = 0o700;

pub fn get_permissions<P>(path: P) -> anyhow::Result<u32>
where
    P: AsRef<Path>,
{
    let permissions = fs::metadata(path)?.permissions();
    Ok(permissions.mode())
}

pub fn set_permissions<P>(path: P, mode: u32) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_mode(mode);
    fs::set_permissions(&path, permissions)?;
    Ok(())
}

pub fn ensure_permission<P>(path: P, max: u32) -> anyhow::Result<bool>
where
    P: AsRef<Path>,
{
    let permissions = fs::metadata(path)?.permissions();
    let mode = permissions.mode();

    Ok(is_higher_or_equal_permission(mode, max))
}

fn is_higher_or_equal_permission(smaller: u32, higher: u32) -> bool {
    higher & W_RWX >= smaller & W_RWX
        && higher & G_RWX >= smaller & G_RWX
        && higher & U_RWX >= smaller & U_RWX
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
