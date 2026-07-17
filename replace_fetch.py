with open("src/crm/fetcher.rs", "r") as f:
    code = f.read()

import re

search = """
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
) -> Result<Value> {
    let mut pending = vec![(from_date.to_string(), to_date.to_string())];
    let mut completed: Vec<(String, String, Value)> = Vec::new();
    let mut split_used = false;

    let mut download_tasks = Vec::new();

    while let Some((batch_from, batch_to)) = pending.pop() {
        let result = fetch_single(client, token, endpoint, &batch_from, &batch_to, params).await;

        match result {
            Ok(value) => {
                if download_csv {
                    if let Some(dir) = download_dir {
                        let mut urls = Vec::new();
                        extract_urls_for_key(key_prefix, &value, &mut urls);
                        for (k, url) in urls {
                            let client_clone = client.clone();
                            let dir_clone = dir.to_path_buf();
                            download_tasks.push(tokio::spawn(async move {
                                if let Err(e) = crate::crm::downloader::download_csv(
                                    &client_clone,
                                    &url,
                                    &k,
                                    &dir_clone,
                                )
                                .await
                                {
                                    error!("Download failed for {}: {:#}", k, e);
                                }
                            }));
                        }
                    }
                }
                completed.push((batch_from, batch_to, value))
            }
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

    join_all(download_tasks).await;

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
"""

replace = """
use futures_util::future::BoxFuture;
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
        params.extra_params.to_vec(),
        download_csv,
        download_dir.map(|d| d.to_path_buf()),
        key_prefix.to_string(),
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
    extra_params: Vec<(&'static str, &'static str)>,
    download_csv: bool,
    download_dir: Option<std::path::PathBuf>,
    key_prefix: String,
) -> BoxFuture<'static, Result<Vec<(String, String, Value)>>> {
    async move {
        let params = FetchParams {
            base_url: &base_url,
            email: &email,
            account_id: &account_id,
            application_id: &application_id,
            tz: &tz,
            extra_params: &extra_params,
        };

        let result = fetch_single(&client, &token, &endpoint, &from_date, &to_date, &params).await;

        match result {
            Ok(value) => {
                let mut download_tasks = Vec::new();
                if download_csv {
                    if let Some(dir) = &download_dir {
                        let mut urls = Vec::new();
                        extract_urls_for_key(&key_prefix, &value, &mut urls);
                        for (k, url) in urls {
                            let client_clone = client.clone();
                            let dir_clone = dir.clone();
                            download_tasks.push(tokio::spawn(async move {
                                if let Err(e) = crate::crm::downloader::download_csv(
                                    &client_clone,
                                    &url,
                                    &k,
                                    &dir_clone,
                                )
                                .await
                                {
                                    error!("Download failed for {}: {:#}", k, e);
                                }
                            }));
                        }
                    }
                }

                // We want to return the result, but also we can just wait for downloads to finish here
                // if we want to ensure everything is downloaded before returning.
                // Wait, previously the downloads were awaited at the end of the split loop.
                // Let's await them here for this chunk.
                join_all(download_tasks).await;

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
                    );

                    // Concurrently fetch both halves
                    let (left_res, right_res) = tokio::join!(left_fut, right_fut);

                    let mut combined = left_res?;
                    combined.extend(right_res?);

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
"""

if search.strip() in code.strip():
    code = code.replace(search.strip(), replace.strip())
    with open("src/crm/fetcher.rs", "w") as f:
        f.write(code)
    print("Replaced successfully!")
else:
    print("Search string not found")
