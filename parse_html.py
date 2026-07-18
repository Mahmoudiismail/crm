with open("src/tasker/crm_open_sohail.rs") as f:
    for i, line in enumerate(f):
        if "<th " in line or "<td " in line:
            print(f"Line {i+1}: {line.strip()}")
