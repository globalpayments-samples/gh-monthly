use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn fetch_without_org_fails() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["fetch", "--month", "2026-03"])
        .assert()
        .failure();
}

#[test]
fn fetch_with_invalid_month_fails() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["fetch", "--org", "rust-lang", "--month", "not-a-month"])
        .assert()
        .failure();
}

#[test]
fn fetch_with_month_out_of_range_fails() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["fetch", "--org", "rust-lang", "--month", "2026-13"])
        .assert()
        .failure();
}

#[test]
fn report_with_nonexistent_input_fails() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["report", "--input", "/nonexistent/path/items.jsonl"])
        .assert()
        .failure();
}

#[test]
fn fetch_with_invalid_format_fails() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args([
            "fetch",
            "--org",
            "rust-lang",
            "--month",
            "2026-03",
            "--format",
            "invalid-format",
        ])
        .assert()
        .failure();
}

#[test]
fn window_with_invalid_month_fails() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["window", "--month", "2026-99"])
        .assert()
        .failure();
}

#[test]
fn report_with_valid_existing_input_succeeds() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("items.jsonl");
    std::fs::write(&file, "").unwrap(); // empty file is valid (0 items)

    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["report", "--input", file.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn report_without_input_fails() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["report"])
        .assert()
        .failure();
}

#[test]
fn report_with_malformed_metadata_falls_back_to_unknown_month() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("items.jsonl");
    let meta = dir.path().join("metadata.json");

    std::fs::write(&file, "").unwrap();
    // Write intentionally malformed metadata
    std::fs::write(&meta, "not valid json at all").unwrap();

    let output = Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["report", "--input", file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "report should succeed even with malformed metadata"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("unknown"),
        "should print 'unknown' month when metadata is malformed"
    );
}
