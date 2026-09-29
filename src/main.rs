//! nomansland - a minimal command line greeter.
//!
//! Usage:
//!   nomansland [--name <name>] [--count <n>] [--help] [--]

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
  --                End of options; the next token is a plain value
";

/// Recognized flag tokens (the `--` terminator itself is handled separately).
fn is_flag_token(arg: &str) -> bool {
    matches!(arg, "--name" | "--count" | "--help" | "-h")
}

/// Read the value for the option whose token sits at index `i`.
///
/// A `--` token in the value slot is the end-of-options escape: it is
/// skipped and the token after it is taken verbatim as the value, so
/// `--name -- --help` greets `--help`. Returns the value and the index of
/// the first token after it.
fn read_value(args: &[String], i: usize, opt: &str) -> Result<(String, usize), String> {
    let mut j = i + 1;
    if args.get(j).map(String::as_str) == Some("--") {
        j += 1;
    }
    let value = args
        .get(j)
        .ok_or_else(|| format!("missing value for '{opt}'"))?;
    Ok((value.clone(), j + 1))
}

/// Parse a `--count` value, keeping the shared error style for both the
/// space-separated and `=` forms.
fn parse_count(arg: &str, value: &str) -> Result<u32, String> {
    let count = value.parse::<u32>().map_err(|_| {
        format!("invalid value for '{arg}': expected a positive integer, got '{value}'")
    })?;
    validate_count(arg, count)?;
    Ok(count)
}

/// Parse raw arguments into [`Options`].
///
/// Flag-like tokens are accepted as option values (`--name --help` greets
/// `--help`), with one exception: a recognized flag in the `--count` value
/// slot is reported as a missing value, since a flag can never be a valid
/// count. A `--` token ends option parsing; see [`read_value`].
///
/// Each of `--name` and `--count` may appear at most once, in either the
/// space or `=` form; a second occurrence is rejected as a duplicate
/// (issue #25). `--name` values must not be empty or whitespace-only.
///
/// Returns `Err(message)` when an argument is unknown, malformed, or
/// duplicated.
fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut opts = Options::default();
    let mut i = 0;
    let mut seen_name = false;
    let mut seen_count = false;

    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--" {
            // End of options. The program takes no positional values, so a
            // lone trailing `--` is a no-op and anything after it is an error.
            if let Some(rest) = args.get(i + 1) {
                return Err(format!("unexpected argument: '{rest}'"));
            }
            break;
        }
        match arg {
            "--help" | "-h" => {
                opts.help = true;
                return Ok(opts);
            }
            "--name" => {
                if seen_name {
                    return Err(format!("duplicate argument: '{arg}'"));
                }
                seen_name = true;
                let (value, next) = read_value(args, i, arg)?;
                validate_name(arg, &value)?;
                opts.name = value;
                i = next;
            }
            "--count" => {
                if seen_count {
                    return Err(format!("duplicate argument: '{arg}'"));
                }
                seen_count = true;
                // A recognized flag in the value slot means the number itself
                // is missing (`--count --name`), not that it is invalid.
                if args
                    .get(i + 1)
                    .map(String::as_str)
                    .is_some_and(is_flag_token)
                {
                    return Err(format!("missing value for '{arg}'"));
                }
                let (value, next) = read_value(args, i, arg)?;
                opts.count = parse_count(arg, &value)?;
                i = next;
            }
            other => {
                if let Some(rest) = other.strip_prefix("--name=") {
                    if seen_name {
                        return Err("duplicate argument: '--name'".to_string());
                    }
                    seen_name = true;
                    validate_name("--name", rest)?;
                    opts.name = rest.to_string();
                    i += 1;
                } else if let Some(rest) = other.strip_prefix("--count=") {
                    if seen_count {
                        return Err("duplicate argument: '--count'".to_string());
                    }
                    seen_count = true;
                    opts.count = parse_count("--count", rest)?;
                    i += 1;
                } else {
                    return Err(format!("unknown argument: '{other}'"));
                }
            }
        }
    }

    Ok(opts)
}

