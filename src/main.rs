use clap::Parser;
use gh_monthly::cli::{Cli, Commands};
use gh_monthly::error::AppError;
use gh_monthly::time_window::{resolve_explicit_month, resolve_previous_month};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli).await {
        eprintln!("error: {}", err);
        std::process::exit(err.exit_code());
    }
}

async fn run(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Commands::Window(args) => {
            let window = match args.month {
                Some(ref m) => resolve_explicit_month(m)?,
                None => resolve_previous_month(chrono::Utc::now())?,
            };
            println!("month:  {}", window.month_id);
            println!("start:  {}", window.start.format("%Y-%m-%dT%H:%M:%SZ"));
            println!("end:    {}", window.end.format("%Y-%m-%dT%H:%M:%SZ"));
            Ok(())
        }
        Commands::Fetch(_args) => Err(AppError::Fetch("fetch not yet implemented".into())),
        Commands::Report(_args) => Err(AppError::ReportInput("report not yet implemented".into())),
    }
}
