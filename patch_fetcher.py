import re
with open("src/crm/fetcher.rs", "r") as f:
    code = f.read()

struct_def = """struct DownloadTask {
    client: reqwest::Client,
    url: String,
    report_key: String,
    dir: std::path::PathBuf,
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
    context_opt: Option<Arc<FetchContext>>,
    key_prefix: String,
}

"""

code = code.replace("struct FetchParams<'a> {", struct_def + "struct FetchParams<'a> {")

old_chan_type = """    let (download_tx, download_rx) = tokio::sync::mpsc::unbounded_channel::<(
        reqwest::Client,
        String,
        String,
        std::path::PathBuf,
    )>();"""
new_chan_type = """    let (download_tx, download_rx) = tokio::sync::mpsc::unbounded_channel::<DownloadTask>();"""
code = code.replace(old_chan_type, new_chan_type)

old_processor = """    // Spawn a background task to process downloads concurrently (limit 6)
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
    });"""

new_processor = """    // Spawn a background task to process downloads concurrently (limit 6)
    let download_processor = tokio::spawn(async move {
        let stream = futures_util::stream::unfold(download_rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        });
        stream
            .map(Ok)
            .try_for_each_concurrent(6, |task: DownloadTask| async move {
                let current_url = task.url.clone();
                let mut download_success = false;

                for _attempt in 1..=3 {
                    if crate::crm::downloader::download_csv(&task.client, &current_url, &task.report_key, &task.dir).await.is_ok() {
                        download_success = true;
                        break;
                    }
                }

                if !download_success {
                    info!("Failed to download {} after 3 attempts, requesting fresh URL", task.report_key);
                    let ep_refs: Vec<(&str, &str)> = task.extra_params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                    let params = FetchParams {
                        base_url: &task.base_url,
                        email: &task.email,
                        account_id: &task.account_id,
                        application_id: &task.application_id,
                        tz: &task.tz,
                        extra_params: &ep_refs,
                    };

                    let value = fetch_single(&task.client, &task.token, &task.endpoint, &task.from_date, &task.to_date, &params, task.context_opt.clone()).await?;

                    let mut fresh_urls = Vec::new();
                    extract_urls_for_key(&task.key_prefix, &value, &mut fresh_urls);

                    let fresh_url = fresh_urls.into_iter().find(|(k, _)| k == &task.report_key).map(|(_, u)| u);

                    if let Some(url) = fresh_url {
                        let mut fresh_success = false;
                        for _attempt in 1..=3 {
                            if crate::crm::downloader::download_csv(&task.client, &url, &task.report_key, &task.dir).await.is_ok() {
                                fresh_success = true;
                                break;
                            }
                        }
                        if !fresh_success {
                            anyhow::bail!("Failed to download fresh URL for {}", task.report_key);
                        }
                    } else {
                        anyhow::bail!("Fresh URL did not contain {}", task.report_key);
                    }
                }

                Ok::<(), anyhow::Error>(())
            })
            .await
    });"""
code = code.replace(old_processor, new_processor)


old_await = """    // Await all downloads to complete
    let _ = download_processor.await;"""

new_await = """    // Await all downloads to complete
    let download_result = download_processor.await;
    match download_result {
        Ok(Err(e)) => anyhow::bail!("A download failed: {}", e),
        Err(e) => anyhow::bail!("Download processor panicked: {}", e),
        _ => {}
    }"""
code = code.replace(old_await, new_await)


old_arg = """    download_tx: tokio::sync::mpsc::UnboundedSender<(
        reqwest::Client,
        String,
        String,
        std::path::PathBuf,
    )>,"""
new_arg = """    download_tx: tokio::sync::mpsc::UnboundedSender<DownloadTask>,"""
code = code.replace(old_arg, new_arg)


old_send = """                        for (k, url) in urls {
                            let _ = download_tx.send((client.clone(), url, k, dir.clone()));
                        }"""
new_send = """                        for (k, url) in urls {
                            let task = DownloadTask {
                                client: client.clone(),
                                url,
                                report_key: k,
                                dir: dir.clone(),
                                token: token.clone(),
                                endpoint: endpoint.clone(),
                                from_date: from_date.clone(),
                                to_date: to_date.clone(),
                                base_url: base_url.clone(),
                                email: email.clone(),
                                account_id: account_id.clone(),
                                application_id: application_id.clone(),
                                tz: tz.clone(),
                                extra_params: extra_params.clone(),
                                context_opt: context_opt.clone(),
                                key_prefix: key_prefix.clone(),
                            };
                            let _ = download_tx.send(task);
                        }"""
code = code.replace(old_send, new_send)


with open("src/crm/fetcher.rs", "w") as f:
    f.write(code)
