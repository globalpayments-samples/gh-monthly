# GH Monthly Alignment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Bring `gh-monthly` v1 into alignment with the agreed design by fixing the exported schema, metadata contract, streaming fetch behavior, retry policy, report output, and final verification coverage.

**Architecture:** Preserve the current module boundaries, but tighten the product contract at the seams. The main changes are: expand raw and normalized models to match the canonical export schema, convert fetch to a streaming export-first flow, enrich metadata so incomplete runs are explicitly detectable, and make reporting derive the required monthly summary from local exports only.

**Tech Stack:** Rust stable, Cargo, `clap`, `reqwest`, `tokio`, `serde`, `serde_json`, `csv`, `chrono`, `thiserror`, `wiremock`, `assert_cmd`, `predicates`

---

### Task 1: Lock the canonical item schema in tests

**Files:**
- Modify: `tests/model_normalization.rs`
- Modify: `tests/export_items.rs`
- Modify: `tests/report_command.rs`

**Step 1: Expand normalization tests to assert the full v1 schema**

Add assertions for:
- `id`
- `node_id`
- `author_login`
- `merged_at`
- `api_url`
- `is_pull_request`
- nullable author handling

Use fixture shapes like:

```rust
assert_eq!(item.id, 987654321);
assert_eq!(item.node_id, "MDU6SXNzdWU5ODc2NTQzMjE=");
assert_eq!(item.author_login.as_deref(), Some("octocat"));
assert_eq!(item.api_url, "https://api.github.com/repos/rust-lang/rust/issues/12345");
assert!(!item.is_pull_request);
assert_eq!(item.merged_at, None);
```

**Step 2: Add export tests that assert canonical JSONL field names**

Assert serialized JSONL lines contain:

```rust
assert_eq!(value["author_login"], "alice");
assert_eq!(value["is_pull_request"], false);
assert!(value.get("api_url").is_some());
assert!(value.get("merged_at").is_some());
```

**Step 3: Add report-fixture updates for the renamed and added fields**

Update local fixture constructors so tests compile only when the new canonical struct is in place.

**Step 4: Run the targeted tests to verify failure**

Run: `cargo test --test model_normalization --test export_items --test report_command`
Expected: FAIL with missing fields / serialization mismatches

**Step 5: Commit**

```bash
git add tests/model_normalization.rs tests/export_items.rs tests/report_command.rs
git commit -m "test: lock canonical export schema"
```

### Task 2: Implement the canonical normalized item model

**Files:**
- Modify: `src/model.rs`
- Modify: `tests/model_normalization.rs`

**Step 1: Expand raw API structs only as far as the design requires**

Add fields to `RawSearchItem` and related types:

```rust
pub struct RawSearchItem {
    pub id: u64,
    pub node_id: String,
    pub number: u64,
    pub title: String,
    pub state: String,
    pub created_at: String,
    pub closed_at: Option<String>,
    pub html_url: String,
    pub url: String,
    pub user: Option<RawUser>,
    pub labels: Vec<RawLabel>,
    pub pull_request: Option<RawPullRequestLink>,
}

pub struct RawPullRequestLink {
    pub url: String,
    pub merged_at: Option<String>,
}
```

If GitHub’s search payload does not provide `merged_at`, keep it nullable and document that it is `None` in v1 unless later hydrated.

**Step 2: Replace the current normalized struct with the canonical shape**

Implement:

```rust
pub struct NormalizedItem {
    pub item_type: ItemType,
    pub org: String,
    pub repo: String,
    pub number: u64,
    pub id: u64,
    pub node_id: String,
    pub title: String,
    pub author_login: Option<String>,
    pub state: String,
    pub created_at: String,
    pub closed_at: Option<String>,
    pub merged_at: Option<String>,
    pub html_url: String,
    pub api_url: String,
    pub labels: Vec<String>,
    pub is_pull_request: bool,
}
```

**Step 3: Update normalization logic**

Map issue vs PR like:

