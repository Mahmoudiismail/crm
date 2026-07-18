import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "let current_month = chrono::Local::now()" in line or "dataset.month" in line:
        pass

print("Searching for month manipulation...")
for i, line in enumerate(lines):
    if "month" in line.lower() and "filter" in line.lower():
        print(f"{i+1}: {line}")
