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

fn rate_limit_headers(remaining: u32, reset_at: u64) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .append_header("x-ratelimit-remaining", remaining.to_string().as_str())
        .append_header("x-ratelimit-reset", reset_at.to_string().as_str())
        .append_header("x-ratelimit-limit", "30")
}

#[tokio::test]
async fn fetch_command_waits_for_rate_limit_reset_then_succeeds() {
    let server = MockServer::start().await;
    let reset_at = (chrono::Utc::now() + chrono::TimeDelta::seconds(1)).timestamp() as u64;

    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("per_page", "20"))
        .respond_with(rate_limit_headers(0, reset_at).set_body_json(search_page(vec![], 1)))
        .expect(2)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .and(query_param("per_page", "100"))
        .respond_with(
            rate_limit_headers(100, 9_999_999_999)
                .set_body_json(search_page(vec![issue_item(1)], 1)),
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

    assert!(dir.path().join("items.jsonl").exists());
    assert!(dir.path().join("metadata.json").exists());
}
