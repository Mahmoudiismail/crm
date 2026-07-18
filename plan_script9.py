import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "Get-SlicerItems" in line:
        print(f"Found Get-SlicerItems at line {i+1}")
        for j in range(max(0, i-5), min(len(lines), i+150)):
            print(f"{j+1}: {lines[j]}")
        break
