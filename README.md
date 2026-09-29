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

Names are printed exactly as supplied. Every error goes to stderr followed by the
usage block.

## Repository layout

```
Cargo.toml            package manifest, no dependencies
src/main.rs           the entire program
tests/cli.rs          end-to-end tests against the built binary (cargo test)
docs/                 notes that are not about the code
.github/workflows/    agentic workflow sources (.md) and generated locks (.lock.yml)
.github/pi-run.sh     inference launcher used by those workflows
```

## Further reading

[Agentic workflows](docs/agentic-workflows.md) — how issues in this repo get
implemented, reviewed, and merged by AI agents, plus the model routing, secrets,
operational gotchas, and security trade-offs behind that setup.
