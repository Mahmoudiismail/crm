import re

with open("src/bin/runner.rs", "r") as f:
    text = f.read()

text = text.replace(
"""    let runner_cfg = {
        let cfg = crm_tool::runner::config::RunnerConfig::load(&runner_config_path_str)
            .unwrap_or_default();
        if let Err(e) = cfg.validate() {
            eprintln!("Runner configuration validation failed: {}", e);
            std::process::exit(1);
        }""",
"""    let runner_cfg = {
        let cfg = if config_exists {
            match crm_tool::runner::config::RunnerConfig::load(&runner_config_path_str) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("CRITICAL ERROR: Failed to parse existing runner_config.json: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            crm_tool::runner::config::RunnerConfig::default()
        };
        if let Err(e) = cfg.validate() {
            eprintln!("Runner configuration validation failed: {}", e);
            std::process::exit(1);
        }"""
)

with open("src/bin/runner.rs", "w") as f:
    f.write(text)
print("success")
