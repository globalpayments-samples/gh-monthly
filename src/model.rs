use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Raw label as returned by the GitHub Search API.
#[derive(Debug, Deserialize)]
pub struct RawLabel {
    pub name: String,
}

/// Raw user object as returned by the GitHub Search API.
#[derive(Debug, Deserialize)]
pub struct RawUser {
    pub login: String,
}

/// Minimal pull-request link from the GitHub Search API.
/// When `pull_request` field is present (non-null) the item is a PR.
/// `merged_at` is included when available; it may be `None` in search payloads
/// if the PR has not been merged or the field is not populated by the endpoint.
#[derive(Debug, Deserialize)]
pub struct RawPullRequestLink {
    pub url: String,
    pub merged_at: Option<String>,
}

/// Raw search result item from the GitHub Search API.
#[derive(Debug, Deserialize)]
pub struct RawSearchItem {
    pub id: u64,
    pub node_id: String,
    pub number: u64,
    pub title: String,
    pub state: String,
    pub created_at: String,
    pub closed_at: Option<String>,
    pub html_url: String,
    /// The REST API URL for this item.
    pub url: String,
    /// `None` when the user account no longer exists (ghost).
    pub user: Option<RawUser>,
    pub labels: Vec<RawLabel>,
    /// Presence of this field indicates the item is a pull request.
    pub pull_request: Option<RawPullRequestLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Issue,
    PullRequest,
}

/// Canonical normalized representation of a GitHub issue or pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedItem {
    pub item_type: ItemType,
    pub org: String,
    pub repo: String,
    pub number: u64,
    pub id: u64,
    pub node_id: String,
    pub title: String,
    /// `None` when the original author account no longer exists (ghost).
    pub author_login: Option<String>,
    pub state: String,
    pub created_at: String,
    pub closed_at: Option<String>,
    /// `None` for issues or unmerged PRs.
    /// Note: the GitHub Search API may not populate this field; it is kept
    /// nullable and should not be synthesised.
    pub merged_at: Option<String>,
    pub html_url: String,
    /// REST API URL for this item (`url` field in the search response).
    pub api_url: String,
    pub labels: Vec<String>,
    pub is_pull_request: bool,
}

/// Parse `org` and `repo` from a GitHub HTML URL.
/// Expected format: `https://github.com/<org>/<repo>/...`
fn parse_org_repo(html_url: &str) -> Result<(String, String), AppError> {
    let path = html_url
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/");
    let mut parts = path.splitn(3, '/');
    let org = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Fetch(format!("cannot parse org from url: {html_url}")))?;
    let repo = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Fetch(format!("cannot parse repo from url: {html_url}")))?;
    Ok((org.to_string(), repo.to_string()))
}

impl TryFrom<RawSearchItem> for NormalizedItem {
    type Error = AppError;

    fn try_from(raw: RawSearchItem) -> Result<Self, Self::Error> {
        let is_pull_request = raw.pull_request.is_some();
        let merged_at = raw
            .pull_request
            .as_ref()
            .and_then(|pr| pr.merged_at.clone());

        let (org, repo) = parse_org_repo(&raw.html_url)?;

        Ok(NormalizedItem {
            item_type: if is_pull_request {
                ItemType::PullRequest
            } else {
                ItemType::Issue
            },
            org,
            repo,
            number: raw.number,
            id: raw.id,
            node_id: raw.node_id,
            title: raw.title,
            author_login: raw.user.map(|u| u.login),
            state: raw.state,
            created_at: raw.created_at,
            closed_at: raw.closed_at,
            merged_at,
            html_url: raw.html_url,
            api_url: raw.url,
            labels: raw.labels.into_iter().map(|l| l.name).collect(),
            is_pull_request,
        })
    }
}
