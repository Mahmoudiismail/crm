import json

data = json.loads(open("tasker_config.json.example").read())
print(json.dumps(data, indent=2))
