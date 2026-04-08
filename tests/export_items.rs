use gh_monthly::export::{write_csv, write_jsonl};
use gh_monthly::model::{ItemType, NormalizedItem};
use std::fs;
use tempfile::tempdir;

fn make_item(number: u64, item_type: ItemType) -> NormalizedItem {
    NormalizedItem {
        item_type: item_type.clone(),
        org: "rust-lang".to_string(),
        repo: "rust".to_string(),
        number,
        id: 100000 + number,
        node_id: format!("NODE{number}"),
        title: format!("Item {number}"),
        author_login: Some("alice".to_string()),
        state: "open".to_string(),
        created_at: "2026-03-05T10:00:00Z".to_string(),
        closed_at: None,
        merged_at: None,
        html_url: format!("https://github.com/rust-lang/rust/issues/{number}"),
        api_url: format!("https://api.github.com/repos/rust-lang/rust/issues/{number}"),
        labels: vec!["bug".to_string()],
        is_pull_request: matches!(item_type, ItemType::PullRequest),
    }
}

#[test]
fn writes_jsonl_one_record_per_line() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("items.jsonl");

    let items = vec![
        make_item(1, ItemType::Issue),
        make_item(2, ItemType::PullRequest),
    ];

    write_jsonl(&path, &items).expect("write_jsonl should succeed");

    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 lines");

    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["number"], 1);
    assert_eq!(first["item_type"], "issue");

    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(second["number"], 2);
    assert_eq!(second["item_type"], "pull_request");
}

#[test]
fn jsonl_contains_canonical_fields() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("items.jsonl");

    let items = vec![make_item(1, ItemType::Issue)];
    write_jsonl(&path, &items).expect("write_jsonl should succeed");

    let content = fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(content.lines().next().unwrap()).unwrap();

    assert_eq!(value["author_login"], "alice");
    assert_eq!(value["is_pull_request"], false);
    assert!(value.get("api_url").is_some());
    assert!(value.get("merged_at").is_some());
    assert!(value.get("id").is_some());
    assert!(value.get("node_id").is_some());
}

#[test]
fn writes_csv_with_headers_and_records() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("items.csv");

    let items = vec![
        make_item(1, ItemType::Issue),
        make_item(2, ItemType::PullRequest),
    ];

    write_csv(&path, &items).expect("write_csv should succeed");

    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Header row + 2 data rows
    assert!(lines.len() >= 3, "expected at least 3 lines");
    assert!(
        lines[0].contains("number"),
        "header should contain 'number'"
    );
    assert!(
        lines[0].contains("item_type"),
        "header should contain 'item_type'"
    );
    assert!(
        lines[1].contains("rust-lang"),
        "first data row should contain org"
    );
}

#[test]
fn csv_has_canonical_header_order() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("items.csv");

    let items = vec![make_item(1, ItemType::Issue)];
    write_csv(&path, &items).expect("write_csv should succeed");

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains(
        "item_type,org,repo,number,id,node_id,title,author_login,state,created_at,closed_at,merged_at,html_url,api_url,labels,is_pull_request"
    ));
}

#[test]
fn creates_parent_directory_if_missing() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("subdir").join("items.jsonl");

    let items = vec![make_item(1, ItemType::Issue)];
    write_jsonl(&path, &items).expect("write_jsonl should create parent dir");

    assert!(path.exists());
}
