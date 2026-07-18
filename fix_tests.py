import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

# Fix the two missing fields in tests.
content = content.replace("dashboard_pivot_name: None,", "dashboard_pivot_name: None,\ntable_column_widths: None,")

with open("src/tasker/crm_open_sohail.rs", "w") as f:
    f.write(content)
