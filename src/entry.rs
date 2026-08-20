use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
    pub is_broken_symlink: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub readonly: bool,
    #[cfg(unix)]
    pub mode: u32,
    #[cfg(unix)]
    pub uid: u32,
    #[cfg(unix)]
    pub gid: u32,
}

impl FileEntry {
    /// Build a `FileEntry` from a directory-listing path. `symlink_metadata`
    /// is used first so that broken symlinks are still listed instead of
    /// causing an error.
    pub fn from_path(path: &Path) -> std::io::Result<FileEntry> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let lstat = fs::symlink_metadata(path)?;
        let is_symlink = lstat.file_type().is_symlink();

        let (target, broken) = if is_symlink {
            match fs::read_link(path) {
                Ok(target) => {
                    let resolved = if target.is_relative() {
                        path.parent()
                            .unwrap_or_else(|| Path::new("."))
                            .join(&target)
                    } else {
                        target.clone()
                    };
                    let broken = fs::metadata(resolved).is_err();
                    (Some(target), broken)
                }
                Err(_) => (None, true),
            }
        } else {
            (None, false)
        };

        // Prefer metadata following the symlink (so directories-that-are-
        // symlinks are recognised as such); fall back to lstat if the link
        // is broken.
        let stat = fs::metadata(path).unwrap_or(lstat.clone());

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            use std::os::unix::fs::PermissionsExt;
            Ok(FileEntry {
                path: path.to_path_buf(),
                name,
                is_dir: stat.is_dir(),
                is_symlink,
                symlink_target: target,
                is_broken_symlink: broken,
                size: stat.len(),
                modified: stat.modified().ok(),
                readonly: stat.permissions().readonly(),
                mode: lstat.permissions().mode(),
                uid: stat.uid(),
                gid: stat.gid(),
            })
        }

        #[cfg(not(unix))]
        {
            Ok(FileEntry {
                path: path.to_path_buf(),
                name,
                is_dir: stat.is_dir(),
                is_symlink,
                symlink_target: target,
                is_broken_symlink: broken,
                size: stat.len(),
                modified: stat.modified().ok(),
                readonly: stat.permissions().readonly(),
            })
        }
    }

    pub fn extension(&self) -> Option<String> {
        self.path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
    }

    pub fn is_hidden(&self) -> bool {
        self.name.starts_with('.')
    }

    pub fn is_executable(&self) -> bool {
        #[cfg(unix)]
        {
            self.mode & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            matches!(
                self.extension().as_deref(),
                Some("exe") | Some("bat") | Some("cmd") | Some("msi") | Some("ps1")
            )
        }
    }
}

/// Build the synthetic `.` and `..` entries shown by `-a` (but not `-A`),
/// mirroring classic `ls -a` semantics.
pub fn dot_entries(dir: &Path) -> std::io::Result<[FileEntry; 2]> {
    let mut here = FileEntry::from_path(dir)?;
    here.name = ".".to_string();

    // `Path::parent()` on "." yields `Some("")` (an empty, unusable path)
    // rather than `None`, so resolve via the absolute path instead of
    // relying on `parent()` directly on a possibly-relative `dir`.
    let parent = std::fs::canonicalize(dir)
        .ok()
        .and_then(|abs| abs.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| dir.to_path_buf());
    let mut up = FileEntry::from_path(&parent)?;
    up.name = "..".to_string();

    Ok([here, up])
}

/// Read a directory's immediate children as `FileEntry`s, silently skipping
/// entries that vanish between the readdir and the stat call (a normal race
/// on any real filesystem) but surfacing every other I/O error.
pub fn read_dir_entries(dir: &Path) -> std::io::Result<Vec<FileEntry>> {
    let mut out = Vec::new();
    for item in fs::read_dir(dir)? {
        let item = item?;
        match FileEntry::from_path(&item.path()) {
            Ok(entry) => out.push(entry),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extension_lowercased() {
        let dir = std::env::temp_dir().join(format!("colorls_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("Example.RS");
        std::fs::File::create(&file_path)
            .unwrap()
            .write_all(b"fn main(){}")
            .unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        assert_eq!(entry.extension().as_deref(), Some("rs"));
        assert!(!entry.is_dir);
        assert!(!entry.is_hidden());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn hidden_file_detected() {
        let dir = std::env::temp_dir().join(format!("colorls_test_hidden_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join(".env");
        std::fs::File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        assert!(entry.is_hidden());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dot_entries_resolve_from_relative_dot() {
        // Regression test: `Path::new(".").parent()` returns `Some("")`
        // (not `None`), which previously made `..` resolution fail.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(std::env::temp_dir()).unwrap();

        let result = dot_entries(Path::new("."));
        std::env::set_current_dir(original_dir).unwrap();

        let dots = result.unwrap();
        assert_eq!(dots[0].name, ".");
        assert_eq!(dots[1].name, "..");
    }

    #[test]
    fn broken_symlink_detected() {
        #[cfg(unix)]
        {
            let dir =
                std::env::temp_dir().join(format!("colorls_test_link_{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let link_path = dir.join("broken_link");
            std::os::unix::fs::symlink(dir.join("does_not_exist"), &link_path).unwrap();

            let entry = FileEntry::from_path(&link_path).unwrap();
            assert!(entry.is_symlink);
            assert!(entry.is_broken_symlink);

            std::fs::remove_dir_all(&dir).ok();
        }
    }
}
