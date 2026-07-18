import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "let extracted_data" in line:
        print(f"Found extracted_data at line {i+1}")
        for j in range(max(0, i-10), min(len(lines), i+30)):
            print(f"{j+1}: {lines[j]}")
        break
