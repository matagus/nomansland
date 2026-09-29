//! nomansland - a minimal command line greeter.
//!
//! Usage:
//!   nomansland [--name <name>] [--count <n>] [--help]

use std::env;
use std::process::ExitCode;

const DEFAULT_NAME: &str = "world";

/// Parsed command line options.
#[derive(Debug, PartialEq, Eq)]
struct Options {
    name: String,
    count: u32,
    help: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            name: DEFAULT_NAME.to_string(),
            count: 1,
            help: false,
        }
    }
}

const USAGE: &str = "\
nomansland - say hello from the no man's land

Usage:
  nomansland [OPTIONS]

Options:
  --name <name>     Who to greet (default: \"world\")
  --count <n>       How many times to greet (default: 1)
  --help, -h        Show this help text and exit
";

/// Parse raw arguments into [`Options`].
///
/// Returns `Err(message)` when an argument is unknown or malformed.
fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut opts = Options::default();
    let mut i = 0;

    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--help" | "-h" => {
                opts.help = true;
                return Ok(opts);
            }
            "--name" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("missing value for '{arg}'"))?;
                opts.name = value.clone();
                i += 2;
            }
            "--count" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("missing value for '{arg}'"))?;
                opts.count = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid value for '{arg}': expected a positive integer, got '{value}'"))?;
                validate_count(arg, opts.count)?;
                i += 2;
            }
            other => {
                if let Some(rest) = other.strip_prefix("--name=") {
                    opts.name = rest.to_string();
                    i += 1;
                } else if let Some(rest) = other.strip_prefix("--count=") {
                    opts.count = rest.parse::<u32>().map_err(|_| {
                        format!("invalid value for '--count': expected a positive integer, got '{rest}'")
                    })?;
                    validate_count("--count", opts.count)?;
                    i += 1;
                } else {
                    return Err(format!("unknown argument: '{other}'"));
                }
            }
        }
    }

    Ok(opts)
}

