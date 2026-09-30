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

```
nomansland [OPTIONS]

Options:
  --name <name>     Who to greet (default: "world")
  --count <n>       How many times to greet (default: 1)
  --help, -h        Show this help text and exit
  --                End of options; the next token is a plain value
```

Both flags also accept the `--flag=value` form:

```bash
./target/debug/nomansland --name=ada --count=3
```

No third-party dependencies — argument parsing is hand-rolled so the crate
builds offline.

## Behaviour

Every row below is enforced by `cargo test`: unit tests in `src/main.rs`
cover argument parsing, and end-to-end tests in `tests/cli.rs` run the built
binary and assert on its exact output and exit codes.

| Input | Output | Exit code |
| --- | --- | --- |
| *(no arguments)* | `hello, world!` | 0 |
| `--name ada` | `hello, ada!` | 0 |
| `--name=ada --count 3` | three greeting lines | 0 |
| `--count 0` or `--count=0` | `error: invalid value for '--count'…` | 1 |
| `--count abc` | `error: invalid value for '--count'…` | 1 |
| `--name` *(no value)* | `error: missing value for '--name'` | 1 |
| `--bogus` | `error: unknown argument: '--bogus'` | 1 |
| `--help`, `-h` | usage text | 0 |
| `--name --help` | `hello, --help!` (flag-like values are accepted) | 0 |
| `--count --name` | `error: missing value for '--count'` | 1 |
| `--name a --name b` | `error: duplicate argument: '--name'` | 1 |
| `--count 2 --count 5` | `error: duplicate argument: '--count'` | 1 |
| `--name ""` or `--name="   "` | `error: invalid value for '--name': must not be empty` | 1 |
| `--name -- --help` | `hello, --help!` (`--` forces the value) | 0 |
| `--` *(alone)* | `hello, world!` (terminator is a no-op) | 0 |
| `-- --name ada` | `error: unexpected argument: '--name'` | 1 |

Names are printed exactly as supplied (validation rejects empty names but never
trims or otherwise rewrites accepted ones). Every error goes to stderr followed by the
usage block.

## Further reading

[Agentic workflows](docs/agentic-workflows.md) — how issues in this repo get
implemented, reviewed, and merged by AI agents, plus the model routing, secrets,
operational gotchas, and security trade-offs behind that setup.
