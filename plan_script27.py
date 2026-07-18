with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "let mut team_to_owner" in line:
        for j in range(max(0, i-5), min(len(lines), i+40)):
            print(f"{j+1}: {lines[j]}")
        break
