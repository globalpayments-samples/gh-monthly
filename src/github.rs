use crate::error::AppError;

/// The kind of GitHub search query to build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchKind {
    Issue,
    PullRequest,
}

impl SearchKind {
    pub fn build_query(&self, org: &str, start: &str, end: &str) -> String {
        let kind_filter = match self {
            SearchKind::Issue => "is:issue",
            SearchKind::PullRequest => "is:pr",
        };
        format!("org:{org} {kind_filter} created:{start}..{end}")
    }
}

/// Build an issue search query string.
pub fn build_issue_query(org: &str, start: &str, end: &str) -> String {
    SearchKind::Issue.build_query(org, start, end)
}

/// Build a pull request search query string.
pub fn build_pr_query(org: &str, start: &str, end: &str) -> String {
    SearchKind::PullRequest.build_query(org, start, end)
}

/// Build an issue search query string, returning an error for invalid inputs.
pub fn build_issue_query_validated(
    org: &str,
    start: &str,
    end: &str,
) -> Result<String, AppError> {
    if org.is_empty() {
        return Err(AppError::Usage("org name must not be empty".to_string()));
    }
    Ok(build_issue_query(org, start, end))
}

/// Build a PR search query string, returning an error for invalid inputs.
pub fn build_pr_query_validated(org: &str, start: &str, end: &str) -> Result<String, AppError> {
    if org.is_empty() {
        return Err(AppError::Usage("org name must not be empty".to_string()));
    }
    Ok(build_pr_query(org, start, end))
}

/// Snapshot of GitHub Search API rate-limit headers.
#[derive(Debug, Clone)]
pub struct RateLimitSnapshot {
    /// Requests remaining in the current window.
    pub remaining: u32,
    /// Unix timestamp when the window resets.
    pub reset_at: u64,
    /// Total request limit for the window.
    pub limit: u32,
}

/// Estimated request count needed to satisfy a fetch run.
#[derive(Debug, Clone)]
pub struct PreflightEstimate {
    /// Total estimated page requests needed.
    pub estimated_requests: u32,
    /// Month being fetched.
    pub month_id: String,
    /// Orgs included in this run.
    pub orgs: Vec<String>,
    /// Query families with high result counts (issue or pr per org).
    pub high_volume_drivers: Vec<String>,
    /// Rate-limit snapshot at preflight time.
    pub rate_limit: RateLimitSnapshot,
}
