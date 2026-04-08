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

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Usage(_) => 2,
            AppError::Preflight(_) => 3,
            AppError::Fetch(_) => 4,
            AppError::Write(_) => 5,
            AppError::ReportInput(_) => 6,
        }
    }
}
