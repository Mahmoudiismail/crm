with open("src/runner/config/loader.rs", "r") as f:
    content = f.read()

import re

# Fix loader.rs
content = re.sub(
    r'(TaskSchedule::Interval\s*\{[^}]*?working_hours: None,)',
    r'\g<1>\n                working_hours_profile_id: None,',
    content
)

with open("src/runner/config/loader.rs", "w") as f:
    f.write(content)

# Fix tests in forms.rs (actually it's in gui/mod.rs tests)
with open("src/runner/gui/mod.rs", "r") as f:
    content = f.read()

# Add empty params for parse_schedules_text tests
content = content.replace('parse_schedules_text(\n            "interval: every 1h\\ndaily: 09:00, 13:00\\nonce: 2026-04-15T09:30:00-05:00",\n        )', 'parse_schedules_text("interval: every 1h\\ndaily: 09:00, 13:00\\nonce: 2026-04-15T09:30:00-05:00", &std::collections::HashMap::new(), &[])')
content = content.replace('parse_schedules_text("interval: every 2h; wh: Monday=09:00-17:00,Friday=10:00-15:00\\n")', 'parse_schedules_text("interval: every 2h; wh: Monday=09:00-17:00,Friday=10:00-15:00\\n", &std::collections::HashMap::new(), &[])')
content = content.replace('parse_schedules_text("weekly: Monday; st: 14:00\\nmonthly: day 15; st: 10:30")', 'parse_schedules_text("weekly: Monday; st: 14:00\\nmonthly: day 15; st: 10:30", &std::collections::HashMap::new(), &[])')


with open("src/runner/gui/mod.rs", "w") as f:
    f.write(content)
