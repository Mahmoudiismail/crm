use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents an error that occurred during validation of an AppManifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Indicates that a required field is empty.
    EmptyField { field: &'static str },
    /// Indicates that a collection contains a duplicate entry where unique entries are required.
    DuplicateValue {
        field: &'static str,
        invalid_value: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::EmptyField { field } => {
                write!(f, "Validation failed: field '{}' cannot be empty", field)
            }
            ValidationError::DuplicateValue {
                field,
                invalid_value,
            } => write!(
                f,
                "Validation failed: duplicate value '{}' found in field '{}'",
                invalid_value, field
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

/// The top-level definition of an external application plugin/manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Represents the expected CLI arguments and settings for a runnable child application.
///
/// The runner application intercepts this payload at startup to dynamically render UI fields
/// without hardcoding child app definitions in the GUI logic.
pub struct AppManifest {
    pub name: String,
    pub description: String,
    pub arguments: Vec<AppArg>,
}

impl AppManifest {
    /// Validates the structure and data of the `AppManifest`.
    ///
    /// # Errors
    /// Returns a `ValidationError` if the `name` is empty, if any argument has an empty `name`,
    /// or if there are duplicate argument names.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.name.trim().is_empty() {
            return Err(ValidationError::EmptyField { field: "name" });
        }

        let mut seen_args = std::collections::HashSet::new();
        for arg in &self.arguments {
            if arg.name.trim().is_empty() {
                return Err(ValidationError::EmptyField {
                    field: "arguments[].name",
                });
            }
            if !seen_args.insert(&arg.name) {
                return Err(ValidationError::DuplicateValue {
                    field: "arguments",
                    invalid_value: arg.name.clone(),
                });
            }
        }

        Ok(())
    }
}

/// The permitted data types for an application argument.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArgType {
    String,
    Number,
    List,
    MultiList,
    Boolean,
    DateVar,
}

/// Defines a single argument (parameter) that an external application accepts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppArg {
    pub name: String,
    pub arg_type: ArgType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autofill: Option<HashMap<String, HashMap<String, String>>>,
}

impl AppArg {
    /// Creates a new `AppArg` with the specified name and type, with `required` defaulting to `false` and all optional fields set to `None`.
    /// Creates a new argument definition with the given name and type.
    pub fn new(name: impl Into<String>, arg_type: ArgType) -> Self {
        Self {
            name: name.into(),
            arg_type,
            required: false,
            default_value: None,
            options: None,
            depends_on: None,
            autofill: None,
        }
    }

    /// Sets whether the argument is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets the default value.
    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    /// Sets the available options.
    pub fn options(mut self, opts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.options = Some(opts.into_iter().map(Into::into).collect());
        self
    }

    /// Sets the depends_on map.
    pub fn depends_on(mut self, depends_on: HashMap<String, Vec<String>>) -> Self {
        self.depends_on = Some(depends_on);
        self
    }

    /// Sets the autofill map.
    pub fn autofill(mut self, autofill: HashMap<String, HashMap<String, String>>) -> Self {
        self.autofill = Some(autofill);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_manifest() {
        let manifest = AppManifest {
            name: "Test App".to_string(),
            description: "A test application".to_string(),
            arguments: vec![
                AppArg::new("--config", ArgType::String).required(true),
                AppArg::new("--verbose", ArgType::Boolean),
            ],
        };
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_empty_manifest_name() {
        let manifest = AppManifest {
            name: "   ".to_string(),
            description: "A test application".to_string(),
            arguments: vec![],
        };
        let err = manifest.validate().unwrap_err();
        assert_eq!(err, ValidationError::EmptyField { field: "name" });
        assert_eq!(
            err.to_string(),
            "Validation failed: field 'name' cannot be empty"
        );
    }

    #[test]
    fn test_empty_arg_name() {
        let manifest = AppManifest {
            name: "Test App".to_string(),
            description: "A test application".to_string(),
            arguments: vec![AppArg::new("   ", ArgType::String)],
        };
        let err = manifest.validate().unwrap_err();
        assert_eq!(
            err,
            ValidationError::EmptyField {
                field: "arguments[].name"
            }
        );
    }

    #[test]
    fn test_duplicate_arg_name() {
        let manifest = AppManifest {
            name: "Test App".to_string(),
            description: "A test application".to_string(),
            arguments: vec![
                AppArg::new("--config", ArgType::String),
                AppArg::new("--config", ArgType::String),
            ],
        };
        let err = manifest.validate().unwrap_err();
        assert_eq!(
            err,
            ValidationError::DuplicateValue {
                field: "arguments",
                invalid_value: "--config".to_string()
            }
        );
        assert_eq!(
            err.to_string(),
            "Validation failed: duplicate value '--config' found in field 'arguments'"
        );
    }

    #[test]
    fn test_serialization_format() {
        let manifest = AppManifest {
            name: "Test App".to_string(),
            description: "A test application".to_string(),
            arguments: vec![
                AppArg::new("--config", ArgType::String).default_value("config.json"),
                AppArg::new("--verbose", ArgType::Boolean),
                AppArg::new("--mode", ArgType::List).options(vec!["fast", "slow"]),
            ],
        };

        let json_value = serde_json::to_value(&manifest).unwrap();
        let expected = json!({
            "name": "Test App",
            "description": "A test application",
            "arguments": [
                {
                    "name": "--config",
                    "arg_type": "string",
                    "required": false,
                    "default_value": "config.json"
                },
                {
                    "name": "--verbose",
                    "arg_type": "boolean",
                    "required": false
                },
                {
                    "name": "--mode",
                    "arg_type": "list",
                    "required": false,
                    "options": ["fast", "slow"]
                }
            ]
        });

        assert_eq!(json_value, expected);
    }
}
