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

/// Minimal pull-request link presence indicator from the GitHub Search API.
/// When `pull_request` field is present (non-null) the item is a PR.
#[derive(Debug, Deserialize)]
pub struct RawPullRequestLink {
    pub url: String,
}

/// Raw search result item from the GitHub Search API.
/// Only fields required for normalization are captured.
#[derive(Debug, Deserialize)]
pub struct RawSearchItem {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub html_url: String,
    pub user: RawUser,
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

/// Normalized representation of a GitHub issue or pull request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedItem {
    pub item_type: ItemType,
    pub org: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub state: String,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub html_url: String,
    pub labels: Vec<String>,
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
        let item_type = if raw.pull_request.is_some() {
            ItemType::PullRequest
        } else {
            ItemType::Issue
        };

        let (org, repo) = parse_org_repo(&raw.html_url)?;

        Ok(NormalizedItem {
            item_type,
            org,
            repo,
            number: raw.number,
            title: raw.title,
            state: raw.state,
            author: raw.user.login,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
            closed_at: raw.closed_at,
            html_url: raw.html_url,
            labels: raw.labels.into_iter().map(|l| l.name).collect(),
        })
    }
}
