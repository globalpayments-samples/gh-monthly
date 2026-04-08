use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "gh-monthly", about = "Collect and report monthly GitHub activity")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Fetch public issues and pull requests for one or more organizations
    Fetch(FetchArgs),
    /// Print a summary report from previously exported data
    Report(ReportArgs),
    /// Show the resolved target month window
    Window(WindowArgs),
}

#[derive(Debug, Parser)]
pub struct FetchArgs {
    /// GitHub organization(s) to fetch
    #[arg(long = "org", required = true)]
    pub orgs: Vec<String>,

    /// Output directory (default: ./data/YYYY-MM/)
    #[arg(long)]
    pub out: Option<std::path::PathBuf>,

    /// Export format
    #[arg(long, default_value = "jsonl")]
    pub format: OutputFormat,

    /// Target month in YYYY-MM format (default: previous calendar month)
    #[arg(long)]
    pub month: Option<String>,

    /// Override GitHub API base URL (for testing)
    #[arg(long)]
    pub base_url: Option<String>,
}

#[derive(Debug, Parser)]
pub struct ReportArgs {
    /// Path to items.jsonl export file
    #[arg(long)]
    pub input: std::path::PathBuf,

    /// Output format
    #[arg(long, default_value = "table")]
    pub format: ReportFormat,
}

#[derive(Debug, Parser)]
pub struct WindowArgs {
    /// Target month in YYYY-MM format (default: previous calendar month)
    #[arg(long)]
    pub month: Option<String>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Jsonl,
    Csv,
    Both,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ReportFormat {
    Table,
    Json,
}