/// Ensure the greeting count is a positive integer, matching the error style
/// used for malformed `--count` values.
fn validate_count(arg: &str, count: u32) -> Result<(), String> {
    if count == 0 {
        return Err(format!(
            "invalid value for '{arg}': expected a positive integer, got '0'"
        ));
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let opts = match parse_args(&args) {
        Ok(opts) => opts,
        Err(err) => {
            eprintln!("error: {err}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    if opts.help {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    for _ in 0..opts.count {
        println!("hello, {}!", opts.name);
    }

    ExitCode::SUCCESS
}

/// Unit tests encoding the README "Behaviour" table (and the extra edge cases
/// pinned down in issue #16) as a machine-checked contract for `parse_args`,
/// `validate_count` and `USAGE`.
///
/// stdout text and exit codes are asserted end-to-end in `tests/cli.rs`,
/// which spawns the built binary via `CARGO_BIN_EXE_nomansland`.
#[cfg(test)]
mod tests {
    use super::*;

    /// Build an argument vector the way `main` sees it (program name skipped).
    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    /// Parse raw string arguments through the real [`parse_args`] entry point.
    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_args(&argv(args))
    }

    // Row: *(no arguments)* -> `hello, world!`, exit 0
    #[test]
    fn no_args_yields_default_greeting() {
        assert_eq!(parse(&[]), Ok(Options::default()));
        assert_eq!(
            parse(&[]),
            Ok(Options {
                name: "world".to_string(),
                count: 1,
                help: false
            })
        );
    }

    // Row: `--name ada` -> `hello, ada!`, exit 0
    #[test]
    fn name_flag_sets_greeting_name() {
        let opts = parse(&["--name", "ada"]).expect("valid");
        assert_eq!(opts.name, "ada");
        assert_eq!(opts.count, 1);
        assert!(!opts.help);
    }

    // Rows: `--name=ada --count 3` -> three greeting lines, exit 0
    #[test]
    fn equals_forms_and_mixed_style_parse() {
        let opts = parse(&["--name=ada", "--count", "3"]).expect("valid");
        assert_eq!(
            opts,
            Options {
                name: "ada".to_string(),
                count: 3,
                help: false
            }
        );
        let opts = parse(&["--name", "ada", "--count=3"]).expect("valid");
        assert_eq!(
            opts,
            Options {
                name: "ada".to_string(),
                count: 3,
                help: false
            }
        );
    }

    // Row: `--count 0` / `--count=0` -> error, exit 1 (both forms share the message)
    #[test]
    fn count_zero_is_rejected_in_both_forms() {
        let expected = "invalid value for '--count': expected a positive integer, got '0'";
        assert_eq!(parse(&["--count", "0"]), Err(expected.to_string()));
        assert_eq!(parse(&["--count=0"]), Err(expected.to_string()));
    }

    // Row: `--count abc` -> error, exit 1
    #[test]
    fn count_non_numeric_is_rejected() {
        assert_eq!(
            parse(&["--count", "abc"]),
            Err("invalid value for '--count': expected a positive integer, got 'abc'".to_string())
        );
    }

    // Edge: `--count=` (empty value) -> error mentioning ''
    #[test]
    fn count_empty_value_is_rejected() {
        assert_eq!(
            parse(&["--count="]),
            Err("invalid value for '--count': expected a positive integer, got ''".to_string())
        );
    }

    // Edge: `--count -3` -> rejected by the u32 parse (no bundled shorts, no negatives)
    #[test]
    fn count_negative_is_rejected() {
        assert_eq!(
            parse(&["--count", "-3"]),
            Err("invalid value for '--count': expected a positive integer, got '-3'".to_string())
        );
    }

    // Edge: `--count 4294967296` -> u32 overflow rejected
    #[test]
    fn count_u32_overflow_is_rejected() {
        assert_eq!(
            parse(&["--count", "4294967296"]),
            Err("invalid value for '--count': expected a positive integer, got '4294967296'".to_string())
        );
    }

    // Edge: `--count 1_0` -> Rust digit separators are not accepted by str::parse
    #[test]
    fn count_digit_separators_are_rejected() {
        assert_eq!(
            parse(&["--count", "1_0"]),
            Err("invalid value for '--count': expected a positive integer, got '1_0'".to_string())
        );
    }

    // Edge: `--count " 2"` -> parse does not trim
    #[test]
    fn count_leading_space_is_rejected() {
        assert_eq!(
            parse(&["--count", " 2"]),
            Err("invalid value for '--count': expected a positive integer, got ' 2'".to_string())
        );
    }

    // Row: `--name` (no value) -> error: missing value for '--name', exit 1
    #[test]
    fn name_without_value_is_missing_value_error() {
        assert_eq!(parse(&["--name"]), Err("missing value for '--name'".to_string()));
    }

    #[test]
    fn count_without_value_is_missing_value_error() {
        assert_eq!(parse(&["--count"]), Err("missing value for '--count'".to_string()));
    }

    // Row: `--bogus` -> error: unknown argument: '--bogus', exit 1
    #[test]
    fn unknown_long_flag_is_rejected() {
        assert_eq!(parse(&["--bogus"]), Err("unknown argument: '--bogus'".to_string()));
    }

    // Row: `-x` / `-` -> unknown arguments; single-dash forms are not bundled shorts
    #[test]
    fn single_dash_forms_are_unknown() {
        assert_eq!(parse(&["-x"]), Err("unknown argument: '-x'".to_string()));
        assert_eq!(parse(&["-"]), Err("unknown argument: '-'".to_string()));
    }

    // Row: `--help`, `-h` -> usage text, exit 0
    #[test]
    fn help_flags_set_help() {
        for flag in ["--help", "-h"] {
            let opts = parse(&[flag]).unwrap_or_else(|err| panic!("{flag} failed: {err}"));
            assert!(opts.help, "{flag} should set help");
        }
    }

    // Edge: `--name ada --count 3 -h` -> only usage, exit 0.
    // --help short-circuits immediately and stops parsing.
    #[test]
    fn help_short_circuits_earlier_flags() {
        let opts = parse(&["--name", "ada", "--count", "3", "-h"]).expect("ok");
        assert!(opts.help);
        // Earlier flags were consumed before the short-circuit, but main only
        // prints usage when help is set.
        assert_eq!(opts.name, "ada");
        assert_eq!(opts.count, 3);
    }

    // Edge: `--bogus --help` -> error, exit 1. First failure wins.
    #[test]
    fn earlier_error_wins_over_later_help() {
        assert_eq!(
            parse(&["--bogus", "--help"]),
            Err("unknown argument: '--bogus'".to_string())
        );
    }

    // Edge: `--name --help` -> `hello, --help!`, exit 0. No -- end-of-options.
    #[test]
    fn value_flags_consume_flag_like_tokens() {
        assert_eq!(
            parse(&["--name", "--help"]),
            Ok(Options {
                name: "--help".to_string(),
                count: 1,
                help: false
            })
        );
    }

    // Edge: `--count --name` -> *invalid* (not missing) value error.
    #[test]
    fn count_with_flag_like_value_is_invalid_not_missing() {
        assert_eq!(
            parse(&["--count", "--name"]),
            Err("invalid value for '--count': expected a positive integer, got '--name'".to_string())
        );
    }

    // Edge: `--name a --name b` -> `hello, b!`, last one silently wins.
    #[test]
    fn repeated_name_flag_is_last_wins() {
        let opts = parse(&["--name", "a", "--name", "b"]).expect("valid");
        assert_eq!(opts.name, "b");
    }

    // Edge: `--name ""` and `--name=` -> `hello, !`, empty names accepted.
    #[test]
    fn empty_name_is_accepted() {
        for args in [&["--name", ""][..], &["--name="][..]] {
            let opts = parse(args).unwrap_or_else(|err| panic!("{args:?} failed: {err}"));
            assert_eq!(opts.name, "");
            assert!(!opts.help);
        }
    }

    /// The usage block is printed verbatim on --help and after every error;
    /// pin its shape so the README description of it stays true.
    #[test]
    fn usage_text_documents_every_option() {
        assert!(USAGE.starts_with("nomansland - say hello from the no man's land\n"));
        assert!(USAGE.contains("Usage:\n  nomansland [OPTIONS]\n"));
        assert!(USAGE.contains("--name <name>     Who to greet (default: \"world\")"));
        assert!(USAGE.contains("--count <n>       How many times to greet (default: 1)"));
        assert!(USAGE.contains("--help, -h        Show this help text and exit"));
        assert!(USAGE.ends_with("Show this help text and exit\n"));
    }
}
