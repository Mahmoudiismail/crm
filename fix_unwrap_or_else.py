import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

search = r"""        .unwrap_or_else\(\|\| "CRM Updated open TKTs".to_string\(\)\);"""
replace = r"""        .unwrap_or("CRM Updated open TKTs".to_string());"""

content = re.sub(search, replace, content)

with open("src/tasker/crm_open_sohail.rs", "w") as f:
    f.write(content)
