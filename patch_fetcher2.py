import re
with open("src/crm/fetcher.rs", "r") as f:
    code = f.read()

code = code.replace("use futures_util::FutureExt;", "use futures_util::FutureExt;\nuse futures_util::TryStreamExt;")

with open("src/crm/fetcher.rs", "w") as f:
    f.write(code)
