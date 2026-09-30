//! End-to-end tests for the README "Behaviour" table.
//!
//! These run the real built binary (cargo provides its path via the
//! `CARGO_BIN_EXE_nomansland` env var) and assert on stdout text, stderr
//! shape and exit codes, so the documented output/exit-code contract is
//! machine-checked rather than manually verified.
//!
//! Argument parsing is clap's, so every error uses clap's standard shape:
//! `error: …` on stderr followed by usage or a help hint, exit code 2.

use std::process::{Command, Output};

/// Run the built binary with the given arguments.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nomansland"))
        .args(args)
        .output()
        .expect("failed to spawn the nomansland binary")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("process should exit with a code")
}

/// Every parse error goes to stderr as clap renders it: an `error: …` line
/// containing `fragment`, exit code 2, nothing on stdout.
fn assert_error_shape(out: &Output, args: &[&str], fragment: &str) {
    assert_eq!(code(out), 2, "expected clap's error exit code for {args:?}");
    assert_eq!(stdout(out), "", "errors must not write to stdout");
    let err = stderr(out);
    assert!(
        err.starts_with("error: "),
        "stderr did not start with 'error: ' for {args:?}:\n{err}"
    );
    assert!(
        err.contains(fragment),
        "stderr for {args:?} did not contain {fragment:?}:\n{err}"
    );
}

