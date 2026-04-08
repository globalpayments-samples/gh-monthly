use assert_cmd::Command;
use tempfile::tempdir;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn issue_item(number: u64) -> serde_json::Value {
    serde_json::json!({
        "id": 100000_u64 + number,
        "node_id": format!("NODE{number}"),
        "number": number,
        "title": format!("Issue {number}"),
        "state": "open",
        "created_at": "2026-03-05T10:00:00Z",
        "closed_at": null,
        "html_url": format!("https://github.com/rust-lang/rust/issues/{number}"),
        "url": format!("https://api.github.com/repos/rust-lang/rust/issues/{number}"),
        "user": { "login": "alice" },
        "labels": [],
        "pull_request": null
    })
}

fn search_page(items: Vec<serde_json::Value>, total: u64) -> serde_json::Value {
    serde_json::json!({
        "total_count": total,
        "incomplete_results": false,
        "items": items
    })
}

fn rate_limit_headers() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .append_header("x-ratelimit-remaining", "100")
        .append_header("x-ratelimit-reset", "9999999999")
        .append_header("x-ratelimit-limit", "30")
}

#[tokio::test]
async fn fetch_command_creates_output_files() {
    let server = MockServer::start().await;

    // Probe requests (per_page=1) for issues and PRs
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("per_page", "1"))
        .respond_with(rate_limit_headers().set_body_json(search_page(vec![], 2)))
        .expect(2) // one issue probe + one PR probe
        .mount(&server)
        .await;

    // Page fetch for issues (per_page=100, page=1)
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .respond_with(
            rate_limit_headers().set_body_json(search_page(vec![issue_item(1), issue_item(2)], 2)),
        )
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let out = dir.path().to_str().unwrap();

    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args([
            "fetch",
            "--org",
            "rust-lang",
            "--month",
            "2026-03",
            "--out",
            out,
            "--base-url",
            &server.uri(),
        ])
        .assert()
        .success();

    assert!(
        dir.path().join("items.jsonl").exists(),
        "items.jsonl should be created"
    );
    assert!(
        dir.path().join("metadata.json").exists(),
        "metadata.json should be created"
    );
}

#[tokio::test]
async fn fetch_command_writes_metadata_on_failure() {
    // Point to a server that returns errors — metadata.json should still appear
    let server = MockServer::start().await;

    // Return 500 for all requests
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let out = dir.path().to_str().unwrap();

    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args([
            "fetch",
            "--org",
            "rust-lang",
            "--month",
            "2026-03",
            "--out",
            out,
            "--base-url",
            &server.uri(),
        ])
        .assert()
        .failure();

    assert!(
        dir.path().join("metadata.json").exists(),
        "metadata.json should be written even on failure"
    );

    let content = std::fs::read_to_string(dir.path().join("metadata.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(value["completed"], false);
    assert_eq!(value["target_month"], "2026-03");
    assert!(value["run_started_at"].is_string());
}

#[tokio::test]
async fn fetch_command_jsonl_exists_even_when_later_fetch_fails() {
    let server = MockServer::start().await;

    // Probe: succeed for both issue and PR probes
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("per_page", "1"))
        .respond_with(rate_limit_headers().set_body_json(search_page(vec![], 1)))
        .expect(2)
        .mount(&server)
        .await;

    // Issue page 1: succeed with one item
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .and(query_param("per_page", "100"))
        .respond_with(
            rate_limit_headers().set_body_json(search_page(vec![issue_item(1)], 1)),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // PR page: always fail (simulates partial failure after some pages were written)
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let out = dir.path().to_str().unwrap();

    // Command should fail due to PR fetch error
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .args([
            "fetch",
            "--org",
            "rust-lang",
            "--month",
            "2026-03",
            "--out",
            out,
            "--base-url",
            &server.uri(),
        ])
        .assert()
        .failure();

    // metadata.json should indicate incomplete run
    let content = std::fs::read_to_string(dir.path().join("metadata.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(value["completed"], false);
}
