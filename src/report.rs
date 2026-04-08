use std::collections::HashMap;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::model::{ItemType, NormalizedItem};

/// Aggregated monthly summary of a set of normalized items.
pub struct ReportSummary {
    /// Target month derived from sibling metadata, or `"unknown"` if unavailable.
    pub target_month: String,
    pub total_items: u64,
    pub issue_count: u64,
    pub pull_request_count: u64,
    /// Per-org item counts: `(org, count)` sorted descending by count.
    pub by_org: Vec<(String, u64)>,
    /// Top repos by item count: `(org/repo, count)` sorted descending.
    pub top_repos: Vec<(String, u64)>,
    /// Top authors by item count: `(author_login, count)` sorted descending.
    pub top_authors: Vec<(String, u64)>,
    /// Top labels by item count: `(label, count)` sorted descending.
    pub top_labels: Vec<(String, u64)>,
    /// Days sorted descending by item count: `(YYYY-MM-DD, count)`.
    pub busiest_days: Vec<(String, u64)>,
}

/// Aggregate a slice of normalized items into a `ReportSummary`.
///
/// `target_month` should be derived from sibling metadata before calling this;
/// pass `"unknown"` if metadata is unavailable.
pub fn aggregate(items: &[NormalizedItem]) -> ReportSummary {
    aggregate_with_month(items, "unknown")
}

/// Aggregate with an explicit target month string.
pub fn aggregate_with_month(items: &[NormalizedItem], target_month: &str) -> ReportSummary {
    let mut by_org: HashMap<String, u64> = HashMap::new();
    let mut by_repo: HashMap<String, u64> = HashMap::new();
    let mut by_author: HashMap<String, u64> = HashMap::new();
    let mut by_label: HashMap<String, u64> = HashMap::new();
    let mut by_day: HashMap<String, u64> = HashMap::new();
    let mut issue_count: u64 = 0;
    let mut pull_request_count: u64 = 0;

    for item in items {
        *by_org.entry(item.org.clone()).or_insert(0) += 1;
        *by_repo
            .entry(format!("{}/{}", item.org, item.repo))
            .or_insert(0) += 1;

        if let Some(login) = &item.author_login {
            *by_author.entry(login.clone()).or_insert(0) += 1;
        }

        for label in &item.labels {
            *by_label.entry(label.clone()).or_insert(0) += 1;
        }

        if item.created_at.len() >= 10 {
            let day = item.created_at[..10].to_string();
            *by_day.entry(day).or_insert(0) += 1;
        }

        match item.item_type {
            ItemType::Issue => issue_count += 1,
            ItemType::PullRequest => pull_request_count += 1,
        }
    }

    ReportSummary {
        target_month: target_month.to_string(),
        total_items: items.len() as u64,
        issue_count,
        pull_request_count,
        by_org: sorted_desc(by_org),
        top_repos: sorted_desc(by_repo),
        top_authors: sorted_desc(by_author),
        top_labels: sorted_desc(by_label),
        busiest_days: sorted_desc(by_day),
    }
}

