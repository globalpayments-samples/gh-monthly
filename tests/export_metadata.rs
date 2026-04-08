use gh_monthly::export::{write_metadata, PreflightMetadata, RunMetadata};
use std::fs;
use tempfile::tempdir;

fn make_metadata(completed: bool) -> RunMetadata {
    RunMetadata {
        completed,
        month_id: "2026-03".to_string(),
        orgs: vec!["rust-lang".to_string(), "tokio-rs".to_string()],
        preflight: PreflightMetadata {
            estimated_requests: 12,
            rate_limit_remaining: 80,
            rate_limit_reset_at: 9_999_999_999,
        },
        total_items: 42,
    }
}

#[test]
fn writes_completed_metadata() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("metadata.json");

    write_metadata(&path, &make_metadata(true)).expect("write_metadata should succeed");

    let content = fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(value["completed"], true);
    assert_eq!(value["month_id"], "2026-03");
}

#[test]
fn orgs_are_preserved() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("metadata.json");

    write_metadata(&path, &make_metadata(true)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();

    let orgs = value["orgs"].as_array().unwrap();
    assert_eq!(orgs.len(), 2);
    assert_eq!(orgs[0], "rust-lang");
    assert_eq!(orgs[1], "tokio-rs");
}

#[test]
fn preflight_fields_are_present() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("metadata.json");

    write_metadata(&path, &make_metadata(false)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(value["preflight"]["estimated_requests"].is_number());
    assert!(value["preflight"]["rate_limit_remaining"].is_number());
    assert!(value["preflight"]["rate_limit_reset_at"].is_number());
}

#[test]
fn output_is_pretty_printed() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("metadata.json");

    write_metadata(&path, &make_metadata(true)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    // Pretty-printed JSON has newlines
    assert!(content.contains('\n'), "output should be pretty-printed");
}
