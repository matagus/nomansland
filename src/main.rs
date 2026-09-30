//! nomansland - a minimal command line greeter.
//!
//! Argument parsing is delegated entirely to clap: `--help` prints clap's
//! generated usage, and every parse failure exits with clap's standard error
//! shape (`error: …` on stderr, exit code 2).

use std::process::ExitCode;

use clap::Parser;

const DEFAULT_NAME: &str = "world";

/// Say hello from the no man's land.
#[derive(Debug, Parser)]
#[command(name = "nomansland", bin_name = "nomansland")]
struct Cli {
    /// Who to greet
    #[arg(
        long,
        value_name = "name",
        default_value = DEFAULT_NAME,
        // Flag-like strings are valid names: `--name --help` greets `--help`.
        allow_hyphen_values = true,
        value_parser = parse_name
    )]
    name: String,

    /// How many times to greet
    #[arg(
        long,
        value_name = "n",
        default_value_t = 1,
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    count: u32,
}

/// Reject empty or whitespace-only names (issue #25) without normalising:
/// accepted values are kept exactly as supplied.
fn parse_name(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        return Err("must not be empty".to_string());
    }
    Ok(value.to_string())
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    for _ in 0..cli.count {
        println!("hello, {}!", cli.name);
    }

    ExitCode::SUCCESS
}

/// Unit tests for the clap-based command line contract. Parsing is exercised
/// through [`Cli::try_parse_from`]; stdout text and exit codes are asserted
/// end-to-end in `tests/cli.rs`, which spawns the built binary via
/// `CARGO_BIN_EXE_nomansland`.
#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    /// Parse raw string arguments through the real clap entry point.
    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("nomansland").chain(args.iter().copied()))
    }

    fn parse_ok(args: &[&str]) -> Cli {
        parse(args).unwrap_or_else(|err| panic!("{args:?} should parse: {err}"))
    }

    fn err_kind(args: &[&str]) -> ErrorKind {
        parse(args)
            .err()
            .unwrap_or_else(|| panic!("{args:?} should fail"))
            .kind()
    }

    // Row: *(no arguments)* -> defaults.
    #[test]
    fn no_args_yields_defaults() {
        let cli = parse_ok(&[]);
        assert_eq!(cli.name, "world");
        assert_eq!(cli.count, 1);
    }

    // Rows: `--name ada`, `--name=ada`, `--count 3`, `--count=3`.
    #[test]
    fn space_and_equals_forms_parse() {
        let cli = parse_ok(&["--name", "ada"]);
        assert_eq!(cli.name, "ada");
        assert_eq!(cli.count, 1);

        let cli = parse_ok(&["--name=ada", "--count", "3"]);
        assert_eq!(cli.name, "ada");
        assert_eq!(cli.count, 3);

        let cli = parse_ok(&["--name", "ada", "--count=3"]);
        assert_eq!((cli.name.as_str(), cli.count), ("ada", 3));
    }

    // Row: `--name --help` -> flag-like values are accepted as names, and a
    // flag-like value is not a duplicate occurrence (`--name --name`).
    #[test]
    fn name_accepts_flag_like_values() {
        assert_eq!(parse_ok(&["--name", "--help"]).name, "--help");
        assert_eq!(parse_ok(&["--name", "--name"]).name, "--name");
        assert_eq!(parse_ok(&["--name", "-h"]).name, "-h");
    }

    // Row: `--help`, `-h` -> clap's generated help, signalled as DisplayHelp.
    #[test]
    fn help_flags_request_help() {
        for flag in ["--help", "-h"] {
            assert_eq!(err_kind(&[flag]), ErrorKind::DisplayHelp, "{flag}");
        }
        // Help still wins when it follows valid flags.
        assert_eq!(
            err_kind(&["--name", "ada", "--count", "3", "-h"]),
            ErrorKind::DisplayHelp
        );
    }

    // Row: `--bogus`, `-x`, `-` -> unknown arguments.
    #[test]
    fn unknown_arguments_are_rejected() {
        for arg in ["--bogus", "-x", "-"] {
            assert_eq!(err_kind(&[arg]), ErrorKind::UnknownArgument, "{arg}");
        }
        // The first error wins, left to right, even before a later --help.
        assert_eq!(err_kind(&["--bogus", "--help"]), ErrorKind::UnknownArgument);
    }

    // Row: `--count 0` / `abc` / overflow / empty -> invalid values. clap's
    // u32 range parser covers zero, negatives-as-flags, separators and
    // overflow in one shot.
    #[test]
    fn bad_count_values_are_rejected() {
        for value in ["0", "abc", "", "4294967296", "1_0", " 2"] {
            assert_eq!(
                err_kind(&["--count", value]),
                ErrorKind::ValueValidation,
                "--count {value:?}"
            );
        }
        assert_eq!(err_kind(&["--count=0"]), ErrorKind::ValueValidation);
        // Without allow_hyphen_values, `-3` is an unknown flag, not a value.
        assert_eq!(err_kind(&["--count", "-3"]), ErrorKind::UnknownArgument);
    }

    // Row: `--name` / `--count` with no value -> missing value errors.
    #[test]
    fn missing_values_are_rejected() {
        assert_eq!(err_kind(&["--name"]), ErrorKind::InvalidValue);
        assert_eq!(err_kind(&["--count"]), ErrorKind::InvalidValue);
    }

    // Row: repeated flags are rejected, in every form mix (issue #25), and
    // the duplicate is reported even when a --help follows it.
    #[test]
    fn duplicate_flags_are_rejected() {
        for args in [
            &["--name", "a", "--name", "b"][..],
            &["--name=a", "--name=b"][..],
            &["--name", "a", "--name=b"][..],
            &["--count", "2", "--count", "5"][..],
            &["--count=2", "--count=5"][..],
            &["--name", "a", "--name", "b", "--help"][..],
        ] {
            assert_eq!(err_kind(args), ErrorKind::ArgumentConflict, "{args:?}");
        }
        // Conversely, --help seen first short-circuits later duplicates.
        assert_eq!(
            err_kind(&["--help", "--name", "a", "--name", "b"]),
            ErrorKind::DisplayHelp
        );
    }

    // Row: `--name ""` / whitespace-only -> rejected (issue #25).
    #[test]
    fn empty_or_whitespace_name_is_rejected() {
        for args in [
            &["--name", ""][..],
            &["--name="][..],
            &["--name", "   "][..],
            &["--name=\t "][..],
        ] {
            assert_eq!(err_kind(args), ErrorKind::ValueValidation, "{args:?}");
        }
    }

    // Edge: validation never normalises; accepted names keep their padding.
    #[test]
    fn name_value_is_stored_verbatim_not_trimmed() {
        assert_eq!(parse_ok(&["--name", " ada "]).name, " ada ");
        assert_eq!(parse_ok(&["--name=\tx\t"]).name, "\tx\t");
    }

    // Row: `--` ends option parsing. A lone terminator is a no-op; anything
    // after it is rejected because the program takes no positional values.
    #[test]
    fn double_dash_terminates_option_parsing() {
        assert_eq!(parse_ok(&["--"]).name, "world");
        assert_eq!(parse_ok(&["--name", "ada", "--"]).name, "ada");
        assert_eq!(
            err_kind(&["--", "--name", "ada"]),
            ErrorKind::UnknownArgument
        );
        // `--name=--` still sets a literal `--`.
        assert_eq!(parse_ok(&["--name=--"]).name, "--");
    }
}
