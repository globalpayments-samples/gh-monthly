# GH Monthly Design

## Overview

`gh-monthly` is a Rust-based CLI for collecting public GitHub activity data for one or more organizations using the unauthenticated GitHub Search API. Version 1 is intentionally narrow:

- Collect only public issues and pull requests
- Limit results to items created in the previous calendar month by default
- Support one or more organizations per run
- Export normalized data first
- Provide a local reporting command over exported data
- Fail before fetching if the estimated request volume cannot complete within unauthenticated rate limits

Discussions are explicitly out of scope for v1 because the viable public unauthenticated API path is not aligned with the Search API approach used for issues and pull requests.

## Goals

- Provide a repeatable monthly snapshot of created issues and pull requests across multiple organizations
- Produce stable machine-readable exports suitable for downstream analysis
- Offer a built-in terminal summary with useful top-level monthly insights
- Keep behavior trustworthy under unauthenticated API constraints
- Keep the architecture open to later authenticated extensions without redesigning the core data model

## Non-Goals

- Collect GitHub Discussions
- Collect items updated in the month but created earlier
- Collect timeline events, comments, reviews, reactions, or audit-style activity
- Perform cross-month trend analysis in v1
- Depend on authentication for the main fetch path

## Product Shape

The CLI exposes two primary workflows:

1. `fetch` retrieves data from GitHub for a resolved month window and writes normalized exports.
2. `report` reads previously exported data and prints a local terminal summary.

A small `window` command is included to make date resolution visible and testable.

The product is export-first. Terminal reporting is a consumer of the normalized dataset rather than an alternate fetch mode.

## Time Window Rules

By default, the tool resolves the previous calendar month relative to the current run time.

Examples:

- A run on any day in March 2026 targets February 2026
- A run on January 3, 2026 targets December 2025

The resolved fetch window should be the full previous month in UTC:

- Start: `YYYY-MM-01T00:00:00Z`
- End: last day of month at `23:59:59Z`

The CLI also supports `--month YYYY-MM` so users can rerun a specific month deterministically. All exported metadata should record both the month identifier and the concrete UTC start and end timestamps.

## Command-Line Interface

Recommended command surface:

```text
gh-monthly fetch --org ORG... [--out DIR] [--format jsonl|csv|both] [--month YYYY-MM] [--base-url URL]
gh-monthly report --input PATH... [--format table|json]
gh-monthly window [--month YYYY-MM]
```

### `fetch`

Responsibilities:

- Accept one or more `--org` values
- Resolve the target month window
- Preflight the request plan against unauthenticated rate limits
- Fetch and normalize issue and pull request data
- Write exports and metadata
- Print a short run summary

Defaults:

- `--out` defaults to `./data/YYYY-MM/`
- `--format` defaults to `jsonl`
- `--month` defaults to the previous calendar month

### `report`

Responsibilities:

- Read normalized local export data only
- Accept one or more `--input` paths
- Display the included month or months from sibling `metadata.json` files when present
- Compute monthly aggregates
- Print human-readable tables by default
- Emit structured JSON when requested

### `window`

Responsibilities:

- Print the resolved target month and UTC start/end timestamps
- Support the same `--month YYYY-MM` override used by `fetch`

## Exit Codes

Exit codes should be stable for shell automation:

- `0`: success
- `2`: invalid usage
- `3`: preflight refusal due to estimated rate-limit insufficiency
- `4`: fetch failure after bounded retries
- `5`: export or write failure
- `6`: invalid report input

## Output Layout

Each target month writes to a predictable directory, for example:

```text
data/2026-03/items.jsonl
data/2026-03/items.csv
data/2026-03/metadata.json
```

`items.jsonl` is the canonical dataset. CSV is optional and derived from the same normalized record shape. `metadata.json` records provenance and trust information for the run.

V1 uses one unified `items` dataset rather than separate files for issues and pull requests. Type distinctions stay explicit inside each record.

## Normalized Record Schema

Each exported line in `items.jsonl` represents one normalized GitHub item:

```json
{
  "item_type": "issue",
  "org": "rust-lang",
  "repo": "rust",
  "number": 12345,
  "id": 987654321,
  "node_id": "MDU6SXNzdWU5ODc2NTQzMjE=",
  "title": "Example title",
  "author_login": "octocat",
  "state": "open",
  "created_at": "2026-02-14T12:34:56Z",
  "closed_at": null,
  "merged_at": null,
  "html_url": "https://github.com/rust-lang/rust/issues/12345",
  "api_url": "https://api.github.com/repos/rust-lang/rust/issues/12345",
  "labels": ["A-docs", "T-compiler"],
  "is_pull_request": false
}
```

Schema notes:

