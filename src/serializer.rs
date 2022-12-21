use serde::{Deserialize, Serialize};
use serde_yaml::Mapping;

#[derive(Debug, Clone, Copy)]
pub enum Extension {
    Yaml,
    Typescript,
    Json,
    Bicep,
    NotSupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildContext {
    pub context: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerAppConfiguration {
    #[serde(skip_serializing)]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildContext>,
}
pub trait Serializer {
    fn deserialize_value(&self, input: &str) -> Result<Vec<ContainerAppConfiguration>, ()>;
    fn serialize_value(&self) -> Result<(), ()>;
}

fn cast_struct_as_value(mut acc: Mapping, service: &ContainerAppConfiguration) -> Mapping {
    acc.insert(
        serde_yaml::to_value(&service.name).unwrap(),
        serde_yaml::to_value(&service).unwrap(),
    );
    acc
}

fn default_configuration() -> ContainerAppConfiguration {
    ContainerAppConfiguration {
        name: String::from("placement"),
        ports: Some(vec!["50006:50006".to_string()]),
        networks: Some(vec!["dapr-network".to_string()]),
        image: Some("daprio/dapr".to_string()),
        command: Some(vec![
            "./placement".to_string(),
            "-port".to_string(),
            "50006".to_string(),
        ]),
        depends_on: None,
        environment: None,
        network_mode: None,
        build: None,
    }
}

fn merge_configuration_with_networks(mut configuration: Mapping, services: Mapping) -> Mapping {
    // Generate API version
    configuration.insert(
        serde_yaml::to_value("version").unwrap(),
        serde_yaml::to_value("3.9").unwrap(),
    );

    configuration.insert(
        serde_yaml::to_value("services").unwrap(),
        serde_yaml::to_value(services).unwrap(),
    );

    let dapr_network = Mapping::new();

    let mut networks = Mapping::new();

    networks.insert(
        serde_yaml::to_value("dapr-network").unwrap(),
        serde_yaml::to_value(dapr_network).unwrap(),
    );

    configuration.insert(
        serde_yaml::to_value("networks").unwrap(),
        serde_yaml::to_value(networks).unwrap(),
    );

    configuration
}

pub fn serialize_to_compose(services: Vec<ContainerAppConfiguration>) -> Result<Vec<u8>, ()> {
    let as_value = vec![services, vec![default_configuration()]]
        .concat()
        .iter()
        .fold(Mapping::new(), |acc, x| cast_struct_as_value(acc, &x));

    let configuration = merge_configuration_with_networks(Mapping::new(), as_value);

    Ok(serde_yaml::to_string(&configuration)
        .unwrap()
        .as_bytes()
        .to_vec())
}
