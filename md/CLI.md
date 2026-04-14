# CLI Reference (Deprecated Runtime Path)

Runtime operation is now configuration-driven (`runner_config.json` + `config.json`).

- The tray + runner scheduler + runner GUI are the primary control interfaces.
- Task behavior is configured in runner config instead of command-line arguments.

If CLI parsing artifacts still exist in legacy files, treat them as transitional and not part of the operational contract.
