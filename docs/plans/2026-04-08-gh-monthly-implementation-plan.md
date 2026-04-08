# GH Monthly Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Rust CLI that fetches public GitHub issues and pull requests created in the previous calendar month for one or more organizations, exports normalized local datasets, and reports monthly summaries.

**Architecture:** The CLI is split into independent modules for argument parsing, month-window resolution, GitHub Search API access, normalization, export, and reporting. The fetch path is export-first and writes canonical JSONL plus metadata, while report operates strictly on local exported data. The implementation should be test-driven and keep HTTP concerns, schema mapping, and terminal formatting decoupled.

**Tech Stack:** Rust stable, Cargo, `clap`, `reqwest`, `tokio`, `serde`, `serde_json`, `csv`, `chrono`, `thiserror`, `wiremock` or `httpmock`, `assert_cmd`, `predicates`

---

### Task 1: Initialize the Rust workspace

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `.gitignore`

**Step 1: Create the Cargo manifest**

Write `Cargo.toml` with:

```toml
[package]
name = "gh-monthly"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
chrono = { version = "0.4", features = ["serde", "clock"] }
csv = "1"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

**Step 2: Create the binary entrypoint**

Write `src/main.rs` with:

```rust
#[tokio::main]
async fn main() {
    eprintln!("not implemented");
    std::process::exit(1);
}
```

**Step 3: Create the library root**

Write `src/lib.rs` with:

```rust
pub mod cli;
pub mod error;
pub mod export;
pub mod github;
pub mod model;
pub mod report;
pub mod time_window;
```

**Step 4: Add a minimal `.gitignore`**

Write `.gitignore` with:

```gitignore
/target
/.DS_Store
/data
```

**Step 5: Run Cargo to verify the project builds**

Run: `cargo check`
Expected: PASS with module-not-found errors only if the next task has not been completed yet

**Step 6: Commit**

```bash
git add Cargo.toml src/main.rs src/lib.rs .gitignore
git commit -m "chore: initialize gh-monthly rust cli"
```

### Task 2: Add the core module skeleton and shared error type

**Files:**
- Create: `src/cli.rs`
- Create: `src/error.rs`
- Create: `src/export.rs`
- Create: `src/github.rs`
- Create: `src/model.rs`
- Create: `src/report.rs`
- Create: `src/time_window.rs`

**Step 1: Create the shared error type**

Write `src/error.rs` with:

```rust
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
```

**Step 2: Create placeholder modules**

Create each file with a single placeholder item, for example:

```rust
pub fn placeholder() {}
```

**Step 3: Run the compiler**

Run: `cargo check`
Expected: PASS

**Step 4: Commit**

```bash
git add src/cli.rs src/error.rs src/export.rs src/github.rs src/model.rs src/report.rs src/time_window.rs
git commit -m "chore: add core module skeleton"
```

### Task 3: Define CLI argument parsing

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Test: `tests/cli_help.rs`

**Step 1: Write the failing CLI help test**

Create `tests/cli_help.rs` with:

```rust
use assert_cmd::Command;

#[test]
fn shows_top_level_help() {
    Command::cargo_bin("gh-monthly")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}
```

**Step 2: Run the test to verify failure**

Run: `cargo test tests::cli_help -- --nocapture`
Expected: FAIL because no real CLI exists yet

**Step 3: Implement CLI structs**

Define:

- `Cli`
- `Commands`
- `FetchArgs`
- `ReportArgs`
- `WindowArgs`
- `OutputFormat`
- `ReportFormat`

Use `clap` derives with repeatable `--org`, optional `--month`, and defaults matching the design doc.

**Step 4: Wire parsing into `main`**

Replace the placeholder `main` with:

```rust
use clap::Parser;
use gh_monthly::cli::Cli;

