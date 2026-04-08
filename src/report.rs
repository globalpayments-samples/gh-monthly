use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::model::NormalizedItem;

/// Aggregated summary of a set of normalized items.
pub struct ReportSummary {
    pub total_items: u64,
    /// Per-org item counts: `(org, count)` sorted descending by count.
    pub by_org: Vec<(String, u64)>,
    /// Top repos by item count: `(org/repo, count)` sorted descending.
    pub top_repos: Vec<(String, u64)>,
    /// Top authors by item count: `(author, count)` sorted descending.
    pub top_authors: Vec<(String, u64)>,
    /// Top labels by item count: `(label, count)` sorted descending.
    pub top_labels: Vec<(String, u64)>,
    /// Day with the most items: `Some((YYYY-MM-DD, count))` or `None` if empty.
    pub busiest_day: Option<(String, u64)>,
}

/// Aggregate a slice of normalized items into a `ReportSummary`.
pub fn aggregate(items: &[NormalizedItem]) -> ReportSummary {
    let mut by_org: HashMap<String, u64> = HashMap::new();
    let mut by_repo: HashMap<String, u64> = HashMap::new();
    let mut by_author: HashMap<String, u64> = HashMap::new();
    let mut by_label: HashMap<String, u64> = HashMap::new();
    let mut by_day: HashMap<String, u64> = HashMap::new();

    for item in items {
        *by_org.entry(item.org.clone()).or_insert(0) += 1;
        *by_repo
            .entry(format!("{}/{}", item.org, item.repo))
            .or_insert(0) += 1;
        *by_author.entry(item.author.clone()).or_insert(0) += 1;

        for label in &item.labels {
            *by_label.entry(label.clone()).or_insert(0) += 1;
        }

        // Parse YYYY-MM-DD from created_at (first 10 chars)
        if item.created_at.len() >= 10 {
            let day = item.created_at[..10].to_string();
            *by_day.entry(day).or_insert(0) += 1;
        }
    }

    ReportSummary {
        total_items: items.len() as u64,
        by_org: sorted_desc(by_org),
        top_repos: sorted_desc(by_repo),
        top_authors: sorted_desc(by_author),
        top_labels: sorted_desc(by_label),
        busiest_day: sorted_desc(by_day).into_iter().next(),
    }
}

/// Sort a `HashMap<String, u64>` into a `Vec<(String, u64)>` descending by count.
fn sorted_desc(map: HashMap<String, u64>) -> Vec<(String, u64)> {
    let mut v: Vec<(String, u64)> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
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
    out.push_str(&format!("Total items: {}\n", summary.total_items));

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

    if let Some((day, count)) = &summary.busiest_day {
        out.push_str(&format!("\nBusiest day: {day} ({count} items)\n"));
    }

    out
}

/// Serializable form of `ReportSummary` for JSON output.
#[derive(Serialize)]
pub struct ReportSummaryJson {
    pub total_items: u64,
    pub by_org: Vec<(String, u64)>,
    pub top_repos: Vec<(String, u64)>,
    pub top_authors: Vec<(String, u64)>,
    pub top_labels: Vec<(String, u64)>,
    pub busiest_day: Option<(String, u64)>,
}

impl From<ReportSummary> for ReportSummaryJson {
    fn from(s: ReportSummary) -> Self {
        ReportSummaryJson {
            total_items: s.total_items,
            by_org: s.by_org,
            top_repos: s.top_repos,
            top_authors: s.top_authors,
            top_labels: s.top_labels,
            busiest_day: s.busiest_day,
        }
    }
}

/// Render a `ReportSummary` as JSON.
pub fn render_json(summary: ReportSummary) -> Result<String, AppError> {
    let json: ReportSummaryJson = summary.into();
    serde_json::to_string_pretty(&json)
        .map_err(|e| AppError::ReportInput(format!("json serialization failed: {e}")))
}
