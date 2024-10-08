use std::env;

fn main() {
    let context = winstall::get_context(env::args().skip(1));
    println!("{context:?}");
}

mod winstall {
    #[derive(PartialEq, Debug)]
    pub struct Context {
        verbose: bool,
        create_parents: bool,
    }

    pub fn get_context<T: Iterator<Item = String>>(args: T) -> Context {
        let collected = args.collect::<Vec<_>>();
        let index_at_end = |index: usize| index < collected.len();

        /*
        The options that are supported from `install` are:
            [ ] --help*
            [ ] --version*
            [ ] -b, --backup[=method]
            [ ] -p, --preserve-timestamps
            [ ] -d, --directory
            [x] -v, --verbose
            [ ] -t, --target-directory=DIRECTORY
            [x] -D
            [ ] -S, --suffix=SUFFIX

        Items marked with a * are halting options - the standard execution
        is prevented, and an alternate action takes place - so they're not
        context items but some sort of "Action".
        */

        let mut idx = 0;
        let mut context = Context { verbose: false, create_parents: false };

        while index_at_end(idx) {
            let opt_or_arg = collected[idx].split("=").nth(0).unwrap();
            match opt_or_arg {
                "-v" | "--verbose" => context.verbose = true,
                "-D" => context.create_parents = true,
                _ => (),
            }

            idx += 1;
        }

        context
    }

    #[cfg(test)]
    mod tests {
        use crate::winstall::{get_context, Context};

        #[test]
        pub fn test_get_context() {
            struct TestCase<'a> {
                args: Vec<&'a str>,
                expected: Context,
            }

            let tests: Vec<TestCase> = vec![
                TestCase {
                    args: vec!["--verbose"],
                    expected: Context { verbose: true, create_parents: false },
                },
                TestCase {
                    args: vec!["-v"],
                    expected: Context { verbose: true, create_parents: false },
                },
                TestCase {
                    args: vec!["-D"],
                    expected: Context { verbose: false, create_parents: true },
                },
            ];

            for test in tests {
                let args = test.args.iter().map(|arg| arg.to_string());
                let outcome = get_context(args);
                assert_eq!(test.expected, outcome, "args: {:?}", test.args);
            }
        }
    }
}
