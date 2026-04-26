# gh-monthly

A Rust CLI that fetches public GitHub issues and pull requests created in the previous calendar month for one or more organizations, exports normalized local datasets, and reports monthly summaries.

## Purpose

`gh-monthly` helps engineering teams track open-source activity across GitHub organizations month by month — without requiring authentication for public repositories.

## V1 Scope

**Included:**
- Fetch public issues and PRs for one or more GitHub orgs via the Search API
- Export to JSON array (default), JSONL, or CSV with a stable, versioned schema
- Summarize exported data locally: totals by type, top repos, top authors, top labels, busiest days
- Preflight budget check to refuse runs that would exceed rate limits
- Automatic retry on transient 5xx server errors (fail-fast on 4xx)
- Incomplete-run detection via `metadata.json`

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
# end:    2026-04-01T00:00:00Z

# Default: previous calendar month
gh-monthly window
```

### Fetch issues and PRs

```sh
# Fetch previous month for one org (default format: json-array)
gh-monthly fetch --org rust-lang

# Fetch a specific month for multiple orgs
gh-monthly fetch --org rust-lang --org tokio-rs --month 2026-03

# Explicit JSON array output (e.g. for Power Automate or other HTTP consumers)
gh-monthly fetch --org rust-lang --month 2026-03 --out ./exports/2026-03 --format json-array

# JSONL + CSV together
gh-monthly fetch --org rust-lang --month 2026-03 --out ./exports/2026-03 --format both
```

Output files written to `--out` (default `./data/YYYY-MM/`):

| File | Description |
|------|-------------|
| `items.jsonl` | One normalized item per line; always written regardless of `--format` |
| `items.json` | JSON array export for HTTP consumers (with `--format json-array`, the default) |
| `items.csv` | Tabular export (with `--format csv` or `--format both`) |
| `metadata.json` | Run metadata including preflight info and completion status |

### Export formats

| `--format` | Files written | Use when |
|------------|---------------|----------|
| `json-array` (default) | `items.jsonl` + `items.json` | Feeding a downstream HTTP endpoint (Power Automate, webhooks, etc.) |
| `jsonl` | `items.jsonl` | Streaming pipelines, log aggregators, `jq` processing |
| `csv` | `items.jsonl` + `items.csv` | Spreadsheet import, SQL `COPY`, pandas |
| `both` | `items.jsonl` + `items.csv` | Same as `csv` |

`items.jsonl` is always written. It is the canonical export and the source of truth for `report`.

> **Rate limits:** V1 is unauthenticated. The GitHub Search API allows ~30 requests per minute for unauthenticated clients. The preflight check refuses runs that would exhaust the current window. Transient 5xx errors are retried automatically; 4xx errors fail immediately.

### Report from local data

```sh
# Table summary (default)
gh-monthly report --input ./data/2026-03/items.jsonl

# Aggregate multiple exports
gh-monthly report --input ./data/2026-02/items.jsonl --input ./data/2026-03/items.jsonl

# JSON output
gh-monthly report --input ./data/2026-03/items.jsonl --format json
```

`report` reads only local files — it never calls GitHub. The `target_month` field is derived from sibling `metadata.json` files when present; when multiple inputs provide different months, the report shows the included months as a comma-separated list. If no month metadata is available, it shows `"unknown"`.

Example table output:

```
Month: 2026-03
Total items: 142
Issues: 80
Pull requests: 62

By org:
  rust-lang: 98
  tokio-rs: 44

Top repos:
  rust-lang/rust: 61
  rust-lang/cargo: 37
  tokio-rs/tokio: 32

Top authors:
  alice: 12
  bob: 9

Top labels:
  bug: 28
  enhancement: 15

Busiest days:
  2026-03-15: 14
  2026-03-22: 11
```

## Export Schema

All three export formats share the same fields (stable in v1). JSONL writes one object per line; JSON array wraps the same objects in `{"data": [...]}`. CSV uses the same field names as column headers with `;`-separated labels.

Each item has the following fields:

| Field | Type | Description |
|-------|------|-------------|
| `item_type` | `"issue"` \| `"pull_request"` | Type of item |
| `org` | string | GitHub organization |
| `repo` | string | Repository name |
| `number` | integer | Issue or PR number |
| `id` | integer | GitHub item ID |
| `node_id` | string | GitHub node ID |
| `title` | string | Issue or PR title |
| `author_login` | string \| null | Author login; `null` for deleted accounts |
| `state` | string | `"open"` or `"closed"` |
| `created_at` | ISO8601 string | Creation timestamp |
| `closed_at` | ISO8601 string \| null | Close timestamp |
| `merged_at` | ISO8601 string \| null | Merge timestamp (PRs only; may be `null` if not available from Search API) |
| `html_url` | string | GitHub web URL |
| `api_url` | string | GitHub REST API URL |
| `labels` | string (`;`-separated in CSV) | Label names |
| `is_pull_request` | boolean | `true` for PRs |

CSV column order matches the table above exactly.

### JSON array structure

`items.json` (written with `--format json-array`) wraps the items array in a single object, which most HTTP connectors (Power Automate, Logic Apps, webhook consumers) expect as a parseable root:

```json
{
  "data": [
    { "item_type": "issue", "org": "rust-lang", "repo": "rust", ... },
    { "item_type": "pull_request", "org": "rust-lang", "repo": "rust", ... }
  ]
}
```

Each element in `data` has the same fields as a JSONL record.

## Metadata Trust Fields

`metadata.json` is written at the start of a run (incomplete) and updated at the end (complete). Check `completed` before trusting the dataset:

```json
{
  "tool": "gh-monthly",
  "run_started_at": "2026-04-08T10:00:00Z",
  "run_completed_at": "2026-04-08T10:05:00Z",
  "completed": true,
  "requested_orgs": ["rust-lang"],
  "target_month": "2026-03",
  "window_start": "2026-03-01T00:00:00Z",
  "window_end": "2026-04-01T00:00:00Z",
  "formats": ["jsonl"],
  "total_records": 142,
  "request_count": 3,
  "preflight": {
    "estimated_requests": 4,
    "rate_limit_remaining": 28,
    "rate_limit_reset_at": "2026-04-08T10:30:00Z",
    "safety_margin_requests": 5
  }
}
```

If `completed` is `false`, the run was interrupted. The JSONL file may contain partial data.

## Building

```sh
cargo build --release
cargo test
```