```rust
let is_pull_request = raw.pull_request.is_some();
let merged_at = raw.pull_request.as_ref().and_then(|pr| pr.merged_at.clone());

Ok(NormalizedItem {
    item_type: if is_pull_request { ItemType::PullRequest } else { ItemType::Issue },
    org,
    repo,
    number: raw.number,
    id: raw.id,
    node_id: raw.node_id,
    title: raw.title,
    author_login: raw.user.map(|u| u.login),
    state: raw.state,
    created_at: raw.created_at,
    closed_at: raw.closed_at,
    merged_at,
    html_url: raw.html_url,
    api_url: raw.url,
    labels: raw.labels.into_iter().map(|l| l.name).collect(),
    is_pull_request,
})
```

**Step 4: Run the targeted tests**

Run: `cargo test --test model_normalization --test export_items`
Expected: PASS

**Step 5: Commit**

```bash
git add src/model.rs tests/model_normalization.rs tests/export_items.rs
git commit -m "feat: align normalized item schema with design"
```

### Task 3: Align CSV export with the canonical dataset

**Files:**
- Modify: `src/export.rs`
- Modify: `tests/export_items.rs`

**Step 1: Add a failing CSV header-order test for the canonical fields**

Assert CSV includes stable headers:

```rust
assert!(content.contains(
    "item_type,org,repo,number,id,node_id,title,author_login,state,created_at,closed_at,merged_at,html_url,api_url,labels,is_pull_request"
));
```

**Step 2: Update CSV writing to derive from the canonical item shape**

Write records using:

```rust
wtr.write_record([
    &item_type,
    &item.org,
    &item.repo,
    &item.number.to_string(),
    &item.id.to_string(),
    &item.node_id,
    &item.title,
    item.author_login.as_deref().unwrap_or(""),
    &item.state,
    &item.created_at,
    item.closed_at.as_deref().unwrap_or(""),
    item.merged_at.as_deref().unwrap_or(""),
    &item.html_url,
    &item.api_url,
    &item.labels.join(";"),
    if item.is_pull_request { "true" } else { "false" },
])
```

**Step 3: Run the targeted test**

Run: `cargo test --test export_items`
Expected: PASS

**Step 4: Commit**

```bash
git add src/export.rs tests/export_items.rs
git commit -m "feat: align csv export with canonical schema"
```

### Task 4: Lock metadata completeness in tests

**Files:**
- Modify: `tests/export_metadata.rs`
- Modify: `tests/fetch_command.rs`

**Step 1: Expand metadata tests to match the design contract**

Assert presence of:
- `tool`
- `run_started_at`
- `run_completed_at`
- `requested_orgs`
- `target_month`
- `window_start`
- `window_end`
- `formats`
- `request_count`
- `preflight.safety_margin_requests`

Use assertions like:

```rust
assert_eq!(value["tool"], "gh-monthly");
assert!(value["run_started_at"].is_string());
assert!(value["window_start"].is_string());
assert!(value["formats"].is_array());
assert!(value["request_count"].is_number());
assert!(value["preflight"]["safety_margin_requests"].is_number());
```

**Step 2: Update fetch integration tests to validate incomplete metadata shape**

On failure, assert:

```rust
assert_eq!(value["completed"], false);
assert_eq!(value["target_month"], "2026-03");
assert!(value["run_started_at"].is_string());
```

**Step 3: Run the targeted tests to verify failure**

Run: `cargo test --test export_metadata --test fetch_command`
Expected: FAIL with missing metadata fields

**Step 4: Commit**

```bash
git add tests/export_metadata.rs tests/fetch_command.rs
git commit -m "test: require complete metadata contract"
```

### Task 5: Implement the full metadata model

**Files:**
- Modify: `src/export.rs`
- Modify: `src/main.rs`
- Modify: `tests/export_metadata.rs`

**Step 1: Replace the minimal metadata structs**

Implement:

```rust
pub struct PreflightMetadata {
    pub estimated_requests: u32,
    pub rate_limit_remaining: u32,
    pub rate_limit_reset_at: String,
    pub safety_margin_requests: u32,
}

pub struct RunMetadata {
    pub tool: String,
    pub run_started_at: String,
    pub run_completed_at: Option<String>,
    pub completed: bool,
    pub requested_orgs: Vec<String>,
    pub target_month: String,
    pub window_start: String,
    pub window_end: String,
    pub formats: Vec<String>,
    pub total_records: u64,
    pub request_count: u32,
    pub preflight: PreflightMetadata,
}
```

**Step 2: Add helpers in `main` to build metadata consistently**

Create a small constructor/helper so success and failure paths cannot drift:

