with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

# Before the HTML generation loop over final_datasets, we need to manipulate the data to group branches correctly.

lines = content.split('\n')
for i, line in enumerate(lines):
    if "let mut sections_html = String::new();" in line:
        for j in range(max(0, i-25), min(len(lines), i+15)):
            print(f"{j+1}: {lines[j]}")
        break
