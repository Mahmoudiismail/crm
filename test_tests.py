import re

with open("tests/tasker/csv_processing.rs", "r") as f:
    code = f.read()

# Verify that test_email_outlook_error_cascade_removal is still testing the right thing
# It looks good.
