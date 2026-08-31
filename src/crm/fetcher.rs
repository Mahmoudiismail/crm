use anyhow::{Context, Result};
use chrono::{Datelike, Duration as ChronoDuration, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;
use tracing::{debug, error, info, trace};

use crate::crm::config::AppConfig;

#[derive(Debug, Deserialize)]
struct IncompleteReservationItem {
    id: String,
}

#[derive(Debug, Deserialize)]
struct IncompleteReservationData {
    items: Vec<IncompleteReservationItem>,
}

#[derive(Debug, Deserialize)]
struct IncompleteReservationResponse {
    data: IncompleteReservationData,
}

#[derive(Debug, Serialize)]
struct BulkTicketPayload<'a> {
    status_id: &'a str,
    ids: Vec<String>,
}

// ──────────────────────────────────────────────────────────────
// Fetch Context Context
// ──────────────────────────────────────────────────────────────

use tokio::sync::Mutex;

struct FetchContext {
    config: Arc<Mutex<AppConfig>>,
}

impl FetchContext {
    async fn get_valid_token_or_refresh(
        &self,
        client: &reqwest::Client,
        current_invalid_token: &str,
    ) -> Result<String> {
        let mut cfg = self.config.lock().await;
        let token_in_cfg = if !cfg.id_token.is_empty() {
            cfg.id_token.clone()
        } else {
            cfg.access_token.clone()
        };
        if !token_in_cfg.is_empty() && token_in_cfg != current_invalid_token {
            return Ok(token_in_cfg);
        }
        tracing::info!("Token expired (401), performing Cognito SRP authentication...");
        let new_token = crate::crm::auth::ensure_authenticated(&mut cfg, client, false)
            .await
            .context("Failed to refresh token after 401 Unauthorized")?;
        Ok(new_token)
    }
}

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
            endpoint: "task/download-ticket-data",
            extra_params: &[("type", "ticket_report")],
        },
        ReportDef {
            key: "calls",
            endpoint: "task/download-call-log-data",
            extra_params: &[],
        },
        ReportDef {
            key: "leads",
            endpoint: "task/download-lead-data",
            extra_params: &[("type", "lead_report")],
        },
        ReportDef {
            key: "users",
            endpoint: "users/download-user-data",
            extra_params: &[],
        },
        ReportDef {
            key: "incomplete_reservation",
            endpoint: "task/bulk-ticket",
            extra_params: &[],
        },
    ]
}

// ──────────────────────────────────────────────────────────────
// Public API
// ──────────────────────────────────────────────────────────────

