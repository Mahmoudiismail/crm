use anyhow::{Context, Result};
use chrono::{Datelike, Duration as ChronoDuration, NaiveDate};
use futures_util::future::join_all;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::crm::config::AppConfig;
use crate::crm::types::ReportType;

// ──────────────────────────────────────────────────────────────
// Report definitions
// ──────────────────────────────────────────────────────────────

struct ReportDef {
    key: &'static str,
    endpoint: &'static str,
    extra_params: &'static [(&'static str, &'static str)],
}

fn report_defs() -> &'static [ReportDef] {
    &[
        ReportDef {
            key: "tickets",
            endpoint: "download-ticket-data",
            extra_params: &[("type", "ticket_report")],
        },
        ReportDef {
            key: "calls",
            endpoint: "download-call-log-data",
            extra_params: &[],
        },
        ReportDef {
            key: "leads",
            endpoint: "download-lead-data",
            extra_params: &[("type", "lead_report")],
        },
    ]
}

// ──────────────────────────────────────────────────────────────
// Public API
// ──────────────────────────────────────────────────────────────

/// Fetch reports based on the requested type. Returns a JSON object keyed by report type.
pub async fn fetch_reports(
    config: &AppConfig,
    client: &reqwest::Client,
    token: &str,
    report_type: ReportType,
) -> Result<Value> {
    let mut results = serde_json::Map::new();

    if report_type == ReportType::None {
        info!("Report type is 'none', skipping all fetches");
        return Ok(Value::Object(results));
    }

    let defs = report_defs();

    let should_fetch = |key: &str| -> bool {
        match report_type {
            ReportType::All => true,
            ReportType::Tickets => key == "tickets",
            ReportType::Calls => key == "calls",
            ReportType::Leads => key == "leads",
            ReportType::None => false,
        }
    };

    // Build task list
    let mut handles: Vec<tokio::task::JoinHandle<(String, Value)>> = Vec::new();

    let token_arc = Arc::new(token.to_string());
    let base_url_arc = Arc::new(config.base_url.clone());
    let email_arc = Arc::new(config.email.clone());
    let account_id_arc = Arc::new(config.account_id.clone());
    let application_id_arc = Arc::new(config.application_id.clone());
    let tz_arc = Arc::new(config.app_timezone_plus_minutes.clone());

    for def in defs {
        if !should_fetch(def.key) {
            continue;
        }

        let endpoint = def.endpoint;
        let extra = def.extra_params;

        if def.key == "calls" {
            // Call logs: split into monthly batches
            let batches = split_monthly(&config.calls_from_date, &config.to_date)?;
            info!(
                "Call logs: {} monthly batches from {} to {}",
                batches.len(),
                config.calls_from_date,
                config.to_date
            );

            for (batch_from, batch_to) in batches {
                let client = client.clone();
                let token = Arc::clone(&token_arc);
                let base_url = Arc::clone(&base_url_arc);
                let email = Arc::clone(&email_arc);
                let account_id = Arc::clone(&account_id_arc);
                let application_id = Arc::clone(&application_id_arc);
                let tz = Arc::clone(&tz_arc);

                handles.push(tokio::spawn(async move {
                    let key = format!("calls_{}_{}", batch_from, batch_to);
                    let params = FetchParams {
                        base_url: &base_url,
                        email: &email,
                        account_id: &account_id,
                        application_id: &application_id,
                        tz: &tz,
                        extra_params: extra,
                    };
                    let result = fetch_with_signed_url_split(
                        &client,
                        &token,
                        endpoint,
                        &batch_from,
                        &batch_to,
                        &params,
                    )
                    .await;
                    match result {
                        Ok(v) => (key, v),
                        Err(e) => {
                            error!("Call log batch {}-{} failed: {}", batch_from, batch_to, e);
                            (key, serde_json::json!({"error": format!("{}", e)}))
                        }
                    }
                }));
            }
        } else {
            // Tickets / Leads: try the full range first, then split if the
            // backend refuses to generate a signed URL for a large file.
            let client = client.clone();
            let token = Arc::clone(&token_arc);
            let base_url = Arc::clone(&base_url_arc);
            let email = Arc::clone(&email_arc);
            let from_date = config.from_date.clone();
            let to_date = config.to_date.clone();
            let account_id = Arc::clone(&account_id_arc);
            let application_id = Arc::clone(&application_id_arc);
            let tz = Arc::clone(&tz_arc);
            let key = def.key.to_string();

            handles.push(tokio::spawn(async move {
                let params = FetchParams {
                    base_url: &base_url,
                    email: &email,
                    account_id: &account_id,
                    application_id: &application_id,
                    tz: &tz,
                    extra_params: extra,
                };
                let result = fetch_with_signed_url_split(
                    &client,
                    &token,
                    endpoint,
                    &from_date,
                    &to_date,
                    &params,
                )
                .await;
                match result {
                    Ok(v) => (key, v),
                    Err(e) => {
                        error!("Report '{}' failed: {}", endpoint, e);
                        (key, serde_json::json!({"error": format!("{}", e)}))
                    }
                }
            }));
        }
    }

    // Await all
    let task_results = join_all(handles).await;

    // Assemble results
    let mut calls_array: Vec<Value> = Vec::new();

    for task_result in task_results {
        match task_result {
            Ok((key, value)) => {
                if key.starts_with("calls_") {
                    calls_array.push(value);
                } else {
                    results.insert(key, value);
                }
            }
            Err(e) => {
                error!("Task join error: {}", e);
            }
        }
    }

    if !calls_array.is_empty() || should_fetch("calls") {
        results.insert("calls".into(), Value::Array(calls_array));
    }

    Ok(Value::Object(results))
}

