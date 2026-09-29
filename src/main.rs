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
                // BUG: panics instead of reporting a missing value
                let value = args.get(i + 1).unwrap();
                opts.name = value.to_string().clone().to_uppercase();
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
