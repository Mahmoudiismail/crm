with open("md/FETCHER.md", "r") as f:
    content = f.read()

search = """
## Signed URL Failure Range Splitting

For `tickets`, `calls`, and `leads`, report fetches use `fetch_with_signed_url_split(...)`.

Behavior:

1. Try the requested date range first.
2. If the CRM API error text contains `Failed to generate signed url`, split that date range in half.
3. Retry each half, recursively splitting again when the same signed-URL error occurs.
4. Stop splitting at a single-day range; if that still fails, return the API error with single-day context.

Operational impact:

- Large ticket or lead exports can still be fetched when the backend refuses to sign one oversized CSV.
- Call logs are still pre-split monthly, and any failing monthly batch can be split further.
- A split report returns an array of successful API payloads instead of a single payload.
- Non-signed-URL failures are not retried by this splitter.
"""

replace = """
## Signed URL Failure Range Splitting

For `tickets`, `calls`, and `leads`, report fetches use `fetch_with_signed_url_split(...)`, which delegates to a concurrent recursive execution strategy (`fetch_recursive`).

Behavior:

1. Try the requested date range first.
2. If the CRM API error text contains `Failed to generate signed url`, split that date range in half.
3. Concurrently retry each half (using `tokio::join!`), recursively splitting again when the same signed-URL error occurs.
4. Stop splitting at a single-day range; if that still fails, return the API error with single-day context.

Operational impact:

- Large ticket or lead exports can still be fetched when the backend refuses to sign one oversized CSV.
- Fetch operations that fall back to date splits happen in parallel rather than sequentially, significantly accelerating the extraction of large data sets.
- Call logs are still pre-split monthly, and any failing monthly batch can be split further in parallel.
- A split report returns an array of successful API payloads instead of a single payload.
- Non-signed-URL failures are not retried by this splitter.
"""

if search.strip() in content.strip():
    content = content.replace(search.strip(), replace.strip())
    with open("md/FETCHER.md", "w") as f:
        f.write(content)
    print("Replaced successfully in FETCHER.md!")
else:
    print("Search string not found in FETCHER.md")
