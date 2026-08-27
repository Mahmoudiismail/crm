with open("src/runner/engine/dispatcher.rs", "r") as f:
    disp = f.read()

import re
print("Matches Interval working_hours:", len(re.findall(r"TaskSchedule::Interval \{[\s\S]*?working_hours", disp)))
