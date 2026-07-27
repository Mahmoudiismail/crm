import sys

with open('src/runner/gui/templates.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
skip = False
for line in lines:
    if line.startswith("pub(crate) fn days_of_week_options("):
        skip = True

    if skip and line.startswith("}"):
        skip = False
        continue

    if not skip:
        new_lines.append(line)

with open('src/runner/gui/templates.rs', 'w') as f:
    f.writelines(new_lines)

print("Done")