#[tokio::main]
async fn main() {
    let _cli = Cli::parse();
}
```

**Step 5: Run the targeted test**

Run: `cargo test --test cli_help`
Expected: PASS

**Step 6: Commit**

```bash
git add src/cli.rs src/main.rs tests/cli_help.rs
git commit -m "feat: add clap command surface"
```

### Task 4: Add month-window parsing and resolution

**Files:**
- Modify: `src/time_window.rs`
- Test: `tests/time_window.rs`

**Step 1: Write the failing date-resolution tests**

Create `tests/time_window.rs` covering:

- March 15, 2026 resolves to February 2026
- January 3, 2026 resolves to December 2025
- `--month 2026-03` resolves to the full March 2026 UTC window

Use tests shaped like:

```rust
#[test]
fn resolves_previous_month() {
    let window = resolve_previous_month(/* fixed date */).unwrap();
    assert_eq!(window.month_id, "2026-02");
}
```

**Step 2: Run the tests to verify failure**

Run: `cargo test --test time_window`
Expected: FAIL because the resolver does not exist yet

**Step 3: Implement `MonthWindow` and resolver functions**

Add:

- `MonthWindow { month_id, start, end }`
- `resolve_previous_month(now)`
- `resolve_explicit_month("YYYY-MM")`
- parsing validation for invalid month strings

Use UTC consistently and avoid hidden local-time behavior.

**Step 4: Run the targeted tests**

Run: `cargo test --test time_window`
Expected: PASS

**Step 5: Commit**

```bash
git add src/time_window.rs tests/time_window.rs
git commit -m "feat: add previous-month window resolution"
```

### Task 5: Add exit code mapping and top-level command dispatch

**Files:**
- Modify: `src/main.rs`
- Modify: `src/error.rs`
- Modify: `src/cli.rs`
- Test: `tests/window_command.rs`

**Step 1: Write the failing window command test**

Create `tests/window_command.rs` with a test that runs:

```rust
Command::cargo_bin("gh-monthly")
    .unwrap()
    .args(["window", "--month", "2026-03"])
    .assert()
    .success()
    .stdout(predicates::str::contains("2026-03"));
```

**Step 2: Run the test to verify failure**

Run: `cargo test --test window_command`
Expected: FAIL because dispatch is not implemented

**Step 3: Implement dispatch helpers**

Add:

- an async `run(cli: Cli) -> Result<(), AppError>`
- exit code mapping on `AppError`
- `window` handler that prints month id, start, and end
- placeholder `fetch` and `report` handlers returning structured `AppError`s

**Step 4: Run the targeted test**

Run: `cargo test --test window_command`
Expected: PASS

**Step 5: Commit**

```bash
git add src/main.rs src/error.rs src/cli.rs tests/window_command.rs
git commit -m "feat: add command dispatch and exit codes"
```

### Task 6: Define raw API models and normalized item schema

**Files:**
- Modify: `src/model.rs`
- Test: `tests/model_normalization.rs`

**Step 1: Write the failing normalization tests**

Create fixtures inline or in test code for:

- one issue search item
- one pull request search item

Assert:

- issue maps to `item_type = issue`
- PR maps to `item_type = pull_request`
- `org` and `repo` are parsed correctly
- labels become `Vec<String>`

**Step 2: Run the tests to verify failure**

Run: `cargo test --test model_normalization`
Expected: FAIL because the model layer is not implemented

**Step 3: Implement model types**

Define:

- raw search response structs matching only required GitHub fields
- `NormalizedItem`
- `ItemType`
- `impl TryFrom<RawSearchItem> for NormalizedItem`

Keep the raw structs minimal and ignore unused upstream fields.

**Step 4: Run the targeted tests**

Run: `cargo test --test model_normalization`
Expected: PASS

**Step 5: Commit**

```bash
git add src/model.rs tests/model_normalization.rs
git commit -m "feat: add normalized item mapping"
```

### Task 7: Build query construction and rate-limit metadata types

**Files:**
- Modify: `src/github.rs`
- Test: `tests/github_query.rs`

**Step 1: Write the failing query tests**

Create tests asserting that:

- issue queries include `org:<name> is:issue created:<start>..<end>`
- PR queries include `org:<name> is:pr created:<start>..<end>`
- invalid empty org names are rejected

**Step 2: Run the tests to verify failure**

Run: `cargo test --test github_query`
Expected: FAIL because query builders do not exist

**Step 3: Implement GitHub query helpers**

Add:

- `SearchKind`
- query-builder functions
- `RateLimitSnapshot`
- `PreflightEstimate`

Keep this step focused on data shaping, not HTTP.

**Step 4: Run the targeted tests**

Run: `cargo test --test github_query`
Expected: PASS

**Step 5: Commit**

```bash
git add src/github.rs tests/github_query.rs
git commit -m "feat: add github query builders"
```

### Task 8: Add HTTP client wrapper and probe requests

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/github.rs`
- Test: `tests/github_preflight.rs`

