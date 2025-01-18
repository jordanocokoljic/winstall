use crate::files::{copy, IoError};
use crate::foundation::{BackupStrategy, MessageRouter, Operation};
use std::fs;
use std::fs::{FileTimes, OpenOptions};
use std::path::{Path, PathBuf};

const HELP: &str = include_str!("help.txt");
const VERSION: &str = include_str!("version.txt");

pub fn perform_operation<C, R>(operation: Operation, container: &C, router: &mut R)
where
    C: AsRef<Path>,
    R: MessageRouter,
{
    match operation {
        Operation::ShowHelp => router.out(Box::new(HELP)),
        Operation::ShowVersion => router.out(Box::new(VERSION)),
        Operation::CopyFiles {
            preserve_timestamps,
            files,
            directory,
        } => {
            for file in files {
                let source = container.as_ref().join(&file);

                let file_times = if preserve_timestamps {
                    try_get_file_times(&source)
                } else {
                    None
                };

                let filename = match file.file_name() {
                    Some(filename) => filename,
                    None => {
                        let msg = format!("omitting directory '{}'", file.display());
                        router.err(Box::new(msg));
                        continue;
                    }
                };

                let destination = container.as_ref().join(&directory).join(filename);

                match copy(&source, &destination, BackupStrategy::None) {
                    Ok(_) => {
                        try_set_file_times(&destination, file_times);
                    }
                    Err(IoError::DirectoryArgument(_)) => {
                        let msg = format!("omitting directory '{}'", file.display());
                        router.err(Box::new(msg));
                    }
                    Err(IoError::PermissionDenied(path)) => {
                        let msg = format!(
                            "cannot stat '{}': Permission denied",
                            strip_prefix(&path, &container).display()
                        );

                        router.err(Box::new(msg));
                    }
                    Err(IoError::NotFound(path)) => {
                        let msg = format!(
                            "cannot stat '{}': No such file or directory",
                            strip_prefix(&path, &container).display()
                        );

                        router.err(Box::new(msg));
                    }
                }
            }
        }
        Operation::CreateDirectories(directories) => {
            for directory in directories {
                let mut aggregate = container.as_ref().to_path_buf();

                for component in directory.components() {
                    let target = aggregate.join(component);

                    match fs::create_dir(&target) {
                        Ok(_) => (),
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
                        Err(e) => {
                            panic!("failed to create directory '{}': {}", target.display(), e);
                        }
                    }

                    aggregate = target;
                }
            }
        }
    }
}

fn try_get_file_times<P: AsRef<Path>>(path: P) -> Option<FileTimes> {
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return None,
        Err(e) => panic!(
            "failed to get metadata for '{}': {}",
            path.as_ref().display(),
            e
        ),
    };

    let accessed = metadata
        .accessed()
        .expect("failed to get last accessed time");

    let modified = metadata
        .modified()
        .expect("failed to get last modified time");

    Some(
        FileTimes::new()
            .set_accessed(accessed)
            .set_modified(modified),
    )
}

fn try_set_file_times<P: AsRef<Path>>(path: P, ft: Option<FileTimes>) {
    ft.map(|ft| {
        let file = OpenOptions::new()
            .write(true)
            .open(path)
            .expect("failed to open file to update timestamps");

        file.set_times(ft)
            .expect("failed to update file timestamps");
    });
}