// ──────────────────────────────────────────────────────────────
// Single report fetch
// ──────────────────────────────────────────────────────────────

struct FetchParams<'a> {
    base_url: &'a str,
    email: &'a str,
    account_id: &'a str,
    application_id: &'a str,
    tz: &'a str,
    extra_params: &'a [(&'a str, &'a str)],
}

async fn fetch_with_signed_url_split(
    client: &reqwest::Client,
    token: &str,
    endpoint: &str,
    from_date: &str,
    to_date: &str,
    params: &FetchParams<'_>,
) -> Result<Value> {
    let mut pending = vec![(from_date.to_string(), to_date.to_string())];
    let mut completed: Vec<(String, String, Value)> = Vec::new();
    let mut split_used = false;

    while let Some((batch_from, batch_to)) = pending.pop() {
        let result = fetch_single(
            client,
            token,
            endpoint,
            &batch_from,
            &batch_to,
            params,
        )
        .await;

        match result {
            Ok(value) => completed.push((batch_from, batch_to, value)),
            Err(err) if is_signed_url_generation_failure(&err) => {
                if let Some((left, right)) = split_range_in_half(&batch_from, &batch_to)? {
                    split_used = true;
                    info!(
                        "{} [{} to {}] failed to generate signed URL; retrying as [{} to {}] and [{} to {}]",
                        endpoint, batch_from, batch_to, left.0, left.1, right.0, right.1
                    );
                    pending.push(right);
                    pending.push(left);
                } else {
                    return Err(err).with_context(|| {
                        format!(
                            "{} failed to generate a signed URL for single-day range {}",
                            endpoint, batch_from
                        )
                    });
                }
            }
            Err(err) => return Err(err),
        }
    }

    completed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    if split_used || completed.len() > 1 {
        Ok(Value::Array(
            completed.into_iter().map(|(_, _, value)| value).collect(),
        ))
    } else {
        completed
            .pop()
            .map(|(_, _, value)| value)
            .context("No report fetch result was produced")
    }
}

