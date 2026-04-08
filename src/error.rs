use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid usage: {0}")]
    Usage(String),
    #[error("preflight refused: {0}")]
    Preflight(String),
    #[error("fetch failed: {0}")]
    Fetch(String),
    #[error("write failed: {0}")]
    Write(String),
    #[error("report input invalid: {0}")]
    ReportInput(String),
}
