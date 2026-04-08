# gh-monthly

A Rust CLI that fetches public GitHub issues and pull requests created in the previous calendar month for one or more organizations, exports normalized local datasets, and reports monthly summaries.

## Purpose

`gh-monthly` helps engineering teams track open-source activity across GitHub organizations month by month — without requiring authentication for public repositories.

## V1 Scope

**Included:**
- Fetch public issues and PRs for one or more GitHub orgs via the Search API
- Export to JSONL (canonical) or CSV
- Summarize exported data locally: totals, top repos, top authors, top labels, busiest day
- Preflight budget check to refuse runs that would exceed rate limits

**Not included in v1:**
- Private repository access (no auth token support)
- Concurrent fetching across orgs
- Incremental / delta fetches

## Usage

### Resolve the target window

```sh
gh-monthly window --month 2026-03
# month:  2026-03
# start:  2026-03-01T00:00:00Z
# end:    2026-03-31T23:59:59Z

# Default: previous calendar month
gh-monthly window
```

### Fetch issues and PRs

```sh
# Fetch previous month for one org
gh-monthly fetch --org rust-lang

# Fetch a specific month for multiple orgs
gh-monthly fetch --org rust-lang --org tokio-rs --month 2026-03

# Choose output directory and format
gh-monthly fetch --org rust-lang --month 2026-03 --out ./exports/2026-03 --format both
```

Output files written to `--out` (default `./data/YYYY-MM/`):

| File | Description |
|------|-------------|
| `items.jsonl` | One normalized item per line (canonical) |
| `items.csv` | Tabular export (with `--format csv` or `--format both`) |
| `metadata.json` | Run metadata including preflight and completion status |

> **Note:** V1 is unauthenticated. The GitHub Search API allows ~30 requests per minute for unauthenticated clients. Large organizations may trigger the preflight budget check and refuse to run.

### Report from local data

```sh
# Table summary (default)
gh-monthly report --input ./data/2026-03/items.jsonl

# JSON output
gh-monthly report --input ./data/2026-03/items.jsonl --format json
```

Example table output:

```
Total items: 142

By org:
  rust-lang: 98
  tokio-rs: 44

Top repos:
  rust-lang/rust: 61
  tokio-rs/tokio: 32
  rust-lang/cargo: 37

Top authors:
  alice: 12
  bob: 9
  ...
```

## Building

```sh
cargo build --release
cargo test
```