/// Initiates concurrent API requests to generate and download CRM reports.
///
/// This function coordinates the fetching of requested report types (e.g., tickets, calls, leads).
/// It handles network retry logic natively. If the CRM API responds with a file size error
/// (e.g., `Failed to generate signed url...`), the fetcher will dynamically split the requested
/// date range into smaller chunks and recursively fetch them concurrently.
///
/// Once a signed URL is successfully resolved from the API, it immediately spawns an asynchronous
/// task to begin downloading the payload to the local disk.
///
/// # Returns
/// A JSON structure containing diagnostic results and the downloaded file paths.
pub async fn fetch_reports(
    config_mutex: Arc<tokio::sync::Mutex<AppConfig>>,
    client: &reqwest::Client,
    report_type: Vec<String>,
    download_dir: &Path,
) -> Result<Value> {
    let mut results = serde_json::Map::new();

    if report_type.is_empty() || report_type.iter().any(|r| r == "none") {
        info!("Report type is 'none', skipping all fetches");
        return Ok(Value::Object(results));
    }

    let (download_tx, download_rx) = tokio::sync::mpsc::unbounded_channel::<(
        reqwest::Client,
        String,
        String,
        std::path::PathBuf,
    )>();

    // Spawn a background task to process downloads concurrently (limit 6)
    let download_processor = tokio::spawn(async move {
        let stream = futures_util::stream::unfold(download_rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        });
        stream
            .for_each_concurrent(6, |(client, url, k, dir)| async move {
                if let Err(e) = crate::crm::downloader::download_csv(&client, &url, &k, &dir).await
                {
                    error!("Download failed for {}: {:#}", k, e);
                }
            })
            .await;
    });

    let defs = report_defs();

    let should_fetch = |key: &str| -> bool { report_type.iter().any(|r| r == "all" || r == key) };

    // Build task list using Futures so we can process them with buffer_unordered
    let mut futures = Vec::new();

    let context = Arc::new(FetchContext {
        config: config_mutex.clone(),
    });

    let (
        base_url,
        email,
        account_id,
        application_id,
        tz,
        calls_from_date,
        to_date,
        from_date,
        download_csv,
        incomplete_reservation_limit,
        initial_token,
    ) = {
        let cfg = config_mutex.lock().await;
        let t = if !cfg.id_token.is_empty() {
            cfg.id_token.clone()
        } else {
            cfg.access_token.clone()
        };
        (
            cfg.base_url.clone(),
            cfg.email.clone(),
            cfg.account_id.clone(),
            cfg.application_id.clone(),
            cfg.app_timezone_plus_minutes.clone(),
            cfg.calls_from_date.clone(),
            cfg.to_date.clone(),
            cfg.from_date.clone(),
            cfg.download_csv,
            cfg.incomplete_reservation_limit,
            t,
        )
    };
    let token = initial_token;

    for def in defs {
        trace!("Checking report definition: {}", def.key);
        if !should_fetch(def.key) {
            trace!("Skipping report '{}' per report_type filter", def.key);
            continue;
        }

        let prefix = match def.key {
            "tickets" => "ticket_report_",
            "calls" => "call_logs_",
            "leads" => "lead_report_",
            "users" => "user_report_",
            _ => "",
        };

        if !prefix.is_empty() && has_recent_download(download_dir, prefix).await {
            info!(
                "Skipping fetch for '{}': A recent file (<30s old) already exists in Downloads",
                def.key
            );
            continue;
        }

        trace!("Preparing to fetch report: {}", def.key);
        let endpoint = def.endpoint;
        let extra = def.extra_params;

        if def.key == "calls" {
            // Call logs: split into monthly batches
            let batches = split_monthly(&calls_from_date, &to_date)?;
            info!(
                "Call logs: {} monthly batches from {} to {}",
                batches.len(),
                calls_from_date,
                to_date
            );

            for (batch_from, batch_to) in batches {
                let client = client.clone();
                let context = Arc::clone(&context);
                let base_url = base_url.clone();
                let _email = email.clone();
                let account_id = account_id.clone();
                let application_id = application_id.clone();
                let tz = tz.clone();
                let token = token.clone();

                let download_dir = download_dir.to_path_buf();
                let download_tx = download_tx.clone();

                futures.push(
                    async move {
                        let key = format!("calls_{}_{}", batch_from, batch_to);
                        let params = FetchParams {
                            base_url: &base_url,
                            email: &_email,
                            account_id: &account_id,
                            application_id: &application_id,
                            tz: &tz,
                            extra_params: extra,
                        };
                        let v = fetch_with_signed_url_split(
                            &client,
                            &token,
                            endpoint,
                            &batch_from,
                            &batch_to,
                            &params,
                            download_csv,
                            Some(&download_dir),
                            &key,
                            download_tx,
                            Some(context.clone()),
                        )
                        .await
                        .unwrap_or_else(|e| {
                            error!("Call log batch {}-{} failed: {}", batch_from, batch_to, e);
                            serde_json::json!({"error": format!("{}", e)})
                        });

                        (key, v)
                    }
                    .boxed(),
                );
            }
        } else if def.key == "users" {
            // Users report: direct GET request, no dates, returns Base64 CSV
            let client = client.clone();
            let context = Arc::clone(&context);
            let base_url = base_url.clone();
            let _email = email.clone();
            let account_id = account_id.clone();
            let application_id = application_id.clone();
            let tz = tz.clone();
            let token = token.clone();
            let key = def.key.to_string();

            let download_dir = download_dir.to_path_buf();

            futures.push(
                async move {
                    let params = FetchParams {
                        base_url: &base_url,
                        email: &_email,
                        account_id: &account_id,
                        application_id: &application_id,
                        tz: &tz,
                        extra_params: extra,
                    };

                    let v = fetch_users_report(
                        &client,
                        &token,
                        endpoint,
                        &params,
                        Some(context.clone()),
                    )
                    .await
                    .unwrap_or_else(|e| {
                        error!("Report '{}' failed: {}", endpoint, e);
                        serde_json::json!({"error": format!("{}", e)})
                    });

                    if download_csv {
                        if let Some(base64_val) = v.get("base64_data").and_then(|b| b.as_str()) {
                            if let Err(e) = crate::crm::downloader::process_base64_payload(
                                base64_val,
                                &key,
                                &download_dir,
                            )
                            .await
                            {
                                error!("Failed to process {} Base64 payload: {:#}", key, e);
                            }
                        }
                    }

                    (key, v)
                }
                .boxed(),
            );
        } else if def.key == "incomplete_reservation" {
            // Incomplete Reservation task: fetches tickets and bulk-updates them
            let client = client.clone();
            let context = Arc::clone(&context);
            let base_url = base_url.clone();
            let _email = email.clone();
            let account_id = account_id.clone();
            let application_id = application_id.clone();
            let tz = tz.clone();
            let token = token.clone();
            let key = def.key.to_string();
            let limit = incomplete_reservation_limit;
            let batch_size = {
                let cfg = context.config.lock().await;
                cfg.incomplete_reservation_batch_size
            };

            futures.push(
                async move {
                    let v = fetch_and_update_incomplete_reservations(
                        &client,
                        &token,
                        &base_url,
                        &account_id,
                        &application_id,
                        &tz,
                        limit,
                        batch_size,
                        Some(context.clone()),
                    )
                    .await
                    .unwrap_or_else(|e| {
                        error!("Report '{}' failed: {}", key, e);
                        serde_json::json!({"error": format!("{}", e)})
                    });
                    (key, v)
                }
                .boxed(),
            );
        } else {
            // Tickets / Leads: try the full range first, then split if the
            // backend refuses to generate a signed URL for a large file.
            let client = client.clone();
            let context = Arc::clone(&context);
            let base_url = base_url.clone();
            let _email = email.clone();
            let account_id = account_id.clone();
            let application_id = application_id.clone();
            let tz = tz.clone();
            let token = token.clone();
            let from_date = from_date.clone();
            let to_date = to_date.clone();
            let key = def.key.to_string();

            let download_dir = download_dir.to_path_buf();
            let download_tx = download_tx.clone();

            futures.push(
                async move {
                    let params = FetchParams {
                        base_url: &base_url,
                        email: &_email,
                        account_id: &account_id,
                        application_id: &application_id,
                        tz: &tz,
                        extra_params: extra,
                    };
                    let v = fetch_with_signed_url_split(
                        &client,
                        &token,
                        endpoint,
                        &from_date,
                        &to_date,
                        &params,
                        download_csv,
                        Some(&download_dir),
                        &key,
                        download_tx,
                        Some(context.clone()),
                    )
                    .await
                    .unwrap_or_else(|e| {
                        error!("Report '{}' failed: {}", endpoint, e);
                        serde_json::json!({"error": format!("{}", e)})
                    });

                    (key, v)
                }
                .boxed(),
            );
        }
    }

    // Drop the sender so the download_processor can finish when all downloads are queued
    drop(download_tx);

    // Process up to 4 concurrent top-level fetch streams
    let task_results: Vec<(String, Value)> = futures_util::stream::iter(futures)
        .buffer_unordered(4)
        .collect()
        .await;

    // Await all downloads to complete
    let _ = download_processor.await;

    // Assemble results
    let mut calls_array: Vec<Value> = Vec::new();

    for (key, value) in task_results {
        if key.starts_with("calls_") {
            calls_array.push(value);
        } else {
            results.insert(key, value);
        }
    }

    if !calls_array.is_empty() || should_fetch("calls") {
        results.insert("calls".into(), Value::Array(calls_array));
    }

    Ok(Value::Object(results))
}