```rust
fn build_run_metadata(/* explicit inputs */) -> RunMetadata { /* ... */ }
```

Prefer one path for:
- initial incomplete metadata
- post-preflight incomplete metadata
- terminal success metadata
- terminal fetch-failure metadata

**Step 3: Serialize rate-limit reset timestamps as UTC strings**

Convert unix seconds into RFC3339/`%Y-%m-%dT%H:%M:%SZ` output before storing in metadata.

**Step 4: Run targeted tests**

Run: `cargo test --test export_metadata`
Expected: PASS

**Step 5: Commit**

```bash
git add src/export.rs src/main.rs tests/export_metadata.rs
git commit -m "feat: implement complete run metadata"
```

### Task 6: Add retry-aware HTTP behavior and request accounting

**Files:**
- Modify: `src/github.rs`
- Modify: `src/error.rs`
- Modify: `tests/github_preflight.rs`
- Modify: `tests/github_fetch.rs`

**Step 1: Add failing tests for HTTP status handling and retries**

Cover:
- `probe` rejects non-success status codes with a fetch error
- `fetch_all` retries transient `500`
- `fetch_all` honors bounded retry count
- `fetch_all` surfaces failure after retries are exhausted

Use a mock sequence shaped like:

```rust
Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(500))
    .up_to_n_times(1)
    .mount(&server)
    .await;

Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(200).set_body_json(search_page(...)))
    .mount(&server)
    .await;
```

**Step 2: Implement a reusable send helper**

Add a helper like:

```rust
async fn send_with_retries(&self, request: RequestBuilder) -> Result<Response, AppError>
```

Behavior:
- retry bounded times on `5xx`
- retry bounded times on timeout
- if `429`/secondary rate limit includes `retry-after`, sleep once up to a small bound
- fail fast on non-retryable `4xx`

**Step 3: Make `probe` and `fetch_all` check status before JSON parsing**

Use:

```rust
let resp = self.send_with_retries(builder).await?;
let resp = resp.error_for_status().map_err(|e| AppError::Fetch(e.to_string()))?;
```

**Step 4: Return actual request counts from fetch operations**

Introduce a result type like:

```rust
pub struct FetchPageResult {
    pub items: Vec<NormalizedItem>,
    pub request_count: u32,
}
```

This will let metadata record actual request volume rather than only the estimate.

**Step 5: Run targeted tests**

Run: `cargo test --test github_preflight --test github_fetch`
Expected: PASS

**Step 6: Commit**

```bash
git add src/github.rs src/error.rs tests/github_preflight.rs tests/github_fetch.rs
git commit -m "feat: add retry-aware github client behavior"
```

### Task 7: Convert fetch to an export-first streaming flow

**Files:**
- Modify: `src/export.rs`
- Modify: `src/main.rs`
- Modify: `src/github.rs`
- Modify: `tests/fetch_command.rs`

**Step 1: Add failing tests for streaming-oriented behavior**

Cover:
- `items.jsonl` exists even when a later fetch page fails after some records were written
- `metadata.json` still ends with `completed: false`
- successful fetch still writes complete JSONL and optional CSV

**Step 2: Add a JSONL append writer**

Implement:

```rust
pub struct JsonlItemWriter { /* BufWriter<File> */ }

impl JsonlItemWriter {
    pub fn create(path: &Path) -> Result<Self, AppError>;
    pub fn write_item(&mut self, item: &NormalizedItem) -> Result<(), AppError>;
    pub fn flush(&mut self) -> Result<(), AppError>;
}
```

**Step 3: Add a fetch iterator/callback path**

Refactor `GitHubClient` so page fetches can be consumed incrementally:

```rust
pub async fn fetch_all_into<F>(
    &self,
    query: &str,
    kind: SearchKind,
    per_page: u32,
    mut on_item: F,
) -> Result<u32, AppError>
where
    F: FnMut(NormalizedItem) -> Result<(), AppError>,
```

Return request count; write each item immediately via the callback.

**Step 4: Refactor `main` to stop buffering the full dataset**

The `fetch` branch should:
- create initial metadata
- open JSONL writer immediately after preflight passes
- stream each normalized item to disk as received
- optionally collect a small `Vec<NormalizedItem>` only when CSV export is requested
- update final metadata with actual `request_count`

