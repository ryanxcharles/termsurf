//! A temporary directory created on disk that is removed on drop (port of upstream
//! `os/TempDir`).

use std::path::{Path, PathBuf};

use crate::os::file;

/// A temporary directory; removed (with its contents) when dropped.
pub(crate) struct TempDir {
    /// The full path of the created directory.
    path: PathBuf,
    /// The basename of the directory (not the full path).
    name: String,
}

impl TempDir {
    /// Create a fresh temporary directory under the system temp directory (upstream
    /// `TempDir.init`). Loops over random basenames until one can be created.
    pub(crate) fn new() -> std::io::Result<TempDir> {
        let parent = file::tmp_dir();
        loop {
            let name = file::random_basename();
            let mut path = PathBuf::from(&parent);
            path.push(&name);
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(TempDir { path, name }),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err),
            }
        }
    }

    /// The basename of the directory, not the full path (upstream `TempDir.name`).
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The full path of the directory (the Rust handle-equivalent; upstream holds a `Dir`).
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Delete the directory and all its contents (upstream `deinit`). A failure is
        // ignored — `Drop` cannot propagate, and upstream likewise logs-and-continues.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_then_removes_on_drop() {
        let path = {
            let td = TempDir::new().expect("create temp dir");

            // The name is the basename only, of the expected length.
            assert_eq!(td.name().len(), file::RANDOM_BASENAME_LEN);
            // The directory exists and lives under the system temp dir.
            assert!(td.path().is_dir());
            assert!(td.path().starts_with(file::tmp_dir()));
            assert_eq!(
                td.path().file_name().and_then(|n| n.to_str()),
                Some(td.name())
            );

            td.path().to_path_buf()
        };

        // After the TempDir is dropped, the directory no longer exists.
        assert!(!path.exists());
    }

    #[test]
    fn distinct_temp_dirs() {
        let a = TempDir::new().expect("create temp dir a");
        let b = TempDir::new().expect("create temp dir b");
        assert_ne!(a.name(), b.name());
        assert_ne!(a.path(), b.path());
    }
}
