use std::process::Command;

fn ecra() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ecra"))
}

#[test]
fn help_lists_store_commands() {
    let output = ecra().arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("version"));
    assert!(stdout.contains("new"));
    assert!(stdout.contains("generate-game"));
    assert!(stdout.contains("add-players"));
    assert!(stdout.contains("report"));
    assert!(stdout.contains("manage"));
    assert!(stdout.contains("check-orders"));
    assert!(stdout.contains("import-orders"));
    assert!(stdout.contains("help"));
}

#[test]
fn reports_available_and_game_agents_as_text_and_json() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("game.redb");
    let available_json = directory.path().join("available-agents.json");
    let game_json = directory.path().join("game-agents.json");
    let factions_json = directory.path().join("agent-factions.json");

    let available = ecra()
        .args(["report", "available-agents"])
        .output()
        .unwrap();
    assert!(available.status.success());
    assert_eq!(
        String::from_utf8(available.stdout).unwrap(),
        "AGENT  IDENTIFIER    NAME\n    1  uncontrolled  Uncontrolled\n"
    );
    assert!(
        ecra()
            .args([
                "report",
                "available-agents",
                "--json",
                available_json.to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .success()
    );

    assert!(
        ecra()
            .args(["new", store.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        ecra()
            .args([
                "generate-game",
                store.to_str().unwrap(),
                "AGENTS",
                "--seed",
                "42",
            ])
            .status()
            .unwrap()
            .success()
    );

    let game_agents = ecra()
        .args([
            "report",
            "game-agents",
            store.to_str().unwrap(),
            "AGENTS",
            "--json",
            game_json.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(game_agents.status.success());
    let agent_factions = ecra()
        .args([
            "report",
            "agent-factions",
            store.to_str().unwrap(),
            "AGENTS",
            "--json",
            factions_json.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(agent_factions.status.success());

    let agent_json = "[\n  {\n    \"id\": 1,\n    \"identifier\": \"uncontrolled\",\n    \"name\": \"Uncontrolled\"\n  }\n]\n";
    assert_eq!(std::fs::read_to_string(available_json).unwrap(), agent_json);
    assert_eq!(std::fs::read_to_string(game_json).unwrap(), agent_json);
    assert_eq!(
        std::fs::read_to_string(factions_json).unwrap(),
        "[\n  {\n    \"id\": 1,\n    \"identifier\": \"uncontrolled\",\n    \"name\": \"Uncontrolled\",\n    \"factions\": []\n  }\n]\n"
    );
}

#[test]
fn reports_stellia_to_stdout_and_deterministic_json() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("game.redb");
    let first_json = directory.path().join("stellia.json");
    let second_json = directory.path().join("stellia-again.json");
    assert!(
        ecra()
            .args(["new", store.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        ecra()
            .args([
                "generate-game",
                store.to_str().unwrap(),
                "REPORT",
                "--seed",
                "42",
            ])
            .status()
            .unwrap()
            .success()
    );

    let text = ecra()
        .args(["report", "stellia", store.to_str().unwrap(), "REPORT"])
        .output()
        .unwrap();
    assert!(text.status.success());
    let stdout = String::from_utf8(text.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "STELLIUM   X   Y   Z  STARS");
    assert_eq!(lines.len(), 101);

    for path in [&first_json, &second_json] {
        let output = ecra()
            .args([
                "report",
                "stellia",
                store.to_str().unwrap(),
                "REPORT",
                "--json",
                path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let first = std::fs::read_to_string(first_json).unwrap();
    let second = std::fs::read_to_string(second_json).unwrap();
    assert_eq!(first, second);
    let entries: serde_json::Value = serde_json::from_str(&first).unwrap();
    let entries = entries.as_array().unwrap();
    assert_eq!(entries.len(), 100);
    assert!(entries.iter().all(|entry| {
        entry.get("id").is_some()
            && entry.get("x").is_some()
            && entry.get("y").is_some()
            && entry.get("z").is_some()
            && entry.get("stars").is_some()
    }));
}

#[test]
fn adds_players_idempotently_and_reports_text_and_json() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("game.redb");
    let json = directory.path().join("players.json");
    assert!(
        ecra()
            .args(["new", store.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        ecra()
            .args([
                "generate-game",
                store.to_str().unwrap(),
                "PLAYERS",
                "--seed",
                "42",
            ])
            .status()
            .unwrap()
            .success()
    );

    let added = ecra()
        .args([
            "add-players",
            store.to_str().unwrap(),
            "PLAYERS",
            "zoe@example.com",
            "amy@example.com",
            "zoe@example.com",
        ])
        .output()
        .unwrap();
    assert!(added.status.success());
    assert_eq!(
        String::from_utf8(added.stdout).unwrap(),
        "Added 2 players to game PLAYERS\n"
    );

    let repeated = ecra()
        .args([
            "add-players",
            store.to_str().unwrap(),
            "PLAYERS",
            "amy@example.com",
        ])
        .output()
        .unwrap();
    assert!(repeated.status.success());
    assert_eq!(
        String::from_utf8(repeated.stdout).unwrap(),
        "Added 0 players to game PLAYERS\n"
    );

    let text = ecra()
        .args(["report", "players", store.to_str().unwrap(), "PLAYERS"])
        .output()
        .unwrap();
    assert!(text.status.success());
    assert_eq!(
        String::from_utf8(text.stdout).unwrap(),
        "EMAIL\namy@example.com\nzoe@example.com\n"
    );

    let saved = ecra()
        .args([
            "report",
            "players",
            store.to_str().unwrap(),
            "PLAYERS",
            "--json",
            json.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(saved.status.success());
    assert_eq!(
        std::fs::read_to_string(json).unwrap(),
        "[\n  {\n    \"email\": \"amy@example.com\"\n  },\n  {\n    \"email\": \"zoe@example.com\"\n  }\n]\n"
    );
}

#[test]
fn prints_application_version() {
    let output = ecra().arg("version").output().unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ecra 0.1.0-beta\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn creates_then_manages_a_store() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("game.redb");

    let created = ecra()
        .args(["new", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    assert!(path.is_file());

    let managed = ecra()
        .args(["manage", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        managed.status.success(),
        "{}",
        String::from_utf8_lossy(&managed.stderr)
    );
    let stdout = String::from_utf8(managed.stdout).unwrap();
    assert!(stdout.contains("Format version: 4"));
    assert!(stdout.contains("Current turn: 1"));
}

#[test]
fn new_refuses_to_replace_an_existing_store() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("game.redb");
    assert!(
        ecra()
            .args(["new", path.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );

    let duplicate = ecra()
        .args(["new", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    assert!(
        String::from_utf8(duplicate.stderr)
            .unwrap()
            .contains("already exists")
    );

    assert!(
        ecra()
            .args(["manage", path.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn generates_multiple_seeded_games_and_rejects_duplicate_codes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.redb");
    assert!(
        ecra()
            .args(["new", path.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );

    for (code, seed) in [("ALPHA", "10"), ("BETA-2", "20")] {
        let output = ecra()
            .args([
                "generate-game",
                path.to_str().unwrap(),
                code,
                "--seed",
                seed,
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains(&format!("game {code} with seed {seed}")));
        assert!(stdout.contains("status: setup, stellia: 100"));
        assert!(stdout.contains("minimum distance: 3"));
    }

    let custom_distance = ecra()
        .args([
            "generate-game",
            path.to_str().unwrap(),
            "GAMMA",
            "--seed",
            "30",
            "--minimum-distance",
            "5",
        ])
        .output()
        .unwrap();
    assert!(custom_distance.status.success());
    assert!(
        String::from_utf8(custom_distance.stdout)
            .unwrap()
            .contains("minimum distance: 5")
    );

    let managed = ecra()
        .args(["manage", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        String::from_utf8(managed.stdout)
            .unwrap()
            .contains("Games: 3")
    );

    let duplicate = ecra()
        .args([
            "generate-game",
            path.to_str().unwrap(),
            "ALPHA",
            "--seed",
            "99",
        ])
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    assert!(
        String::from_utf8(duplicate.stderr)
            .unwrap()
            .contains("already exists")
    );
}

#[test]
fn generate_game_rejects_a_non_uppercase_code() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("games.redb");
    assert!(
        ecra()
            .args(["new", path.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );

    let output = ecra()
        .args(["generate-game", path.to_str().unwrap(), "lowercase"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("invalid game code")
    );
}

#[test]
fn manage_rejects_a_missing_store() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing.redb");

    let output = ecra()
        .args(["manage", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("could not access store")
    );
    assert!(!path.exists());
}

#[test]
fn new_rejects_a_missing_directory_without_creating_it() {
    let directory = tempfile::tempdir().unwrap();
    let missing_directory = directory.path().join("missing").join("nested");
    let path = missing_directory.join("game.redb");

    let output = ecra()
        .args(["new", path.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("parent directory")
    );
    assert!(!directory.path().join("missing").exists());
}

#[test]
fn seed_accounts_is_idempotent() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("game.redb");
    assert!(
        ecra()
            .args(["new", path.to_str().unwrap()])
            .output()
            .unwrap()
            .status
            .success()
    );

    let first = ecra()
        .args(["seed-accounts", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(first.status.success());
    assert!(
        String::from_utf8(first.stdout)
            .unwrap()
            .contains("Created 13")
    );

    let second = ecra()
        .args(["seed-accounts", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(second.status.success());
    assert!(
        String::from_utf8(second.stdout)
            .unwrap()
            .contains("Created 0")
    );
}

#[test]
fn check_orders_succeeds_for_valid_syntax() {
    let output = ecra()
        .args([
            "check-orders",
            "tests/fixtures/orders/valid-complete.orders",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("No syntax errors found")
    );
}

#[test]
fn check_orders_reports_all_errors_and_fails() {
    let output = ecra()
        .args([
            "check-orders",
            "tests/fixtures/orders/multiple-syntax-errors.orders",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("multiple-syntax-errors.orders:1:"));
    assert!(stderr.contains("multiple-syntax-errors.orders:2:"));
    assert!(stderr.contains("multiple-syntax-errors.orders:3:"));
    assert!(stderr.contains("found 3 syntax errors"));
}

#[test]
fn import_orders_persists_and_parses_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("game.redb");
    let orders = directory.path().join("player.orders");
    assert!(
        ecra()
            .args(["new", store.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(
        &orders,
        concat!(
            "game ECRA turn 1;\n",
            "authenticate email account.0002@example.com with token \"amp.rocks.0002\";\n",
            "MOVE 1001 12;\n",
        ),
    )
    .unwrap();

    let output = ecra()
        .args([
            "import-orders",
            store.to_str().unwrap(),
            orders.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("order import 1"));
    assert!(stdout.contains("Parsed 1 player orders successfully"));
}

#[test]
fn import_orders_does_not_authenticate_the_token() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("game.redb");
    let orders = directory.path().join("rejected.orders");
    assert!(
        ecra()
            .args(["new", store.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(
        &orders,
        concat!(
            "game ECRA turn 1;\n",
            "authenticate email nobody@example.com with token \"not checked\";\n",
            "MOVE 1001 12;\n",
        ),
    )
    .unwrap();

    let output = ecra()
        .args([
            "import-orders",
            store.to_str().unwrap(),
            orders.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("order import 1")
    );
}

#[test]
fn import_orders_reports_body_syntax_errors_after_import() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("game.redb");
    let orders = directory.path().join("bad.orders");
    assert!(
        ecra()
            .args(["new", store.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    std::fs::write(
        &orders,
        concat!(
            "game ECRA turn 1;\n",
            "authenticate email account.0002@example.com with token \"amp.rocks.0002\";\n",
            "MOVE unknown 12;\n",
            "TRANSFER 1 FOOD LOST 2 3;\n",
        ),
    )
    .unwrap();

    let output = ecra()
        .args([
            "import-orders",
            store.to_str().unwrap(),
            orders.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("order import 1")
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("bad.orders:3:"));
    assert!(stderr.contains("bad.orders:4:"));
    assert!(stderr.contains("contains 2 syntax errors"));
}