async fn fetch_users_report(
    client: &reqwest::Client,
    token: &str,
    endpoint: &str,
    params: &FetchParams<'_>,
    context_opt: Option<Arc<FetchContext>>,
) -> Result<Value> {
    let url = format!("{}/{}", params.base_url.trim_end_matches('/'), endpoint);

    info!("Fetching {} ...", endpoint);
    let start_time = std::time::Instant::now();

    let mut current_token = token.to_string();
    let mut resp = client
        .get(&url)
        .header("account_id", params.account_id)
        .header("app-timezone-plus-minutes", params.tz)
        .header("application_id", params.application_id)
        .header("auth-type", "cognito")
        .header("authorization", format!("Bearer {}", current_token))
        .header("content-type", "application/json")
        .header("accept", "*/*")
        .send()
        .await
        .with_context(|| format!("HTTP request to {} failed", endpoint))?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        if let Some(ctx) = context_opt.as_ref() {
            let new_token = ctx
                .get_valid_token_or_refresh(client, &current_token)
                .await?;
            current_token = new_token;
            resp = client
                .get(&url)
                .header("account_id", params.account_id)
                .header("app-timezone-plus-minutes", params.tz)
                .header("application_id", params.application_id)
                .header("auth-type", "cognito")
                .header("authorization", format!("Bearer {}", current_token))
                .header("content-type", "application/json")
                .header("accept", "*/*")
                .send()
                .await
                .with_context(|| format!("Retry HTTP request to {} failed", endpoint))?;
        }
    }
    let status = resp.status();
    let body = resp
        .text()
        .await
        .with_context(|| format!("Failed to read response body from {}", endpoint))?;

    if !status.is_success() {
        anyhow::bail!("{} HTTP {}: {}", endpoint, status, body);
    }

    let duration = start_time.elapsed();
    info!("Request to {} completed in {:?}", endpoint, duration);

    if body.is_empty() {
        anyhow::bail!("{} HTTP response is empty", endpoint);
    }

    // The endpoint returns Base64 encoded payload directly.
    Ok(serde_json::json!({
        "base64_data": body
    }))
}

