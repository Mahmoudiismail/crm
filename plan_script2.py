import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "final_datasets.push(EnrichedDataset" in line:
        print(f"Found final_datasets.push at line {i+1}")
        for j in range(max(0, i-50), min(len(lines), i+20)):
            print(f"{j+1}: {lines[j]}")
        break
