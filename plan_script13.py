with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "let target_sheet_name" in line:
        pass
print("Finding logic for table styling and output")
