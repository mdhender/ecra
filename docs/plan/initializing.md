# Initialize a Rust Turn-Based Game Engine Repository

Create a new Rust repository that provides the initial scaffolding for a deterministic, turn-based strategy game engine.

The project does **not** need a web server, HTTP API, authentication, or frontend yet.

The goal is to establish the core architecture for:

1. storing game data,
2. importing player orders from flat files,
3. loading an immutable game-state snapshot for a turn,
4. resolving that turn deterministically,
5. saving the resulting facts/state,
6. generating player and GM reports,
7. advancing the game to the next turn.

The implementation should emphasize simplicity, deterministic behavior, immutable data, explicit state transitions, testability, and easy deployment as a native executable.

---

# Technology

Use:

* Rust stable edition
* Cargo workspace if useful, but do not create unnecessary crates
* `redb` as the embedded data store
* `serde` for serialization where appropriate
* `thiserror` or an equally simple error-handling crate for library errors
* `anyhow` only at the executable/application boundary if useful
* standard Rust collections unless persistent immutable collections materially simplify the implementation
* `imbl` may be used for persistent immutable collections if justified

Do not introduce:

* Tokio
* async code
* a web framework
* an ORM
* SQL
* SQLite
* dependency injection frameworks
* message queues
* distributed components

This is currently a single-process turn-processing application.

---

# Architectural Principle

Model the engine as transformations of immutable state.

Conceptually:

```text
GameState(T)
    +
Orders(T)
    ↓
play_turn()
    ↓
TurnResult
    ├── GameState(T+1)
    ├── Facts
    └── Reports
```

Prefer APIs shaped like:

```rust
fn play_turn(
    state: &GameState,
    orders: &[Order],
) -> Result<TurnResult, GameError>
```

rather than functions that mutate global or shared state.

The existing state passed to an engine function must remain valid and unchanged after the function returns.

A caller should be able to retain:

```text
state_before
state_after
```

and inspect both independently.

Avoid interior mutability unless there is a compelling reason for it.

---

# Immutable Facts

Treat persisted game history as immutable facts whenever practical.

Examples:

```text
Entity 42 belonged to Player 7 effective Turn 3
Entity 42 moved to Stellium 12 effective Turn 8
Entity 42 FOOD AVAILABLE quantity became 700 effective Turn 10
Squire Bob's CHARISMA became 14 effective Turn 12
```

Do not update historical facts in place.

When a value changes, append a new effective value.

The basic temporal convention is:

```text
effective from turn N
until superseded
```

A fact valid at turn `T` is the latest fact for that logical key whose effective turn is less than or equal to `T`.

Do not store an explicit `effective_to` unless implementation experience demonstrates that it is needed.

For example:

```text
(Bob, CHARISMA, 8)  -> 13
(Bob, CHARISMA, 12) -> 14
(Bob, CHARISMA, 17) -> 12
```

means Bob's Charisma is:

```text
13 for turns 8..11
14 for turns 12..16
12 from turn 17 onward
```

---

# Initial Domain Model

Do not attempt to model an entire production game.

Create only enough domain objects to prove the architecture.

At minimum include:

```text
Player
Agent
Entity
Unit
Inventory
Order
GameState
Turn
TurnResult
Fact
Report
```

Use strongly typed IDs rather than passing raw integers everywhere.

For example:

```rust
struct PlayerId(u64);
struct EntityId(u64);
struct UnitId(u64);
struct Turn(u32);
```

IDs should not accidentally be interchangeable.

An Entity is owned by either a Player or an Agent.

Model that explicitly rather than using magic values.

For example:

```rust
enum OwnerId {
    Player(PlayerId),
    Agent(AgentId),
}
```

Inventory should support the conceptual relationship:

```text
(player | agent)
    └── entity
          └── inventory
                └── unit + status + quantity
```

A reasonable initial inventory record might contain:

```text
entity
unit
status
quantity
effective_turn
```

Use an enum for status.

Provide only a few demonstration statuses, such as:

```text
Available
Reserved
Damaged
```

Do not build an extensible plugin system for statuses.

---

# Game State

`GameState` represents the world as seen at a specific turn.

It should be immutable from the perspective of callers.

It should contain enough information to answer questions such as:

```text
Who owns Entity 42 at Turn 7?
What inventory does Entity 42 have at Turn 7?
How much FOOD is available?
What is the current value of an attribute?
```

The storage layer may reconstruct `GameState` from immutable temporal facts.

Do not expose `redb` types through the domain model.

The engine should operate on domain types, not database records.

Keep these concerns distinct:

```text
storage
domain
engine
application
reporting
order parsing
```

---

# Data Store

Use `redb`.

Create a storage abstraction around it.

The rest of the application should not directly manipulate redb tables.

Start with simple typed repository/store APIs.

Examples:

```rust
trait GameStore {
    fn current_turn(&self) -> Result<Turn, StoreError>;

    fn load_turn(&self, turn: Turn) -> Result<GameState, StoreError>;

    fn save_turn_result(
        &mut self,
        result: &TurnResult,
    ) -> Result<(), StoreError>;
}
```

Do not over-engineer the trait structure if concrete types are simpler.

The important boundary is:

```text
redb implementation
        ↓
domain-oriented storage API
        ↓
engine
```

The database should support at least:

* metadata
* players
* agents
* entities
* units
* temporal ownership facts
* temporal inventory facts
* processed orders
* generated facts
* turn completion metadata

Define keys so that related temporal facts can be efficiently range-scanned.

For example, inventory may conceptually use an ordered key:

```text
(entity_id, unit_id, status, effective_turn)
```

Do not depend on scanning the entire database to reconstruct one entity.

---

# Turn Lifecycle

Implement an explicit application-level lifecycle:

```text
import-orders
      ↓
load-turn
      ↓
play-turn
      ↓
save-results
      ↓
generate-reports
      ↓
advance-turn
```

The application must prevent advancing to the next turn unless the current turn has been successfully resolved and saved.

Keep lifecycle state explicit.

A turn should have a status along the lines of:

```rust
enum TurnStatus {
    Open,
    OrdersImported,
    Resolved,
    Reported,
    Complete,
}
```

You may adjust the names if a simpler model emerges.

---

# Orders

Player orders arrive as flat text files.

For now, define an intentionally small order language.

Example input:

```text
MOVE 1001 12;
TRANSFER 1001 FOOD AVAILABLE 25 1002;
```

where these might mean:

```text
MOVE <entity-id> <destination-id>

TRANSFER
    <source-entity>
    <unit>
    <status>
    <quantity>
    <destination-entity>
```

The exact demonstration commands are less important than the parsing architecture.

Implement order parsing as a distinct subsystem:

```text
text
 ↓
lexer/parser or simple line parser
 ↓
Order
```

Do not put database operations inside the parser.

Parsing must produce domain-level `Order` values or useful parse errors.

Errors should identify:

* filename
* line number
* offending text where useful
* explanation

Malformed orders must not panic.

Add fixtures showing valid and invalid order files.

---

# Order Validation

Distinguish parsing from validation.

For example:

```text
MOVE 999999 12;
```

may be syntactically valid but invalid because Entity `999999` does not exist.

The workflow should therefore be:

```text
parse
 ↓
Order
 ↓
validate against GameState
 ↓
ValidatedOrder
 ↓
engine
```

Prefer making illegal states difficult to represent.

If useful, introduce a separate type:

```rust
ValidatedOrder
```

so the engine cannot accidentally receive completely unvalidated player input.

Do not over-generalize this yet.

---

# Engine

Create a simple deterministic engine demonstrating the architecture.

It does not need realistic game rules.

Implement enough rules to prove that:

