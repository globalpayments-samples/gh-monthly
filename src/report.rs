use std::collections::HashMap;

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