- `item_type` is either `issue` or `pull_request`
- `org` and `repo` are parsed from the repository identity in the API payload
- `author_login` may be null if the upstream payload does not include a user
- `closed_at` and `merged_at` are present even though filtering is based only on `created_at`
- `is_pull_request` intentionally overlaps with `item_type` because it mirrors an upstream distinction and may simplify some consumers
- `labels` is a list of label names only in v1

For pull requests:

- `item_type = "pull_request"`
- `is_pull_request = true`

## Metadata Schema

`metadata.json` should include enough information to determine whether a dataset is complete and trustworthy:

```json
{
  "tool": "gh-monthly",
  "run_started_at": "2026-04-08T14:00:00Z",
  "run_completed_at": "2026-04-08T14:02:15Z",
  "completed": true,
  "requested_orgs": ["rust-lang", "tokio-rs"],
  "target_month": "2026-03",
  "window_start": "2026-03-01T00:00:00Z",
  "window_end": "2026-03-31T23:59:59Z",
  "formats": ["jsonl", "csv"],
  "total_records": 1234,
  "request_count": 29,
  "preflight": {
    "estimated_requests": 29,
    "rate_limit_remaining": 10,
    "rate_limit_reset_at": "2026-04-08T14:30:00Z",
    "safety_margin_requests": 5
  }
}
```

Additional metadata may be added later, but v1 should at minimum record run timing, requested orgs, resolved window, formats, request volume, and preflight observations.

## Fetch Strategy

The fetch path should be deterministic and conservative.

For each organization, run exactly two search query families:

- `org:<ORG> is:issue created:<START>..<END>`
- `org:<ORG> is:pr created:<START>..<END>`

The client should paginate through search results using the standard REST Search API. Queries should be processed sequentially, not concurrently, to reduce the risk of secondary rate limiting.

Normalization should happen as each item is received. The implementation should stream normalized JSONL records directly to disk instead of buffering the entire month in memory.

## Preflight Behavior

Before performing the full fetch, the CLI should estimate whether the run can complete under the current unauthenticated budget.

Recommended approach:

1. Run a first-page probe for each org/query pair with `per_page=1`
2. Read `total_count` from the search response
3. Convert `total_count` into estimated page counts for the chosen page size
4. Sum the expected request volume across all queries
5. Compare the total against the current search rate-limit headers with a safety margin

If the estimated work cannot complete safely, the command should fail with exit code `3` before performing the fetch.

The refusal message should clearly identify:

- Target month
- Organizations requested
- Estimated request count
- Remaining unauthenticated budget
- Which query families were large enough to drive refusal

This is intentionally strict. Partial datasets are not acceptable in v1 when the reason for incompleteness is known before the fetch begins.

## Error Handling

Separate fatal trust failures from bounded transient retries.

Fatal errors:

- Invalid organization input
- Unwritable output path
- Preflight refusal due to insufficient budget
- Invalid or unreadable report input

Retryable errors:

- Transient `5xx` responses
- Request timeouts
- Secondary rate-limit responses with `retry-after` guidance

Retries should be bounded. If a fetch fails after retries begin, the tool should still write `metadata.json` with `completed: false` so consumers can reject the dataset intentionally.

## Reporting Behavior

The `report` command reads normalized local data and prints a compact monthly summary.

Default terminal output should include:

- Target month
- Total items
- Issue count
- Pull request count
- Counts by organization
- Top 10 repositories by created items
- Top 10 authors by created items
- Top 10 labels by occurrence
- Busiest days in the month by item count

Label counting should be occurrence-based. A single item with multiple labels contributes one count to each label.

When `--format json` is used, the same aggregate information should be emitted as structured JSON.

## Proposed Module Boundaries

A clean Rust module split for v1:

- `cli`: argument parsing and command dispatch
- `time_window`: previous-month resolution and `--month` parsing
- `github`: REST client, query builder, pagination, rate-limit handling, retry policy
- `model`: raw API types and normalized record types
- `export`: JSONL, CSV, and metadata writing
- `report`: local aggregations and formatting

This structure keeps the boundaries between acquisition, normalization, persistence, and presentation explicit.

## Testing Strategy

V1 should emphasize determinism and local testability.

Recommended test coverage:

- Previous-calendar-month resolution, including January rollover
- CLI parsing for repeated `--org`, `--month`, and output formats
- Query construction for issue and pull request searches
- Preflight estimation logic from synthetic search counts and rate-limit headers
- Raw API payload to normalized record mapping
- JSONL and CSV export correctness
- Report aggregation correctness from fixture datasets
- Metadata completeness for both successful and incomplete runs

Where possible, reporting and normalization tests should run from fixtures without live API access.

## Future Extensions

The v1 design intentionally leaves room for later additions:

- Authenticated mode with higher rate limits
- GitHub Discussions support through a different API path
- Cross-month trend reports
- Repository allowlists or denylists within organizations
- Additional aggregates such as close or merge timing

These are postponed to keep v1 aligned with a trustworthy unauthenticated monthly snapshot.
