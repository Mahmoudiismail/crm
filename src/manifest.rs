use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    pub name: String,
    pub description: String,
    pub arguments: Vec<AppArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgType {
    String,
    Number,
    List,
    MultiList,
    Boolean,
    DateVar,
}

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
    pub depends_on: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autofill:
        Option<std::collections::HashMap<String, std::collections::HashMap<String, String>>>,
}
