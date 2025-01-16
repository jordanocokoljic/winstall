use crate::foundation::BackupStrategy;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

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
        BackupStrategy::Simple(suffix) => simple_backup(&path, suffix),
        BackupStrategy::Existing(suffix) => {
            let mut backup_path = path.as_ref().to_path_buf();
            backup_path.set_file_name(format!(
                "{}.~1~",
                path.as_ref().file_name().unwrap().to_string_lossy(),
            ));

            if backup_path.exists() {
                numbered_backup(&path)
            } else {
                simple_backup(&path, suffix)
            }
        }
        BackupStrategy::Numbered => numbered_backup(&path),
    }
}

fn numbered_backup<P: AsRef<Path>>(path: &P) -> io::Result<File> {
    match OpenOptions::new()
        .write(true)
        .truncate(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            let mut backup_number = 1;

            'count: loop {
                let mut backup_path = path.as_ref().to_path_buf();
                backup_path.set_file_name(format!(
                    "{}.~{}~",
                    path.as_ref().file_name().unwrap().to_string_lossy(),
                    backup_number
                ));

                if backup_path.exists() {
                    backup_number += 1;
                    continue;
                }

                match fs::copy(&path, &backup_path) {
                    Ok(_) => break 'count,
                    Err(e) => panic!("copy: unable to create backup: {}", e),
                }
            }

            OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(path)
        }
        Err(e) => Err(e),
    }
}

