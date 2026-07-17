with open("src/crm/fetcher.rs", "r") as f:
    code = f.read()

import re

search = """
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
"""

replace = """
#[allow(clippy::too_many_arguments)]
fn fetch_recursive<'a>(
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
    extra_params: Vec<(&'a str, &'a str)>,
    download_csv: bool,
    download_dir: Option<std::path::PathBuf>,
    key_prefix: String,
) -> BoxFuture<'a, Result<Vec<(String, String, Value)>>> {
"""

if search.strip() in code.strip():
    code = code.replace(search.strip(), replace.strip())
    with open("src/crm/fetcher.rs", "w") as f:
        f.write(code)
    print("Replaced successfully!")
else:
    print("Search string not found")
