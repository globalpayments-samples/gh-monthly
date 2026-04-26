use std::path::PathBuf;

use clap::Parser;
use gh_monthly::cli::ReportFormat;
use gh_monthly::cli::{Cli, Commands, OutputFormat};
use gh_monthly::error::AppError;
use gh_monthly::export::{write_json_array, write_metadata, CsvItemWriter, JsonlItemWriter, PreflightMetadata, RunMetadata};
use gh_monthly::github::{wait_for_preflight_budget, GitHubClient, SearchKind};
use gh_monthly::model::NormalizedItem;
use gh_monthly::report::{read_and_aggregate, read_and_aggregate_many, render_json, render_table};
use gh_monthly::time_window::{resolve_explicit_month, resolve_previous_month};

const SAFETY_MARGIN: u32 = 10;
const DEFAULT_BASE_URL: &str = "https://api.github.com";

/// Format a Unix timestamp (seconds) as a UTC RFC3339 string.
fn unix_to_rfc3339(secs: u64) -> String {
    use chrono::{DateTime, TimeZone, Utc};
    let dt: DateTime<Utc> = Utc
        .timestamp_opt(secs as i64, 0)
        .single()
        .unwrap_or_default();
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

struct FetchContext<'a> {
    run_started_at: &'a str,
    orgs: &'a [String],
    target_month: &'a str,
    window_start: &'a str,
    window_end: &'a str,
    formats: &'a [String],
}

fn build_incomplete_metadata(
    ctx: &FetchContext<'_>,
    preflight: Option<&PreflightMetadata>,
    total_records: u64,
) -> RunMetadata {
    let preflight = preflight.cloned().unwrap_or(PreflightMetadata {
        estimated_requests: 0,
        rate_limit_remaining: 0,
        rate_limit_reset_at: String::new(),
        safety_margin_requests: 0,
    });
    RunMetadata {
        tool: "gh-monthly".to_string(),
        run_started_at: ctx.run_started_at.to_string(),
        run_completed_at: None,
        completed: false,
        requested_orgs: ctx.orgs.to_vec(),
        target_month: ctx.target_month.to_string(),
        window_start: ctx.window_start.to_string(),
        window_end: ctx.window_end.to_string(),
        formats: ctx.formats.to_vec(),
        total_records,
        request_count: 0,
        preflight,
    }
}

fn build_complete_metadata(
    ctx: &FetchContext<'_>,
    run_completed_at: &str,
    preflight: PreflightMetadata,
    total_records: u64,
    request_count: u32,
) -> RunMetadata {
    RunMetadata {
        tool: "gh-monthly".to_string(),
        run_started_at: ctx.run_started_at.to_string(),
        run_completed_at: Some(run_completed_at.to_string()),
        completed: true,
        requested_orgs: ctx.orgs.to_vec(),
        target_month: ctx.target_month.to_string(),
        window_start: ctx.window_start.to_string(),
        window_end: ctx.window_end.to_string(),
        formats: ctx.formats.to_vec(),
        total_records,
        request_count,
        preflight,
    }
}

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

            let formats = match args.format {
                OutputFormat::Jsonl => vec!["jsonl".to_string()],
                OutputFormat::Csv => vec!["jsonl".to_string(), "csv".to_string()],
                OutputFormat::JsonArray => vec!["jsonl".to_string(), "json-array".to_string()],
                OutputFormat::Both => vec!["jsonl".to_string(), "csv".to_string()],
            };

            let run_started_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

            let ctx = FetchContext {
                run_started_at: &run_started_at,
                orgs: &args.orgs,
                target_month: &window.month_id,
                window_start: &start,
                window_end: &end,
                formats: &formats,
            };

            let client = GitHubClient::new(&base_url);

            // --- Preflight ---
            let estimate = client
                .preflight_estimate(&args.orgs, &start, &end, &window.month_id)
                .await;

            let estimate = match estimate {
                Ok(e) => e,
                Err(err) => {
                    let _ = write_metadata(
                        &out_dir.join("metadata.json"),
                        &build_incomplete_metadata(&ctx, None, 0),
                    );
                    return Err(err);
                }
            };

            let preflight_meta = PreflightMetadata {
                estimated_requests: estimate.estimated_requests,
                rate_limit_remaining: estimate.rate_limit.remaining,
                rate_limit_reset_at: unix_to_rfc3339(estimate.rate_limit.reset_at),
                safety_margin_requests: SAFETY_MARGIN,
            };

            // Write incomplete metadata before budget check
            let _ = write_metadata(
                &out_dir.join("metadata.json"),
                &build_incomplete_metadata(&ctx, Some(&preflight_meta), 0),
            );

            wait_for_preflight_budget(&estimate, SAFETY_MARGIN).await?;

            // --- Fetch + Export (streaming) ---
            let mut jsonl_writer = JsonlItemWriter::create(&out_dir.join("items.jsonl"))?;
            let mut csv_writer: Option<CsvItemWriter> = match args.format {
                OutputFormat::Csv | OutputFormat::Both => {
                    Some(CsvItemWriter::create(&out_dir.join("items.csv"))?)
                }
                _ => None,
            };
            let mut json_array_buf: Option<Vec<NormalizedItem>> = match args.format {
                OutputFormat::JsonArray => Some(Vec::new()),
                _ => None,
            };

            let mut total_records: u64 = 0;
            let mut total_requests: u32 = 0;

            for org in &args.orgs {
                for kind in [SearchKind::Issue, SearchKind::PullRequest] {
                    let query = kind.build_query(org, &start, &end);
                    let result = client
                        .fetch_all_into(&query, kind, 100, |item| {
                            jsonl_writer.write_item(&item)?;
                            if let Some(ref mut cw) = csv_writer {
                                cw.write_item(&item)?;
                            }
                            if let Some(ref mut buf) = json_array_buf {
                                buf.push(item);
                            }
                            total_records += 1;
                            Ok(())
                        })
                        .await;
                    match result {
                        Ok(req_count) => total_requests += req_count,
                        Err(err) => {
                            let _ = jsonl_writer.flush();
                            if let Some(ref mut cw) = csv_writer {
                                let _ = cw.flush();
                            }
                            let _ = write_metadata(
                                &out_dir.join("metadata.json"),
                                &build_incomplete_metadata(
                                    &ctx,
                                    Some(&preflight_meta),
                                    total_records,
                                ),
                            );
                            return Err(err);
                        }
                    }
                }
            }

            jsonl_writer.flush()?;
            if let Some(ref mut cw) = csv_writer {
                cw.flush()?;
            }
            if let Some(ref buf) = json_array_buf {
                write_json_array(&out_dir.join("items.json"), buf)?;
            }

            let run_completed_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

            write_metadata(
                &out_dir.join("metadata.json"),
                &build_complete_metadata(
                    &ctx,
                    &run_completed_at,
                    preflight_meta,
                    total_records,
                    total_requests,
                ),
            )?;

            println!(
                "fetched {} items for {} → {}",
                total_records,
                window.month_id,
                out_dir.display()
            );
            Ok(())
        }

        Commands::Report(args) => {
            let summary = if args.input.len() == 1 {
                read_and_aggregate(&args.input[0])?
            } else {
                read_and_aggregate_many(&args.input)?
            };
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
