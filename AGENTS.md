# AGENTS.md

This repository is a Rust scaffold for a deterministic, turn-based strategy game engine. Read `docs/plan/initializing.md` before making architectural or domain changes; it is the source of truth for the initial implementation goals.

## Core invariants

- Model turn processing as a deterministic transformation from an immutable `GameState` and validated orders into an explicit `TurnResult`.
- Keep the input state valid and unchanged after resolution. Do not use global mutable state or interior mutability without a compelling, documented reason.
- Treat historical game facts as append-only temporal records. A changed value creates a new fact with an effective turn; never rewrite past game state or update historical facts in place.
- Resolve a complete turn in memory before persistence. Persist accepted orders, generated facts, turn metadata, and next-turn initialization in one atomic `redb` transaction.
- Historical states and reports must remain reproducible after later turns have completed.
- Avoid wall-clock time, implicit randomness, and order-sensitive iteration over unordered collections. Any future randomness must come from explicit seeded state.

## Boundaries

Keep these concerns separate:

1. parsing order-file text into domain-level `Order` values,
2. validating parsed orders against a `GameState`,
3. resolving validated orders in the engine,
4. persisting and reconstructing temporal state,
5. generating reports from a defined immutable state or `TurnResult`, and
6. orchestrating the lifecycle in the application/CLI layer.

Domain and engine code must not depend on `redb`, CLI types, filesystem details, or report formatting. Order parsers must not access storage. Reports must not read whichever database state happens to be current.

Use strongly typed IDs and explicit enums for domain distinctions; do not use interchangeable raw integers or magic sentinel values. Prefer compiler-enforced invariants where they make illegal states harder to represent, such as passing `ValidatedOrder` rather than unchecked input to the engine.

## Implementation guidance

- Use stable, idiomatic, readable Rust. Favor direct data flow over clever code.
- Keep the repository as one crate unless independently reusable packages provide a genuine reason for a workspace.
- Use `redb` behind a domain-oriented storage API, `serde` where serialization is needed, and simple error types such as `thiserror`. Reserve `anyhow` for the executable boundary if useful.
- Do not add Tokio, async code, web frameworks, HTTP APIs, ORMs, SQL databases, message queues, distributed components, or dependency-injection frameworks unless explicitly requested.
- Prefer standard collections. Add persistent collections such as `imbl` only when they materially simplify immutable state handling.
- Avoid unnecessary traits, wrappers, generalized frameworks, macros, premature optimization, and speculative extensibility.
- Comments should explain architectural intent or non-obvious constraints, not restate the code.
- Keep lifecycle transitions explicit. A turn must not advance until resolution and persistence have succeeded.

## Tests and fixtures

Every game rule must be deterministic and directly testable. Add or update focused tests for behavior changes. Preserve coverage of:

- temporal lookup at, before, and after effective turns,
- input-state immutability,
- parsing diagnostics, including filename and line number,
- separation of syntactic parsing from state-dependent validation,
- identical facts, resulting state, and reports for identical inputs,
- persistence and reconstruction of old and new turns after reopening the database,
- atomic failure behavior with no partially official turn,
- reproducible historical reports, and
- the documented end-to-end demonstration turn.

Malformed or invalid player input must return useful errors and must not panic. Keep representative valid and invalid order files under test fixtures.

## Git workflow

Prefer committing directly to `main` while the project is in alpha and beta. Revisit the branching workflow once the project leaves beta.

## Verification

Before declaring repository changes complete, run the checks relevant to the change. For a full implementation or broad change, run all required gates:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

For lifecycle, persistence, or reporting changes, also manually exercise the clean-database flow documented in the README: initialize, import, validate/resolve, atomically save, report, advance, then reload both historical and current state. Keep the README's reproduction commands accurate.