**Step 5: Run targeted tests**

Run: `cargo test --test fetch_command`
Expected: PASS

**Step 6: Commit**

```bash
git add src/export.rs src/main.rs src/github.rs tests/fetch_command.rs
git commit -m "feat: stream canonical jsonl during fetch"
```

### Task 8: Bring report aggregation into alignment with the design

**Files:**
- Modify: `src/report.rs`
- Modify: `tests/report_aggregate.rs`
- Modify: `tests/report_command.rs`

**Step 1: Add failing tests for missing report fields**

Assert output includes:
- target month
- total items
- issue count
- pull request count
- counts by org
- top repos
- top authors
- top labels
- busiest days

For JSON output, assert the same structure is present with explicit keys:

```rust
assert_eq!(value["target_month"], "2026-03");
assert_eq!(value["issue_count"], 2);
assert_eq!(value["pull_request_count"], 1);
assert!(value["busiest_days"].is_array());
```

**Step 2: Refactor report input to derive month information from local metadata**

Implement a small loader that reads:
- `items.jsonl`
- sibling `metadata.json` when present

Prefer metadata for `target_month`; for multiple inputs, show the sorted included months as a comma-separated list. If no month metadata is available, fall back to `"unknown"` without calling GitHub.

**Step 3: Expand aggregation types**

Add:

```rust
pub struct ReportSummary {
    pub target_month: String,
    pub total_items: u64,
    pub issue_count: u64,
    pub pull_request_count: u64,
    pub by_org: Vec<(String, u64)>,
    pub top_repos: Vec<(String, u64)>,
    pub top_authors: Vec<(String, u64)>,
    pub top_labels: Vec<(String, u64)>,
    pub busiest_days: Vec<(String, u64)>,
}
```

**Step 4: Update rendering**

Table output should include:

```text
Month: 2026-03
Total items: 142
Issues: 80
Pull requests: 62
```

Then print org counts, top 10 lists, and top busiest days.

**Step 5: Run targeted tests**

Run: `cargo test --test report_aggregate --test report_command`
Expected: PASS

**Step 6: Commit**

```bash
git add src/report.rs tests/report_aggregate.rs tests/report_command.rs
git commit -m "feat: align report output with monthly summary design"
```

### Task 9: Tighten end-user validation and README examples to the new contract

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `tests/cli_validation.rs`

**Step 1: Add failing validation/docs tests where behavior changed**

Cover:
- report on malformed metadata still reads local data and prints `"unknown"` month
- fetch error text still uses stable exit codes after retry exhaustion

**Step 2: Update README examples**

Document:
- canonical JSONL / CSV schema language
- metadata trust fields
- incomplete-run semantics
- repeated `report --input` usage and included-month display
- rate-limit refusal and retry behavior
- report output including month / issue / PR totals

**Step 3: Run targeted tests**

Run: `cargo test --test cli_validation --test report_command`
Expected: PASS

**Step 4: Commit**

```bash
git add src/cli.rs src/main.rs README.md tests/cli_validation.rs tests/report_command.rs
git commit -m "docs: align usage and validation with final contract"
```

### Task 10: Re-run full verification and fix any fallout

**Files:**
- Modify: only files required by verification failures

**Step 1: Run formatting**

Run: `cargo fmt --all`
Expected: PASS

**Step 2: Run linting**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: PASS

**Step 3: Run the full test suite**

Run: `cargo test`
Expected: PASS

**Step 4: Re-run fetch integration tests in an environment that allows local port binding**

Run: `cargo test --test fetch_command`
Expected: PASS

If the current sandbox still blocks `wiremock`, rerun with the required permissions or in a normal local shell and record the exact date of verification: April 8, 2026.

**Step 5: Commit any final verification fixes**

```bash
git add .
git commit -m "chore: align gh-monthly implementation with design"
```

## Execution Notes

- Keep the work TDD-first. Do not batch Tasks 2, 5, 6, 7, and 8 into one large change.
- Do not reintroduce full-run buffering for the canonical JSONL path.
- Keep `report` strictly local; it must not call GitHub to discover the month.
- Prefer one metadata-construction path to avoid drift across success and failure branches.
- If `merged_at` cannot be sourced from Search API payloads alone, keep it nullable and document that limitation in code comments and README rather than inventing data.
