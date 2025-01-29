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

enum BackupOutcome {
    Removed(PathBuf),
    BackedUp(PathBuf),
}

impl Backup {
    pub fn open_for(&self, p: impl AsRef<Path>, file: File) -> io::Result<File> {
        fn numbered(p: impl AsRef<Path>, mut file: File) -> io::Result<File> {
            let mut backup_count = 1;

            loop {
                let backup_name = format!(
                    "{}.~{}~",
                    p.as_ref().file_name().unwrap().to_string_lossy(),
                    backup_count
                );

                let backup_path = p.as_ref().with_file_name(backup_name);

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

        fn simple(ext: impl AsRef<str>, p: impl AsRef<Path>, mut file: File) -> io::Result<File> {
            let backup_name = format!(
                "{}{}",
                p.as_ref().file_name().unwrap().to_string_lossy(),
                ext.as_ref()
            );

            let backup_path = p.as_ref().with_file_name(backup_name);

            match OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&backup_path)
            {
                Ok(mut backup) => {
                    io::copy(&mut file, &mut backup)?;
                    file.set_len(0)?;
                    file.rewind()?;
                    Ok(file)
                }
                Err(e) => Err(e),
            }
        }

        fn existing(ext: impl AsRef<str>, p: impl AsRef<Path>, file: File) -> io::Result<File> {
            let numbered_backup_name =
                format!("{}.~1~", p.as_ref().file_name().unwrap().to_string_lossy(),);

            let numbered_backup_path = p.as_ref().with_file_name(numbered_backup_name);

            if numbered_backup_path.exists() {
                return numbered(p, file);
            }

            simple(ext, p, file)
        }

        match self {
            Backup::None => Ok(file),
            Backup::Numbered => numbered(p, file),
            Backup::Simple(ext) => simple(ext, p, file),
            Backup::Existing(ext) => existing(ext, p, file),
        }
    }
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
                verbose,
            } => {
                let open_destination = |p: &Path| -> io::Result<File> {
                    match OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .open(&p)
                    {
                        Ok(file) => backup.open_for(p, file),
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
                        Ok(_) => {
                            if *verbose {
                                _ = writeln!(
                                    write_err,
                                    "'{}' -> '{}'",
                                    strip_prefix(file, &container).display(),
                                    strip_prefix(destination, &container).display()
                                );
                            }
                        },
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

fn strip_prefix<F: AsRef<Path>, P: AsRef<Path>>(file: F, prefix: P) -> PathBuf {
    file.as_ref()
        .strip_prefix(prefix)
        .unwrap_or(file.as_ref())
        .to_path_buf()
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

    #[test]
    fn copy_files_backs_up_existing_files_if_backup_is_simple() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_backs_up_existing_files_if_backup_is_simple")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "new").expect("unable to create a.txt");

        new_file_with_content(root.join("destination/a.txt"), "old")
            .expect("unable to create destination/a.txt");

        new_file_with_content(root.join("destination/a.txt.bak"), "veryold")
            .expect("unable to create destination/a.txt.bak");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::Simple(".bak".to_string()),
            preserve_timestamps: false,
            verbose: false,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.is_empty());

        assert_eq!(
            read_to_string(root.join("destination/a.txt")).unwrap(),
            "new",
        );

        assert_eq!(
            read_to_string(root.join("destination/a.txt.bak")).unwrap(),
            "old",
        );
    }

    #[test]
    fn copy_files_creates_numbered_backups_for_existing_if_present() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_creates_numbered_backups_for_existing_if_present")
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
            backup: Backup::Existing(".bak".to_string()),
            preserve_timestamps: false,
            verbose: false,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.is_empty());

        assert_eq!(
            read_to_string(root.join("destination/a.txt")).unwrap(),
            "new",
        );

        assert_eq!(
            read_to_string(root.join("destination/a.txt.~1~")).unwrap(),
            "veryold",
        );

        assert_eq!(
            read_to_string(root.join("destination/a.txt.~2~")).unwrap(),
            "old",
        );
    }

    #[test]
    fn copy_files_creates_simple_backups_for_existing_if_none_present() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_creates_simple_backups_for_existing_if_none_present")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "new").expect("unable to create a.txt");

        new_file_with_content(root.join("destination/a.txt"), "old")
            .expect("unable to create destination/a.txt");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::Existing(".bak".to_string()),
            preserve_timestamps: false,
            verbose: false,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.is_empty());

        assert_eq!(
            read_to_string(root.join("destination/a.txt")).unwrap(),
            "new",
        );

        assert_eq!(
            read_to_string(root.join("destination/a.txt.bak")).unwrap(),
            "old",
        );
    }

    #[test]
    fn copy_files_creates_mixed_backups_for_existing_if_present() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_creates_mixed_backups_for_existing_if_present")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "new-a").expect("unable to create a.txt");
        new_file_with_content(root.join("b.txt"), "new-b").expect("unable to create b.txt");

        new_file_with_content(root.join("destination/a.txt"), "old-a")
            .expect("unable to create a.txt");

        new_file_with_content(root.join("destination/b.txt"), "old-b")
            .expect("unable to create b.txt");

        new_file_with_content(root.join("destination/a.txt.bak"), "veryold-a")
            .expect("unable to create a.txt.bak");

        new_file_with_content(root.join("destination/b.txt.~1~"), "veryold-b")
            .expect("unable to create b.txt.bak");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::Existing(".bak".to_string()),
            preserve_timestamps: false,
            verbose: false,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.is_empty());

        assert_eq!(
            read_to_string(root.join("destination/a.txt")).unwrap(),
            "new-a",
        );

        assert_eq!(
            read_to_string(root.join("destination/b.txt")).unwrap(),
            "new-b",
        );

        assert_eq!(
            read_to_string(root.join("destination/a.txt.bak")).unwrap(),
            "old-a",
        );

        assert_eq!(
            read_to_string(root.join("destination/b.txt.~2~")).unwrap(),
            "old-b",
        );
    }

    #[test]
    fn copy_files_announces_file_changes_in_verbose_mode() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_announces_file_changes_in_verbose_mode")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "new-a").expect("unable to create a.txt");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::None,
            preserve_timestamps: false,
            verbose: true,
        };

        operation.execute(&root, &mut err_out);

        assert!(err_out.contains(
            format!(
                "'{}' -> '{}'",
                Path::new("a.txt").display(),
                Path::new("destination").join("a.txt").display()
            )
            .as_str()
        ));
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
