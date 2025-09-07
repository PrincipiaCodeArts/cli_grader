#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
pub fn create_dummy_executable() -> PathBuf {
    use tempfile::NamedTempFile;

    let mut file = NamedTempFile::with_suffix(".exe").unwrap();
    file.disable_cleanup(true);
    let path = file.path().to_path_buf();

    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(perms.mode() | 0o111);
        fs::set_permissions(&path, perms).unwrap();
    }
    path
}
