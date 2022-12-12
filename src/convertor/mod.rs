use std::collections::HashMap;

use serde::Deserialize;
use serde_yaml::Value;

pub mod convertor;
pub mod typescript;

struct ContainerAppConfiguration {
    dapr: Option<DaprConfiguration>,
    name: String,
    depends_on: String,
    networks: Vec<String>,
    image: String,
    environment: String,
    ports: Option<HashMap<i32, i32>>,
}

struct DaprConfiguration {
    name: String,
    depends_on: String,
    network_mode: String,
    command: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum Extension {
    Yaml,
    Typescript,
    Json,
    Bicep,
    NotSupported,
}

pub fn deserialize_yaml(input: &str) -> Option<()> {
    let deserialized_map = serde_yaml::Deserializer::from_str(input);
    let value = Value::deserialize(deserialized_map);

    match value {
        Ok(v) => {
            // Check if a resources property exists
            let resources = v.get("resources")?;

            // If resources exists, then iterate over containersApp applications
            let as_mapping = resources.as_mapping()?;

            fn filter_by_type(val: &&Value) -> bool {
                match val.get("type") {
                    Some(x) => x.as_str() == Some("azure-native:app:ContainerApp"),
                    None => false,
                }
            }

            fn parse_app_configuration() {}

            fn parse_dapr_configuration() -> DaprConfiguration {
                DaprConfiguration {
                    name: String::from("name_dapr"),
                    depends_on: String::from("name"),
                    network_mode: format!("service:{}", String::from("name")),
                    command: vec![
                        "./daprd".to_string(),
                        "-app-id".to_string(),
                        String::from("name"),
                        "-app-port".to_string(),
                        String::from("port"),
                        "-placement-host-address".to_string(),
                        "placement:50006".to_string(),
                        "air".to_string(),
                    ],
                }
            }

            let container_apps = as_mapping.values().filter(filter_by_type);

            let mut services: Vec<ContainerAppConfiguration> = Vec::new();

            for a in container_apps {
                let dapr_configuration = a.get("properties")?.get("configuration")?.get("dapr");

                if dapr_configuration.is_some() {
                    services.push(ContainerAppConfiguration {
                        dapr: Some(parse_dapr_configuration()),
                        name: String::from("name"),
                        depends_on: String::from("name"),
                        networks: vec![String::from("name")],
                        image: String::from("name"),
                        environment: String::from("name"),
                        ports: None,
                    });
                } else {
                    services.push(ContainerAppConfiguration {
                        dapr: None,
                        name: String::from("name"),
                        depends_on: String::from("name"),
                        networks: vec![String::from("name")],
                        image: String::from("name"),
                        environment: String::from("name"),
                        ports: None,
                    });
                }
                println!("{:?}", dapr_configuration);
            }

            /*

                Ici le but est le suivant. Il s'agit d'extraire deux resources principales dans un premier temps:
                - les applications containerapps qui deviendront des container dans docker compose
                - les services dapr qui deviendront des sidecars dans docker compose

                La méthodologie est donc la suivante:
                    - Créer un Vec qui contiendra les applications container apps et leur sidecar afférents
                    - Trouver et extraire les applications containers apps
                    - Trouver pour chaque application container apps la propriété dapr si elle existe
                        - si non, alors pousser dans le Vec l'application sous forme de service
                        - si oui, alors pousser dans le Vec
                            - l'application sous forme de service
                            - le sidecar dapr
            */
            Some(())
        }

        Err(e) => None,
    }
}
pub struct Pulumi {
    output: String,
    language: Extension,
}

pub trait Convertor {
    fn deserialize_value(&self, extension: Extension, input: &str) -> Result<(), ()>;
}

impl Pulumi {
    pub fn new(output: String, language: Extension) -> Pulumi {
        // Test if the language is supported for the provider
        Pulumi { output, language }
    }
}

impl Convertor for Pulumi {
    fn deserialize_value(&self, extension: Extension, input: &str) -> Result<(), ()> {
        match extension {
            Extension::Yaml => {
                if let deserialized = Some(deserialize_yaml(input)) {
                    Ok(())
                } else {
                    // No data to process, exit
                    Err(())
                }
            }
            Extension::Typescript => todo!(),
            // Return an error with context
            _ => Err(()),
        }
    }
}
