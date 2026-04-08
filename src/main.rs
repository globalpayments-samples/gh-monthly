use std::path::PathBuf;

use clap::Parser;
use gh_monthly::cli::ReportFormat;
use gh_monthly::cli::{Cli, Commands, OutputFormat};
use gh_monthly::error::AppError;
use gh_monthly::export::{write_csv, write_jsonl, write_metadata, PreflightMetadata, RunMetadata};
use gh_monthly::github::{ensure_preflight_budget, GitHubClient, SearchKind};
use gh_monthly::model::NormalizedItem;
use gh_monthly::report::{aggregate, read_jsonl, render_json, render_table};
use gh_monthly::time_window::{resolve_explicit_month, resolve_previous_month};

const SAFETY_MARGIN: u32 = 5;
const DEFAULT_BASE_URL: &str = "https://api.github.com";

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

        Commands::Fetch(args) => {
            let window = match args.month {
                Some(ref m) => resolve_explicit_month(m)?,
                None => resolve_previous_month(chrono::Utc::now())?,
            };

            let start = window.start.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let end = window.end.format("%Y-%m-%dT%H:%M:%SZ").to_string();

            let out_dir: PathBuf = args
                .out
                .unwrap_or_else(|| PathBuf::from(format!("data/{}", window.month_id)));

            let base_url = args
                .base_url
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

            let client = GitHubClient::new(&base_url);

            // --- Preflight ---
            let estimate = client
                .preflight_estimate(&args.orgs, &start, &end, &window.month_id)
                .await;

            // Write incomplete metadata and propagate any error
            let estimate = match estimate {
                Ok(e) => e,
                Err(err) => {
                    let _ = write_metadata(
                        &out_dir.join("metadata.json"),
                        &RunMetadata {
                            completed: false,
                            month_id: window.month_id.clone(),
                            orgs: args.orgs.clone(),
                            preflight: PreflightMetadata {
                                estimated_requests: 0,
                                rate_limit_remaining: 0,
                                rate_limit_reset_at: 0,
                            },
                            total_items: 0,
                        },
                    );
                    return Err(err);
                }
            };

            let preflight_meta = PreflightMetadata {
                estimated_requests: estimate.estimated_requests,
                rate_limit_remaining: estimate.rate_limit.remaining,
                rate_limit_reset_at: estimate.rate_limit.reset_at,
            };

            // Write incomplete metadata before budget check
            let _ = write_metadata(
                &out_dir.join("metadata.json"),
                &RunMetadata {
                    completed: false,
                    month_id: window.month_id.clone(),
                    orgs: args.orgs.clone(),
                    preflight: PreflightMetadata {
                        estimated_requests: preflight_meta.estimated_requests,
                        rate_limit_remaining: preflight_meta.rate_limit_remaining,
                        rate_limit_reset_at: preflight_meta.rate_limit_reset_at,
                    },
                    total_items: 0,
                },
            );

            ensure_preflight_budget(&estimate, SAFETY_MARGIN)?;

            // --- Fetch ---
            let mut all_items: Vec<NormalizedItem> = Vec::new();

            for org in &args.orgs {
                for kind in [SearchKind::Issue, SearchKind::PullRequest] {
                    let query = kind.build_query(org, &start, &end);
                    let items = client
                        .fetch_all(&query, kind, 100)
                        .await
                        .inspect_err(|_| {
                            let _ = write_metadata(
                                &out_dir.join("metadata.json"),
                                &RunMetadata {
                                    completed: false,
                                    month_id: window.month_id.clone(),
                                    orgs: args.orgs.clone(),
                                    preflight: PreflightMetadata {
                                        estimated_requests: preflight_meta.estimated_requests,
                                        rate_limit_remaining: preflight_meta.rate_limit_remaining,
                                        rate_limit_reset_at: preflight_meta.rate_limit_reset_at,
                                    },
                                    total_items: all_items.len() as u64,
                                },
                            );
                        })?;
                    all_items.extend(items);
                }
            }

            // --- Export ---
            write_jsonl(&out_dir.join("items.jsonl"), &all_items)?;

            match args.format {
                OutputFormat::Csv => {
                    write_csv(&out_dir.join("items.csv"), &all_items)?;
                }
                OutputFormat::Both => {
                    write_csv(&out_dir.join("items.csv"), &all_items)?;
                }
                OutputFormat::Jsonl => {}
            }

            write_metadata(
                &out_dir.join("metadata.json"),
                &RunMetadata {
                    completed: true,
                    month_id: window.month_id.clone(),
                    orgs: args.orgs.clone(),
                    preflight: preflight_meta,
                    total_items: all_items.len() as u64,
                },
            )?;

            println!(
                "fetched {} items for {} → {}",
                all_items.len(),
                window.month_id,
                out_dir.display()
            );
            Ok(())
        }

        Commands::Report(args) => {
            let items = read_jsonl(&args.input)?;
            let summary = aggregate(&items);
            match args.format {
                ReportFormat::Json => {
                    println!("{}", render_json(summary)?);
                }
                ReportFormat::Table => {
                    print!("{}", render_table(&summary));
                }
            }
            Ok(())
        }
    }
}
