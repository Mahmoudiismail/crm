with open("src/tasker/crm_open_sohail.rs") as f:
    for i, line in enumerate(f):
        if "EnrichedDataset" in line:
            print(f"Line {i+1}: {line.strip()}")
