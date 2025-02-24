enum Backup {
    Numbered,
    Simple(String),
    Existing(String),
}

struct Options {
    backup: Option<Option<String>>,
    suffix: Option<String>,
    verbose: bool,
    preserve_timestamps: bool,
    make_all_directories: bool,
    no_target_directory: bool,
    target_directory: Option<String>,
    directory_arguments: bool,
}

fn main() {
    let mut opts = Options {
        backup: None,
        suffix: None,
        verbose: false,
        preserve_timestamps: false,
        make_all_directories: false,
        no_target_directory: false,
        target_directory: None,
        directory_arguments: false,
    };

    let mut args = Vec::<String>::new();

    let mut peekable = std::env::args().skip(1).peekable();
    'arguments: while let Some(arg) = peekable.next() {
        let mut split = arg.split('=');
        let argument = split.next().unwrap();

        let mut try_capture =
            || -> Option<String> { split.next().map(str::to_owned).or_else(|| peekable.next()) };

        'recognized: {
            match argument {
                "-v" | "--verbose" => opts.verbose = true,
                "-p" | "--preserve-timestamps" => opts.preserve_timestamps = true,
                "-T" | "--no-target-directory" => opts.no_target_directory = true,
                "-D" => opts.make_all_directories = true,
                "-d" | "--directory" => opts.directory_arguments = true,
                "-b" => opts.backup = Some(None),
                "--backup" => opts.backup = Some(split.next().map(str::to_owned)),
                "-S" | "--suffix" => match try_capture() {
                    Some(s) => opts.suffix = Some(s),
                    None => {
                        eprintln!("winstall: option --suffix (-S) requires an argument");
                        eprintln!("Try 'winstall --help' for more information.");
                        std::process::exit(1);
                    }
                },
                "-t" | "--target-directory" => match try_capture() {
                    Some(s) => opts.target_directory = Some(s),
                    None => {
                        eprintln!("winstall: option --target-directory (-t) requires an argument");
                        eprintln!("Try 'winstall --help' for more information.");
                        std::process::exit(1);
                    }
                },
                "--help" => {
                    println!(include_str!("usage.txt"));
                    std::process::exit(0);
                }
                "--version" => {
                    println!(include_str!("version.txt"));
                    std::process::exit(0);
                }

                // Ignored UNIX specific options that don't expect a value (or expect an equals
                // separated one).
                "-C" | "--compare" | "--debug" | "-g" | "-m" | "-o" | "--preserve-context"
                | "-s" | "--strip" | "-Z" | "--context" => (),

                // Ignored UNIX specific options that do expect a value
                "--group" | "--mode" | "--owner" => {
                    if try_capture().is_none() {
                        eprintln!(
                            "winstall: unix compatability option '{}' requires an argument",
                            argument
                        );

                        std::process::exit(1);
                    }

                    ()
                }
                _ => break 'recognized,
            }

            continue 'arguments;
        }

        args.push(argument.to_owned());
    }

    if args.is_empty() {
        eprintln!("winstall: missing file operand");
        eprintln!("Try 'winstall --help' for more information.");
        std::process::exit(1);
    }

    if opts.no_target_directory && opts.target_directory.is_some() {
        eprintln!("winstall: cannot combine --target-directory (-t) and no-target-directory (-T)");
        std::process::exit(1);
    }

    if opts.directory_arguments {
        let mut was_error = false;

        for directory in args.iter() {
            if !create_directory(directory, true, opts.verbose) {
                was_error = true;
            }
        }

        std::process::exit(if was_error { 1 } else { 0 });
    }

    if args.len() < 2 {
        eprintln!(
            "winstall: missing destination file operand after '{}'",
            args[0]
        );

        eprintln!("Try 'winstall --help' for more information.");
        std::process::exit(1);
    }

    let backup_method = opts.backup.and_then(|o| {
        let suffix = opts
            .suffix
            .or(std::env::var("SIMPLE_BACKUP_SUFFIX").ok())
            .unwrap_or("~".to_string());

        o.and_then(|mode| match mode.as_str() {
            "none" | "off" => None,
            "numbered" | "t" => Some(Backup::Numbered),
            "simple" | "never" => Some(Backup::Simple(suffix.clone())),
            "existing" | "nil" => Some(Backup::Existing(suffix.clone())),
            _ => {
                eprintln!(
                    concat!(
                        "install: invalid argument ‘{}’ for ‘backup type’\n",
                        "Valid arguments are:\n",
                        "  - ‘none’, ‘off’\n",
                        "  - ‘simple’, ‘never’\n",
                        "  - ‘existing’, ‘nil’\n",
                        "  - ‘numbered’, ‘t’\n",
                        "Try 'install --help' for more information.",
                    ),
                    mode
                );

                std::process::exit(1);
            }
        })
        .or(Some(Backup::Existing(suffix.clone())))
    });

    let is_file_target =
        opts.no_target_directory || (args.len() == 2 && !std::path::Path::new(&args[1]).is_dir());

    match is_file_target {
        true => file_target(
            &args[0],
            &args[1],
            backup_method,
            opts.make_all_directories,
            opts.preserve_timestamps,
            opts.verbose,
        ),
        false => {
            let target = opts.target_directory.unwrap_or_else(|| args.pop().unwrap());
            directory_target(
                args,
                target,
                backup_method,
                opts.make_all_directories,
                opts.preserve_timestamps,
                opts.verbose,
            );
        }
    }
}

