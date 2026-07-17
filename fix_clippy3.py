import sys

def fix_file(path):
    content = open(path, "r").read()

    old_stdout_loop = "for l in reader.lines().filter_map(|r| r.ok()) {"
    new_stdout_loop = "for l in reader.lines().map_while(Result::ok) {"
    content = content.replace(old_stdout_loop, new_stdout_loop)

    open(path, "w").write(content)

fix_file("src/tasker/dashboard_updater.rs")
fix_file("src/tasker/crm_open_sohail.rs")
print("Fixed clippy errors 3")
