use std::fmt;

#[derive(Debug)]
pub enum EngineError {
    TaskNotFound(String),
    TaskAlreadyExists(String),
    ConfigError(String),
    ExecutionFailed(String),
    Validation(String),
}

impl std::error::Error for EngineError {}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::TaskNotFound(id) => write!(f, "Task '{}' not found", id),
            EngineError::TaskAlreadyExists(id) => write!(f, "Task '{}' already exists", id),
            EngineError::ConfigError(msg) => write!(f, "Config error: {}", msg),
            EngineError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            EngineError::Validation(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}