fn create_directory<P: AsRef<std::path::Path>>(
    p: P,
    make_all_directories: bool,
    verbose: bool,
) -> bool {
    let result = match make_all_directories {
        true => std::fs::create_dir_all(p.as_ref()),
        false => std::fs::create_dir(p.as_ref()),
    };

    match result {
        Ok(_) => {
            if verbose {
                eprintln!("winstall: creating directory '{}'", p.as_ref().display());
            }
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::AlreadyExists => (),
            _ => {
                eprintln!(
                    "winstall: cannot create directory '{}': {}",
                    p.as_ref().display(),
                    e
                );

                return false;
            }
        },
    }

    true
}

fn file_target<F: AsRef<std::path::Path>, T: AsRef<std::path::Path>>(
    from: F,
    to: T,
    backup_method: Option<Backup>,
    make_all_directories: bool,
    preserve_timestamps: bool,
    verbose: bool,
) {
    if from.as_ref().is_dir() {
        eprintln!("winstall: omitting directory '{}'", from.as_ref().display());
        std::process::exit(1);
    }

    let parent = to
        .as_ref()
        .parent()
        .and_then(|p| {
            if p == std::path::Path::new("") {
                return None;
            }

            Some(p)
        })
        .unwrap_or(std::path::Path::new("."));

    if !create_directory(parent, make_all_directories, verbose) {
        std::process::exit(1);
    }

    let success = copy_file(
        from.as_ref(),
        to.as_ref(),
        &backup_method,
        preserve_timestamps,
        verbose,
    );

    std::process::exit(if success { 0 } else { 1 });
}

fn directory_target<F: AsRef<std::path::Path>, T: AsRef<std::path::Path>>(
    files: Vec<F>,
    target: T,
    backup_method: Option<Backup>,
    make_all_directories: bool,
    preserve_timestamps: bool,
    verbose: bool,
) {
    if !create_directory(target.as_ref(), make_all_directories, verbose) {
        std::process::exit(1);
    }

    let mut any_errors = false;

    for file in files {
        if file.as_ref().is_dir() {
            eprintln!("winstall: omitting directory '{}'", file.as_ref().display());
            continue;
        }

        let source_name = file
            .as_ref()
            .file_name()
            .expect("source file should have name");

        let dest_path = target.as_ref().join(source_name);

        let success = copy_file(
            file.as_ref(),
            dest_path,
            &backup_method,
            preserve_timestamps,
            verbose,
        );

        if !success {
            any_errors = true;
        }
    }

    std::process::exit(if !any_errors { 0 } else { 1 });
}

