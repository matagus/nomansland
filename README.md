# nomansland

A deliberately minimal Rust command line app, used as a playground for
agentic GitHub workflows (issue → implementation PR → AI review → merge).

## Build & run

```bash
cargo build
./target/debug/nomansland --name matagus --count 2
```

Run the test suite (argument-parsing unit tests plus end-to-end checks
against the built binary) with:

```bash
cargo test
```

## Usage

Argument parsing is delegated to [clap](https://docs.rs/clap) 4.6.7 (derive
API), so `--help` prints clap's generated usage and parse failures use clap's
standard error shape: `error: …` on stderr followed by a usage/help hint,
with exit code 2.

```
Say hello from the no man's land

Usage: nomansland [OPTIONS]

Options:
      --name <name>  Who to greet [default: world]
      --count <n>    How many times to greet [default: 1]
  -h, --help         Print help
```

Both flags also accept the `--flag=value` form:

```bash
./target/debug/nomansland --name=ada --count=3
```

## Behaviour

Every row below is enforced by `cargo test`: unit tests in `src/main.rs`
cover argument parsing, and end-to-end tests in `tests/cli.rs` run the built
binary and assert on its exact output and exit codes.

| Input | Output | Exit code |
| --- | --- | --- |
| *(no arguments)* | `hello, world!` | 0 |
| `--name ada` | `hello, ada!` | 0 |
| `--name=ada --count 3` | three greeting lines | 0 |
| `--count 0` or `--count=0` | `error: invalid value '0' for '--count <n>'…` | 2 |
| `--count abc` | `error: invalid value 'abc' for '--count <n>'…` | 2 |
| `--count -3` | `error: unexpected argument '-3' found` | 2 |
| `--name` *(no value)* | `error: a value is required for '--name <name>'…` | 2 |
| `--bogus` | `error: unexpected argument '--bogus' found` | 2 |
| `--help`, `-h` | clap's generated help | 0 |
| `--name --help` | `hello, --help!` (flag-like values are accepted) | 0 |
| `--name a --name b` | `error: the argument '--name <name>' cannot be used multiple times` | 2 |
| `--count 2 --count 5` | `error: the argument '--count <n>' cannot be used multiple times` | 2 |
| `--name ""` or `--name="   "` | `error: invalid value … must not be empty` | 2 |
| `--` *(alone)* | `hello, world!` (terminator is a no-op) | 0 |
| `-- --name ada` | `error: unexpected argument '--name' found` | 2 |

Names are printed exactly as supplied (validation rejects empty names but never
trims or otherwise rewrites accepted ones). Errors are reported left to right:
the first offending token wins, even before a later `--help`.

## Further reading

[Agentic workflows](docs/agentic-workflows.md) — how issues in this repo get
implemented, reviewed, and merged by AI agents, plus the model routing, secrets,
operational gotchas, and security trade-offs behind that setup.