fn strip_prefix<F: AsRef<Path>, P: AsRef<Path>>(file: F, prefix: P) -> PathBuf {
    file.as_ref()
        .strip_prefix(prefix)
        .unwrap_or(file.as_ref())
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use crate::files::testing::create_file_with_content;
    use crate::foundation::{MessageRouter, Operation};
    use crate::ghost::EphemeralPath;
    use crate::winstall::{perform_operation, HELP, VERSION};
    use std::fmt::{Debug, Display, Formatter};
    use std::fs::File;
    use std::path::{Path, PathBuf};
    use std::{env, fs, thread, time};

    #[test]
    fn test_show_help() {
        let mut router = RouterDouble::new();
        perform_operation(Operation::ShowHelp, &"", &mut router);
        assert!(router.out_contains(HELP));
    }

    #[test]
    fn test_show_version() {
        let mut router = RouterDouble::new();
        perform_operation(Operation::ShowVersion, &"", &mut router);
        assert!(router.out_contains(VERSION));
    }

    #[test]
    fn test_copy_files() {
        let path = EphemeralPath::new("test_copy_files");
        fs::create_dir(path.join("target")).expect("failed to create target directory");
        create_file_with_content(path.join("one.txt"), "one").expect("failed to create file");
        create_file_with_content(path.join("two.txt"), "two").expect("failed to create file");

        let mut router = RouterDouble::new();

        let operation = Operation::CopyFiles {
            preserve_timestamps: false,
            files: vec![PathBuf::from("one.txt"), PathBuf::from("two.txt")],
            directory: PathBuf::from("target"),
        };

        perform_operation(operation, &path, &mut router);

        assert!(router.err_is_empty());

        assert!(path.join("target/one.txt").exists());
        assert_eq!(
            fs::read_to_string(path.join("target/one.txt")).unwrap(),
            "one"
        );

        assert!(path.join("target/two.txt").exists());
        assert_eq!(
            fs::read_to_string(path.join("target/two.txt")).unwrap(),
            "two"
        );
    }

    #[test]
    fn test_copy_files_with_directory_source() {
        let path = EphemeralPath::new("test_copy_files_with_directory_source");
        fs::create_dir(path.join("source")).expect("failed to create source directory");
        fs::create_dir(path.join("target")).expect("failed to create target directory");

        let mut router = RouterDouble::new();

        let operation = Operation::CopyFiles {
            preserve_timestamps: false,
            files: vec![PathBuf::from("source")],
            directory: PathBuf::from("target"),
        };

        perform_operation(operation, &path, &mut router);

        assert!(router.err_contains("omitting directory 'source'"));
    }

    #[test]
    fn test_copy_files_with_permission_restricted_source() {
        let cwd = env::current_dir().expect("failed to get current directory");
        let mut router = RouterDouble::new();

        let operation = Operation::CopyFiles {
            preserve_timestamps: true,
            files: vec![Path::new("readonly_directory")
                .join("file.txt")
                .to_path_buf()],
            directory: PathBuf::from("target"),
        };

        perform_operation(operation, &cwd, &mut router);

        assert!(router.err_contains(format!(
            "cannot stat '{}': Permission denied",
            Path::new("readonly_directory").join("file.txt").display()
        )));
    }

    #[test]
    fn test_copy_files_with_permission_restricted_destination() {
        let path = EphemeralPath::new("test_copy_files_with_permission_restricted_destination");
        File::create(path.join("from.txt")).expect("failed to create file");

        let cwd = env::current_dir().expect("failed to get current directory");
        let mut router = RouterDouble::new();

        let operation = Operation::CopyFiles {
            preserve_timestamps: true,
            files: vec![path.join("from.txt")],
            directory: PathBuf::from("readonly_directory"),
        };

        perform_operation(operation, &cwd, &mut router);

        assert!(router.err_contains(format!(
            "cannot stat '{}': Permission denied",
            Path::new("readonly_directory").join("from.txt").display()
        )));
    }

    #[test]
    fn test_copy_files_with_invalid_source_filename() {
        let operation = Operation::CopyFiles {
            preserve_timestamps: false,
            files: vec![
                PathBuf::from(".."),
                PathBuf::from("/"),
                PathBuf::from("C:\\"),
            ],
            directory: PathBuf::from("target"),
        };

        let mut router = RouterDouble::new();

        perform_operation(operation, &"", &mut router);

        assert!(router.err_contains("omitting directory '..'"));
        assert!(router.err_contains("omitting directory '/'"));
        assert!(router.err_contains("omitting directory 'C:\\'"));
    }

    #[test]
    fn test_copy_files_with_missing_source() {
        let path = EphemeralPath::new("test_copy_files_with_missing_source");
        fs::create_dir(path.join("target")).expect("failed to create target directory");

        let operation = Operation::CopyFiles {
            preserve_timestamps: false,
            files: vec![PathBuf::from("missing.txt")],
            directory: PathBuf::from("target"),
        };

        let mut router = RouterDouble::new();

        perform_operation(operation, &path, &mut router);

        assert!(router.err_contains(format!(
            "cannot stat '{}': No such file or directory",
            Path::new("missing.txt").display()
        )));
    }

    #[test]
    fn test_create_directories() {
        let path = EphemeralPath::new("test_create_directories");

        let operation = Operation::CreateDirectories(vec![
            PathBuf::from("one/two/three"),
            PathBuf::from("four/five/six"),
            PathBuf::from("seven/eight/nine"),
        ]);

        let mut router = RouterDouble::new();

        perform_operation(operation, &path, &mut router);

        assert!(router.err_is_empty());
        assert!(path.join("one/two/three").exists());
        assert!(path.join("four/five/six").exists());
        assert!(path.join("seven/eight/nine").exists());
    }

    #[test]
    fn test_create_directories_with_existing_directories() {
        let path = EphemeralPath::new("test_create_directories_with_existing_directories");
        fs::create_dir_all(path.join("one/two")).expect("failed to create directory");
        fs::create_dir_all(path.join("four")).expect("failed to create directory");

        let operation = Operation::CreateDirectories(vec![
            PathBuf::from("one/two/three"),
            PathBuf::from("four/five/six"),
            PathBuf::from("seven/eight/nine"),
        ]);

        let mut router = RouterDouble::new();

        perform_operation(operation, &path, &mut router);

        assert!(router.err_is_empty());
        assert!(path.join("one/two/three").exists());
        assert!(path.join("four/five/six").exists());
        assert!(path.join("seven/eight/nine").exists());
    }

    #[test]
    fn test_without_preserved_timestamps_copy_updates_timestamps() {
        let path = EphemeralPath::new("test_without_preserved_timestamps_copy_updates_timestamps");
        fs::create_dir(path.join("target")).expect("failed to create target directory");
        create_file_with_content(path.join("one.txt"), "one").expect("failed to create file");

        let pre_meta = fs::metadata(path.join("one.txt")).expect("failed to get metadata");
        let original_atime = pre_meta.accessed().expect("failed to get accessed time");
        let original_mtime = pre_meta.modified().expect("failed to get modified time");

        thread::sleep(time::Duration::from_millis(50));

        let operation = Operation::CopyFiles {
            preserve_timestamps: false,
            files: vec![PathBuf::from("one.txt")],
            directory: PathBuf::from("target"),
        };

        let mut router = RouterDouble::new();

        perform_operation(operation, &path, &mut router);

        let post_meta = fs::metadata(path.join("target/one.txt")).expect("failed to get metadata");
        let post_atime = post_meta.accessed().expect("failed to get accessed time");
        let post_mtime = post_meta.modified().expect("failed to get modified time");

        assert!(post_atime > original_atime);
        assert!(post_mtime > original_mtime);
    }

    #[test]
    fn test_with_preserved_timestamps_copy_does_not_update_timestamps() {
        let path =
            EphemeralPath::new("test_with_preserved_timestamps_copy_does_not_update_timestamps");

        fs::create_dir(path.join("target")).expect("failed to create target directory");
        let from =
            create_file_with_content(path.join("one.txt"), "one").expect("failed to create file");

        let pre = from.metadata().expect("failed to get metadata");
        let original_atime = pre.accessed().expect("failed to get accessed time");
        let original_mtime = pre.modified().expect("failed to get modified time");

        let operation = Operation::CopyFiles {
            preserve_timestamps: true,
            files: vec![PathBuf::from("one.txt")],
            directory: PathBuf::from("target"),
        };

        let mut router = RouterDouble::new();

        thread::sleep(time::Duration::from_millis(100));

        perform_operation(operation, &path, &mut router);

        assert!(router.err_is_empty());

        let post = fs::metadata(path.join("target/one.txt")).expect("failed to get metadata");
        let post_atime = post.accessed().expect("failed to get accessed time");
        let post_mtime = post.modified().expect("failed to get modified time");

        assert_eq!(
            original_atime, post_atime,
            "last accessed time should not have changed"
        );

        assert_eq!(
            original_mtime, post_mtime,
            "last modified time should not have changed"
        );
    }

    struct RouterDouble {
        out: Vec<Box<dyn Display>>,
        err: Vec<Box<dyn Display>>,
    }

    impl RouterDouble {
        fn new() -> Self {
            Self {
                out: vec![],
                err: vec![],
            }
        }

        fn out_contains<T: Display>(&self, message: T) -> bool {
            self.out
                .iter()
                .any(|m| m.to_string() == message.to_string())
        }

        fn err_contains<T: Display>(&self, message: T) -> bool {
            self.err
                .iter()
                .any(|m| m.to_string() == message.to_string())
        }

        fn err_is_empty(&self) -> bool {
            self.err.is_empty()
        }
    }

    impl MessageRouter for RouterDouble {
        fn out(&mut self, message: Box<dyn Display>) {
            self.out.push(message);
        }

        fn err(&mut self, message: Box<dyn Display>) {
            self.err.push(message);
        }
    }

    impl Debug for RouterDouble {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "out:")?;
            for item in &self.out {
                writeln!(f, "  \"{}\"", item)?;
            }

            writeln!(f, "err:")?;
            for item in &self.err {
                writeln!(f, "  \"{}\"", item)?;
            }

            Ok(())
        }
    }
}
