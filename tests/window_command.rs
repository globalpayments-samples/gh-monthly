use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn window_explicit_month_prints_month_id() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["window", "--month", "2026-03"])
        .assert()
        .success()
        .stdout(contains("2026-03"));
}

#[test]
fn window_invalid_month_exits_nonzero() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["window", "--month", "not-valid"])
        .assert()
        .failure();
}
