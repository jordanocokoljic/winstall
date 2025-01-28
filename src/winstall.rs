use std::fs::{File, FileTimes, OpenOptions};
use std::io;
use std::io::{ErrorKind, Seek};
use std::path::{Path, PathBuf};

#[derive(PartialEq, Debug)]
pub enum Backup {
    None,
    Numbered,
    Simple(String),
    Existing(String),
}

#[derive(PartialEq, Debug)]
pub enum Operation {
    CopyFiles {
        files: Vec<PathBuf>,
        destination: PathBuf,
        backup: Backup,
        preserve_timestamps: bool,
        verbose: bool,
    },
}

impl Operation {
    pub fn execute<P, E>(&self, container: P, write_err: &mut E)
    where
        P: AsRef<Path>,
        E: io::Write,
    {
        match self {
            Operation::CopyFiles {
                files,
                destination,
                backup,
                preserve_timestamps,
                ..
            } => {
                let open_destination = |p: &Path| -> io::Result<File> {
                    match OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(&p)
                    {
                        Ok(mut file) => match backup {
                            Backup::None => Ok(file),
                            Backup::Numbered => {
                                let mut backup_count = 1;

                                loop {
                                    let backup_name = format!(
                                        "{}.~{}~",
                                        p.file_name().unwrap().to_string_lossy(),
                                        backup_count
                                    );

                                    let backup_path = p.with_file_name(backup_name);

                                    match OpenOptions::new()
                                        .create_new(true)
                                        .write(true)
                                        .open(&backup_path)
                                    {
                                        Ok(mut backup) => {
                                            io::copy(&mut file, &mut backup)?;
                                            file.set_len(0)?;
                                            file.rewind()?;
                                            return Ok(file);
                                        }
                                        Err(e) => match e.kind() {
                                            ErrorKind::AlreadyExists => {
                                                backup_count += 1;
                                            }
                                            _ => return Err(e),
                                        },
                                    };
                                }
                            }
                            _ => todo!(),
                        },
                        Err(e) => Err(e),
                    }
                };

                for file in files {
                    let source = container.as_ref().join(file);

                    if source.is_dir() {
                        _ = writeln!(
                            write_err,
                            "winstall: skipping directory '{}'",
                            file.display()
                        );

                        continue;
                    }

                    let name = file.file_name().unwrap();
                    let destination = container.as_ref().join(destination).join(name);

                    let mut file_times: Option<FileTimes> = None;

                    let mut from = match OpenOptions::new().read(true).open(source) {
                        Ok(from) => {
                            if *preserve_timestamps {
                                let meta = from.metadata().expect("unable to get file metadata");

                                file_times = Some(
                                    FileTimes::new()
                                        .set_accessed(
                                            meta.accessed()
                                                .expect("failed to get last accessed time"),
                                        )
                                        .set_modified(
                                            meta.modified()
                                                .expect("failed to get last modified time"),
                                        ),
                                );
                            }

                            from
                        }
                        Err(e) => match e.kind() {
                            ErrorKind::NotFound => {
                                _ = writeln!(
                                    write_err,
                                    "winstall: file '{}' could not be found",
                                    file.display()
                                );

                                return;
                            }
                            _ => panic!("unable to open source file: {}", e),
                        },
                    };

                    let mut to = match open_destination(&destination) {
                        Ok(to) => to,
                        Err(e) => panic!("unable to open destination file: {}", e),
                    };

                    match io::copy(&mut from, &mut to) {
                        Ok(_) => (),
                        Err(e) => panic!("unable to copy file: {}", e),
                    };

                    if let Some(times) = file_times {
                        to.set_times(times).expect("unable to set file times");
                    }
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::interim::Interim;
    use crate::winstall::{Backup, Operation};
    use std::fs::{read_to_string, File, OpenOptions};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::{fs, io, thread};

    #[test]
    fn copy_files_copies_files() {
        let root = Interim::new("copy_files_copies_files").expect("unable to create test root");
        let mut err_out = Vec::<u8>::new();

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "a").expect("unable to create a.txt");
        new_file_with_content(root.join("b.txt"), "b").expect("unable to create b.txt");
        new_file_with_content(root.join("c.txt"), "c").expect("unable to create c.txt");

        let operation = Operation::CopyFiles {
            files: vec![
                PathBuf::from("a.txt"),
                PathBuf::from("b.txt"),
                PathBuf::from("c.txt"),
            ],
            destination: PathBuf::from("destination"),
            backup: Backup::None,
            preserve_timestamps: false,
            verbose: false,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.is_empty());
        assert_eq!(read_to_string(root.join("destination/a.txt")).unwrap(), "a");
        assert_eq!(read_to_string(root.join("destination/b.txt")).unwrap(), "b");
        assert_eq!(read_to_string(root.join("destination/c.txt")).unwrap(), "c");
    }

    #[test]
    fn copy_files_indicates_if_directory_is_omitted() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_indicates_if_directory_is_omitted")
            .expect("unable to create test root");

        fs::create_dir(root.join("directory")).expect("unable to create directory");
        fs::create_dir(root.join("destination")).expect("unable to create destination");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("directory")],
            destination: PathBuf::from("destination"),
            backup: Backup::None,
            preserve_timestamps: false,
            verbose: false,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.contains("winstall: skipping directory 'directory'"));
    }