fn copy_file<F: AsRef<std::path::Path>, T: AsRef<std::path::Path>>(
    from: F,
    to: T,
    backup_method: &Option<Backup>,
    preserve_timestamps: bool,
    verbose: bool,
) -> bool {
    let mut source = match std::fs::OpenOptions::new().read(true).open(from.as_ref()) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "winstall: cannot open file to read '{}': {}",
                from.as_ref().display(),
                e
            );

            return false;
        }
    };

    let timestamps = if preserve_timestamps {
        source
            .metadata()
            .and_then(|m| {
                Ok(Option::zip(
                    m.accessed()
                        .map_err(|e| {
                            eprintln!(
                                "winstall: unable to get last accessed time for '{}': {}",
                                from.as_ref().display(),
                                e
                            );

                            e
                        })
                        .ok(),
                    m.modified()
                        .map_err(|e| {
                            eprintln!(
                                "winstall: unable to get last modified time for '{}': {}",
                                from.as_ref().display(),
                                e
                            );

                            e
                        })
                        .ok(),
                )
                .and_then(|(accessed, modified)| {
                    Some(
                        std::fs::FileTimes::new()
                            .set_accessed(accessed)
                            .set_modified(modified),
                    )
                }))
            })
            .unwrap_or(None)
    } else {
        None
    };

    let mut backup_path = None::<std::path::PathBuf>;

    let mut dest = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to.as_ref())
    {
        Ok(f) => f,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                eprintln!(
                    "winstall: cannot open file to write '{}': {}",
                    to.as_ref().display(),
                    e
                );

                return false;
            }

            let backup_file = match backup_method {
                None => std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(to.as_ref())
                    .and_then(|f| {
                        if verbose {
                            eprintln!("removed '{}'", to.as_ref().display())
                        }

                        Ok(f)
                    }),
                Some(b) => {
                    let name = match b {
                        Backup::Simple(suffix) => add_suffix(to.as_ref(), suffix),
                        Backup::Numbered => next_numbered_backup(to.as_ref()).0,
                        Backup::Existing(suffix) => match next_numbered_backup(to.as_ref()) {
                            (_, true) => add_suffix(to.as_ref(), suffix),
                            (numbered, false) => numbered,
                        },
                    };

                    _ = std::fs::rename(to.as_ref(), &name).map_err(|e| {
                        eprintln!(
                            "winstall: unable preserve '{}' as backup '{}': {}",
                            to.as_ref().display(),
                            name.display(),
                            e
                        )
                    });

                    backup_path = Some(name.clone());

                    std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(to.as_ref())
                }
            };

            match backup_file {
                Ok(f) => f,
                Err(e) => {
                    eprintln!(
                        "winstall: cannot open file to write '{}': {}",
                        to.as_ref().display(),
                        e
                    );

                    return false;
                }
            }
        }
    };

    match std::io::copy(&mut source, &mut dest) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("winstall: cannot copy file: {}", e);
            return false;
        }
    };

    if let Some(t) = timestamps {
        if let Err(e) = dest.set_times(t) {
            eprintln!(
                "winstall: unable to set file times for '{}': {}",
                to.as_ref().display(),
                e
            );
        }
    }

    if verbose {
        print!(
            "'{}' -> '{}'",
            from.as_ref().display(),
            to.as_ref().display()
        );

        backup_path.map(|path| print!(" (backup: '{}')", path.display()));

        print!("\n");
    }

    true
}

fn next_numbered_backup<P: AsRef<std::path::Path>>(p: P) -> (std::path::PathBuf, bool) {
    let parent = p
        .as_ref()
        .parent()
        .and_then(|parent| {
            if parent == std::path::Path::new("") {
                None
            } else {
                Some(parent)
            }
        })
        .unwrap_or(std::path::Path::new("."));

    let file_name = p
        .as_ref()
        .file_name()
        .expect("file argument should have a name")
        .to_string_lossy()
        .to_string();

    std::fs::read_dir(parent)
        .and_then(|entries| {
            let mut max = 0;

            for entry in entries {
                _ = entry.map(|e| {
                    let entry_name = e.file_name().to_string_lossy().to_string();
                    if entry_name.starts_with(&file_name) && entry_name.ends_with("~") {
                        let num = entry_name
                            .strip_prefix(&file_name)
                            .and_then(|s| s.strip_prefix(".~"))
                            .and_then(|s| s.strip_suffix("~"))
                            .and_then(|s| s.parse::<u32>().ok());

                        num.map(|n| max = n.max(max));
                    }
                });
            }

            Ok((add_suffix(p.as_ref(), &format!(".~{}~", max + 1)), max == 0))
        })
        .unwrap_or((add_suffix(p.as_ref(), ".~1~"), true))
}

fn add_suffix<P: AsRef<std::path::Path>>(p: P, suffix: &str) -> std::path::PathBuf {
    p.as_ref().with_file_name(format!(
        "{}{}",
        p.as_ref()
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or("".into()),
        suffix,
    ))
}