* entities can move,
* inventory can be transferred,
* state before resolution remains available,
* a new state is produced,
* immutable facts describing changes are emitted.

For example:

```text
MOVE Entity 1001 from location 5 to location 12
```

might emit:

```text
EntityLocationChanged {
    entity: 1001,
    from: 5,
    to: 12,
    effective_turn: 4,
}
```

A transfer might emit facts reflecting quantities at the next effective turn.

The resulting state should be derived from the old state plus generated facts.

Avoid mutating database state during individual engine operations.

The preferred pattern is:

```text
load immutable state
        ↓
resolve entirely in memory
        ↓
produce TurnResult
        ↓
commit result atomically
```

---

# Atomic Turn Commit

Saving a successful turn must be transactional.

A partial turn must never become the official game state.

The database commit should include, as appropriate:

```text
accepted orders
generated facts
turn metadata
result metadata
next-turn initialization
```

If writing the result fails, the current official game state must remain unchanged.

Use redb transactions to enforce this.

---

# Reports

Generate plain text or Markdown reports initially.

Do not build HTML.

Generate at least:

```text
reports/
    turn-0001/
        player-0001.txt
        player-0002.txt
        gm.txt
```

Player reports should be generated from a defined immutable state or `TurnResult`, not from whatever happens to be currently stored in the database.

This is important.

A report for Turn 3 must be reproducible after Turn 20 has been completed.

Provide a reporting API approximately shaped like:

```rust
fn generate_player_report(
    player: PlayerId,
    result: &TurnResult,
) -> Result<String, ReportError>;
```

Reports should include enough demonstration information to verify:

* prior state,
* accepted orders,
* resulting entity state,
* inventory changes.

---

# Determinism

The same:

```text
initial database
+
orders
```

must produce the same:

```text
facts
state
reports
```

every time.

Tests must demonstrate this.

Do not use:

* wall-clock time,
* random number generation without an explicit supplied seed,
* unordered iteration where ordering affects results,
* global mutable state.

If randomness is later required, it should be supplied explicitly to the engine through deterministic seeded state.

---

# CLI

Create one executable with straightforward subcommands.

For example:

```bash
game init game.redb

game import-orders game.redb orders/

game show-turn game.redb

game play-turn game.redb

game report game.redb reports/

game advance-turn game.redb
```

It is acceptable to combine some lifecycle operations if doing so makes the interface cleaner.

Also provide a convenient command such as:

```bash
game run-turn game.redb orders/ reports/
```

which performs:

```text
import
validate
resolve
save
report
advance
```

while still keeping the underlying application operations separate internally.

Do not make CLI parsing itself part of the domain layer.

Use a small established CLI crate such as `clap`.

---

# Initialization

`game init` should create a demonstration database containing enough seed data to exercise the application.

For example:

```text
Player 1
Player 2

Entity 1001 owned by Player 1
Entity 2001 owned by Player 2

several Unit definitions

some initial inventory

current turn = 1
```

This seed/demo state should make it possible to clone the repository and immediately run a complete example turn.

---

# Repository Layout

Prefer a simple layout.

Something along these lines is appropriate:

```text
.
├── Cargo.toml
├── README.md
├── AGENTS.md
├── src/
│   ├── main.rs
│   ├── domain/
│   ├── engine/
│   ├── orders/
│   ├── reports/
│   ├── storage/
│   └── app/
├── tests/
│   ├── fixtures/
│   └── ...
└── examples/
    └── orders/
```

Do not create separate Cargo crates merely to enforce boundaries that Rust modules already enforce.

A workspace is appropriate only if there is a genuine reason for independently reusable packages.

---

# Testing

Testing is a major goal of this scaffold.

Provide unit and integration tests for at least:

## Temporal lookup

Given:

```text
turn 1 -> quantity 100
turn 4 -> quantity 75
turn 8 -> quantity 120
```

verify:

