use gh_monthly::model::{ItemType, NormalizedItem, RawSearchItem};

fn make_issue_json() -> serde_json::Value {
    serde_json::json!({
        "id": 987654321_u64,
        "node_id": "MDU6SXNzdWU5ODc2NTQzMjE=",
        "number": 12345,
        "title": "Fix a bug",
        "state": "open",
        "created_at": "2026-03-05T10:00:00Z",
        "closed_at": null,
        "html_url": "https://github.com/rust-lang/rust/issues/12345",
        "url": "https://api.github.com/repos/rust-lang/rust/issues/12345",
        "user": { "login": "octocat" },
        "labels": [
            { "name": "bug" },
            { "name": "help wanted" }
        ],
        "pull_request": null
    })
}

fn make_pr_json() -> serde_json::Value {
    serde_json::json!({
        "id": 111222333_u64,
        "node_id": "MDExOlB1bGxSZXF1ZXN0MTExMjIyMzMz",
        "number": 99,
        "title": "Add a feature",
        "state": "closed",
        "created_at": "2026-03-10T08:00:00Z",
        "closed_at": "2026-03-11T09:00:00Z",
        "html_url": "https://github.com/rust-lang/cargo/pull/99",
        "url": "https://api.github.com/repos/rust-lang/cargo/pulls/99",
        "user": { "login": "bob" },
        "labels": [
            { "name": "enhancement" }
        ],
        "pull_request": {
            "url": "https://api.github.com/repos/rust-lang/cargo/pulls/99",
            "merged_at": "2026-03-11T09:00:00Z"
        }
    })
}

fn make_null_user_json() -> serde_json::Value {
    serde_json::json!({
        "id": 555666777_u64,
        "node_id": "MDU6SXNzdWU1NTU2NjY3Nzc=",
        "number": 7,
        "title": "Ghost issue",
        "state": "closed",
        "created_at": "2026-03-01T00:00:00Z",
        "closed_at": "2026-03-02T00:00:00Z",
        "html_url": "https://github.com/rust-lang/rust/issues/7",
        "url": "https://api.github.com/repos/rust-lang/rust/issues/7",
        "user": null,
        "labels": [],
        "pull_request": null
    })
}

#[test]
fn issue_maps_to_item_type_issue() {
    let raw: RawSearchItem = serde_json::from_value(make_issue_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.item_type, ItemType::Issue);
}

#[test]
fn pr_maps_to_item_type_pull_request() {
    let raw: RawSearchItem = serde_json::from_value(make_pr_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.item_type, ItemType::PullRequest);
}

#[test]
fn issue_org_and_repo_parsed_correctly() {
    let raw: RawSearchItem = serde_json::from_value(make_issue_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.org, "rust-lang");
    assert_eq!(item.repo, "rust");
}

#[test]
fn pr_org_and_repo_parsed_correctly() {
    let raw: RawSearchItem = serde_json::from_value(make_pr_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.org, "rust-lang");
    assert_eq!(item.repo, "cargo");
}

#[test]
fn labels_become_vec_of_strings() {
    let raw: RawSearchItem = serde_json::from_value(make_issue_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.labels, vec!["bug", "help wanted"]);
}

#[test]
fn author_login_is_parsed_correctly() {
    let raw: RawSearchItem = serde_json::from_value(make_issue_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.author_login.as_deref(), Some("octocat"));
}

#[test]
fn pr_author_login_is_parsed_correctly() {
    let raw: RawSearchItem = serde_json::from_value(make_pr_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.author_login.as_deref(), Some("bob"));
}

#[test]
fn null_user_results_in_none_author_login() {
    let raw: RawSearchItem = serde_json::from_value(make_null_user_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.author_login, None);
}

#[test]
fn issue_id_and_node_id_are_present() {
    let raw: RawSearchItem = serde_json::from_value(make_issue_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.id, 987654321);
    assert_eq!(item.node_id, "MDU6SXNzdWU5ODc2NTQzMjE=");
}

#[test]
fn issue_api_url_is_captured() {
    let raw: RawSearchItem = serde_json::from_value(make_issue_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(
        item.api_url,
        "https://api.github.com/repos/rust-lang/rust/issues/12345"
    );
}

#[test]
fn issue_is_not_pull_request() {
    let raw: RawSearchItem = serde_json::from_value(make_issue_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert!(!item.is_pull_request);
    assert_eq!(item.merged_at, None);
}

#[test]
fn pr_is_pull_request_with_merged_at() {
    let raw: RawSearchItem = serde_json::from_value(make_pr_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert!(item.is_pull_request);
    assert_eq!(item.merged_at.as_deref(), Some("2026-03-11T09:00:00Z"));
}
