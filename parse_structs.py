import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

def find_structs(content):
    matches = re.finditer(r'(pub\s+)?struct\s+(\w+)\s*\{', content)
    for m in matches:
        print(f"Struct found: {m.group(2)}")

find_structs(content)
