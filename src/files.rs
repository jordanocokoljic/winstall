use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum IoError {
    DirectorySource,
    PermissionDenied(PathBuf),
    NotFound(PathBuf),
}

pub fn copy<S, D>(source: S, destination: D) -> Result<u64, IoError>
where
    S: AsRef<Path>,
    D: AsRef<Path>,
{
    if source.as_ref().is_dir() {
        return Err(IoError::DirectorySource);
    }

    match fs::copy(&source, &destination) {
        Ok(bytes) => Ok(bytes),
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => {
                if !fs::metadata(&source).is_ok() {
                    Err(IoError::PermissionDenied(source.as_ref().to_path_buf()))
                } else {
                    Err(IoError::PermissionDenied(
                        destination.as_ref().to_path_buf(),
                    ))
                }
            }
            ErrorKind::NotFound => Err(IoError::NotFound(source.as_ref().to_path_buf())),
            _ => {
                panic!("unexpected error: {}", e);
            }
        },
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

        let new = copy(path.join("file.txt"), path.join("new.txt"));
        assert!(new.is_ok());
        assert_eq!(read_to_string(path.join("new.txt")).unwrap(), "pass");

        let existing = copy(path.join("file.txt"), path.join("existing.txt"));
        assert!(existing.is_ok());
        assert_eq!(read_to_string(path.join("existing.txt")).unwrap(), "pass");
    }

    #[test]
    fn test_copy_when_source_is_a_directory() {
        let path = EphemeralPath::new("test_copy_when_source_is_a_directory");
        fs::create_dir(path.join("source")).expect("failed to create source directory");

        let result = copy(path.join("source"), path.join("target"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), IoError::DirectorySource);
    }

    #[test]
    fn test_copy_when_source_is_permission_restricted() {
        let result = copy("readonly_directory/file.txt", "file.txt");
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

        let result = copy(path.join("file.txt"), "readonly_directory/file.txt");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            IoError::PermissionDenied(PathBuf::from("readonly_directory/file.txt"))
        );
    }

    #[test]
    fn test_copy_when_source_is_missing() {
        let path = EphemeralPath::new("test_copy_when_source_is_missing");

        let result = copy(path.join("file.txt"), path.join("target"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            IoError::NotFound(path.join("file.txt"))
        );
    }
}
