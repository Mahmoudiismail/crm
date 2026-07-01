with open("md/ARCHITECTURE.md", "r") as f:
    content = f.read()
if "CrmFetch" in content:
    print("Found CrmFetch in ARCHITECTURE")
else:
    print("Clean ARCHITECTURE")

with open("md/SCHEDULER_TRAY.md", "r") as f:
    content = f.read()
if "CRM Fetch:" in content or "Yasweb:" in content:
    print("Found old terms in SCHEDULER_TRAY")
else:
    print("Clean SCHEDULER_TRAY")
