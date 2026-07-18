with open("src/tasker/config.rs", "r") as f:
    content = f.read()

lines = content.split('\n')
for i, line in enumerate(lines):
    if "pub struct CrmOpenSohailConfig" in line:
        for j in range(max(0, i-5), min(len(lines), i+20)):
            print(f"{j+1}: {lines[j]}")
        break