fn simple_backup<P: AsRef<Path>>(path: &P, suffix: String) -> io::Result<File> {
    match OpenOptions::new()
        .write(true)
        .truncate(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            let mut backup_path = path.as_ref().to_path_buf();

            backup_path.set_file_name(format!(
                "{}{}",
                path.as_ref().file_name().unwrap().to_string_lossy(),
                suffix
            ));

            match fs::copy(&path, &backup_path) {
                Ok(_) => OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(path),
                Err(e) => panic!("copy: unable to create backup: {}", e),
            }
        }
        Err(e) => Err(e),
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

    #[test]
    fn test_copy_with_numbered_backup() {
        let path = EphemeralPath::new("test_copy_with_numbered_backups");
        create_file_with_content(path.join("first.txt"), "first")
            .expect("failed to create first file");

        create_file_with_content(path.join("second.txt"), "second")
            .expect("failed to create second file");

        create_file_with_content(path.join("to.txt"), "fail").expect("failed to create to file");

        let first = copy(
            path.join("first.txt"),
            path.join("to.txt"),
            BackupStrategy::Numbered,
        );

        assert!(first.is_ok());
        assert_eq!(read_to_string(path.join("to.txt")).unwrap(), "first");
        assert_eq!(read_to_string(path.join("to.txt.~1~")).unwrap(), "fail");

        let second = copy(
            path.join("second.txt"),
            path.join("to.txt"),
            BackupStrategy::Numbered,
        );

        assert!(second.is_ok());
        assert_eq!(read_to_string(path.join("to.txt")).unwrap(), "second");
        assert_eq!(read_to_string(path.join("to.txt.~1~")).unwrap(), "fail");
        assert_eq!(read_to_string(path.join("to.txt.~2~")).unwrap(), "first");
    }

    #[test]
    fn test_copy_with_existing_backup_when_prior_numbered_backups() {
        let path = EphemeralPath::new("test_copy_with_existing_backup_when_prior_numbered_backups");

        create_file_with_content(path.join("simple_src.txt"), "new")
            .expect("failed to create numbered source file");

        create_file_with_content(path.join("simple_dst.txt"), "old")
            .expect("failed to create numbered destination file");

        create_file_with_content(path.join("simple_dst.txt.~1~"), "backup")
            .expect("failed to create numbered backup file");

        let result = copy(
            path.join("simple_src.txt"),
            path.join("simple_dst.txt"),
            BackupStrategy::Existing(".bak".to_string()),
        );

        assert!(result.is_ok());
        assert_eq!(read_to_string(path.join("simple_dst.txt")).unwrap(), "new");
        assert_eq!(
            read_to_string(path.join("simple_dst.txt.~1~")).unwrap(),
            "backup"
        );
        assert_eq!(
            read_to_string(path.join("simple_dst.txt.~2~")).unwrap(),
            "old"
        );
    }

    #[test]
    fn test_copy_with_existing_backup_when_prior_simple_backups() {
        let path = EphemeralPath::new("test_copy_with_existing_backup_when_prior_simple_backups");

        create_file_with_content(path.join("simple_src.txt"), "new")
            .expect("failed to create simple source file");

        create_file_with_content(path.join("simple_dst.txt"), "old")
            .expect("failed to create simple destination file");

        create_file_with_content(path.join("simple_dst.txt.bak"), "backup")
            .expect("failed to create simple backup file");

        let result = copy(
            path.join("simple_src.txt"),
            path.join("simple_dst.txt"),
            BackupStrategy::Existing(".bak".to_string()),
        );

        assert!(result.is_ok());
        assert_eq!(read_to_string(path.join("simple_dst.txt")).unwrap(), "new");
        assert_eq!(
            read_to_string(path.join("simple_dst.txt.bak")).unwrap(),
            "old"
        );
    }

    #[test]
    fn test_copy_with_existing_backup_when_prior_mixed_backups() {
        let path = EphemeralPath::new("test_copy_with_existing_backup_when_prior_mixed_backups");

        create_file_with_content(path.join("src.txt"), "new")
            .expect("failed to create source file");

        create_file_with_content(path.join("dst.txt"), "old")
            .expect("failed to create destination file");

        create_file_with_content(path.join("dst.txt.~1~"), "backup")
            .expect("failed to create numbered backup file");

        let first = copy(
            path.join("src.txt"),
            path.join("dst.txt"),
            BackupStrategy::Existing(".bak".to_string()),
        );

        assert!(first.is_ok());
        assert_eq!(read_to_string(path.join("dst.txt")).unwrap(), "new");
        assert_eq!(read_to_string(path.join("dst.txt.~2~")).unwrap(), "old");

        fs::remove_file(path.join("dst.txt.~1~")).expect("failed to remove numbered backup 1");

        create_file_with_content(path.join("dst.txt"), "old")
            .expect("failed to re-create destination file");

        let second = copy(
            path.join("src.txt"),
            path.join("dst.txt"),
            BackupStrategy::Existing(".bak".to_string()),
        );

        assert!(second.is_ok());
        assert_eq!(read_to_string(path.join("dst.txt")).unwrap(), "new");
        assert_eq!(read_to_string(path.join("dst.txt.bak")).unwrap(), "old");
    }

    #[test]
    fn test_copy_with_simple_backup() {
        let path = EphemeralPath::new("test_copy_with_simple_backups");
        create_file_with_content(path.join("from.txt"), "pass").expect("failed to create file");
        create_file_with_content(path.join("to.txt"), "fail").expect("failed to create file");

        let result = copy(
            path.join("from.txt"),
            path.join("to.txt"),
            BackupStrategy::Simple(".bak".to_string()),
        );

        assert!(result.is_ok());
        assert_eq!(read_to_string(path.join("to.txt")).unwrap(), "pass");
        assert_eq!(read_to_string(path.join("to.txt.bak")).unwrap(), "fail");
    }

    #[test]
    fn test_copy_returns_same_number_of_bytes_as_fs_copy() {
        let path = EphemeralPath::new("test_copy_returns_same_number_of_bytes_as_fs");
        create_file_with_content(path.join("from.txt"), "copy").expect("failed to create file");

        let fs_result = fs::copy(path.join("from.txt"), path.join("fs_to.txt"));

        let win_result = copy(
            path.join("from.txt"),
            path.join("win_to.txt"),
            BackupStrategy::None,
        );

        assert!(fs_result.is_ok());
        assert!(win_result.is_ok());
        assert_eq!(fs_result.unwrap(), win_result.unwrap());
    }
}
