with open("md/TASKER.md", "r") as f:
    content = f.read()

import re

search = r"""\*\*Team Grouping:\*\* Teams are grouped case-insensitively using Title Case formatting to ensure consistency \(e.g. "support" and "SUPPORT" are correctly merged\)."""
replace = r"""**Team Grouping:** Teams are grouped case-insensitively using Title Case formatting to ensure consistency (e.g. "support" and "SUPPORT" are correctly merged).
* **CRM Open Sohail Grouping & Styling:** Modified Slicer extraction logic to query "All Months Except Current" and "Current Month" directly from PowerShell. Executive Clinic extracts all months combined. The HTML styling uses a fixed layout with configurable column widths, 5px padding, center alignment, and no background color for data rows."""

content = re.sub(search, replace, content)

with open("md/TASKER.md", "w") as f:
    f.write(content)
