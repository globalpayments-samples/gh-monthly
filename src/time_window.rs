use chrono::{DateTime, Datelike, TimeZone, Utc};

use crate::error::AppError;

pub struct MonthWindow {
    pub month_id: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

pub fn resolve_previous_month(now: DateTime<Utc>) -> Result<MonthWindow, AppError> {
    let (year, month) = if now.month() == 1 {
        (now.year() - 1, 12u32)
    } else {
        (now.year(), now.month() - 1)
    };
    build_window(year, month)
}

pub fn resolve_explicit_month(month_str: &str) -> Result<MonthWindow, AppError> {
    let parts: Vec<&str> = month_str.splitn(2, '-').collect();
    if parts.len() != 2 {
        return Err(AppError::Usage(format!(
            "invalid month format '{}': expected YYYY-MM",
            month_str
        )));
    }

    let year: i32 = parts[0].parse().map_err(|_| {
        AppError::Usage(format!(
            "invalid month format '{}': year is not a number",
            month_str
        ))
    })?;

    let month: u32 = parts[1].parse().map_err(|_| {
        AppError::Usage(format!(
            "invalid month format '{}': month is not a number",
            month_str
        ))
    })?;

    if !(1..=12).contains(&month) {
        return Err(AppError::Usage(format!(
            "invalid month '{}': month must be between 01 and 12",
            month_str
        )));
    }

    build_window(year, month)
}

fn build_window(year: i32, month: u32) -> Result<MonthWindow, AppError> {
    let start = Utc
        .with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::Usage(format!("invalid date: {}-{:02}", year, month)))?;

    let last_day = days_in_month(year, month);
    let end = Utc
        .with_ymd_and_hms(year, month, last_day, 23, 59, 59)
        .single()
        .ok_or_else(|| AppError::Usage(format!("invalid end date: {}-{:02}", year, month)))?;

    Ok(MonthWindow {
        month_id: format!("{}-{:02}", year, month),
        start,
        end,
    })
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 {
        Utc.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0).unwrap()
    } else {
        Utc.with_ymd_and_hms(year, month + 1, 1, 0, 0, 0).unwrap()
    };
    (next_month - Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap()).num_days() as u32
}
