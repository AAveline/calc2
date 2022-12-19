use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};

use crate::serializer::{ContainerAppConfiguration, Extension, Serializer};

pub struct Pulumi<'a> {
    output: String,
    language: &'a Extension,
}

impl Pulumi<'_> {
    pub fn new(output: String, language: &Extension) -> Pulumi {
        // Test if the language is supported for the provider
        Pulumi { output, language }
    }
}

impl Serializer for Pulumi<'_> {
    fn deserialize_value(&self, input: &str) -> Result<Vec<ContainerAppConfiguration>, ()> {
        match self.language {
            Extension::Yaml => match deserialize_yaml(input) {
                Some(value) => Ok(value),
                None => Err(()),
            },
            Extension::Typescript => todo!(),
            // Return an error with context
            _ => Err(()),
        }
    }

    fn serialize_value(&self) -> Result<(), ()> {
        Ok(())
    }
}

fn check_and_match_reference(resources: &Value, reference: String) {
    let a = resources.get(reference);

    println!("{:?}", a)
}
pub fn deserialize_yaml(input: &str) -> Option<Vec<ContainerAppConfiguration>> {
    let deserialized_map = serde_yaml::Deserializer::from_str(input);
    let value = Value::deserialize(deserialized_map);

    match value {
        Ok(v) => {
            // Check if a resources property exists
            let resources = v.get("resources")?;

            // If resources exists, then iterate over containersApp applications
            let as_mapping = resources.as_mapping()?;
            check_and_match_reference(resources, "myImage".to_string());
            fn filter_by_type(val: &&Value, resource_type: &str) -> bool {
                match val.get("type") {
                    Some(x) => x.as_str() == Some(resource_type),
                    None => false,
                }
            }

            let container_apps = as_mapping
                .values()
                .filter(|x| filter_by_type(x, "azure-native:app:ContainerApp"));

            let images = as_mapping
                .values()
                .filter(|x| filter_by_type(x, "docker:RegistryImage"));

            for i in images {
                println!("{:?}", i);
            }

            let mut services: Vec<ContainerAppConfiguration> = Vec::new();

            for app in container_apps {
                let containers = app
                    .get("properties")?
                    .get("template")?
                    .get("containers")?
                    .as_sequence()?;

                let dapr_configuration = app.get("properties")?.get("configuration")?.get("dapr");

                fn parse_app_configuration(
                    container: &Value,
                    dapr_configuration: Option<&Value>,
                ) -> Vec<ContainerAppConfiguration> {
                    // Handle build  context
                    let image = match container.get("image") {
                        Some(name) => name.as_str().unwrap().to_string(),
                        // Fallback image name: Empty String
                        None => String::from(""),
                    };

                    let name = match container.get("name") {
                        Some(name) => name.as_str().unwrap().to_string(),
                        // TODO: define fallback value for name, should be yaml service name
                        None => String::from(""),
                    };

                    if dapr_configuration.is_some() {
                        // Push DaprContainerAppConfig too
                        vec![
                            ContainerAppConfiguration {
                                // Get container image
                                image: String::from(&image),
                                // Get container name
                                name: String::from(&name),
                                depends_on: Some(vec!["placement".to_string()]),
                                networks: Some(vec![String::from("dapr-network")]),
                                network_mode: None,
                                // TODO
                                environment: None,
                                ports: None,
                                command: None,
                            },
                            // Dapr Sidecar config
                            ContainerAppConfiguration {
                                image: String::from("daprio/daprd:edge"),
                                // Get container name
                                name: format!("{}_dapr", String::from(&name)),
                                depends_on: Some(vec![String::from(&name)]),
                                network_mode: Some(format!("service:{}", String::from(&name))),
                                // TODO
                                environment: None,
                                ports: None,
                                networks: None,
                                command: Some(vec![
                                    "./daprd".to_string(),
                                    "-app-id".to_string(),
                                    String::from(&name),
                                    "-app-port".to_string(),
                                    String::from("port"),
                                    "-placement-host-address".to_string(),
                                    "placement:50006".to_string(),
                                    "air".to_string(),
                                ]),
                            },
                        ]
                    } else {
                        vec![ContainerAppConfiguration {
                            // Get container image
                            image,
                            // Get container name
                            name,
                            depends_on: None,
                            // No Dapr network
                            networks: None,
                            // TODO
                            environment: None,
                            network_mode: None,
                            ports: None,
                            command: None,
                        }]
                    }
                }

                let mut a: Vec<ContainerAppConfiguration> = containers
                    .iter()
                    .flat_map(|val| parse_app_configuration(val, dapr_configuration))
                    .collect();

                services.append(&mut a);
            }

            Some(services)
        }

        Err(e) => None,
    }
}
