use std::fs::{File, FileTimes, OpenOptions};
use std::io::{ErrorKind, Seek};
use std::path::{Path, PathBuf};
use std::{fs, io};

pub enum Backup {
    None,
    Numbered,
    Simple(String),
    Existing(String),
}

pub enum BackupOutcome {
    Removed(PathBuf),
    BackedUp(PathBuf),
}

impl Backup {
    pub fn open_for(&self, p: impl AsRef<Path>, file: File) -> io::Result<(File, BackupOutcome)> {
        fn numbered(p: impl AsRef<Path>, mut file: File) -> io::Result<(File, BackupOutcome)> {
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
                        return Ok((file, BackupOutcome::BackedUp(backup_path)));
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

        fn simple(
            ext: impl AsRef<str>,
            p: impl AsRef<Path>,
            mut file: File,
        ) -> io::Result<(File, BackupOutcome)> {
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
                    Ok((file, BackupOutcome::BackedUp(backup_path)))
                }
                Err(e) => Err(e),
            }
        }

        fn existing(
            ext: impl AsRef<str>,
            p: impl AsRef<Path>,
            file: File,
        ) -> io::Result<(File, BackupOutcome)> {
            let numbered_backup_name =
                format!("{}.~1~", p.as_ref().file_name().unwrap().to_string_lossy());

            let numbered_backup_path = p.as_ref().with_file_name(numbered_backup_name);

            if numbered_backup_path.exists() {
                return numbered(p, file);
            }

            simple(ext, p, file)
        }

        match self {
            Backup::None => Ok((file, BackupOutcome::Removed(p.as_ref().to_path_buf()))),
            Backup::Numbered => numbered(p, file),
            Backup::Simple(ext) => simple(ext, p, file),
            Backup::Existing(ext) => existing(ext, p, file),
        }
    }
}

pub enum Operation {
    CopyFiles {
        files: Vec<PathBuf>,
        destination: PathBuf,
        backup: Backup,
        preserve_timestamps: bool,
        make_all_directories: bool,
        verbose: bool,
    },
    CreateDirectories {
        directories: Vec<PathBuf>,
        verbose: bool,
    },
}

