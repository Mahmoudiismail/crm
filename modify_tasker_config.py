import sys

content = open("src/bin/tasker.rs", "r").read()

old_crm_task = """    {
      "type": "crm_open_sohail",
      "download_path": "../crm_windows/Downloads",
      "users_file": "./task3/users.csv",
      "assignment_settings_file": "./task3/assignments.csv",
      "minutes_ago": 15,
      "exclude_branches": [],
      "exclude_categories": [],
      "output_file": "./crm_open_sohail_results.csv",
      "dashboard_file": "./dashboard_sohail.xlsx",
      "team_mapping_file": "./task3/teams.csv",
      "fallback_oul": "N/A"
    }"""

new_crm_task = """    {
      "type": "crm_open_sohail",
      "download_path": "../crm_windows/Downloads",
      "users_file": "./task3/users.csv",
      "assignment_settings_file": "./task3/assignments.csv",
      "minutes_ago": 15,
      "start_date": null,
      "exclude_branches": [],
      "exclude_categories": [],
      "category_exceptions": null,
      "output_file": "./crm_open_sohail_results.csv",
      "dashboard_file": "./dashboard_sohail.xlsx",
      "email_to": "",
      "email_cc": "",
      "save_email_as_html": false,
      "indentation_spaces": 4,
      "team_mapping_file": "./task3/teams.csv",
      "body_template_file": null,
      "subject_template": null,
      "branch_filter": null,
      "month_filter": null,
      "fallback_oul": "N/A"
    }"""

if old_crm_task in content:
    content = content.replace(old_crm_task, new_crm_task)
else:
    print("Could not find the crm block to replace!")

open("src/bin/tasker.rs", "w").write(content)
print("Updated tasker.rs")
