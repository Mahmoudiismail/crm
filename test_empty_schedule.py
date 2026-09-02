import json

runner_config = {
    "gui_host": "127.0.0.1",
    "gui_port": 8787,
    "tasks": [
        {
            "id": "test_empty_schedules",
            "name": "Test Manual",
            "enabled": True,
            "repetition": "once",
            "frequency_seconds": 0,
            "next_run_at": "",
            "schedules": [],
            "steps": [],
            "post_run_steps": [],
            "last_run_at": "",
            "last_status": "",
            "timeout_seconds": 0
        }
    ]
}

with open("test_config.json", "w") as f:
    json.dump(runner_config, f)
