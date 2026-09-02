import sys

with open('src/utils.rs', 'r') as f:
    content = f.read()

content = content.replace("let _base = chrono::Local::now().date_naive();", "let _base = chrono::Local::now().date_naive();\n        let _ = _base;")

with open('src/utils.rs', 'w') as f:
    f.write(content)
