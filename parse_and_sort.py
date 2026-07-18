import json

def parse_and_sort(file_path):
    with open(file_path, 'r') as f:
        print(f.read())
parse_and_sort("task_csv_analysis.log")
