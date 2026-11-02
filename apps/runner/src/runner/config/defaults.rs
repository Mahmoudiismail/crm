pub(crate) fn default_gui_host() -> String {
    "127.0.0.1".to_string()
}

pub(crate) fn default_gui_port() -> u16 {
    8787
}

pub(crate) fn default_poll_interval() -> u64 {
    30
}

pub(crate) fn default_allow_shell_tasks() -> bool {
    false
}

pub(crate) fn default_shell_timeout() -> u64 {
    900
}

pub(crate) fn default_post_run_timeout() -> u64 {
    900
}

pub(crate) fn default_min_task_interval() -> u64 {
    5
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_frequency() -> u64 {
    3600
}

pub(crate) fn default_day() -> u32 {
    1
}

pub(crate) fn default_log_retention_days() -> u64 {
    30
}

pub(crate) fn default_stdout_log_level() -> String {
    "DEBUG".to_string()
}

pub(crate) fn default_file_log_level() -> String {
    "TRACE".to_string()
}