// Row: *(no arguments)* -> `hello, world!`, exit 0
#[test]
fn no_args_prints_default_greeting() {
    let out = run(&[]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, world!\n");
    assert_eq!(stderr(&out), "");
}

// Row: `--name ada` -> `hello, ada!`, exit 0
#[test]
fn name_flag_greets_the_given_name() {
    let out = run(&["--name", "ada"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, ada!\n");
}

// Row: `--name=ada --count 3` -> three greeting lines, exit 0
#[test]
fn count_flag_repeats_the_greeting() {
    let out = run(&["--name=ada", "--count", "3"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, ada!\nhello, ada!\nhello, ada!\n");
}

// Row: `--count 0` / `--count=0` -> clap range error, exit 2
#[test]
fn count_zero_fails() {
    for args in [&["--count", "0"][..], &["--count=0"][..]] {
        let out = run(args);
        assert_error_shape(&out, args, "invalid value '0' for '--count <n>'");
    }
}

// Row: `--count abc` -> clap u32 parse error, exit 2
#[test]
fn count_non_numeric_fails() {
    let out = run(&["--count", "abc"]);
    assert_error_shape(&out, &["--count", "abc"], "invalid value 'abc'");
}

// Edge: `--count=` (empty), overflow and digit separators are invalid
// values; `-3` is an unexpected argument because --count does not allow
// hyphen-leading values.
#[test]
fn count_rejects_surprising_numeric_forms() {
    for value in ["", "4294967296", "1_0", " 2"] {
        let args = ["--count", value];
        let out = run(&args);
        assert_error_shape(&out, &args, &format!("invalid value '{value}'"));
    }
    let out = run(&["--count", "-3"]);
    assert_error_shape(&out, &["--count", "-3"], "unexpected argument '-3'");
}

// Row: `--name` / `--count` (no value) -> clap missing-value error, exit 2
#[test]
fn missing_values_fail() {
    let out = run(&["--name"]);
    assert_error_shape(&out, &["--name"], "a value is required for '--name <name>'");
    let out = run(&["--count"]);
    assert_error_shape(&out, &["--count"], "a value is required for '--count <n>'");
}

// Row: `--bogus` -> clap unknown-argument error, exit 2
#[test]
fn unknown_argument_fails() {
    let out = run(&["--bogus"]);
    assert_error_shape(&out, &["--bogus"], "unexpected argument '--bogus'");
}

// Edge: single-dash forms are unknown arguments, not bundled shorts.
#[test]
fn single_dash_forms_fail() {
    for arg in ["-x", "-"] {
        let out = run(&[arg]);
        assert_error_shape(&out, &[arg], &format!("unexpected argument '{arg}'"));
    }
}

// Row: `--help`, `-h` -> clap's generated help on stdout, exit 0
#[test]
fn help_prints_usage_to_stdout() {
    for flag in ["--help", "-h"] {
        let out = run(&[flag]);
        assert_eq!(code(&out), 0, "{flag} should succeed");
        assert_eq!(stderr(&out), "", "{flag} must not write to stderr");
        let text = stdout(&out);
        assert!(
            text.starts_with("Say hello from the no man's land\n"),
            "unexpected help text: {text}"
        );
        assert!(text.contains("Usage: nomansland [OPTIONS]"), "{text}");
        assert!(text.contains("--name <name>"), "{text}");
        assert!(text.contains("--count <n>"), "{text}");
        assert!(text.contains("-h, --help"), "{text}");
        assert!(text.contains("-V, --version"), "{text}");
    }
}

// Row: `--version`, `-V` -> clap's standard version output on stdout,
// exit 0 (issue #29). The version comes from Cargo.toml via clap's
// `#[command(version)]`, which expands `env!("CARGO_PKG_VERSION")` in this
// crate, so asserting against the same env var keeps the test in sync.
#[test]
fn version_prints_package_version() {
    for flag in ["--version", "-V"] {
        let out = run(&[flag]);
        assert_eq!(code(&out), 0, "{flag} should succeed");
        assert_eq!(stderr(&out), "", "{flag} must not write to stderr");
        assert_eq!(
            stdout(&out),
            format!("nomansland {}\n", env!("CARGO_PKG_VERSION")),
            "unexpected output for {flag}"
        );
    }
}

// Edge: `--name --version` greets the literal flag-like value instead of
// printing the version (allow_hyphen_values keeps working, issue #29).
#[test]
fn version_flag_as_name_value_is_consumed() {
    let out = run(&["--name", "--version"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, --version!\n");
}

// Edge: `--bogus --version` -> error, exit 2; the first token decides,
// same left-to-right rule as --help.
#[test]
fn earlier_error_wins_over_later_version() {
    let out = run(&["--bogus", "--version"]);
    assert_error_shape(&out, &["--bogus", "--version"], "unexpected argument");
}

// Edge: `--name ada --count 3 -h` prints only the help, no greetings.
#[test]
fn help_short_circuits_other_flags() {
    let out = run(&["--name", "ada", "--count", "3", "-h"]);
    assert_eq!(code(&out), 0);
    assert!(
        !stdout(&out).contains("hello, "),
        "help must suppress greetings"
    );
}

// Edge: `--bogus --help` -> error, exit 2; the first token decides.
#[test]
fn earlier_error_wins_over_later_help() {
    let out = run(&["--bogus", "--help"]);
    assert_error_shape(&out, &["--bogus", "--help"], "unexpected argument");
}

// Edge: `--name --help` -> `hello, --help!`, exit 0. --name allows
// hyphen-leading values, so flag-like strings are valid names.
#[test]
fn flag_like_values_are_consumed() {
    let out = run(&["--name", "--help"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, --help!\n");
}

// Row: `-- --name ada` -> the program takes no positional values, so
// anything after the `--` terminator is rejected, exit 2.
#[test]
fn arguments_after_double_dash_are_rejected() {
    let out = run(&["--", "--name", "ada"]);
    assert_error_shape(&out, &["--", "--name", "ada"], "unexpected argument");
}

// Edge: `--` alone (and after complete options) is a no-op, exit 0.
#[test]
fn lone_double_dash_is_a_noop() {
    let out = run(&["--"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, world!\n");

    let out = run(&["--name", "ada", "--"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, ada!\n");
}

// Edge: `--name=--` -> `hello, --!`: the `=` form sets a literal `--`.
#[test]
fn equals_form_accepts_literal_double_dash() {
    let out = run(&["--name=--"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, --!\n");
}

// Row: `--name a --name b` -> clap duplicate-argument error, exit 2
// (issue #25: repeated flags are rejected in all form mixes).
#[test]
fn repeated_name_flag_fails() {
    for args in [
        &["--name", "a", "--name", "b"][..],
        &["--name=a", "--name=b"][..],
        &["--name", "a", "--name=b"][..],
        &["--name=a", "--name", "b"][..],
    ] {
        let out = run(args);
        assert_error_shape(&out, args, "cannot be used multiple times");
    }
}

// Row: `--count 2 --count 5` -> duplicate argument error, exit 2.
#[test]
fn repeated_count_flag_fails() {
    for args in [
        &["--count", "2", "--count", "5"][..],
        &["--count=2", "--count", "5"][..],
    ] {
        let out = run(args);
        assert_error_shape(&out, args, "cannot be used multiple times");
    }
}

// Edge (issue #25): the duplicate fires before a later --help can take over.
#[test]
fn duplicate_before_help_wins() {
    let out = run(&["--name", "a", "--name", "b", "--help"]);
    assert_error_shape(
        &out,
        &["--name", "a", "--name", "b", "--help"],
        "cannot be used multiple times",
    );
}

// Row: `--name ""` / `--name=` / whitespace-only -> error, exit 2
// (issue #25: the empty greeting is rejected, not printed).
#[test]
fn empty_names_are_rejected() {
    for args in [
        &["--name", ""][..],
        &["--name="][..],
        &["--name", "   "][..],
        &["--name=\t "][..],
    ] {
        let out = run(args);
        assert_error_shape(&out, args, "must not be empty");
    }
}

// Edge (issue #25): empty-name validation does not normalise; a padded
// name still prints exactly as supplied.
#[test]
fn padded_names_print_verbatim() {
    let out = run(&["--name", " ada "]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello,  ada !\n");
}