#[allow(clippy::too_many_arguments)]
async fn fetch_and_update_incomplete_reservations(
    client: &reqwest::Client,
    token: &str,
    base_url: &str,
    account_id: &str,
    application_id: &str,
    tz: &str,
    limit: u32,
    batch_size: usize,
    context_opt: Option<Arc<FetchContext>>,
) -> Result<Value> {
    let mut all_ids = Vec::new();
    let status_ids = vec![
        "f38fafc3-bd3f-4dd0-ac6a-09dd7d51b6a0",
        "275688a4-df41-4725-ae56-fd2deab9c1c9",
    ];
    let batch_size_u32 = batch_size as u32;

    for status_id in status_ids {
        let endpoint = "task/ticket";
        let mut offset = 0;
        let mut current_token = token.to_string();

        let mut status_done = false;
        loop {
            if all_ids.len() as u32 >= limit {
                info!(
                    "Reached total limit of {} for incomplete reservations.",
                    limit
                );
                status_done = true;
                break;
            }

            let mut url = format!(
                "{}{}?offset={}&limit={}&status_id={}&event_name=cc_booking_incomplete_reservation",
                base_url, endpoint, offset, batch_size_u32, status_id
            );
            if status_id == "f38fafc3-bd3f-4dd0-ac6a-09dd7d51b6a0" {
                url.push_str("&sort_column=follow_up_date&sort_val=ASC");
            }

            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("accept", reqwest::header::HeaderValue::from_static("*/*"));
            headers.insert(
                "account_id",
                reqwest::header::HeaderValue::from_str(account_id)?,
            );
            headers.insert(
                "application_id",
                reqwest::header::HeaderValue::from_str(application_id)?,
            );
            headers.insert(
                "app-timezone-plus-minutes",
                reqwest::header::HeaderValue::from_str(tz)?,
            );
            headers.insert(
                "auth-type",
                reqwest::header::HeaderValue::from_static("cognito"),
            );
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", current_token))?,
            );
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            );

            debug!(
                "GET {} - headers: {}",
                url,
                format_redacted_headers(&headers)
            );

            let mut res = client.get(&url).headers(headers.clone()).send().await?;

            if res.status() == reqwest::StatusCode::UNAUTHORIZED {
                if let Some(ctx) = context_opt.as_ref() {
                    let new_token = ctx
                        .get_valid_token_or_refresh(client, &current_token)
                        .await?;
                    current_token = new_token;
                    let mut retry_headers = headers.clone();
                    retry_headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!(
                            "Bearer {}",
                            current_token
                        ))?,
                    );
                    res = client.get(&url).headers(retry_headers).send().await?;
                }
            }

            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                anyhow::bail!("GET {} failed with status {}: {}", url, status, body);
            }

            let text: String = res.text().await.unwrap_or_default();
            let parsed: IncompleteReservationResponse = serde_json::from_str(&text)
                .with_context(|| format!("Failed to parse response: {}", text))?;

            let fetched_count = parsed.data.items.len();
            for item in parsed.data.items {
                all_ids.push(item.id);
            }

            if fetched_count < batch_size {
                break;
            }

            offset += batch_size_u32;
        }

        if status_done {
            break;
        }
    }

    if all_ids.is_empty() {
        info!("No incomplete reservations found. Skipping PATCH request.");
        return Ok(serde_json::json!({
            "statusCode": 200,
            "message": "success - no items to update"
        }));
    }

    info!("Found {} incomplete reservations to update.", all_ids.len());

    let patch_endpoint = "task/bulk-ticket";
    let patch_url = format!("{}{}", base_url, patch_endpoint);

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("accept", reqwest::header::HeaderValue::from_static("*/*"));
    headers.insert(
        "account_id",
        reqwest::header::HeaderValue::from_str(account_id)?,
    );
    headers.insert(
        "application_id",
        reqwest::header::HeaderValue::from_str(application_id)?,
    );
    headers.insert(
        "app-timezone-plus-minutes",
        reqwest::header::HeaderValue::from_str(tz)?,
    );
    headers.insert(
        "auth-type",
        reqwest::header::HeaderValue::from_static("cognito"),
    );
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );

    let mut current_token = token.to_string();
    let mut last_parsed = Value::Null;

    for chunk in all_ids.chunks(batch_size) {
        let payload = BulkTicketPayload {
            status_id: "46282444-7951-42eb-a27e-b2bc65c53727",
            ids: chunk.to_vec(),
        };

        let payload_str = serde_json::to_string(&payload)?;

        debug!(
            "PATCH {} (chunk size {}) - headers: {}, body: {}",
            patch_url,
            chunk.len(),
            format_redacted_headers(&headers),
            payload_str
        );

        let mut res = client
            .patch(&patch_url)
            .headers(headers.clone())
            .body(payload_str.clone())
            .send()
            .await?;

        if res.status() == reqwest::StatusCode::UNAUTHORIZED {
            if let Some(ctx) = context_opt.as_ref() {
                let new_token = ctx
                    .get_valid_token_or_refresh(client, &current_token)
                    .await?;
                current_token = new_token;
                let mut retry_headers = headers.clone();
                retry_headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", current_token))?,
                );
                // Also update the loop headers for future chunks
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", current_token))?,
                );
                res = client
                    .patch(&patch_url)
                    .headers(retry_headers)
                    .body(payload_str)
                    .send()
                    .await?;
            }
        }

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!(
                "PATCH {} failed with status {}: {}",
                patch_url,
                status,
                body
            );
        }

        let text = res.text().await?;
        let parsed: Value = serde_json::from_str(&text)
            .with_context(|| format!("Failed to parse response: {}", text))?;
        last_parsed = parsed;
    }

    Ok(last_parsed)
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

