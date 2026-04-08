use reqwest::Client;
use serde::Deserialize;

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

/// Minimal search response envelope used for probing total_count.
#[derive(Debug, Deserialize)]
struct SearchEnvelope {
    total_count: u64,
}

/// GitHub Search API HTTP client.
pub struct GitHubClient {
    base_url: String,
    client: Client,
}

const PER_PAGE: u64 = 100;
const USER_AGENT: &str = concat!("gh-monthly/", env!("CARGO_PKG_VERSION"));

impl GitHubClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("failed to build reqwest client");
        Self {
            base_url: base_url.into(),
            client,
        }
    }

    /// Send a probe request (`per_page=1`) and return `(total_count, rate_limit)`.
    pub async fn probe(
        &self,
        query: &str,
        _kind: SearchKind,
    ) -> Result<(u64, RateLimitSnapshot), AppError> {
        let url = format!("{}/search/issues", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("q", query), ("per_page", "1")])
            .send()
            .await
            .map_err(|e| AppError::Fetch(e.to_string()))?;

        let rate_limit = parse_rate_limit_headers(&resp)?;

        let envelope: SearchEnvelope = resp
            .json()
            .await
            .map_err(|e| AppError::Fetch(e.to_string()))?;

        Ok((envelope.total_count, rate_limit))
    }

    /// Run probes for all orgs × kinds and compute a `PreflightEstimate`.
    pub async fn preflight_estimate(
        &self,
        orgs: &[String],
        start: &str,
        end: &str,
        month_id: &str,
    ) -> Result<PreflightEstimate, AppError> {
        let mut total_requests: u32 = 0;
        let mut last_rate_limit: Option<RateLimitSnapshot> = None;
        let mut high_volume_drivers: Vec<String> = Vec::new();

        for org in orgs {
            for kind in [SearchKind::Issue, SearchKind::PullRequest] {
                let query = kind.build_query(org, start, end);
                let (total_count, rate_limit) = self.probe(&query, kind.clone()).await?;
                let pages = total_count.div_ceil(PER_PAGE) as u32;
                if pages > 1 {
                    let label = format!(
                        "{}:{}",
                        org,
                        match kind {
                            SearchKind::Issue => "issue",
                            SearchKind::PullRequest => "pr",
                        }
                    );
                    high_volume_drivers.push(label);
                }
                total_requests += pages.max(1);
                last_rate_limit = Some(rate_limit);
            }
        }

        Ok(PreflightEstimate {
            estimated_requests: total_requests,
            month_id: month_id.to_string(),
            orgs: orgs.to_vec(),
            high_volume_drivers,
            rate_limit: last_rate_limit
                .ok_or_else(|| AppError::Fetch("no orgs provided".to_string()))?,
        })
    }
}

/// Parse GitHub rate-limit headers from a response.
pub fn parse_rate_limit_headers(
    resp: &reqwest::Response,
) -> Result<RateLimitSnapshot, AppError> {
    let headers = resp.headers();
    let remaining = header_u32(headers, "x-ratelimit-remaining")?;
    let reset_at = header_u64(headers, "x-ratelimit-reset")?;
    let limit = header_u32(headers, "x-ratelimit-limit")?;
    Ok(RateLimitSnapshot {
        remaining,
        reset_at,
        limit,
    })
}

fn header_u32(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<u32, AppError> {
    headers
        .get(name)
        .ok_or_else(|| AppError::Fetch(format!("missing header: {name}")))?
        .to_str()
        .map_err(|_| AppError::Fetch(format!("non-ascii header: {name}")))?
        .parse()
        .map_err(|_| AppError::Fetch(format!("non-numeric header: {name}")))
}

fn header_u64(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<u64, AppError> {
    headers
        .get(name)
        .ok_or_else(|| AppError::Fetch(format!("missing header: {name}")))?
        .to_str()
        .map_err(|_| AppError::Fetch(format!("non-ascii header: {name}")))?
        .parse()
        .map_err(|_| AppError::Fetch(format!("non-numeric header: {name}")))
}

/// Refuse a fetch run when the rate-limit budget is insufficient.
///
/// The check is: `estimated_requests + safety_margin <= remaining`.
pub fn ensure_preflight_budget(
    estimate: &PreflightEstimate,
    safety_margin: u32,
) -> Result<(), AppError> {
    let required = estimate.estimated_requests + safety_margin;
    let available = estimate.rate_limit.remaining;

    if required <= available {
        return Ok(());
    }

    let orgs = estimate.orgs.join(", ");
    let drivers = if estimate.high_volume_drivers.is_empty() {
        String::new()
    } else {
        format!(
            " High-volume queries: {}.",
            estimate.high_volume_drivers.join(", ")
        )
    };

    Err(AppError::Preflight(format!(
        "month={} orgs=[{orgs}] requires {required} requests \
         but only {available} remain in the current rate-limit window.{drivers}",
        estimate.month_id,
    )))
}
