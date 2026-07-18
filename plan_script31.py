with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "Extracting data for Branch:" in line:
        for j in range(max(0, i-50), min(len(lines), i+80)):
            print(f"{j+1}: {lines[j]}")
        break