**Step 1: Add an HTTP mocking dev dependency**

Add either:

```toml
wiremock = "0.6"
```

or:

```toml
httpmock = "0.7"
```

Prefer `wiremock` for async compatibility.

**Step 2: Write the failing preflight test**

Create a test server that returns:

- a search response with `total_count`
- rate-limit headers for search remaining and reset time

Assert that preflight computes the right estimated request count.

**Step 3: Run the test to verify failure**

Run: `cargo test --test github_preflight`
Expected: FAIL because the HTTP client and preflight logic do not exist

**Step 4: Implement the GitHub client**

Add:

- `GitHubClient`
- configurable `base_url`
- shared `reqwest::Client`
- method to run a `per_page=1` probe
- method to parse rate-limit headers into `RateLimitSnapshot`

Set a descriptive user agent explicitly.

**Step 5: Run the targeted test**

Run: `cargo test --test github_preflight`
Expected: PASS

**Step 6: Commit**

```bash
git add Cargo.toml src/github.rs tests/github_preflight.rs
git commit -m "feat: add github preflight client"
```

### Task 9: Implement strict preflight refusal logic

**Files:**
- Modify: `src/github.rs`
- Modify: `src/error.rs`
- Test: `tests/preflight_refusal.rs`

**Step 1: Write the failing refusal test**

Create a test that feeds synthetic estimates and rate-limit snapshots and asserts:

- preflight passes when `estimated_requests + safety_margin <= remaining`
- preflight fails with a descriptive message when the budget is insufficient

**Step 2: Run the test to verify failure**

Run: `cargo test --test preflight_refusal`
Expected: FAIL because no refusal policy exists

**Step 3: Implement the refusal policy**

Add a function shaped like:

```rust
pub fn ensure_preflight_budget(estimate: &PreflightEstimate, safety_margin: u32) -> Result<(), AppError>
```

The error message should include month, orgs, estimated requests, remaining budget, and high-volume query drivers.

**Step 4: Run the targeted test**

Run: `cargo test --test preflight_refusal`
Expected: PASS

**Step 5: Commit**

```bash
git add src/github.rs src/error.rs tests/preflight_refusal.rs
git commit -m "feat: add strict preflight refusal policy"
```

### Task 10: Implement paginated fetch and normalization streaming

**Files:**
- Modify: `src/github.rs`
- Modify: `src/model.rs`
- Test: `tests/github_fetch.rs`

**Step 1: Write the failing fetch test**

Create an HTTP mock scenario with:

- two pages of issue results
- one page of PR results

Assert that the fetch layer returns normalized items in order and counts requests correctly.

**Step 2: Run the test to verify failure**

Run: `cargo test --test github_fetch`
Expected: FAIL because pagination fetch is not implemented

**Step 3: Implement paginated search fetch**

Add:

- page fetch method with configurable `per_page`
- bounded retry for transient failures
- sequential page iteration
- immediate normalization to `NormalizedItem`

