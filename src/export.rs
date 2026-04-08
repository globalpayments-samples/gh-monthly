use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::model::NormalizedItem;

/// Preflight information captured before a fetch run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightMetadata {
    pub estimated_requests: u32,
    pub rate_limit_remaining: u32,
    /// Rate-limit reset time as a UTC RFC3339 string (e.g. "2026-04-08T10:30:00Z").
    pub rate_limit_reset_at: String,
    pub safety_margin_requests: u32,
}

/// Metadata written alongside a fetch run's exported data.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunMetadata {
    pub tool: String,
    pub run_started_at: String,
    pub run_completed_at: Option<String>,
    pub completed: bool,
    pub requested_orgs: Vec<String>,
    pub target_month: String,
    pub window_start: String,
    pub window_end: String,
    pub formats: Vec<String>,
    pub total_records: u64,
    pub request_count: u32,
    pub preflight: PreflightMetadata,
}

/// Write run metadata as pretty-printed JSON.
pub fn write_metadata(path: &Path, metadata: &RunMetadata) -> Result<(), AppError> {
    ensure_parent(path)?;
    let json = serde_json::to_string_pretty(metadata)
        .map_err(|e| AppError::Write(format!("metadata serialization failed: {e}")))?;
    fs::write(path, json).map_err(|e| AppError::Write(format!("metadata write failed: {e}")))?;
    Ok(())
}

/// Ensure the parent directory of `path` exists, creating it if necessary.
fn ensure_parent(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::Write(format!("cannot create directory: {e}")))?;
        }
    }
    Ok(())
}

/// Write normalized items to a JSONL file (one JSON object per line).
pub fn write_jsonl(path: &Path, items: &[NormalizedItem]) -> Result<(), AppError> {
    ensure_parent(path)?;
    let file =
        File::create(path).map_err(|e| AppError::Write(format!("cannot create file: {e}")))?;
    let mut writer = BufWriter::new(file);
    for item in items {
        let line = serde_json::to_string(item)
            .map_err(|e| AppError::Write(format!("serialization failed: {e}")))?;
        writeln!(writer, "{line}").map_err(|e| AppError::Write(format!("write failed: {e}")))?;
    }
    writer
        .flush()
        .map_err(|e| AppError::Write(format!("flush failed: {e}")))?;
    Ok(())
}

/// Write normalized items to a CSV file with the canonical stable header ordering.
pub fn write_csv(path: &Path, items: &[NormalizedItem]) -> Result<(), AppError> {
    ensure_parent(path)?;
    let file =
        File::create(path).map_err(|e| AppError::Write(format!("cannot create file: {e}")))?;
    let mut wtr = csv::Writer::from_writer(file);

    wtr.write_record([
        "item_type",
        "org",
        "repo",
        "number",
        "id",
        "node_id",
        "title",
        "author_login",
        "state",
        "created_at",
        "closed_at",
        "merged_at",
        "html_url",
        "api_url",
        "labels",
        "is_pull_request",
    ])
    .map_err(|e| AppError::Write(format!("csv header write failed: {e}")))?;

    for item in items {
        let item_type = serde_json::to_value(&item.item_type)
            .map(|v| v.as_str().unwrap_or("").to_string())
            .unwrap_or_default();
        let number = item.number.to_string();
        let id = item.id.to_string();
        let labels = item.labels.join(";");
        let is_pr = if item.is_pull_request {
            "true"
        } else {
            "false"
        };
        wtr.write_record([
            item_type.as_str(),
            item.org.as_str(),
            item.repo.as_str(),
            number.as_str(),
            id.as_str(),
            item.node_id.as_str(),
            item.title.as_str(),
            item.author_login.as_deref().unwrap_or(""),
            item.state.as_str(),
            item.created_at.as_str(),
            item.closed_at.as_deref().unwrap_or(""),
            item.merged_at.as_deref().unwrap_or(""),
            item.html_url.as_str(),
            item.api_url.as_str(),
            labels.as_str(),
            is_pr,
        ])
        .map_err(|e| AppError::Write(format!("csv record write failed: {e}")))?;
    }

    wtr.flush()
        .map_err(|e| AppError::Write(format!("csv flush failed: {e}")))?;
    Ok(())
}
