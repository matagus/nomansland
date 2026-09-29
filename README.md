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

## Exit codes

The binary returns only two exit codes (`ExitCode::SUCCESS` and
`ExitCode::FAILURE` in `src/main.rs`):

| Code | When |
| --- | --- |
| `0` | Success: the greeting lines were printed, or `--help`/`-h` printed the usage text. |
| `1` | Any argument error: unknown or unexpected arguments, missing option values, duplicate flags, invalid `--count` values, and empty/whitespace-only `--name` values. The `error: …` message and the usage block go to stderr; stdout stays empty. |

There are no other exit codes.

## Option values and `--` (the parsing contract)

The contract for flag-like strings (like `--help` or `-h`) appearing where
an option value is expected:

- **`--name` accepts any token as its value, including flag-like ones.**
  The value of an option is simply the next token, whatever it looks like,
  so `nomansland --name --help` prints `hello, --help!` and exits 0. This
  is intended, not a bug. The value must however contain at least one
  non-whitespace character: an empty or whitespace-only name (`--name ""`,
  `--name=`, `--name "   "`) fails with
  `error: invalid value for '--name': must not be empty`. Validation never
  normalises: `--name " ada "` still greets ` ada ` exactly as supplied.
- **Each flag may appear at most once.** Repeating `--name` or `--count`
  — in the space form, the `=` form, or any mix — fails with
  `error: duplicate argument: '<flag>'` and exit 1; there is no silent
  last-wins. The duplicate is reported at the flag token itself, so a
  later `--help` cannot rescue it (`--name a --name b --help` errors),
  while a `--help` seen first still short-circuits before any later
  duplicate is reached.
- **`--count` reports a *missing* value when the next token is a recognized
  flag** (`--name`, `--count`, `--help`, `-h`): a flag can never be a valid
  count, so `nomansland --count --name` fails with
  `error: missing value for '--count'` rather than claiming `--name` is an
  invalid number. Any other non-numeric token is still reported as an
  invalid value (`--count abc`).
- **`--` is the end-of-options terminator.** In a value slot it forces the
  next token to be taken verbatim as the value even where the rules above
  would reject or reinterpret it, so `nomansland --name -- --help` prints
  `hello, --help!` unambiguously and `nomansland --count -- --name` fails
  with `invalid value …` (the `--name` was explicitly supplied as the
  value). At the option position a lone trailing `--` is a no-op, and
  because the program takes no positional values, anything after `--`
  there is rejected with `error: unexpected argument: '<arg>'`.
- To pass a literal `--` as a value, use the `=` form (`--name=--`) or the
  escape (`--name -- --`).

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
