# ecra

A Rust scaffold for a deterministic, turn-based strategy game engine and CLI.

ECRA is being built around pure turn processing: an immutable `GameState` plus
validated orders will produce an explicit `TurnResult`. The architecture requires
append-only game history and reproducible facts, states, and reports. Turn resolution,
historical facts, and reporting are planned but not yet implemented.

The project is at `0.1.0-beta`. See [AGENTS.md](AGENTS.md) for the architectural
invariants and [docs/plan/initializing.md](docs/plan/initializing.md) for the
implementation plan.

## Requirements

Rust 1.85 or newer — `Cargo.toml` declares `edition = "2024"`, and older toolchains
fail to parse the manifest. If you do not have a current toolchain:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup update stable
```

## Build

```bash
cargo build              # debug binary at target/debug/ecra
cargo build --release    # optimized binary at target/release/ecra
```

## Commands

```
ecra version                        Print the application version
ecra new <store>                    Create a new game store
ecra generate-game <store> <code>   Generate a game (`--seed <u64>` is optional)
ecra manage <store>                 Open and inspect an existing game store
ecra seed-accounts <store>          Seed an existing store with accounts for testing
ecra check-orders <file>            Check an order file for syntax errors
ecra import-orders <store> <file>   Import and parse a raw order file
```

`<store>` is a path to a [redb](https://github.com/cberner/redb) database file.
`new` refuses to replace an existing store and will not create missing parent
directories.

`generate-game` adds a game to an existing store. A store can contain multiple games,
each identified by a unique 1-16 character code beginning with `A-Z` and containing
only `A-Z`, `0-9`, or `-`. Generation creates 100 stellia with one to five stars each
and status `setup`. Supplying `--seed` makes the generated cluster reproducible; when
omitted, the generated seed is recorded and printed so the game can still be replayed.

`check-orders` needs no store: it reads a file, reports every independent syntax
error, and exits non-zero if any were found. `import-orders` persists the raw
submission first, then parses the persisted source, so a file with syntax errors is
still recorded along with its diagnostics — it simply yields no orders ready for
validation.

`seed-accounts` is idempotent. It creates 13 fixed testing accounts — number 1 is an
administrator, 2 through 13 are users — with predictable emails and tokens
(`account.0001@example.com` / `amp.rocks.0001`). Re-running it reports `Created 0`.

## Walkthrough

Every command below is reproducible from a clean checkout:

```bash
cargo run -- new /tmp/demo.redb
cargo run -- generate-game /tmp/demo.redb ECRA-01 --seed 42
cargo run -- seed-accounts /tmp/demo.redb
cargo run -- check-orders tests/fixtures/orders/valid-complete.orders
cargo run -- import-orders /tmp/demo.redb tests/fixtures/orders/valid-complete.orders
cargo run -- manage /tmp/demo.redb
```

Which produces:

```
Created ECRA store at /tmp/demo.redb
Generated game ECRA-01 with seed 42 (status: setup, stellia: 100)
Created 13 test accounts in /tmp/demo.redb
No syntax errors found in tests/fixtures/orders/valid-complete.orders
Imported tests/fixtures/orders/valid-complete.orders as order import 1
Parsed 2 player orders successfully
Store: /tmp/demo.redb
Format version: 1
Current turn: 1
Games: 1
```

Reopening the store — as `manage` does above — reads the persisted state back, so the
import survives the process exiting.

Malformed input is reported per line and never panics:

```bash
cargo run -- check-orders tests/fixtures/orders/multiple-syntax-errors.orders
```

```
tests/fixtures/orders/multiple-syntax-errors.orders:1: turn number must be an unsigned 32-bit integer (found `tomorrow`)
tests/fixtures/orders/multiple-syntax-errors.orders:2: player ID must be an unsigned 64-bit integer (found `nobody`)
tests/fixtures/orders/multiple-syntax-errors.orders:3: entity ID must be an unsigned 64-bit integer (found `unknown`)
error: found 3 syntax errors
```

Parsing continues past an error to the next order, so one invocation reports all
independent problems rather than only the first.

## Order file format

An order file is a sequence of orders terminated by semicolons. Whitespace, including
newlines, only separates tokens — an order may span lines, and the terminating
semicolon may be attached to the preceding word. Errors are reported at the line where
the offending token appears.

The first two orders are required and fixed in position:

```
game <GAME-CODE> turn <TURN-NUMBER>;
authenticate <owner> with token <token>;
```

The owner is `email <address>`, `player <id>`, or `faction <id>`, where IDs are
unsigned 64-bit integers. The token must be quoted; quoted text may contain spaces but
not newlines or control characters. **The token is not verified during parsing or
import** — authentication is a later, state-dependent concern, kept out of the parser
by design.

Player orders follow:

```
MOVE <entity> <destination>;
TRANSFER <source-entity> <unit> <status> <quantity> <destination-entity>;
```

`<status>` is `AVAILABLE`, `RESERVED`, or `DAMAGED`. Keywords are case-sensitive:
structural words (`game`, `turn`, `authenticate`, `with`, `token`, `email`, `player`,
`faction`) are lowercase; order verbs and inventory statuses are uppercase.

A complete example, from `tests/fixtures/orders/valid-complete.orders`:

```
game ECRA-01 turn 7;
authenticate email admiral.sato@example.com with token "opaque token.value";
MOVE 1001 12;
TRANSFER 1001 FOOD AVAILABLE 25 1002;
```

More valid and invalid samples live in `tests/fixtures/orders/`.

## Layout

The implemented pipeline stages are kept separate: parsing does not touch storage,
and the order domain does not depend on `redb` or CLI types. Future validation,
resolution, and reporting stages will follow the boundaries in `AGENTS.md` and the
implementation plan.

| Path | Responsibility |
| --- | --- |
| `src/orders.rs` | Tokenizing and parsing order-file text into domain `Order` values |
| `src/accounts.rs` | Account identity and roles |
| `src/game.rs` | Game identity and deterministic star-cluster generation |
| `src/storage.rs` | `GameStore`, the domain-oriented `redb` API |
| `src/app.rs` | Lifecycle orchestration across parsing and storage |
| `src/main.rs` | CLI argument parsing and command dispatch |
| `tests/cli.rs` | End-to-end tests over the built binary |

## Development

Run the checks relevant to your change. For a full implementation or a broad change,
run all four gates:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

For lifecycle, persistence, or reporting changes, also exercise the clean-database
walkthrough above by hand, and keep its commands and output in this file accurate.

Commit directly to `main` while the project is in alpha and beta.

## Status

The order pipeline currently covers parsing, raw import, and persistence. Validation
against a `GameState`, turn resolution, atomic turn commit, report generation, and
turn advancement are described in `AGENTS.md` and the plan document but are not yet
implemented.

## License

See [LICENSE](LICENSE).