use futures_util::future::BoxFuture;
use futures_util::stream::StreamExt;
use futures_util::FutureExt;

#[allow(clippy::too_many_arguments)]
async fn fetch_with_signed_url_split(
    client: &reqwest::Client,
    token: &str,
    endpoint: &str,
    from_date: &str,
    to_date: &str,
    params: &FetchParams<'_>,
    download_csv: bool,
    download_dir: Option<&Path>,
    key_prefix: &str,
    download_tx: tokio::sync::mpsc::UnboundedSender<(
        reqwest::Client,
        String,
        String,
        std::path::PathBuf,
    )>,
    context_opt: Option<Arc<FetchContext>>,
) -> Result<Value> {
    let mut completed = fetch_recursive(
        client.clone(),
        token.to_string(),
        endpoint.to_string(),
        from_date.to_string(),
        to_date.to_string(),
        params.base_url.to_string(),
        params.email.to_string(),
        params.account_id.to_string(),
        params.application_id.to_string(),
        params.tz.to_string(),
        params
            .extra_params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        download_csv,
        download_dir.map(|d| d.to_path_buf()),
        key_prefix.to_string(),
        download_tx,
        context_opt.clone(),
    )
    .await?;

    completed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    if completed.len() > 1 {
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

#[allow(clippy::too_many_arguments)]
fn fetch_recursive(
    client: reqwest::Client,
    token: String,
    endpoint: String,
    from_date: String,
    to_date: String,
    base_url: String,
    email: String,
    account_id: String,
    application_id: String,
    tz: String,
    extra_params: Vec<(String, String)>,
    download_csv: bool,
    download_dir: Option<std::path::PathBuf>,
    key_prefix: String,
    download_tx: tokio::sync::mpsc::UnboundedSender<(
        reqwest::Client,
        String,
        String,
        std::path::PathBuf,
    )>,
    context_opt: Option<Arc<FetchContext>>,
) -> BoxFuture<'static, Result<Vec<(String, String, Value)>>> {
    async move {
        let ep_refs: Vec<(&str, &str)> = extra_params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let params = FetchParams {
            base_url: &base_url,
            email: &email,
            account_id: &account_id,
            application_id: &application_id,
            tz: &tz,
            extra_params: &ep_refs,
        };

        let result = fetch_single(&client, &token, &endpoint, &from_date, &to_date, &params, context_opt.clone()).await;

        match result {
            Ok(value) => {
                if download_csv {
                    if let Some(dir) = &download_dir {
                        let mut urls = Vec::new();
                        extract_urls_for_key(&key_prefix, &value, &mut urls);
                        for (k, url) in urls {
                            let _ = download_tx.send((client.clone(), url, k, dir.clone()));
                        }
                    }
                }

                Ok(vec![(from_date, to_date, value)])
            }
            Err(err) if is_signed_url_generation_failure(&err) => {
                if let Some((left, right)) = split_range_in_half(&from_date, &to_date)? {
                    info!(
                        "{} [{} to {}] failed to generate signed URL; retrying concurrently as [{} to {}] and [{} to {}]",
                        endpoint, from_date, to_date, left.0, left.1, right.0, right.1
                    );

                    let left_fut = fetch_recursive(
                        client.clone(),
                        token.clone(),
                        endpoint.clone(),
                        left.0.clone(),
                        left.1.clone(),
                        base_url.clone(),
                        email.clone(),
                        account_id.clone(),
                        application_id.clone(),
                        tz.clone(),
                        extra_params.clone(),
                        download_csv,
                        download_dir.clone(),
                        key_prefix.clone(),
                        download_tx.clone(),
                        context_opt.clone(),
                    );

                    let right_fut = fetch_recursive(
                        client,
                        token,
                        endpoint,
                        right.0.clone(),
                        right.1.clone(),
                        base_url,
                        email,
                        account_id,
                        application_id,
                        tz,
                        extra_params,
                        download_csv,
                        download_dir,
                        key_prefix,
                        download_tx,
                        context_opt.clone(),
                    );

                    // Concurrently fetch both halves using tokio::spawn for true parallel execution
                    let left_handle = tokio::spawn(left_fut);
                    let right_handle = tokio::spawn(right_fut);

                    let (left_res, right_res) = tokio::join!(left_handle, right_handle);

                    let mut combined = left_res.context("Left half task panicked")??;
                    combined.extend(right_res.context("Right half task panicked")??);

                    Ok(combined)
                } else {
                    Err(err).with_context(|| {
                        format!(
                            "{} failed to generate a signed URL for single-day range {}",
                            endpoint, from_date
                        )
                    })
                }
            }
            Err(err) => Err(err),
        }
    }
    .boxed()
}

async fn fetch_single(
    client: &reqwest::Client,
    token: &str,
    endpoint: &str,
    from_date: &str,
    to_date: &str,
    params: &FetchParams<'_>,
    context_opt: Option<Arc<FetchContext>>,
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

    let mut current_token = token.to_string();
    let mut resp = client
        .get(&url)
        .query(&query)
        .header("account_id", params.account_id)
        .header("app-timezone-plus-minutes", params.tz)
        .header("application_id", params.application_id)
        .header("auth-type", "cognito")
        .header("authorization", format!("Bearer {}", current_token))
        .header("content-type", "application/json")
        .header("accept", "*/*")
        .send()
        .await
        .with_context(|| format!("HTTP request to {} failed", endpoint))?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        if let Some(ctx) = context_opt.as_ref() {
            let new_token = ctx
                .get_valid_token_or_refresh(client, &current_token)
                .await?;
            current_token = new_token;
            resp = client
                .get(&url)
                .query(&query)
                .header("account_id", params.account_id)
                .header("app-timezone-plus-minutes", params.tz)
                .header("application_id", params.application_id)
                .header("auth-type", "cognito")
                .header("authorization", format!("Bearer {}", current_token))
                .header("content-type", "application/json")
                .header("accept", "*/*")
                .send()
                .await
                .with_context(|| format!("Retry HTTP request to {} failed", endpoint))?;
        }
    }
    let status = resp.status();
    let headers = format_redacted_headers(resp.headers());
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
        let err_msg = format!("{} returned HTTP {}: {}", endpoint, status, body);
        let err = anyhow::anyhow!("{}", err_msg);
        if is_signed_url_generation_failure(&err) {
            info!("{} failed with HTTP {}: {}", endpoint, status, body);
        } else {
            error!("{} failed with HTTP {}: {}", endpoint, status, body);
        }
        return Err(err);
    }

    trace!("Parsing JSON response from {}...", endpoint);
    let parsed: Value =
        serde_json::from_str(&body).with_context(|| format!("Invalid JSON from {}", endpoint))?;
    trace!("Successfully parsed JSON from {}.", endpoint);
    Ok(parsed)
}

