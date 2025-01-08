use crate::files::{copy, IoError};
use crate::foundation::{MessageRouter, Operation};
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
        Operation::CopyFiles(files, destination) => {
            for file in files {
                let source = container.as_ref().join(&file);

                let filename = match file.file_name() {
                    Some(filename) => filename,
                    None => {
                        let msg = format!("omitting directory '{}'", file.display());
                        router.err(Box::new(msg));
                        continue;
                    }
                };

                let destination = container
                    .as_ref()
                    .join(&destination)
                    .join(filename);

                match copy(source, destination) {
                    Ok(_) => (),
                    Err(IoError::DirectorySource) => {
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
                    Err(_) => (),
                }
            }
        }
    }
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
    use std::{env, fs};

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

        let operation = Operation::CopyFiles(
            vec![PathBuf::from("one.txt"), PathBuf::from("two.txt")],
            PathBuf::from("target"),
        );

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

        let operation =
            Operation::CopyFiles(vec![PathBuf::from("source")], PathBuf::from("target"));

        perform_operation(operation, &path, &mut router);

        assert!(router.err_contains("omitting directory 'source'"));
    }

    #[test]
    fn test_copy_files_with_permission_restricted_source() {
        let cwd = env::current_dir().expect("failed to get current directory");
        let mut router = RouterDouble::new();

        let operation = Operation::CopyFiles(
            vec![Path::new("readonly_directory")
                .join("file.txt")
                .to_path_buf()],
            PathBuf::from("target"),
        );

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

        let operation = Operation::CopyFiles(
            vec![path.join("from.txt")],
            PathBuf::from("readonly_directory"),
        );

        perform_operation(operation, &cwd, &mut router);

        assert!(router.err_contains(format!(
            "cannot stat '{}': Permission denied",
            Path::new("readonly_directory").join("from.txt").display()
        )));
    }

    #[test]
    fn test_copy_files_with_invalid_source_filename() {
        let operation = Operation::CopyFiles(
            vec![PathBuf::from(".."), PathBuf::from("/"), PathBuf::from("C:\\")],
            PathBuf::from("target"),
        );

        let mut router = RouterDouble::new();

        perform_operation(operation, &"", &mut router);

        dbg!(&router);

        assert!(router.err_contains("omitting directory '..'"));
        assert!(router.err_contains("omitting directory '/'"));
        assert!(router.err_contains("omitting directory 'C:\\'"));
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
