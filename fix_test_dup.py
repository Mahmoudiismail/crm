import sys

with open('tests/runner/loader.rs', 'r') as f:
    content = f.read()

content = content.replace("#[test]\n\n#[test]\nfn test_empty_schedule_is_manual_persistence", "#[test]\nfn test_empty_schedule_is_manual_persistence")

with open('tests/runner/loader.rs', 'w') as f:
    f.write(content)
