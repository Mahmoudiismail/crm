import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "let mut ds_closed_total = 0.0;" in line:
        pass
print("Finding logic for table styling and output")