```text
turn 1 => 100
turn 3 => 100
turn 4 => 75
turn 7 => 75
turn 8 => 120
turn 20 => 120
```

## Immutability

Resolve a turn and verify that the original `GameState` remains unchanged.

## Order parsing

Verify:

* valid order parsing,
* blank/comment lines,
* malformed command,
* invalid numeric ID,
* invalid quantity,
* line-number reporting.

## Validation

Verify that syntactically valid orders referencing nonexistent entities are rejected before engine execution.

## Deterministic execution

Given the same starting state and order set, resolve the turn twice.

Assert that:

```text
facts are identical
resulting states are identical
reports are identical
```

## Persistence

Save a resolved turn.

Close the database.

Reopen it.

Load both:

```text
state at old turn
state at new turn
```

and verify both are correct.

## Atomicity

A failed save must not leave half of a turn persisted.

## Reporting

Generate a historical report after later turns exist and confirm that the historical report remains unchanged.

---

# Example End-to-End Scenario

Create an integration test or scripted demonstration equivalent to:

```text
Initialize game at Turn 1.

Player 1 owns Entity 1001.
Player 2 owns Entity 2001.

Entity 1001:
    location = 5
    FOOD AVAILABLE = 100

Entity 2001:
    location = 12
    FOOD AVAILABLE = 20
```

Orders:

```text
MOVE 1001 12
TRANSFER 1001 FOOD AVAILABLE 25 2001
```

After resolution:

```text
Entity 1001:
    location = 12
    FOOD AVAILABLE = 75

Entity 2001:
    FOOD AVAILABLE = 45
```

The Turn 1 starting state must still report:

```text
Entity 1001:
    location = 5
    FOOD AVAILABLE = 100

Entity 2001:
    FOOD AVAILABLE = 20
```

Persist both historical and resulting facts such that the program can reconstruct either state later.

---

# Documentation

Create a useful `README.md` explaining:

* what the project currently demonstrates,
* architectural principles,
* immutable state model,
* temporal fact model,
* turn lifecycle,
* directory structure,
* CLI commands,
* how to run the demonstration turn,
* how to run tests.

Create `AGENTS.md` instructing future coding agents to preserve these principles:

1. Prefer immutable inputs and explicit outputs.
2. Historical facts are append-only.
3. Do not rewrite past game state.
4. A turn resolves completely in memory before persistence.
5. Persist a resolved turn atomically.
6. Parsing, validation, resolution, persistence, and reporting are separate concerns.
7. Domain types must not depend on redb.
8. No web-server dependencies until explicitly requested.
9. Avoid unnecessary abstraction.
10. Favor readable Rust over clever Rust.
11. Every game rule should be deterministic and directly testable.
12. Historical reports must be reproducible.

---

# Implementation Style

Favor straightforward, idiomatic Rust.

This project is exploratory.

We are evaluating whether Rust's ownership, immutable-by-default model, persistent data structures, and native embedded storage make it a good foundation for complex deterministic turn-based game engines.

Therefore:

* expose Rust's strengths,
* do not hide everything behind layers of traits,
* do not prematurely optimize,
* do not build generalized frameworks,
* avoid macros unless they clearly improve readability,
* make ownership and data-flow easy to understand,
* use compiler-enforced invariants where they genuinely simplify the game model.

Comments should explain architectural reasons, not restate obvious code.

---

# Deliverable

Produce a repository that builds and whose tests pass.

Before finishing:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

must succeed.

Also manually exercise the example lifecycle from a clean database:

```text
init
import orders
play turn
save result
generate reports
advance turn
reload historical turn
reload current turn
```

Update `README.md` with the exact commands required to reproduce that demonstration.

Do not implement a web server or frontend.

The finished repository should prove one architectural proposition:

> A turn-based game can be modeled as a deterministic transformation from one immutable world state to another, with append-only temporal facts providing durable history and reproducible reporting.
