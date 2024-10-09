use std::env;

fn main() {
    let context = winstall::get_options(env::args().skip(1));
    println!("{context:?}");
}

mod winstall {
    #[derive(PartialEq, Debug, Default)]
    pub struct Options {
        verbose: bool,
        create_parents: bool,
        directory_args: bool,
        preserve_timestamps: bool,
    }

    pub fn get_options<T: Iterator<Item = String>>(args: T) -> Options {
        let collected = args.collect::<Vec<_>>();
        let index_at_end = |index: usize| index < collected.len();

        /*
        The options that are supported from `install` are:
            [ ] --help*
            [ ] --version*
            [ ] -b, --backup[=method]
            [x] -p, --preserve-timestamps
            [x] -d, --directory
            [x] -v, --verbose
            [ ] -t, --target-directory=DIRECTORY
            [x] -D
            [ ] -S, --suffix=SUFFIX

        Items marked with a * are halting options - the standard execution
        is prevented, and an alternate action takes place - so they're not
        context items but some sort of "Action".
        */

        let mut idx = 0;
        let mut context = Options::default();

        while index_at_end(idx) {
            let opt_or_arg = collected[idx].split("=").nth(0).unwrap();
            match opt_or_arg {
                "-v" | "--verbose" => context.verbose = true,
                "-D" => context.create_parents = true,
                "-d" | "--directory" => context.directory_args = true,
                "-p" | "--preserve-timestamps" => context.preserve_timestamps = true,
                _ => (),
            }

            idx += 1;
        }

        context
    }

    #[cfg(test)]
    mod tests {
        use crate::winstall::{get_options, Options};

        #[test]
        pub fn test_get_options() {
            struct TestCase<'a> {
                args: Vec<&'a str>,
                expected: Options,
            }

            let tests: Vec<TestCase> = vec![
                TestCase {
                    args: vec!["--verbose"],
                    expected: Options {
                        verbose: true,
                        ..Default::default()
                    },
                },
                TestCase {
                    args: vec!["-v"],
                    expected: Options {
                        verbose: true,
                        ..Default::default()
                    },
                },
                TestCase {
                    args: vec!["-D"],
                    expected: Options {
                        create_parents: true,
                        ..Default::default()
                    },
                },
                TestCase {
                    args: vec!["--directory"],
                    expected: Options {
                        directory_args: true,
                        ..Default::default()
                    },
                },
                TestCase {
                    args: vec!["-d"],
                    expected: Options {
                        directory_args: true,
                        ..Default::default()
                    },
                },
                TestCase {
                    args: vec!["--preserve-timestamps"],
                    expected: Options {
                        preserve_timestamps: true,
                        ..Default::default()
                    },
                },
                TestCase {
                    args: vec!["-p"],
                    expected: Options {
                        preserve_timestamps: true,
                        ..Default::default()
                    },
                },
            ];

            for test in tests {
                let args = test.args.iter().map(|arg| arg.to_string());
                let outcome = get_options(args);
                assert_eq!(test.expected, outcome, "args: {:?}", test.args);
            }
        }
    }
}