Keep this step return-value based first. File writing comes in the next task.

**Step 4: Run the targeted test**

Run: `cargo test --test github_fetch`
Expected: PASS

**Step 5: Commit**

```bash
git add src/github.rs src/model.rs tests/github_fetch.rs
git commit -m "feat: add paginated github search fetch"
```

### Task 11: Implement JSONL and CSV exporters

**Files:**
- Modify: `src/export.rs`
- Test: `tests/export_items.rs`

**Step 1: Write the failing exporter tests**

Create tests that:

- write two `NormalizedItem` values to JSONL
- write the same items to CSV
- assert file contents contain expected headers and records

Use a temp directory per test.

**Step 2: Run the tests to verify failure**

Run: `cargo test --test export_items`
Expected: FAIL because exporters do not exist

**Step 3: Implement export writers**

Add:

- JSONL writer that serializes one record per line
- CSV writer with stable header ordering
- helper that creates the output directory if missing

Keep JSONL canonical. CSV should be optional.

**Step 4: Run the targeted tests**

Run: `cargo test --test export_items`
Expected: PASS

**Step 5: Commit**

```bash
git add src/export.rs tests/export_items.rs
git commit -m "feat: add item exporters"
```

### Task 12: Implement metadata writing

**Files:**
- Modify: `src/export.rs`
- Modify: `src/github.rs`
- Test: `tests/export_metadata.rs`

**Step 1: Write the failing metadata test**

Create a test that builds a metadata struct, writes `metadata.json`, and asserts:

- `completed` is serialized
- org list is preserved
- preflight fields are present

**Step 2: Run the test to verify failure**

Run: `cargo test --test export_metadata`
Expected: FAIL because metadata writing does not exist

**Step 3: Implement metadata structs and writer**

Add:

- `RunMetadata`
- nested `PreflightMetadata`
- JSON writer with pretty output for readability

**Step 4: Run the targeted test**

Run: `cargo test --test export_metadata`
Expected: PASS

**Step 5: Commit**

```bash
git add src/export.rs src/github.rs tests/export_metadata.rs
git commit -m "feat: add run metadata export"
```

### Task 13: Wire the `fetch` command end-to-end

**Files:**
- Modify: `src/main.rs`
- Modify: `src/cli.rs`
- Modify: `src/export.rs`
- Modify: `src/github.rs`
- Modify: `src/time_window.rs`
- Test: `tests/fetch_command.rs`

**Step 1: Write the failing fetch command test**

Create an integration test that:

- points `--base-url` at a mock server
- runs `fetch --org rust-lang --month 2026-03 --out <tempdir>`
- asserts `items.jsonl` and `metadata.json` exist

**Step 2: Run the test to verify failure**

Run: `cargo test --test fetch_command`
Expected: FAIL because the command is not fully wired

**Step 3: Implement the `fetch` command flow**

Wire:

- CLI args to month resolution
- preflight probe requests
- refusal policy
- paginated fetch for issue and PR query families
- JSONL output
- optional CSV output
- final metadata write

Ensure incomplete runs still emit `metadata.json` with `completed: false`.

**Step 4: Run the targeted test**

Run: `cargo test --test fetch_command`
Expected: PASS

**Step 5: Commit**

```bash
git add src/main.rs src/cli.rs src/export.rs src/github.rs src/time_window.rs tests/fetch_command.rs
git commit -m "feat: add end-to-end fetch command"
```

### Task 14: Implement local report aggregation

**Files:**
- Modify: `src/report.rs`
- Test: `tests/report_aggregate.rs`

**Step 1: Write the failing aggregation tests**

Create fixture items covering:

- multiple orgs
- repeated repos
- repeated authors
- repeated labels
- multiple days

Assert the aggregate output contains correct totals and rankings.

**Step 2: Run the tests to verify failure**

Run: `cargo test --test report_aggregate`
Expected: FAIL because report aggregation does not exist

