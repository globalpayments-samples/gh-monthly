use gh_monthly::model::{ItemType, NormalizedItem, RawSearchItem};

fn make_issue_json() -> serde_json::Value {
    serde_json::json!({
        "number": 42,
        "title": "Fix a bug",
        "state": "open",
        "created_at": "2026-03-05T10:00:00Z",
        "updated_at": "2026-03-06T12:00:00Z",
        "closed_at": null,
        "html_url": "https://github.com/rust-lang/rust/issues/42",
        "user": { "login": "alice" },
        "labels": [
            { "name": "bug" },
            { "name": "help wanted" }
        ],
        "pull_request": null
    })
}

fn make_pr_json() -> serde_json::Value {
    serde_json::json!({
        "number": 99,
        "title": "Add a feature",
        "state": "closed",
        "created_at": "2026-03-10T08:00:00Z",
        "updated_at": "2026-03-11T09:00:00Z",
        "closed_at": "2026-03-11T09:00:00Z",
        "html_url": "https://github.com/rust-lang/cargo/pull/99",
        "user": { "login": "bob" },
        "labels": [
            { "name": "enhancement" }
        ],
        "pull_request": { "url": "https://api.github.com/repos/rust-lang/cargo/pulls/99" }
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
fn author_is_parsed_correctly() {
    let raw: RawSearchItem = serde_json::from_value(make_issue_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.author, "alice");
}

#[test]
fn pr_author_is_parsed_correctly() {
    let raw: RawSearchItem = serde_json::from_value(make_pr_json()).unwrap();
    let item = NormalizedItem::try_from(raw).unwrap();
    assert_eq!(item.author, "bob");
}
