with open('src/crm/mod.rs', 'r') as f:
    content = f.read()

# We need to compute download_dir before calling fetch_reports.
# We will extract download_dir computation from below and place it above fetch_reports.

old_block = """    let results = fetcher::fetch_reports(&config, &client, &token, report).await?;

    if config.download_csv {
        let urls = fetcher::extract_urls(&results);
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let download_dir = exe_dir.join("Downloads");
        tokio::fs::create_dir_all(&download_dir).await?;

        let download_futures = urls.iter().map(|(key, url)| {"""

new_block = """    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let download_dir = exe_dir.join("Downloads");

    let results = fetcher::fetch_reports(&config, &client, &token, report, &download_dir).await?;

    if config.download_csv {
        let urls = fetcher::extract_urls(&results);
        tokio::fs::create_dir_all(&download_dir).await?;

        let download_futures = urls.iter().map(|(key, url)| {"""

content = content.replace(old_block, new_block)

with open('src/crm/mod.rs', 'w') as f:
    f.write(content)