    #[test]
    fn copy_files_indicates_if_file_is_missing() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_indicates_if_file_is_missing")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("missing.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::None,
            preserve_timestamps: false,
            verbose: false,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.contains("winstall: file 'missing.txt' could not be found"));
    }

    #[test]
    fn copy_files_does_not_update_file_timestamps_if_not_requested() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_does_not_update_file_timestamps_if_not_requested")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        let file = new_file_with_content(root.join("a.txt"), "a").expect("unable to create a.txt");

        let original_meta = file.metadata().expect("unable to get file metadata");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::None,
            preserve_timestamps: false,
            verbose: false,
        };

        thread::sleep(Duration::from_millis(100));

        operation.execute(&root, &mut err_out);

        let copy_meta = File::open(root.join("destination/a.txt"))
            .expect("unable to open copy")
            .metadata()
            .expect("unable to get file metadata");

        assert!(err_out.is_empty());

        assert_ne!(
            original_meta.accessed().unwrap(),
            copy_meta.accessed().unwrap(),
            "accessed time should have changed"
        );

        assert_ne!(
            original_meta.modified().unwrap(),
            copy_meta.modified().unwrap(),
            "modified time should have changed"
        );
    }

    #[test]
    fn copy_files_updates_file_timestamps_if_requested() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_updates_file_timestamps_if_requested")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        let file = new_file_with_content(root.join("a.txt"), "a").expect("unable to create a.txt");

        let original_meta = file.metadata().expect("unable to get file metadata");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::None,
            preserve_timestamps: true,
            verbose: false,
        };

        thread::sleep(Duration::from_millis(100));

        operation.execute(&root, &mut err_out);

        let copy_meta =
            fs::metadata(root.join("destination/a.txt")).expect("unable to get file metadata");

        assert!(err_out.is_empty());

        assert_eq!(
            original_meta.accessed().unwrap(),
            copy_meta.accessed().unwrap(),
            "accessed time should not have changed"
        );

        assert_eq!(
            original_meta.modified().unwrap(),
            copy_meta.modified().unwrap(),
            "modified time should not have changed"
        );
    }

    #[test]
    fn copy_files_overwrites_existing_files_if_backup_is_none() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_overwrites_existing_files_if_backup_is_none")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "pass").expect("unable to create a.txt");
        new_file_with_content(root.join("destination/a.txt"), "fail")
            .expect("unable to create destination/a.txt");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::None,
            preserve_timestamps: false,
            verbose: false,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.is_empty());
        assert_eq!(
            read_to_string(root.join("destination/a.txt")).unwrap(),
            "pass"
        );
    }

    #[test]
    fn copy_files_backs_up_existing_files_if_backup_is_numbered() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_backs_up_existing_files_if_backup_is_numbered")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "new").expect("unable to create a.txt");

        new_file_with_content(root.join("destination/a.txt"), "old")
            .expect("unable to create destination/a.txt");

        new_file_with_content(root.join("destination/a.txt.~1~"), "veryold")
            .expect("unable to create destination/a.txt.~1~");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::Numbered,
            preserve_timestamps: false,
            verbose: false,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.is_empty());

        assert_eq!(
            read_to_string(root.join("destination/a.txt")).unwrap(),
            "new"
        );

        assert_eq!(
            read_to_string(root.join("destination/a.txt.~1~")).unwrap(),
            "veryold"
        );

        assert_eq!(
            read_to_string(root.join("destination/a.txt.~2~")).unwrap(),
            "old"
        );
    }

    fn new_file_with_content<P: AsRef<Path>>(path: P, content: &str) -> io::Result<File> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(content.as_bytes())?;
        Ok(file)
    }

    struct TestOutputWriter(Vec<u8>);

    impl TestOutputWriter {
        fn new() -> Self {
            TestOutputWriter(Vec::new())
        }

        fn contains(&self, pattern: &str) -> bool {
            match String::from_utf8(self.0.clone()) {
                Ok(s) => s.contains(pattern),
                Err(_) => false,
            }
        }

        fn is_empty(&self) -> bool {
            self.0.is_empty()
        }
    }

    impl Write for TestOutputWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
}
