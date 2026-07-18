import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

search_pattern = r"""        // Start Table
        sections_html.push_str\("<table style=\\"border-collapse: collapse; font-family: Calibri, sans-serif; font-size: 14px; border: 1px solid #8EA9DB; margin-bottom: 20px;\\">"\);"""

replace_pattern = r"""        // Start Table
        sections_html.push_str("<table style=\"table-layout: fixed; width: 100%; border-collapse: collapse; font-family: Calibri, sans-serif; font-size: 14px; border: 1px solid #8EA9DB; margin-bottom: 20px;\">");"""

content = re.sub(search_pattern, replace_pattern, content)

with open("src/tasker/crm_open_sohail.rs", "w") as f:
    f.write(content)
