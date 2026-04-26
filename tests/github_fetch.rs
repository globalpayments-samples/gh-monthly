use gh_monthly::github::{GitHubClient, SearchKind};
use gh_monthly::model::ItemType;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_item(number: u64, kind: &str) -> serde_json::Value {
    let html_url = if kind == "issue" {
        format!("https://github.com/rust-lang/rust/issues/{number}")
    } else {
        format!("https://github.com/rust-lang/rust/pull/{number}")
    };
    let pull_request = if kind == "pr" {
        serde_json::json!({
            "url": format!("https://api.github.com/repos/rust-lang/rust/pulls/{number}"),
            "merged_at": null
        })
    } else {
        serde_json::Value::Null
    };
    serde_json::json!({
        "id": 100000_u64 + number,
        "node_id": format!("NODE{number}"),
        "number": number,
        "title": format!("Item {number}"),
        "state": "open",
        "created_at": "2026-03-05T10:00:00Z",
        "closed_at": null,
        "html_url": html_url,
        "url": format!("https://api.github.com/repos/rust-lang/rust/issues/{number}"),
        "user": { "login": "alice" },
        "labels": [],
        "pull_request": pull_request
    })
}

fn search_page(items: Vec<serde_json::Value>, total_count: u64) -> serde_json::Value {
    serde_json::json!({
        "total_count": total_count,
        "incomplete_results": false,
        "items": items
    })
}

fn rate_headers(remaining: u32) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .append_header("x-ratelimit-remaining", remaining.to_string().as_str())
        .append_header("x-ratelimit-reset", "9999999999")
        .append_header("x-ratelimit-limit", "30")
}

#[tokio::test]
async fn fetch_issues_two_pages() {
    let server = MockServer::start().await;

    // Page 1: 2 issues, total=3
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .respond_with(rate_headers(50).set_body_json(search_page(
            vec![make_item(1, "issue"), make_item(2, "issue")],
            3,
        )))
        .mount(&server)
        .await;

    // Page 2: 1 issue
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "2"))
        .respond_with(rate_headers(49).set_body_json(search_page(vec![make_item(3, "issue")], 3)))
        .mount(&server)
        .await;

    let client = GitHubClient::new(server.uri());
    let query = "org:rust-lang is:issue created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z";
    let items = client
        .fetch_all(query, SearchKind::Issue, 2)
        .await
        .expect("fetch should succeed");

    assert_eq!(items.len(), 3);
    assert!(items.iter().all(|i| i.item_type == ItemType::Issue));
    assert_eq!(items[0].number, 1);
    assert_eq!(items[2].number, 3);
}

#[tokio::test]
async fn fetch_prs_single_page() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .respond_with(rate_headers(50).set_body_json(search_page(
            vec![make_item(10, "pr"), make_item(11, "pr")],
            2,
        )))
        .mount(&server)
        .await;

    let client = GitHubClient::new(server.uri());
    let query = "org:rust-lang is:pr created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z";
    let items = client
        .fetch_all(query, SearchKind::PullRequest, 2)
        .await
        .expect("fetch should succeed");

    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| i.item_type == ItemType::PullRequest));
}

#[tokio::test]
async fn fetch_all_retries_transient_500() {
    let server = MockServer::start().await;

    // First request fails with 500
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second request succeeds
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .respond_with(rate_headers(50).set_body_json(search_page(vec![make_item(1, "issue")], 1)))
        .mount(&server)
        .await;

    let client = GitHubClient::new(server.uri());
    let query = "org:rust-lang is:issue created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z";
    let items = client
        .fetch_all(query, SearchKind::Issue, 100)
        .await
        .expect("fetch should succeed after retry");

    assert_eq!(items.len(), 1);
}

#[tokio::test]
async fn fetch_all_fails_after_retry_exhaustion() {
    let server = MockServer::start().await;

    // All requests fail with 500
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let client = GitHubClient::new(server.uri());
    let query = "org:rust-lang is:issue created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z";
    let result = client.fetch_all(query, SearchKind::Issue, 100).await;
    assert!(result.is_err(), "fetch should fail after retries exhausted");
}

#[tokio::test]
async fn fetch_all_into_calls_callback_per_item_and_returns_request_count() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .respond_with(rate_headers(50).set_body_json(search_page(
            vec![make_item(1, "issue"), make_item(2, "issue")],
            2,
        )))
        .mount(&server)
        .await;

    let client = GitHubClient::new(server.uri());
    let query = "org:rust-lang is:issue created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z";

    let mut collected = Vec::new();
    let req_count = client
        .fetch_all_into(query, SearchKind::Issue, 100, |item| {
            collected.push(item);
            Ok(())
        })
        .await
        .expect("fetch should succeed");

    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].number, 1);
    assert_eq!(req_count, 1);
}

#[tokio::test]
async fn fetch_all_into_paginates_correctly() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .respond_with(rate_headers(50).set_body_json(search_page(
            vec![make_item(1, "issue"), make_item(2, "issue")],
            3,
        )))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "2"))
        .respond_with(rate_headers(49).set_body_json(search_page(vec![make_item(3, "issue")], 3)))
        .mount(&server)
        .await;

    let client = GitHubClient::new(server.uri());
    let query = "org:rust-lang is:issue created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z";

    let mut numbers = Vec::new();
    let req_count = client
        .fetch_all_into(query, SearchKind::Issue, 2, |item| {
            numbers.push(item.number);
            Ok(())
        })
        .await
        .expect("fetch should succeed");

    assert_eq!(numbers, vec![1, 2, 3]);
    assert_eq!(req_count, 2);
}

#[tokio::test]
async fn fetch_all_fails_fast_on_4xx() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .respond_with(ResponseTemplate::new(422))
        .expect(1) // must not retry
        .mount(&server)
        .await;

    let client = GitHubClient::new(server.uri());
    let query = "org:rust-lang is:issue created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z";
    let result = client.fetch_all(query, SearchKind::Issue, 100).await;
    assert!(result.is_err(), "fetch should fail fast on 4xx");
}
