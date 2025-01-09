use crate::foundation::BackupStrategy;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum IoError {
    DirectoryArgument(PathBuf),
    PermissionDenied(PathBuf),
    NotFound(PathBuf),
}

// TODO(jordan): There are still a lot of panic's in here.
pub fn copy<S, D>(src: S, dst: D, backup: BackupStrategy) -> Result<u64, IoError>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
{
    if src.as_ref().is_dir() {
        return Err(IoError::DirectoryArgument(src.as_ref().to_path_buf()));
    }

    if dst.as_ref().is_dir() {
        return Err(IoError::DirectoryArgument(dst.as_ref().to_path_buf()));
    }

    let mut from = match OpenOptions::new().read(true).open(&src) {
        Ok(file) => file,
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => {
                return Err(IoError::PermissionDenied(src.as_ref().to_path_buf()))
            }
            ErrorKind::NotFound => return Err(IoError::NotFound(src.as_ref().to_path_buf())),
            _ => {
                panic!("copy: unable to open source: {}", e);
            }
        },
    };

    let mut to = match open_file_with_backup(&dst, backup) {
        Ok(file) => file,
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => {
                return Err(IoError::PermissionDenied(dst.as_ref().to_path_buf()))
            }
            _ => {
                panic!("copy: unable to open destination: {}", e);
            }
        },
    };

    let mut buf = [0u8; 1024];
    let mut bytes_copied = 0u64;

    loop {
        let n = match from.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => panic!("copy: read failed: {}", e),
        };

        match to.write_all(&buf[..n]) {
            Ok(_) => bytes_copied += n as u64,
            Err(e) => panic!("copy: write failed: {}", e),
        }
    }

    Ok(bytes_copied)
}

fn open_file_with_backup<P: AsRef<Path>>(path: P, strategy: BackupStrategy) -> io::Result<File> {
    match strategy {
        BackupStrategy::None => OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path),
        _ => todo!(),
    }
}

pub mod testing {
    use std::fs::File;
    use std::io;
    use std::io::Write;
    use std::path::Path;

    pub fn create_file_with_content(path: impl AsRef<Path>, content: &str) -> io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use crate::files::testing::create_file_with_content;
    use crate::files::{copy, IoError};
    use crate::foundation::BackupStrategy;
    use crate::ghost::EphemeralPath;
    use std::fs;
    use std::fs::read_to_string;
    use std::fs::File;
    use std::path::PathBuf;

    #[test]
    fn test_copy() {
        let path = EphemeralPath::new("test_copy");
        create_file_with_content(path.join("file.txt"), "pass").expect("failed to create file");
        create_file_with_content(path.join("existing.txt"), "fail").expect("failed to create file");

        let new = copy(
            path.join("file.txt"),
            path.join("new.txt"),
            BackupStrategy::None,
        );
        assert!(new.is_ok());
        assert_eq!(read_to_string(path.join("new.txt")).unwrap(), "pass");

        let existing = copy(
            path.join("file.txt"),
            path.join("existing.txt"),
            BackupStrategy::None,
        );
        assert!(existing.is_ok());
        assert_eq!(read_to_string(path.join("existing.txt")).unwrap(), "pass");
    }

    #[test]
    fn test_copy_when_source_is_a_directory() {
        let path = EphemeralPath::new("test_copy_when_source_is_a_directory");
        fs::create_dir(path.join("source")).expect("failed to create source directory");

        let result = copy(
            path.join("source"),
            path.join("target"),
            BackupStrategy::None,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            IoError::DirectoryArgument(path.join("source"))
        );
    }

    #[test]
    fn test_copy_when_destination_is_a_directory() {
        let path = EphemeralPath::new("test_copy_when_destination_is_a_directory");
        File::create(path.join("source")).expect("failed to create source file");
        fs::create_dir(path.join("target")).expect("failed to create target directory");

        let result = copy(
            path.join("source"),
            path.join("target"),
            BackupStrategy::None,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            IoError::DirectoryArgument(path.join("target"))
        );
    }

    #[test]
    fn test_copy_when_source_is_permission_restricted() {
        let result = copy(
            "readonly_directory/file.txt",
            "file.txt",
            BackupStrategy::None,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            IoError::PermissionDenied(PathBuf::from("readonly_directory/file.txt"))
        );
    }

    #[test]
    fn test_copy_when_destination_is_permission_restricted() {
        let path = EphemeralPath::new("test_copy_when_destination_is_permission_restricted");
        File::create(path.join("file.txt")).expect("failed to create file");

        let result = copy(
            path.join("file.txt"),
            "readonly_directory/file.txt",
            BackupStrategy::None,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            IoError::PermissionDenied(PathBuf::from("readonly_directory/file.txt"))
        );
    }

    #[test]
    fn test_copy_when_source_is_missing() {
        let path = EphemeralPath::new("test_copy_when_source_is_missing");

        let result = copy(
            path.join("file.txt"),
            path.join("target"),
            BackupStrategy::None,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            IoError::NotFound(path.join("file.txt"))
        );
    }
}
