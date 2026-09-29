//! End-to-end tests for the README "Behaviour" table.
//!
//! These run the real built binary (cargo provides its path via the
//! `CARGO_BIN_EXE_nomansland` env var) and assert on stdout text, stderr
//! shape and exit codes, so the documented output/exit-code contract is
//! machine-checked rather than manually verified.

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

/// Every error goes to stderr as `error: {msg}\n\n{USAGE}`: message line,
/// blank line, then the usage block starting with the program summary.
fn assert_error_shape(out: &Output, message: &str) {
    assert_eq!(code(out), 1, "expected failure exit code for {message:?}");
    assert_eq!(stdout(out), "", "errors must not write to stdout");
    let err = stderr(out);
    assert!(
        err.starts_with(&format!("error: {message}\n\nnomansland - say hello")),
        "stderr did not start with the expected 'error: <msg>\\n\\n<usage>' shape:\n{err}"
    );
    assert!(err.contains("Usage:"), "error output must include the usage block");
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

// Row: `--count 0` / `--count=0` -> invalid value error, exit 1
#[test]
fn count_zero_fails() {
    for args in [&["--count", "0"][..], &["--count=0"][..]] {
        let out = run(args);
        assert_error_shape(
            &out,
            "invalid value for '--count': expected a positive integer, got '0'",
        );
    }
}

// Row: `--count abc` -> invalid value error, exit 1
#[test]
fn count_non_numeric_fails() {
    let out = run(&["--count", "abc"]);
    assert_error_shape(
        &out,
        "invalid value for '--count': expected a positive integer, got 'abc'",
    );
}

// Edge: `--count=` (empty), `-3`, overflow, `1_0`, leading space all rejected.
#[test]
fn count_rejects_surprising_numeric_forms() {
    for value in ["", "-3", "4294967296", "1_0", " 2"] {
        let out = run(&["--count", value]);
        assert_error_shape(
            &out,
            &format!("invalid value for '--count': expected a positive integer, got '{value}'"),
        );
    }
}

// Row: `--name` (no value) -> missing value error, exit 1
#[test]
fn name_without_value_fails() {
    let out = run(&["--name"]);
    assert_error_shape(&out, "missing value for '--name'");
}

// Row: `--bogus` -> unknown argument error, exit 1
#[test]
fn unknown_argument_fails() {
    let out = run(&["--bogus"]);
    assert_error_shape(&out, "unknown argument: '--bogus'");
}

// Edge: single-dash forms are unknown arguments, not bundled shorts.
#[test]
fn single_dash_forms_fail() {
    for arg in ["-x", "-"] {
        let out = run(&[arg]);
        assert_error_shape(&out, &format!("unknown argument: '{arg}'"));
    }
}

// Row: `--help`, `-h` -> usage text on stdout, exit 0
#[test]
fn help_prints_usage_to_stdout() {
    for flag in ["--help", "-h"] {
        let out = run(&[flag]);
        assert_eq!(code(&out), 0, "{flag} should succeed");
        assert_eq!(stderr(&out), "", "{flag} must not write to stderr");
        let text = stdout(&out);
        assert!(
            text.starts_with("nomansland - say hello from the no man's land\n"),
            "unexpected help text: {text}"
        );
        assert!(text.contains("Usage:") && text.contains("Options:"));
        assert!(text.ends_with("Show this help text and exit\n"));
    }
}

// Edge: `--name ada --count 3 -h` prints only the usage, no greetings.
#[test]
fn help_short_circuits_other_flags() {
    let out = run(&["--name", "ada", "--count", "3", "-h"]);
    assert_eq!(code(&out), 0);
    assert!(!stdout(&out).contains("hello, "), "help must suppress greetings");
}

// Edge: `--bogus --help` -> error, exit 1; argument order decides.
#[test]
fn earlier_error_wins_over_later_help() {
    let out = run(&["--bogus", "--help"]);
    assert_error_shape(&out, "unknown argument: '--bogus'");
}

// Edge: `--name --help` -> `hello, --help!`, exit 0 (no end-of-options).
#[test]
fn flag_like_values_are_consumed() {
    let out = run(&["--name", "--help"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, --help!\n");
}

// Edge: `--count --name` -> invalid (not missing) value error, exit 1.
#[test]
fn count_with_flag_like_value_errors_as_invalid() {
    let out = run(&["--count", "--name"]);
    assert_error_shape(
        &out,
        "invalid value for '--count': expected a positive integer, got '--name'",
    );
}

// Edge: `--name a --name b` -> `hello, b!`, last one wins.
#[test]
fn repeated_name_flag_is_last_wins() {
    let out = run(&["--name", "a", "--name", "b"]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out), "hello, b!\n");
}

// Edge: `--name ""` and `--name=` -> `hello, !`, exit 0.
#[test]
fn empty_names_are_accepted() {
    for args in [&["--name", ""][..], &["--name="][..]] {
        let out = run(args);
        assert_eq!(code(&out), 0, "{args:?} should succeed");
        assert_eq!(stdout(&out), "hello, !\n");
    }
}
