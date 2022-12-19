use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub enum Extension {
    Yaml,
    Typescript,
    Json,
    Bicep,
    NotSupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerAppConfiguration {
    #[serde(skip_serializing)]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<Vec<String>>,
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_mode: Option<String>,
}
pub trait Serializer {
    fn deserialize_value(&self, input: &str) -> Result<Vec<ContainerAppConfiguration>, ()>;
    fn serialize_value(&self) -> Result<(), ()>;
}
