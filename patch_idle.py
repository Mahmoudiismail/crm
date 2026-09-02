with open("src/bin/yasweb.rs", "r") as f:
    content = f.read()

content = content.replace('.idle_browser_timeout(std::time::Duration::from_secs(120))', '.idle_browser_timeout(std::time::Duration::MAX)')
content = content.replace('.idle_browser_timeout(std::time::Duration::from_secs(900))', '.idle_browser_timeout(std::time::Duration::MAX)')

with open("src/bin/yasweb.rs", "w") as f:
    f.write(content)