fn is_signed_url_generation_failure(err: &anyhow::Error) -> bool {
    err.to_string()
        .to_ascii_lowercase()
        .contains("failed to generate signed url")
}

type DateRange = (String, String);

fn split_range_in_half(from: &str, to: &str) -> Result<Option<(DateRange, DateRange)>> {
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
        let month_end = last_day_of_month(cursor.year(), cursor.month())
            .context("Failed to calculate the last day of the month")?;

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
fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1)?.pred_opt()
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
    let mut stack = vec![val];
    while let Some(current) = stack.pop() {
        if let Some(url) = extract_data_url(current) {
            urls.push((key.to_string(), url));
            continue;
        }

        if let Value::Array(arr) = current {
            for item in arr.iter().rev() {
                if item.is_object() || item.is_array() {
                    stack.push(item);
                }
            }
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
// Redacted Headers
// ──────────────────────────────────────────────────────────────

fn format_redacted_headers(headers: &reqwest::header::HeaderMap) -> String {
    let mut s = String::new();
    s.push('{');
    let mut first = true;
    for (name, value) in headers.iter() {
        if !first {
            s.push_str(", ");
        }
        first = false;

        let name_str = name.as_str();
        s.push('"');
        s.push_str(name_str);
        s.push_str("\": ");

        match name_str.to_ascii_lowercase().as_str() {
            "authorization" | "set-cookie" | "cookie" => {
                s.push_str("\"<REDACTED>\"");
            }
            _ => {
                // Safely convert header value to a string, or format it as opaque bytes
                if let Ok(v) = value.to_str() {
                    s.push('"');
                    s.push_str(v);
                    s.push('"');
                } else {
                    write!(&mut s, "{:?}", value.as_bytes()).unwrap_or_default();
                }
            }
        }
    }
    s.push('}');
    s
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

async fn has_recent_download(download_dir: &Path, prefix: &str) -> bool {
    let threshold = SystemTime::now() - std::time::Duration::from_secs(30);

    if let Ok(mut entries) = fs::read_dir(download_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with(prefix) && name.ends_with(".csv") {
                            if let Ok(modified) = metadata.modified() {
                                if modified >= threshold {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

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
            NaiveDate::from_ymd_opt(2024, 2, 29)
        );
        assert_eq!(
            last_day_of_month(2025, 2),
            NaiveDate::from_ymd_opt(2025, 2, 28)
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

    #[test]
    fn test_format_redacted_headers() {
        use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Bearer super_secret_token"),
        );
        headers.insert(
            HeaderName::from_static("set-cookie"),
            HeaderValue::from_static("session_id=12345; Secure; HttpOnly"),
        );
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            HeaderName::from_static("x-api-version"),
            HeaderValue::from_static("v1"),
        );

        let redacted = format_redacted_headers(&headers);

        // Sensitive headers should be redacted
        assert!(redacted.contains("\"authorization\": \"<REDACTED>\""));
        assert!(redacted.contains("\"set-cookie\": \"<REDACTED>\""));

        // Non-sensitive headers should be present and unredacted
        assert!(redacted.contains("\"content-type\": \"application/json\""));
        assert!(redacted.contains("\"x-api-version\": \"v1\""));

        // Verify the sensitive values are NOT present anywhere in the string
        assert!(!redacted.contains("super_secret_token"));
        assert!(!redacted.contains("session_id=12345"));
    }
}

#[test]
fn test_is_signed_url_generation_failure() {
    let err = anyhow::anyhow!("500 Internal Server Error: Failed to generate signed url");
    assert!(is_signed_url_generation_failure(&err));

    let err2 = anyhow::anyhow!("Some other error");
    assert!(!is_signed_url_generation_failure(&err2));
}
