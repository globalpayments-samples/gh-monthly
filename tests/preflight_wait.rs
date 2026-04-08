use std::time::Duration;

use gh_monthly::github::{preflight_wait_duration, PreflightEstimate, RateLimitSnapshot};

fn make_estimate(estimated_requests: u32, remaining: u32, reset_at: u64) -> PreflightEstimate {
    PreflightEstimate {
        estimated_requests,
        month_id: "2026-03".to_string(),
        orgs: vec!["rust-lang".to_string()],
        high_volume_drivers: vec![],
        rate_limit: RateLimitSnapshot {
            remaining,
            reset_at,
            limit: 30,
        },
    }
}

#[test]
fn skips_wait_when_budget_is_sufficient() {
    let estimate = make_estimate(5, 10, 9_999_999_999);
    assert_eq!(preflight_wait_duration(&estimate, 2, 1_700_000_000), None);
}

#[test]
fn skips_wait_exactly_at_budget_boundary() {
    let estimate = make_estimate(8, 10, 9_999_999_999);
    assert_eq!(preflight_wait_duration(&estimate, 2, 1_700_000_000), None);
}

#[test]
fn waits_until_reset_when_budget_is_insufficient() {
    let estimate = make_estimate(9, 10, 1_700_000_010);
    assert_eq!(
        preflight_wait_duration(&estimate, 2, 1_700_000_000),
        Some(Duration::from_secs(10))
    );
}

#[test]
fn does_not_wait_when_reset_time_has_already_passed() {
    let estimate = make_estimate(9, 10, 1_699_999_999);
    assert_eq!(preflight_wait_duration(&estimate, 2, 1_700_000_000), None);
}