impl Operation {
    pub fn execute<P, E>(&self, container: P, write_err: &mut E) -> bool
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
                make_all_directories,
                verbose,
            } => {
                let mut ok = true;

                let open_destination = |p: &Path| -> io::Result<(File, BackupOutcome)> {
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

                        ok = false;
                        continue;
                    }

                    let name = file.file_name().unwrap();
                    let destination_folder = container.as_ref().join(destination);
                    let destination_file = destination_folder.join(name);

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
                        Err(e) => {
                            match e.kind() {
                                ErrorKind::NotFound => {
                                    _ = writeln!(
                                        write_err,
                                        "winstall: cannot stat '{}': No such file or directory",
                                        strip_prefix(file, &container).display()
                                    );
                                }
                                ErrorKind::PermissionDenied => {
                                    _ = writeln!(
                                        write_err,
                                        "winstall: cannot open '{}' for reading: Permission denied",
                                        strip_prefix(file, &container).display()
                                    )
                                }
                                _ => panic!("unable to open source file: {}", e),
                            }

                            ok = false;
                            continue;
                        }
                    };

                    let create_directory_result = if *make_all_directories {
                        fs::create_dir_all(&destination_folder)
                    } else {
                        fs::create_dir(&destination_folder)
                    };

                    match create_directory_result {
                        Ok(_) => {
                            if *verbose {
                                _ = writeln!(
                                    write_err,
                                    "winstall: creating directory '{}'",
                                    strip_prefix(destination_folder, &container).display()
                                );
                            }
                        }
                        Err(e) => {
                            match e.kind() {
                                ErrorKind::AlreadyExists => (),
                                ErrorKind::NotFound => {
                                    _ = writeln!(
                                        write_err,
                                        "winstall: cannot create regular file '{}': No such file or directory",
                                        strip_prefix(destination_folder, &container).display()
                                    );

                                    ok = false;
                                    continue;
                                }
                                ErrorKind::PermissionDenied => {
                                    _ = writeln!(
                                        write_err,
                                        "winstall: cannot create directory '{}': Permission denied",
                                        strip_prefix(destination_folder, &container).display()
                                    );

                                    ok = false;
                                    continue;
                                }
                                _ => panic!("unable to create destination directory: {}", e),
                            };
                        },
                    };

                    let (mut to, outcome) = match open_destination(&destination_file) {
                        Ok(result) => result,
                        Err(e) => match e.kind() {
                            ErrorKind::PermissionDenied => {
                                _ = writeln!(
                                    write_err,
                                    "winstall: cannot stat '{}': Permission denied",
                                    strip_prefix(destination_file, &container).display()
                                );

                                ok = false;
                                continue;
                            }
                            _ => panic!("unable to open destination file: {}", e),
                        },
                    };

                    if *verbose {
                        if let BackupOutcome::Removed(removed_path) = &outcome {
                            _ = writeln!(
                                write_err,
                                "removed '{}'",
                                strip_prefix(removed_path, &container).display(),
                            );
                        }
                    }

                    match io::copy(&mut from, &mut to) {
                        Ok(_) => {
                            if *verbose {
                                let extended_message =
                                    if let BackupOutcome::BackedUp(backup_path) = &outcome {
                                        format!(
                                            " (backup: '{}')",
                                            strip_prefix(backup_path, &container).display()
                                        )
                                    } else {
                                        "".into()
                                    };

                                _ = writeln!(
                                    write_err,
                                    "'{}' -> '{}'{}",
                                    strip_prefix(file, &container).display(),
                                    strip_prefix(destination_file, &container).display(),
                                    extended_message,
                                );
                            }
                        }
                        Err(e) => panic!("unable to copy file: {}", e),
                    };

                    if let Some(times) = file_times {
                        to.set_times(times).expect("unable to set file times");
                    }
                }

                ok
            }
            Operation::CreateDirectories {
                directories,
                verbose,
            } => {
                let mut ok = true;

                for directory in directories {
                    match fs::create_dir_all(container.as_ref().join(&directory)) {
                        Ok(_) => {
                            if *verbose {
                                _ = writeln!(
                                    write_err,
                                    "winstall: creating directory '{}'",
                                    strip_prefix(directory, &container).display(),
                                );
                            }
                        }
                        Err(e) => match e.kind() {
                            ErrorKind::PermissionDenied => {
                                _ = writeln!(
                                    write_err,
                                    "winstall: cannot create directory '{}': Permission denied",
                                    strip_prefix(directory, &container).display(),
                                );

                                ok = false;
                                continue;
                            }
                            _ => panic!("unable to create directory: {}", e),
                        },
                    };
                }

                ok
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
    use crate::interim::Interim;
    use crate::winstall::{Backup, Operation};
    use std::fmt::{Debug, Formatter};
    use std::fs::{read_to_string, File, OpenOptions};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::{env, fs, io, thread};

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
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");
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
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, false, "execution should report failure");
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
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, false, "execution should report failure");
        assert!(err_out.contains("winstall: cannot stat 'missing.txt': No such file or directory"));
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
            make_all_directories: false,
            verbose: false,
        };

        thread::sleep(Duration::from_millis(100));

        let success = operation.execute(&root, &mut err_out);

        let copy_meta = File::open(root.join("destination/a.txt"))
            .expect("unable to open copy")
            .metadata()
            .expect("unable to get file metadata");

        assert_eq!(success, true, "execution should report success");

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
            make_all_directories: false,
            verbose: false,
        };

        thread::sleep(Duration::from_millis(100));

        let success = operation.execute(&root, &mut err_out);

        let copy_meta =
            fs::metadata(root.join("destination/a.txt")).expect("unable to get file metadata");

        assert_eq!(success, true, "execution should report success");

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
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

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
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

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
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

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
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

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
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

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
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

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
            make_all_directories: false,
            verbose: true,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

        assert!(err_out.contains(
            format!(
                "'{}' -> '{}'",
                Path::new("a.txt").display(),
                Path::new("destination").join("a.txt").display()
            )
            .as_str()
        ));
    }

    #[test]
    fn copy_files_announces_file_overwrites_in_verbose_mode() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_announces_file_overwrites_in_verbose_mode")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "new-a").expect("unable to create a.txt");
        new_file_with_content(root.join("destination/a.txt"), "old-a")
            .expect("unable to create a.txt");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::None,
            preserve_timestamps: false,
            make_all_directories: false,
            verbose: true,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

        assert!(err_out.contains(
            format!(
                "removed '{}'",
                Path::new("destination").join("a.txt").display()
            )
            .as_str()
        ));
    }

    #[test]
    fn copy_files_announces_file_backups_in_verbose_mode() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_announces_file_backups_in_verbose_mode")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "new-a").expect("unable to create a.txt");
        new_file_with_content(root.join("destination/a.txt"), "old-a")
            .expect("unable to create a.txt");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination"),
            backup: Backup::Numbered,
            preserve_timestamps: false,
            make_all_directories: false,
            verbose: true,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

        assert!(err_out.contains(
            format!(
                "'{}' -> '{}' (backup: '{}')",
                Path::new("a.txt").display(),
                Path::new("destination").join("a.txt").display(),
                Path::new("destination").join("a.txt.~1~").display()
            )
            .as_str()
        ));
    }

    #[test]
    fn copy_files_reports_permission_denied_for_target() {
        let mut err_out = TestOutputWriter::new();
        let cwd = env::current_dir().expect("unable to get current directory");

        let root = Interim::new("copy_files_reports_permission_denied_for_target")
            .expect("unable to create test root");

        new_file_with_content(root.join("a.txt"), "a").expect("unable to create a.txt");

        let operation = Operation::CopyFiles {
            files: vec![root.join("a.txt")],
            destination: PathBuf::from("readonly_directory"),
            backup: Backup::None,
            preserve_timestamps: false,
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&cwd, &mut err_out);

        assert_eq!(success, false, "execution should report failure");

        assert!(err_out.contains(
            format!(
                "winstall: cannot stat '{}': Permission denied",
                Path::new("readonly_directory").join("a.txt").display()
            )
            .as_str()
        ));
    }

    #[test]
    fn copy_files_reports_permission_denied_for_source() {
        let mut err_out = TestOutputWriter::new();
        let cwd = env::current_dir().expect("unable to get current directory");

        let root = Interim::new("copy_files_reports_permission_denied_for_source")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");

        let operation = Operation::CopyFiles {
            files: vec![Path::new("readonly_directory").join("file.txt")],
            destination: root.join("destination"),
            backup: Backup::None,
            preserve_timestamps: false,
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&cwd, &mut err_out);

        assert_eq!(success, false, "execution should report failure");

        assert!(err_out.contains(
            format!(
                "winstall: cannot open '{}' for reading: Permission denied",
                Path::new("readonly_directory").join("file.txt").display()
            )
            .as_str()
        ));
    }

    #[test]
    fn copy_files_creates_the_last_component_of_destination() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_creates_the_last_component_of_destination")
            .expect("unable to create test root");

        fs::create_dir(root.join("destination")).expect("unable to create destination");
        new_file_with_content(root.join("a.txt"), "a").expect("unable to create a.txt");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination").join("subdirectory"),
            backup: Backup::None,
            preserve_timestamps: false,
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

        assert_eq!(
            read_to_string(root.join("destination/subdirectory/a.txt")).unwrap(),
            "a"
        );
    }

    #[test]
    fn copy_files_indicates_that_the_target_path_is_incomplete() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_indicates_that_the_target_path_is_incomplete")
            .expect("unable to create test root");

        new_file_with_content(root.join("a.txt"), "a").expect("unable to create a.txt");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination").join("sub_one").join("sub_two"),
            backup: Backup::None,
            preserve_timestamps: false,
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, false, "execution should report failure");

        assert!(err_out.contains(
            format!(
                "winstall: cannot create regular file '{}': No such file or directory",
                Path::new("destination")
                    .join("sub_one")
                    .join("sub_two")
                    .display()
            )
            .as_str()
        ));
    }

    #[test]
    fn copy_files_can_create_leading_destination_directories() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_can_create_leading_destination_directories")
            .expect("unable to create test root");

        new_file_with_content(root.join("a.txt"), "a").expect("unable to create a.txt");

        let operation = Operation::CopyFiles {
            files: vec![PathBuf::from("a.txt")],
            destination: PathBuf::from("destination").join("sub_one").join("sub_two"),
            backup: Backup::None,
            preserve_timestamps: false,
            make_all_directories: true,
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

        assert_eq!(
            read_to_string(root.join("destination/sub_one/sub_two/a.txt")).unwrap(),
            "a"
        );
    }

    #[test]
    fn copy_files_reports_when_unable_to_create_partial_destination() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_reports_when_unable_to_create_partial_destination")
            .expect("unable to create test root");

        let cwd = env::current_dir().expect("unable to get current directory");

        new_file_with_content(root.join("a.txt"), "a").expect("unable to create a.txt");

        let operation = Operation::CopyFiles {
            files: vec![root.join("a.txt")],
            destination: PathBuf::from("readonly_directory").join("sub"),
            backup: Backup::None,
            preserve_timestamps: false,
            make_all_directories: false,
            verbose: false,
        };

        let success = operation.execute(&cwd, &mut err_out);

        assert_eq!(success, false, "execution should report failure");

        assert!(err_out.contains(
            format!(
                "winstall: cannot create directory '{}': Permission denied",
                Path::new("readonly_directory").join("sub").display()
            )
            .as_str()
        ));
    }

    #[test]
    fn copy_files_reports_when_unable_to_create_full_destination() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("copy_files_reports_when_unable_to_create_full_destination")
            .expect("unable to create test root");

        let cwd = env::current_dir().expect("unable to get current directory");

        new_file_with_content(root.join("a.txt"), "a").expect("unable to create a.txt");

        let operation = Operation::CopyFiles {
            files: vec![root.join("a.txt")],
            destination: PathBuf::from("readonly_directory").join("sub").join("dir"),
            backup: Backup::None,
            preserve_timestamps: false,
            make_all_directories: true,
            verbose: false,
        };

        let success = operation.execute(&cwd, &mut err_out);

        assert_eq!(success, false, "execution should report failure");

        assert!(err_out.contains(
            format!(
                "winstall: cannot create directory '{}': Permission denied",
                Path::new("readonly_directory")
                    .join("sub")
                    .join("dir")
                    .display()
            )
            .as_str()
        ));
    }

    #[test]
    fn create_directories_creates_directories() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("create_directories_creates_directories")
            .expect("unable to create test root");

        let operation = Operation::CreateDirectories {
            directories: vec![
                PathBuf::from("a/nested/directory"),
                PathBuf::from("top_level"),
            ],
            verbose: false,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

        assert!(root.join("a").join("nested").join("directory").is_dir());
        assert!(root.join("top_level").is_dir());
    }

    #[test]
    fn create_directory_reports_creation_in_verbose_mode() {
        let mut err_out = TestOutputWriter::new();
        let root = Interim::new("create_directory_reports_creation_in_verbose_mode")
            .expect("unable to create test root");

        let operation = Operation::CreateDirectories {
            directories: vec![PathBuf::from("my/directory")],
            verbose: true,
        };

        let success = operation.execute(&root, &mut err_out);

        assert_eq!(success, true, "execution should report success");

        assert!(err_out.contains(
            format!(
                "winstall: creating directory '{}'",
                Path::new("my/directory").display()
            )
            .as_str()
        ));
    }

    #[test]
    fn create_directory_reports_permission_denied_errors() {
        let mut err_out = TestOutputWriter::new();
        let cwd = env::current_dir().expect("unable to get current directory");

        let operation = Operation::CreateDirectories {
            directories: vec![PathBuf::from("readonly_directory").join("invalid")],
            verbose: false,
        };

        let success = operation.execute(&cwd, &mut err_out);

        assert_eq!(success, false, "execution should report failure");

        assert!(err_out.contains(
            format!(
                "winstall: cannot create directory '{}': Permission denied",
                Path::new("readonly_directory").join("invalid").display()
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
    }

    impl Write for TestOutputWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }

    impl Debug for TestOutputWriter {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", String::from_utf8_lossy(&*self.0.clone()))
        }
    }
}
