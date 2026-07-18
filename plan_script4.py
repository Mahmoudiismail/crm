import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "final_datasets" in line and "push" not in line and "Vec::new" not in line and "len" not in line and "for dataset in &final_datasets" not in line:
        print(f"Found final_datasets at line {i+1}")
        for j in range(max(0, i-5), min(len(lines), i+5)):
            print(f"{j+1}: {lines[j]}")
