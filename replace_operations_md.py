with open("md/OPERATIONS.md", "r") as f:
    content = f.read()

content = content.replace(
    "1. **Failures and Retries (CRM App)**",
    "1. **Failures and Retries (CRM App)**\n   - If a requested report date range is too large and the CRM API refuses to generate a signed URL (returning `Failed to generate signed url`), the CRM application will automatically split the date range in half and recursively retry both halves concurrently. This concurrent execution prevents bottlenecks on massive datasets."
)

with open("md/OPERATIONS.md", "w") as f:
    f.write(content)
print("Replaced successfully in OPERATIONS.md!")