/// Sort a `HashMap<String, u64>` into a `Vec<(String, u64)>` descending by count.
fn sorted_desc(map: HashMap<String, u64>) -> Vec<(String, u64)> {
    let mut v: Vec<(String, u64)> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// Try to read `target_month` from a sibling `metadata.json` next to `items_path`.
///
/// Returns `"unknown"` if the file is absent, malformed, or missing the field.
fn read_target_month_from_metadata(items_path: &Path) -> String {
    let meta_path = items_path.with_file_name("metadata.json");
    let Ok(content) = std::fs::read_to_string(&meta_path) else {
        return "unknown".to_string();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return "unknown".to_string();
    };
    value
        .get("target_month")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Read normalized items from a JSONL file and aggregate them into a `ReportSummary`.
///
/// Automatically reads a sibling `metadata.json` for `target_month` if present.
pub fn read_and_aggregate(path: &Path) -> Result<ReportSummary, AppError> {
    let items = read_jsonl(path)?;
    let target_month = read_target_month_from_metadata(path);
    Ok(aggregate_with_month(&items, &target_month))
}

/// Read normalized items from multiple JSONL files and aggregate them into a `ReportSummary`.
///
/// Uses a single shared month when all inputs agree, or a sorted comma-separated
/// month list when multiple inputs contribute different known months.
pub fn read_and_aggregate_many(paths: &[std::path::PathBuf]) -> Result<ReportSummary, AppError> {
    let mut all_items = Vec::new();
    let mut months = BTreeSet::new();

    for path in paths {
        all_items.extend(read_jsonl(path)?);

        let month = read_target_month_from_metadata(path);
        if month != "unknown" {
            months.insert(month);
        }
    }

    let target_month = match months.len() {
        0 => "unknown".to_string(),
        1 => months.into_iter().next().unwrap_or_else(|| "unknown".to_string()),
        _ => months.into_iter().collect::<Vec<_>>().join(", "),
    };

    Ok(aggregate_with_month(
        &all_items,
        &target_month,
    ))
}

/// Read normalized items from a JSONL file.
pub fn read_jsonl(path: &Path) -> Result<Vec<NormalizedItem>, AppError> {
    let file = File::open(path)
        .map_err(|e| AppError::ReportInput(format!("cannot open {}: {e}", path.display())))?;
    let reader = BufReader::new(file);
    let mut items = Vec::new();
    for (line_num, line) in reader.lines().enumerate() {
        let line =
            line.map_err(|e| AppError::ReportInput(format!("read error at line {line_num}: {e}")))?;
        if line.trim().is_empty() {
            continue;
        }
        let item: NormalizedItem = serde_json::from_str(&line)
            .map_err(|e| AppError::ReportInput(format!("parse error at line {line_num}: {e}")))?;
        items.push(item);
    }
    Ok(items)
}

/// Render a `ReportSummary` as a plain-text table.
pub fn render_table(summary: &ReportSummary) -> String {
    let mut out = String::new();

    out.push_str(&format!("Month: {}\n", summary.target_month));
    out.push_str(&format!("Total items: {}\n", summary.total_items));
    out.push_str(&format!("Issues: {}\n", summary.issue_count));
    out.push_str(&format!("Pull requests: {}\n", summary.pull_request_count));

    if !summary.by_org.is_empty() {
        out.push_str("\nBy org:\n");
        for (org, count) in &summary.by_org {
            out.push_str(&format!("  {org}: {count}\n"));
        }
    }

    if !summary.top_repos.is_empty() {
        out.push_str("\nTop repos:\n");
        for (repo, count) in summary.top_repos.iter().take(10) {
            out.push_str(&format!("  {repo}: {count}\n"));
        }
    }

    if !summary.top_authors.is_empty() {
        out.push_str("\nTop authors:\n");
        for (author, count) in summary.top_authors.iter().take(10) {
            out.push_str(&format!("  {author}: {count}\n"));
        }
    }

    if !summary.top_labels.is_empty() {
        out.push_str("\nTop labels:\n");
        for (label, count) in summary.top_labels.iter().take(10) {
            out.push_str(&format!("  {label}: {count}\n"));
        }
    }

    if !summary.busiest_days.is_empty() {
        out.push_str("\nBusiest days:\n");
        for (day, count) in summary.busiest_days.iter().take(10) {
            out.push_str(&format!("  {day}: {count}\n"));
        }
    }

    out
}

/// Serializable form of `ReportSummary` for JSON output.
#[derive(Serialize)]
pub struct ReportSummaryJson {
    pub target_month: String,
    pub total_items: u64,
    pub issue_count: u64,
    pub pull_request_count: u64,
    pub by_org: Vec<(String, u64)>,
    pub top_repos: Vec<(String, u64)>,
    pub top_authors: Vec<(String, u64)>,
    pub top_labels: Vec<(String, u64)>,
    pub busiest_days: Vec<(String, u64)>,
}

impl From<ReportSummary> for ReportSummaryJson {
    fn from(s: ReportSummary) -> Self {
        ReportSummaryJson {
            target_month: s.target_month,
            total_items: s.total_items,
            issue_count: s.issue_count,
            pull_request_count: s.pull_request_count,
            by_org: s.by_org,
            top_repos: s.top_repos,
            top_authors: s.top_authors,
            top_labels: s.top_labels,
            busiest_days: s.busiest_days,
        }
    }
}

/// Render a `ReportSummary` as JSON.
pub fn render_json(summary: ReportSummary) -> Result<String, AppError> {
    let json: ReportSummaryJson = summary.into();
    serde_json::to_string_pretty(&json)
        .map_err(|e| AppError::ReportInput(format!("json serialization failed: {e}")))
}
