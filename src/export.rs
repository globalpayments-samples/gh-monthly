use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::AppError;
use crate::model::NormalizedItem;

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
        writeln!(writer, "{line}")
            .map_err(|e| AppError::Write(format!("write failed: {e}")))?;
    }
    writer
        .flush()
        .map_err(|e| AppError::Write(format!("flush failed: {e}")))?;
    Ok(())
}

/// Write normalized items to a CSV file with a stable header ordering.
pub fn write_csv(path: &Path, items: &[NormalizedItem]) -> Result<(), AppError> {
    ensure_parent(path)?;
    let file =
        File::create(path).map_err(|e| AppError::Write(format!("cannot create file: {e}")))?;
    let mut wtr = csv::Writer::from_writer(file);

    // Write header with stable ordering
    wtr.write_record([
        "item_type",
        "org",
        "repo",
        "number",
        "title",
        "state",
        "author",
        "created_at",
        "updated_at",
        "closed_at",
        "html_url",
        "labels",
    ])
    .map_err(|e| AppError::Write(format!("csv header write failed: {e}")))?;

    for item in items {
        let item_type = serde_json::to_value(&item.item_type)
            .map(|v| v.as_str().unwrap_or("").to_string())
            .unwrap_or_default();
        wtr.write_record([
            &item_type,
            &item.org,
            &item.repo,
            &item.number.to_string(),
            &item.title,
            &item.state,
            &item.author,
            &item.created_at,
            &item.updated_at,
            item.closed_at.as_deref().unwrap_or(""),
            &item.html_url,
            &item.labels.join(";"),
        ])
        .map_err(|e| AppError::Write(format!("csv record write failed: {e}")))?;
    }

    wtr.flush()
        .map_err(|e| AppError::Write(format!("csv flush failed: {e}")))?;
    Ok(())
}
