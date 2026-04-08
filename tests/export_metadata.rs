use gh_monthly::export::{write_metadata, PreflightMetadata, RunMetadata};
use std::fs;
use tempfile::tempdir;

fn make_metadata(completed: bool) -> RunMetadata {
    RunMetadata {
        tool: "gh-monthly".to_string(),
        run_started_at: "2026-04-08T10:00:00Z".to_string(),
        run_completed_at: if completed {
            Some("2026-04-08T10:05:00Z".to_string())
        } else {
            None
        },
        completed,
        requested_orgs: vec!["rust-lang".to_string(), "tokio-rs".to_string()],
        target_month: "2026-03".to_string(),
        window_start: "2026-03-01T00:00:00Z".to_string(),
        window_end: "2026-04-01T00:00:00Z".to_string(),
        formats: vec!["jsonl".to_string(), "csv".to_string()],
        total_records: 42,
        request_count: 5,
        preflight: PreflightMetadata {
            estimated_requests: 12,
            rate_limit_remaining: 80,
            rate_limit_reset_at: "2026-04-08T10:30:00Z".to_string(),
            safety_margin_requests: 10,
        },
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
    assert_eq!(value["target_month"], "2026-03");
}

#[test]
fn requested_orgs_are_preserved() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("metadata.json");

    write_metadata(&path, &make_metadata(true)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();

    let orgs = value["requested_orgs"].as_array().unwrap();
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
    assert!(value["preflight"]["rate_limit_reset_at"].is_string());
    assert!(value["preflight"]["safety_margin_requests"].is_number());
}

#[test]
fn full_metadata_contract_is_present() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("metadata.json");

    write_metadata(&path, &make_metadata(true)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(value["tool"], "gh-monthly");
    assert!(value["run_started_at"].is_string());
    assert!(value["window_start"].is_string());
    assert!(value["window_end"].is_string());
    assert!(value["formats"].is_array());
    assert!(value["request_count"].is_number());
    assert!(value["preflight"]["safety_margin_requests"].is_number());
}

#[test]
fn output_is_pretty_printed() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("metadata.json");

    write_metadata(&path, &make_metadata(true)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains('\n'), "output should be pretty-printed");
}