**Step 3: Implement report aggregation types**

Add:

- `ReportSummary`
- per-org counts
- top repo calculation
- top author calculation
- top label calculation
- busiest day calculation

Keep aggregation pure and independent of terminal formatting.

**Step 4: Run the targeted tests**

Run: `cargo test --test report_aggregate`
Expected: PASS

**Step 5: Commit**

```bash
git add src/report.rs tests/report_aggregate.rs
git commit -m "feat: add report aggregation"
```

### Task 15: Implement the `report` command output

**Files:**
- Modify: `src/main.rs`
- Modify: `src/report.rs`
- Test: `tests/report_command.rs`

**Step 1: Write the failing report command test**

Create a test that:

- writes a small `items.jsonl` fixture
- runs `report --input <file>`
- asserts the output contains total items and at least one top repo line

Add a second test for `--format json`.

**Step 2: Run the tests to verify failure**

Run: `cargo test --test report_command`
Expected: FAIL because report command output is not implemented

**Step 3: Implement report reading and rendering**

Add:

- JSONL reader for normalized items
- table-like plain-text renderer
- JSON renderer for structured output

**Step 4: Run the targeted tests**

Run: `cargo test --test report_command`
Expected: PASS

**Step 5: Commit**

```bash
git add src/main.rs src/report.rs tests/report_command.rs
git commit -m "feat: add local report command"
```

### Task 16: Improve CLI validation and usage errors

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/error.rs`
- Test: `tests/cli_validation.rs`

**Step 1: Write the failing validation tests**

Cover:

- missing `--org` on `fetch`
- invalid `--month`
- invalid `--format`
- nonexistent report input path

**Step 2: Run the tests to verify failure**

Run: `cargo test --test cli_validation`
Expected: FAIL because validation is incomplete

**Step 3: Implement validation improvements**

Add:

- argument-level validation where possible in `clap`
- post-parse validation for month strings and report paths
- stable error text suitable for end users

**Step 4: Run the targeted tests**

Run: `cargo test --test cli_validation`
Expected: PASS

**Step 5: Commit**

```bash
git add src/cli.rs src/error.rs tests/cli_validation.rs
git commit -m "feat: improve cli validation"
```

### Task 17: Add README usage documentation

**Files:**
- Create: `README.md`

**Step 1: Write the README**

Document:

- project purpose
- v1 scope and non-goals
- example `fetch` command
- example `report` command
- note that v1 is unauthenticated and may refuse large runs
- expected output files

**Step 2: Verify the examples against the implemented CLI**

Run:

- `cargo run -- window --month 2026-03`
- `cargo run -- --help`

Expected: commands succeed and match the documented flags

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add usage documentation"
```

### Task 18: Run full verification before completion

**Files:**
- Modify: none unless a verification failure requires fixes

**Step 1: Run the formatter**

Run: `cargo fmt --all`
Expected: PASS

**Step 2: Run the linter**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: PASS

**Step 3: Run the full test suite**

Run: `cargo test`
Expected: PASS

**Step 4: Run a smoke fetch against the mock-backed integration tests**

Run: `cargo test --test fetch_command`
Expected: PASS

**Step 5: Commit any verification fixes**

```bash
git add .
git commit -m "chore: finalize gh-monthly v1"
```

## Notes For The Implementer

- Use TDD for every behavior change. Do not batch multiple features into a single unverified step.
- Keep the Search API client sequential in v1. Do not add concurrency unless a failing test or requirement proves it necessary.
- Stream JSONL records as they are normalized. Do not accumulate the full dataset in memory.
- Keep reporting strictly local. `report` must never call GitHub.
- Treat preflight refusal as a product feature, not an edge case.
- If GitHub search result payloads do not expose a field needed for PRs, extend the raw models carefully and only as far as required by tests.
- Prefer small, frequent commits exactly as listed above.
