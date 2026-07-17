with open("md/AI_DOC_POLICY.md", "r") as f:
    content = f.read()

content = content.replace(
    "## Recent Fixes",
    "## Recent Fixes\n- **Concurrent API Fetching:** Modified `fetch_with_signed_url_split` in `crm/fetcher.rs` to concurrently fetch split date ranges using a recursive boxed future approach (`fetch_recursive` with `tokio::join!`). This fixes the issue where split fetches were being executed sequentially."
)

with open("md/AI_DOC_POLICY.md", "w") as f:
    f.write(content)
print("Replaced successfully in AI_DOC_POLICY.md!")
