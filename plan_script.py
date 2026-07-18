import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

# Let's inspect the `let branch_filter_ps = ...` section
lines = content.split('\n')
for i, line in enumerate(lines):
    if "let branch_filter_ps =" in line:
        print(f"Found branch_filter_ps at line {i+1}")
        for j in range(i, i+30):
            print(f"{j+1}: {lines[j]}")
        break
