import re

with open("src/bin/tasker.rs", "r") as f:
    content = f.read()

# Make sure it complies with auto healing / docs
# In src/tasker/config.rs we added table_column_widths.
# Oh wait, we already added this to tasker.rs line 148, which covers auto healing.
