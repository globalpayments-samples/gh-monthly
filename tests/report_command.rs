use assert_cmd::Command;
use gh_monthly::model::{ItemType, NormalizedItem};
use std::fs;
use tempfile::tempdir;

fn make_item(number: u64, repo: &str, item_type: ItemType) -> NormalizedItem {
    NormalizedItem {
        item_type: item_type.clone(),
        org: "rust-lang".to_string(),
        repo: repo.to_string(),
        number,
        id: 100000 + number,
        node_id: format!("NODE{number}"),
        title: format!("Issue {number}"),
        author_login: Some("alice".to_string()),
        state: "open".to_string(),
        created_at: "2026-03-05T10:00:00Z".to_string(),
        closed_at: None,
        merged_at: None,
        html_url: format!("https://github.com/rust-lang/{repo}/issues/{number}"),
        api_url: format!("https://api.github.com/repos/rust-lang/{repo}/issues/{number}"),
        labels: vec!["bug".to_string()],
        is_pull_request: matches!(item_type, ItemType::PullRequest),
    }
}

fn make_metadata(target_month: &str) -> serde_json::Value {
    serde_json::json!({
        "tool": "gh-monthly",
        "run_started_at": "2026-04-08T10:00:00Z",
        "run_completed_at": "2026-04-08T10:05:00Z",
        "completed": true,
        "requested_orgs": ["rust-lang"],
        "target_month": target_month,
        "window_start": "2026-03-01T00:00:00Z",
        "window_end": "2026-04-01T00:00:00Z",
        "formats": ["jsonl"],
        "total_records": 3,
        "request_count": 2,
        "preflight": {
            "estimated_requests": 2,
            "rate_limit_remaining": 90,
            "rate_limit_reset_at": "2026-04-08T10:30:00Z",
            "safety_margin_requests": 5
        }
    })
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
        make_item(1, "rust", ItemType::Issue),
        make_item(2, "rust", ItemType::Issue),
        make_item(3, "cargo", ItemType::Issue),
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
        make_item(1, "rust", ItemType::Issue),
        make_item(2, "rust", ItemType::Issue),
        make_item(3, "cargo", ItemType::Issue),
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
fn report_shows_target_month_from_metadata() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("items.jsonl");
    let meta = dir.path().join("metadata.json");

    let items = vec![
        make_item(1, "rust", ItemType::Issue),
        make_item(2, "rust", ItemType::PullRequest),
    ];
    write_fixture_jsonl(&file, &items);
    fs::write(
        &meta,
        serde_json::to_string_pretty(&make_metadata("2026-03")).unwrap(),
    )
    .unwrap();

    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args(["report", "--input", file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("2026-03"));
}

#[test]
fn report_shows_issue_and_pr_counts() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("items.jsonl");

    let items = vec![
        make_item(1, "rust", ItemType::Issue),
        make_item(2, "rust", ItemType::Issue),
        make_item(3, "rust", ItemType::PullRequest),
    ];
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
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["issue_count"], 2);
    assert_eq!(value["pull_request_count"], 1);
    assert!(value["busiest_days"].is_array());
}

#[test]
fn report_json_format_is_valid_json() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("items.jsonl");

    let items = vec![
        make_item(1, "rust", ItemType::Issue),
        make_item(2, "cargo", ItemType::Issue),
    ];
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

#[test]
fn report_aggregates_multiple_inputs() {
    let dir_a = tempdir().unwrap();
    let dir_b = tempdir().unwrap();
    let file_a = dir_a.path().join("items.jsonl");
    let file_b = dir_b.path().join("items.jsonl");
    let meta_a = dir_a.path().join("metadata.json");
    let meta_b = dir_b.path().join("metadata.json");

    write_fixture_jsonl(
        &file_a,
        &[
            make_item(1, "rust", ItemType::Issue),
            make_item(2, "cargo", ItemType::PullRequest),
        ],
    );
    write_fixture_jsonl(&file_b, &[make_item(3, "rust", ItemType::Issue)]);
    fs::write(
        &meta_a,
        serde_json::to_string_pretty(&make_metadata("2026-03")).unwrap(),
    )
    .unwrap();
    fs::write(
        &meta_b,
        serde_json::to_string_pretty(&make_metadata("2026-03")).unwrap(),
    )
    .unwrap();

    let output = Command::cargo_bin("gh-monthly")
        .unwrap()
        .args([
            "report",
            "--input",
            file_a.to_str().unwrap(),
            "--input",
            file_b.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["target_month"], "2026-03");
    assert_eq!(value["total_items"], 3);
    assert_eq!(value["issue_count"], 2);
    assert_eq!(value["pull_request_count"], 1);
    assert_eq!(value["by_org"][0][0], "rust-lang");
    assert_eq!(value["by_org"][0][1], 3);
}

#[test]
fn report_shows_included_months_for_multiple_inputs() {
    let dir_a = tempdir().unwrap();
    let dir_b = tempdir().unwrap();
    let file_a = dir_a.path().join("items.jsonl");
    let file_b = dir_b.path().join("items.jsonl");
    let meta_a = dir_a.path().join("metadata.json");
    let meta_b = dir_b.path().join("metadata.json");

    write_fixture_jsonl(&file_a, &[make_item(1, "rust", ItemType::Issue)]);
    write_fixture_jsonl(&file_b, &[make_item(2, "cargo", ItemType::Issue)]);
    fs::write(
        &meta_a,
        serde_json::to_string_pretty(&make_metadata("2026-02")).unwrap(),
    )
    .unwrap();
    fs::write(
        &meta_b,
        serde_json::to_string_pretty(&make_metadata("2026-03")).unwrap(),
    )
    .unwrap();

    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args([
            "report",
            "--input",
            file_a.to_str().unwrap(),
            "--input",
            file_b.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Month: 2026-02, 2026-03"));
}
