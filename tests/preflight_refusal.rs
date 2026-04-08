use gh_monthly::github::{ensure_preflight_budget, PreflightEstimate, RateLimitSnapshot};

fn make_estimate(estimated_requests: u32, remaining: u32) -> PreflightEstimate {
    PreflightEstimate {
        estimated_requests,
        month_id: "2026-03".to_string(),
        orgs: vec!["rust-lang".to_string()],
        high_volume_drivers: vec![],
        rate_limit: RateLimitSnapshot {
            remaining,
            reset_at: 9_999_999_999,
            limit: 30,
        },
    }
}

#[test]
fn passes_when_budget_is_sufficient() {
    // estimated=5, safety=2, remaining=10 → 5+2=7 ≤ 10 → ok
    let estimate = make_estimate(5, 10);
    assert!(ensure_preflight_budget(&estimate, 2).is_ok());
}

#[test]
fn passes_exactly_at_budget_boundary() {
    // estimated=8, safety=2, remaining=10 → 8+2=10 ≤ 10 → ok
    let estimate = make_estimate(8, 10);
    assert!(ensure_preflight_budget(&estimate, 2).is_ok());
}

#[test]
fn refuses_when_budget_is_insufficient() {
    // estimated=9, safety=2, remaining=10 → 9+2=11 > 10 → error
    let estimate = make_estimate(9, 10);
    let err = ensure_preflight_budget(&estimate, 2).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("2026-03"), "error should mention month");
    assert!(msg.contains("rust-lang"), "error should mention org");
    assert!(msg.contains("11"), "error should mention required count");
    assert!(msg.contains("10"), "error should mention available count");
}

#[test]
fn error_message_includes_high_volume_drivers() {
    let mut estimate = make_estimate(9, 10);
    estimate.high_volume_drivers = vec!["rust-lang:issue".to_string()];
    let err = ensure_preflight_budget(&estimate, 2).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("rust-lang:issue"), "error should mention driver");
}
