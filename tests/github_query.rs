use gh_monthly::github::{build_issue_query, build_pr_query, SearchKind};

#[test]
fn issue_query_contains_required_components() {
    let query = build_issue_query("rust-lang", "2026-03-01T00:00:00Z", "2026-04-01T00:00:00Z");
    assert!(query.contains("org:rust-lang"), "missing org filter");
    assert!(query.contains("is:issue"), "missing is:issue");
    assert!(
        query.contains("created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z"),
        "missing created range"
    );
    assert!(
        query.contains("-author:macroscope-cloud"),
        "missing macroscope-cloud exclusion"
    );
    assert!(
        query.contains("-author:dependabot"),
        "missing dependabot exclusion"
    );
}

#[test]
fn pr_query_contains_required_components() {
    let query = build_pr_query("rust-lang", "2026-03-01T00:00:00Z", "2026-04-01T00:00:00Z");
    assert!(query.contains("org:rust-lang"), "missing org filter");
    assert!(query.contains("is:pr"), "missing is:pr");
    assert!(
        query.contains("created:2026-03-01T00:00:00Z..2026-04-01T00:00:00Z"),
        "missing created range"
    );
    assert!(
        query.contains("-author:macroscope-cloud"),
        "missing macroscope-cloud exclusion"
    );
    assert!(
        query.contains("-author:dependabot"),
        "missing dependabot exclusion"
    );
}

#[test]
fn search_kind_issue_builds_correct_query() {
    let kind = SearchKind::Issue;
    let query = kind.build_query("tokio-rs", "2026-02-01T00:00:00Z", "2026-03-01T00:00:00Z");
    assert!(query.contains("is:issue"));
    assert!(query.contains("org:tokio-rs"));
}

#[test]
fn search_kind_pr_builds_correct_query() {
    let kind = SearchKind::PullRequest;
    let query = kind.build_query("tokio-rs", "2026-02-01T00:00:00Z", "2026-03-01T00:00:00Z");
    assert!(query.contains("is:pr"));
    assert!(query.contains("org:tokio-rs"));
}

#[test]
fn empty_org_name_is_rejected() {
    let result = build_issue_query_validated("", "2026-03-01T00:00:00Z", "2026-04-01T00:00:00Z");
    assert!(result.is_err(), "expected error for empty org");
}

// Helper that wraps the validated variant
fn build_issue_query_validated(
    org: &str,
    start: &str,
    end: &str,
) -> Result<String, gh_monthly::error::AppError> {
    gh_monthly::github::build_issue_query_validated(org, start, end)
}