async fn fetch_single(
    client: &reqwest::Client,
    token: &str,
    endpoint: &str,
    from_date: &str,
    to_date: &str,
    params: &FetchParams<'_>,
) -> Result<Value> {
    let url = format!("{}/{}", params.base_url.trim_end_matches('/'), endpoint);

    let mut query: Vec<(&str, &str)> = vec![
        ("from_date", from_date),
        ("to_date", to_date),
        ("email", params.email),
    ];
    for (k, v) in params.extra_params {
        query.push((*k, *v));
    }

    info!("Fetching {} [{} to {}]...", endpoint, from_date, to_date);

    let resp = client
        .get(&url)
        .query(&query)
        .header("account_id", params.account_id)
        .header("app-timezone-plus-minutes", params.tz)
        .header("application_id", params.application_id)
        .header("auth-type", "cognito")
        .header("authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .header("accept", "*/*")
        .send()
        .await
        .with_context(|| format!("HTTP request to {} failed", endpoint))?;

    let status = resp.status();
    let headers = format!("{:?}", resp.headers());
    let body = resp
        .text()
        .await
        .with_context(|| format!("Failed to read response body from {}", endpoint))?;

    debug!(
        "Response from {} — status: {}, headers: {}",
        endpoint, status, headers
    );
    debug!("Response body from {}: {}", endpoint, body);

    if !status.is_success() {
        anyhow::bail!("{} returned HTTP {}: {}", endpoint, status, body);
    }

    let parsed: Value =
        serde_json::from_str(&body).with_context(|| format!("Invalid JSON from {}", endpoint))?;
    Ok(parsed)
}

fn is_signed_url_generation_failure(err: &anyhow::Error) -> bool {
    err.to_string()
        .to_ascii_lowercase()
        .contains("failed to generate signed url")
}

type DateRange = (String, String);

fn split_range_in_half(
    from: &str,
    to: &str,
) -> Result<Option<(DateRange, DateRange)>> {
    let start = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .with_context(|| format!("Invalid from_date: {}", from))?;
    let end = NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .with_context(|| format!("Invalid to_date: {}", to))?;

    if start > end {
        anyhow::bail!("from_date ({}) is after to_date ({})", from, to);
    }

    if start == end {
        return Ok(None);
    }

    let days = (end - start).num_days();
    let left_end = start + ChronoDuration::days(days / 2);
    let right_start = left_end + ChronoDuration::days(1);

    Ok(Some((
        (
            start.format("%Y-%m-%d").to_string(),
            left_end.format("%Y-%m-%d").to_string(),
        ),
        (
            right_start.format("%Y-%m-%d").to_string(),
            end.format("%Y-%m-%d").to_string(),
        ),
    )))
}

// ──────────────────────────────────────────────────────────────
// Monthly date splitting
// ──────────────────────────────────────────────────────────────

/// Split [from, to] into monthly slices.
/// E.g. 2026-01-01 to 2026-03-15 → [(01-01, 01-31), (02-01, 02-28), (03-01, 03-15)]
pub fn split_monthly(from: &str, to: &str) -> Result<Vec<(String, String)>> {
    let start = NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .with_context(|| format!("Invalid from_date: {}", from))?;
    let end = NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .with_context(|| format!("Invalid to_date: {}", to))?;

    if start > end {
        anyhow::bail!("from_date ({}) is after to_date ({})", from, to);
    }

    let mut batches = Vec::new();
    let mut cursor = start;

    while cursor <= end {
        // End of current month
        let month_end = last_day_of_month(cursor.year(), cursor.month());

        let batch_end = if month_end > end { end } else { month_end };
        batches.push((
            cursor.format("%Y-%m-%d").to_string(),
            batch_end.format("%Y-%m-%d").to_string(),
        ));

        // Move to 1st of next month
        if cursor.month() == 12 {
            cursor = NaiveDate::from_ymd_opt(cursor.year() + 1, 1, 1).unwrap_or(end);
        } else {
            cursor = NaiveDate::from_ymd_opt(cursor.year(), cursor.month() + 1, 1).unwrap_or(end);
        }
    }

    Ok(batches)
}

/// Return the last day of the given month.
fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .unwrap()
            .pred_opt()
            .unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .unwrap()
            .pred_opt()
            .unwrap()
    }
}

/// Extract download URLs from report results.
pub fn extract_urls(results: &Value) -> Vec<(String, String)> {
    let mut urls = Vec::new();

    if let Value::Object(map) = results {
        for (key, val) in map {
            extract_urls_for_key(key, val, &mut urls);
        }
    }
    urls
}

fn extract_urls_for_key(key: &str, val: &Value, urls: &mut Vec<(String, String)>) {
    if let Some(url) = extract_data_url(val) {
        urls.push((key.to_string(), url));
        return;
    }

    if let Value::Array(arr) = val {
        for item in arr {
            extract_urls_for_key(key, item, urls);
        }
    }
}

fn extract_data_url(val: &Value) -> Option<String> {
    val.get("data")
        .and_then(|d| d.get("url"))
        .and_then(|u| u.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_monthly_single_month() {
        let batches = split_monthly("2026-01-05", "2026-01-20").unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0], ("2026-01-05".into(), "2026-01-20".into()));
    }

    #[test]
    fn test_split_monthly_multi() {
        let batches = split_monthly("2026-01-01", "2026-03-15").unwrap();
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0], ("2026-01-01".into(), "2026-01-31".into()));
        assert_eq!(batches[1], ("2026-02-01".into(), "2026-02-28".into()));
        assert_eq!(batches[2], ("2026-03-01".into(), "2026-03-15".into()));
    }

    #[test]
    fn test_split_monthly_leap_year() {
        let batches = split_monthly("2024-02-01", "2024-02-29").unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0], ("2024-02-01".into(), "2024-02-29".into()));
    }

    #[test]
    fn test_last_day_feb() {
        assert_eq!(
            last_day_of_month(2024, 2),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
        assert_eq!(
            last_day_of_month(2025, 2),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        );
    }

    #[test]
    fn test_split_range_in_half_even_days() {
        let split = split_range_in_half("2026-01-01", "2026-01-04")
            .unwrap()
            .unwrap();
        assert_eq!(split.0, ("2026-01-01".into(), "2026-01-02".into()));
        assert_eq!(split.1, ("2026-01-03".into(), "2026-01-04".into()));
    }

    #[test]
    fn test_split_range_in_half_odd_days() {
        let split = split_range_in_half("2026-01-01", "2026-01-05")
            .unwrap()
            .unwrap();
        assert_eq!(split.0, ("2026-01-01".into(), "2026-01-03".into()));
        assert_eq!(split.1, ("2026-01-04".into(), "2026-01-05".into()));
    }

    #[test]
    fn test_split_range_in_half_single_day() {
        let split = split_range_in_half("2026-01-01", "2026-01-01").unwrap();
        assert!(split.is_none());
    }

    #[test]
    fn test_extract_urls_from_any_report_array() {
        let results = serde_json::json!({
            "tickets": [
                {"data": {"url": "https://example.com/tickets-1.csv"}},
                {"data": {"url": "https://example.com/tickets-2.csv"}}
            ],
            "leads": {"data": {"url": "https://example.com/leads.csv"}}
        });

        let urls = extract_urls(&results);
        assert_eq!(
            urls,
            vec![
                ("leads".into(), "https://example.com/leads.csv".into()),
                ("tickets".into(), "https://example.com/tickets-1.csv".into()),
                ("tickets".into(), "https://example.com/tickets-2.csv".into()),
            ]
        );
    }
}
