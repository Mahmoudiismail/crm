import re

with open("src/yasweb/browser/client.rs", "r") as f:
    content = f.read()

# Change the polling loop wait from 500ms to 100ms
content = content.replace('std::thread::sleep(Duration::from_millis(500));', 'std::thread::sleep(Duration::from_millis(100));')

# We can also increase the retry count to 10 so the total wait timeout is 1s, since we decreased sleep time
content = content.replace('for _ in 0..5 {', 'for _ in 0..10 {')

with open("src/yasweb/browser/client.rs", "w") as f:
    f.write(content)
