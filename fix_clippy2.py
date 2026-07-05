with open('src/crm/fetcher.rs', 'r') as f:
    lines = f.readlines()

# Find #[cfg(test)]
test_idx = -1
for i, line in enumerate(lines):
    if line.strip() == "#[cfg(test)]":
        test_idx = i
        break

# Find fn has_recent_download
fn_idx = -1
for i, line in enumerate(lines):
    if line.startswith("fn has_recent_download("):
        fn_idx = i
        break

if test_idx != -1 and fn_idx != -1 and fn_idx > test_idx:
    fn_lines = lines[fn_idx:]
    test_lines = lines[test_idx:fn_idx]
    before_test_lines = lines[:test_idx]

    new_lines = before_test_lines + fn_lines + ["\n"] + test_lines
    with open('src/crm/fetcher.rs', 'w') as f:
        f.writelines(new_lines)
    print("Fixed!")
else:
    print(f"Could not fix. test_idx={test_idx}, fn_idx={fn_idx}")
