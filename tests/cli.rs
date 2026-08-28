use std::process::Command;

fn ecra() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ecra"))
}

#[test]
fn help_lists_store_commands() {
    let output = ecra().arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("new"));
    assert!(stdout.contains("manage"));
    assert!(stdout.contains("check-orders"));
    assert!(stdout.contains("help"));
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
    assert!(stdout.contains("Format version: 1"));
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
