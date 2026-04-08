use gh_monthly::model::{ItemType, NormalizedItem};
use gh_monthly::report::aggregate;

fn item(
    org: &str,
    repo: &str,
    author: &str,
    labels: Vec<&str>,
    created_at: &str,
    item_type: ItemType,
) -> NormalizedItem {
    NormalizedItem {
        item_type,
        org: org.to_string(),
        repo: repo.to_string(),
        number: 1,
        title: "test".to_string(),
        state: "open".to_string(),
        author: author.to_string(),
        created_at: created_at.to_string(),
        updated_at: created_at.to_string(),
        closed_at: None,
        html_url: format!("https://github.com/{org}/{repo}/issues/1"),
        labels: labels.into_iter().map(String::from).collect(),
    }
}

#[test]
fn total_count_is_correct() {
    let items = vec![
        item("rust-lang", "rust", "alice", vec![], "2026-03-01T00:00:00Z", ItemType::Issue),
        item("rust-lang", "rust", "bob", vec![], "2026-03-02T00:00:00Z", ItemType::PullRequest),
        item("tokio-rs", "tokio", "alice", vec![], "2026-03-03T00:00:00Z", ItemType::Issue),
    ];
    let summary = aggregate(&items);
    assert_eq!(summary.total_items, 3);
}

#[test]
fn per_org_counts_are_correct() {
    let items = vec![
        item("rust-lang", "rust", "alice", vec![], "2026-03-01T00:00:00Z", ItemType::Issue),
        item("rust-lang", "cargo", "bob", vec![], "2026-03-02T00:00:00Z", ItemType::Issue),
        item("tokio-rs", "tokio", "alice", vec![], "2026-03-03T00:00:00Z", ItemType::Issue),
    ];
    let summary = aggregate(&items);
    let rust_count = summary
        .by_org
        .iter()
        .find(|(org, _)| org == "rust-lang")
        .map(|(_, c)| *c)
        .unwrap_or(0);
    assert_eq!(rust_count, 2);
}

#[test]
fn top_repo_is_most_frequent() {
    let items = vec![
        item("rust-lang", "rust", "alice", vec![], "2026-03-01T00:00:00Z", ItemType::Issue),
        item("rust-lang", "rust", "bob", vec![], "2026-03-02T00:00:00Z", ItemType::Issue),
        item("rust-lang", "cargo", "alice", vec![], "2026-03-03T00:00:00Z", ItemType::Issue),
    ];
    let summary = aggregate(&items);
    assert_eq!(summary.top_repos[0].0, "rust-lang/rust");
    assert_eq!(summary.top_repos[0].1, 2);
}

#[test]
fn top_author_is_most_frequent() {
    let items = vec![
        item("rust-lang", "rust", "alice", vec![], "2026-03-01T00:00:00Z", ItemType::Issue),
        item("rust-lang", "rust", "alice", vec![], "2026-03-02T00:00:00Z", ItemType::Issue),
        item("rust-lang", "rust", "bob", vec![], "2026-03-03T00:00:00Z", ItemType::Issue),
    ];
    let summary = aggregate(&items);
    assert_eq!(summary.top_authors[0].0, "alice");
    assert_eq!(summary.top_authors[0].1, 2);
}

#[test]
fn top_label_is_most_frequent() {
    let items = vec![
        item("rust-lang", "rust", "alice", vec!["bug", "perf"], "2026-03-01T00:00:00Z", ItemType::Issue),
        item("rust-lang", "rust", "bob", vec!["bug"], "2026-03-02T00:00:00Z", ItemType::Issue),
        item("rust-lang", "rust", "carol", vec!["enhancement"], "2026-03-03T00:00:00Z", ItemType::Issue),
    ];
    let summary = aggregate(&items);
    assert_eq!(summary.top_labels[0].0, "bug");
    assert_eq!(summary.top_labels[0].1, 2);
}

#[test]
fn busiest_day_is_the_day_with_most_items() {
    let items = vec![
        item("rust-lang", "rust", "alice", vec![], "2026-03-05T00:00:00Z", ItemType::Issue),
        item("rust-lang", "rust", "bob", vec![], "2026-03-05T12:00:00Z", ItemType::Issue),
        item("rust-lang", "rust", "carol", vec![], "2026-03-10T00:00:00Z", ItemType::Issue),
    ];
    let summary = aggregate(&items);
    let (day, count) = summary.busiest_day.unwrap();
    assert_eq!(day, "2026-03-05");
    assert_eq!(count, 2);
}