/// Ensure a `--name` value is not empty or whitespace-only (issue #25).
///
/// Validation only: the stored value is never trimmed, so names with
/// surrounding whitespace are still printed exactly as supplied.
fn validate_name(arg: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("invalid value for '{arg}': must not be empty"));
    }
    Ok(())
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

    // Edge: `--count --name` -> *missing* value error (issue #19): a
    // recognized flag in the value slot means the number itself is missing.
    #[test]
    fn count_with_flag_like_value_is_missing_not_invalid() {
        for flag in ["--name", "--count", "--help", "-h"] {
            assert_eq!(
                parse(&["--count", flag]),
                Err("missing value for '--count'".to_string())
            );
        }
    }

    // Edge: `--count -- --name` -> the `--` escape forces the flag-like token
    // to be taken as a literal value, so it is invalid, not missing.
    #[test]
    fn escaped_flag_like_count_value_is_invalid() {
        assert_eq!(
            parse(&["--count", "--", "--name"]),
            Err(
                "invalid value for '--count': expected a positive integer, got '--name'"
                    .to_string()
            )
        );
    }

    // Edge: `--name -- --help` -> `hello, --help!` (issue #19): the `--`
    // terminator in a value slot forces the next token as the value.
    #[test]
    fn double_dash_in_value_slot_forces_next_token_as_value() {
        let opts = parse(&["--name", "--", "--help"]).expect("valid");
        assert_eq!(
            opts,
            Options {
                name: "--help".to_string(),
                count: 1,
                help: false
            }
        );
        // A literal `--` can itself be forced as a value.
        let opts = parse(&["--name", "--", "--"]).expect("valid");
        assert_eq!(opts.name, "--");
        // The escape also works for a normal value.
        let opts = parse(&["--count", "--", "3"]).expect("valid");
        assert_eq!(opts.count, 3);
    }

    // Edge: `--name --` (nothing after the terminator) -> missing value.
    #[test]
    fn double_dash_at_end_of_value_slot_is_missing_value() {
        assert_eq!(
            parse(&["--name", "--"]),
            Err("missing value for '--name'".to_string())
        );
        assert_eq!(
            parse(&["--count", "--"]),
            Err("missing value for '--count'".to_string())
        );
    }

    // Edge: `--` / `--name ada --` -> trailing terminator is a no-op;
    // anything after it at the option position is rejected because the
    // program takes no positional values. `--name=--` sets a literal `--`.
    #[test]
    fn double_dash_terminates_option_parsing() {
        assert_eq!(parse(&["--"]), Ok(Options::default()));
        let opts = parse(&["--name", "ada", "--"]).expect("valid");
        assert_eq!(opts.name, "ada");
        assert_eq!(
            parse(&["--", "--name", "ada"]),
            Err("unexpected argument: '--name'".to_string())
        );
        let opts = parse(&["--name=--"]).expect("valid");
        assert_eq!(opts.name, "--");
    }

    // Row: `--name a --name b` -> error: duplicate argument: '--name',
    // exit 1 (issue #25: repeated flags are rejected, not last-wins).
    #[test]
    fn repeated_name_flag_is_rejected() {
        let expected = "duplicate argument: '--name'".to_string();
        for args in [
            &["--name", "a", "--name", "b"][..],
            &["--name=a", "--name=b"][..],
            &["--name=a", "--name", "b"][..],
            &["--name", "a", "--name=b"][..],
        ] {
            assert_eq!(parse(args), Err(expected.clone()), "{args:?}");
        }
    }

    // Row: `--count 2 --count 5` -> error: duplicate argument: '--count',
    // exit 1 (issue #25). The duplicate is reported at the flag token,
    // regardless of the values involved.
    #[test]
    fn repeated_count_flag_is_rejected() {
        let expected = "duplicate argument: '--count'".to_string();
        for args in [
            &["--count", "2", "--count", "5"][..],
            &["--count=2", "--count=5"][..],
            &["--count=2", "--count", "5"][..],
        ] {
            assert_eq!(parse(args), Err(expected.clone()), "{args:?}");
        }
        // A duplicate wins over any later value error on the same token.
        assert_eq!(parse(&["--count", "2", "--count", "abc"]), Err(expected));
    }

    // Edge (issue #25, constraint 4): the duplicate check participates in
    // the left-to-right short-circuit: it fires before a later --help.
    #[test]
    fn duplicate_before_help_wins() {
        assert_eq!(
            parse(&["--name", "a", "--name", "b", "--help"]),
            Err("duplicate argument: '--name'".to_string())
        );
        assert_eq!(
            parse(&["--count", "2", "--count", "3", "-h"]),
            Err("duplicate argument: '--count'".to_string())
        );
        // Conversely, --help seen first still short-circuits before any
        // later duplicate is reached.
        let opts = parse(&["--help", "--name", "a", "--name", "b"]).expect("ok");
        assert!(opts.help);
    }

    // Edge: a flag token consumed as a *value* does not count as a
    // duplicate occurrence of that flag (`--name --name` greets `--name`).
    #[test]
    fn flag_like_value_is_not_a_duplicate() {
        let opts = parse(&["--name", "--name"]).expect("valid");
        assert_eq!(opts.name, "--name");
    }

    // Row: `--name ""` / `--name=` / whitespace-only -> error, exit 1
    // (issue #25: empty names are rejected, not greeted).
    #[test]
    fn empty_or_whitespace_name_is_rejected() {
        let expected = "invalid value for '--name': must not be empty".to_string();
        for args in [
            &["--name", ""][..],
            &["--name="][..],
            &["--name", "   "][..],
            &["--name= \t "][..],
        ] {
            assert_eq!(parse(args), Err(expected.clone()), "{args:?}");
        }
    }

    // Edge: validation must not normalise. Only the emptiness check trims;
    // the stored value keeps surrounding whitespace (README: printed
    // exactly as supplied).
    #[test]
    fn name_value_is_stored_verbatim_not_trimmed() {
        let opts = parse(&["--name", " ada "]).expect("valid");
        assert_eq!(opts.name, " ada ");
        let opts = parse(&["--name=\tx\t"]).expect("valid");
        assert_eq!(opts.name, "\tx\t");
    }

    // Edge: `--name a --name b` fails before the second value is even
    // inspected, so an invalid second value cannot mask the duplicate.
    #[test]
    fn duplicate_is_reported_at_the_flag_token() {
        assert_eq!(
            parse(&["--name", "a", "--name", ""]),
            Err("duplicate argument: '--name'".to_string())
        );
        assert_eq!(
            parse(&["--name", "a", "--name"]),
            Err("duplicate argument: '--name'".to_string())
        );
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
        assert!(
            USAGE.contains("--                End of options; the next token is a plain value")
        );
        assert!(USAGE.ends_with("the next token is a plain value\n"));
    }
}
