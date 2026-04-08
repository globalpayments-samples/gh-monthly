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
        serde_json::json!({ "url": format!("https://api.github.com/repos/rust-lang/rust/pulls/{number}") })
    } else {
        serde_json::Value::Null
    };
    serde_json::json!({
        "number": number,
        "title": format!("Item {number}"),
        "state": "open",
        "created_at": "2026-03-05T10:00:00Z",
        "updated_at": "2026-03-05T10:00:00Z",
        "closed_at": null,
        "html_url": html_url,
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

#[tokio::test]
async fn fetch_issues_two_pages() {
    let server = MockServer::start().await;

    // Page 1: 2 issues, total=3
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("x-ratelimit-remaining", "50")
                .append_header("x-ratelimit-reset", "9999999999")
                .append_header("x-ratelimit-limit", "30")
                .set_body_json(search_page(
                    vec![make_item(1, "issue"), make_item(2, "issue")],
                    3,
                )),
        )
        .mount(&server)
        .await;

    // Page 2: 1 issue
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("page", "2"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("x-ratelimit-remaining", "49")
                .append_header("x-ratelimit-reset", "9999999999")
                .append_header("x-ratelimit-limit", "30")
                .set_body_json(search_page(vec![make_item(3, "issue")], 3)),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("x-ratelimit-remaining", "50")
                .append_header("x-ratelimit-reset", "9999999999")
                .append_header("x-ratelimit-limit", "30")
                .set_body_json(search_page(
                    vec![make_item(10, "pr"), make_item(11, "pr")],
                    2,
                )),
        )
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
