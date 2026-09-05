import re

# Fix email client
with open("src/tasker/email/client.rs", "r") as f:
    code = f.read()

code = code.replace("use anyhow::{Context, Result};\nuse crate::tasker::utils::with_retry;", "use anyhow::{Context, Result};")
code = code.replace("use anyhow::{Context, Result};", "use anyhow::{Context, Result};\nuse crate::tasker::utils::with_retry;")

with open("src/tasker/email/client.rs", "w") as f:
    f.write(code)

# Fix unused imports
with open("src/tasker/csv_task/mod.rs", "r") as f:
    code = f.read()
code = code.replace("use crate::tasker::utils::with_retry;\n", "")
code = code.replace("use anyhow::{Context, Result};", "use anyhow::{Context, Result};\nuse crate::tasker::utils::with_retry;")
with open("src/tasker/csv_task/mod.rs", "w") as f:
    f.write(code)

with open("src/tasker/department_split.rs", "r") as f:
    code = f.read()
code = code.replace("use crate::tasker::utils::with_retry;\n", "")
code = code.replace("use anyhow::{Context, Result};", "use anyhow::{Context, Result};\nuse crate::tasker::utils::with_retry;")
with open("src/tasker/department_split.rs", "w") as f:
    f.write(code)
