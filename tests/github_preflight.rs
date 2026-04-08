use gh_monthly::github::{GitHubClient, SearchKind};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn search_response(total_count: u64) -> serde_json::Value {
    serde_json::json!({
        "total_count": total_count,
        "incomplete_results": false,
        "items": []
    })
}

#[tokio::test]
async fn preflight_probe_returns_estimate() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("per_page", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("x-ratelimit-remaining", "50")
                .append_header("x-ratelimit-reset", "9999999999")
                .append_header("x-ratelimit-limit", "30")
                .set_body_json(search_response(87)),
        )
        .mount(&server)
        .await;

    let client = GitHubClient::new(server.uri());
    let query = "org:rust-lang is:issue created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z";
    let (total_count, rate_limit) = client
        .probe(query, SearchKind::Issue)
        .await
        .expect("probe should succeed");

    assert_eq!(total_count, 87);
    assert_eq!(rate_limit.remaining, 50);
    assert_eq!(rate_limit.limit, 30);
}

#[tokio::test]
async fn preflight_estimates_request_count() {
    let server = MockServer::start().await;

    // Issue probe: 87 results → ceil(87/100) = 1 page
    Mock::given(method("GET"))
        .and(path("/search/issues"))
        .and(query_param("per_page", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("x-ratelimit-remaining", "100")
                .append_header("x-ratelimit-reset", "9999999999")
                .append_header("x-ratelimit-limit", "30")
                .set_body_json(search_response(87)),
        )
        .expect(2) // one for issues, one for PRs
        .mount(&server)
        .await;

    let client = GitHubClient::new(server.uri());
    let estimate = client
        .preflight_estimate(
            &["rust-lang".to_string()],
            "2026-03-01T00:00:00Z",
            "2026-04-01T00:00:00Z",
            "2026-03",
        )
        .await
        .expect("preflight should succeed");

    // 2 orgs × 2 kinds × ceil(87/100)=1 page each = 2 requests
    assert_eq!(estimate.estimated_requests, 2);
    assert_eq!(estimate.orgs, vec!["rust-lang"]);
    assert_eq!(estimate.month_id, "2026-03");
}
