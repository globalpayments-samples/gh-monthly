use chrono::{TimeZone, Utc};
use gh_monthly::time_window::{resolve_explicit_month, resolve_previous_month};

#[test]
fn resolves_previous_month_march_to_february() {
    let now = Utc.with_ymd_and_hms(2026, 3, 15, 12, 0, 0).unwrap();
    let window = resolve_previous_month(now).unwrap();
    assert_eq!(window.month_id, "2026-02");
}

#[test]
fn resolves_previous_month_january_to_december() {
    let now = Utc.with_ymd_and_hms(2026, 1, 3, 0, 0, 0).unwrap();
    let window = resolve_previous_month(now).unwrap();
    assert_eq!(window.month_id, "2025-12");
}

#[test]
fn resolves_explicit_month_march_2026() {
    let window = resolve_explicit_month("2026-03").unwrap();
    assert_eq!(window.month_id, "2026-03");
    assert_eq!(
        window.start,
        Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap()
    );
    assert_eq!(
        window.end,
        Utc.with_ymd_and_hms(2026, 3, 31, 23, 59, 59).unwrap()
    );
}

#[test]
fn rejects_invalid_month_string() {
    assert!(resolve_explicit_month("not-a-month").is_err());
    assert!(resolve_explicit_month("2026-13").is_err());
    assert!(resolve_explicit_month("2026-00").is_err());
}
