use assert_cmd::Command;
use gh_monthly::model::{ItemType, NormalizedItem};
use std::fs;
use tempfile::tempdir;

fn make_item(number: u64, repo: &str) -> NormalizedItem {
    NormalizedItem {
        item_type: ItemType::Issue,
        org: "rust-lang".to_string(),
        repo: repo.to_string(),
        number,
        title: format!("Issue {number}"),
        state: "open".to_string(),
        author: "alice".to_string(),
        created_at: "2026-03-05T10:00:00Z".to_string(),
        updated_at: "2026-03-05T10:00:00Z".to_string(),
        closed_at: None,
        html_url: format!("https://github.com/rust-lang/{repo}/issues/{number}"),
        labels: vec!["bug".to_string()],
    }
}

fn write_fixture_jsonl(path: &std::path::Path, items: &[NormalizedItem]) {
    let mut lines = String::new();
    for item in items {
        lines.push_str(&serde_json::to_string(item).unwrap());
        lines.push('\n');
    }
    fs::write(path, lines).unwrap();
}

#[test]
fn report_shows_total_item_count() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("items.jsonl");

    let items = vec![
        make_item(1, "rust"),
        make_item(2, "rust"),
        make_item(3, "cargo"),
    ];
    write_fixture_jsonl(&file, &items);

    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["report", "--input", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("3"));
}

#[test]
fn report_shows_top_repo() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("items.jsonl");

    let items = vec![
        make_item(1, "rust"),
        make_item(2, "rust"),
        make_item(3, "cargo"),
    ];
    write_fixture_jsonl(&file, &items);

    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["report", "--input", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("rust-lang/rust"));
}

#[test]
fn report_json_format_is_valid_json() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("items.jsonl");

    let items = vec![make_item(1, "rust"), make_item(2, "cargo")];
    write_fixture_jsonl(&file, &items);

    let output = Command::cargo_bin("gh-monthly")
        .unwrap()
        .args([
            "report",
            "--input",
            file.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    assert!(value["total_items"].is_number());
}
