use regex::Regex;
use serde::Deserialize;
use serde_yaml::Value;

use crate::serializer::{BuildContext, ContainerAppConfiguration, Extension, Serializer};

pub struct Pulumi<'a> {
    output: String,
    language: &'a Extension,
}

#[derive(Debug)]
pub struct DockerImageForPulumi {
    name: Option<String>,
    path: Option<String>,
    is_context: bool,
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
#[derive(Debug)]
struct Resource {
    name: String,
    property: String,
}

fn extract_and_parse_resource_name(s: String) -> Result<Resource, ()> {
    // TODO: Handle case where it's not a reference

    match Regex::new(r"\$\{(.+)\.(.+)\}")
        .expect("Should match previous regex")
        .captures(&s)
    {
        Some(v) => {
            let name = v.get(1).map_or("", |m| m.as_str()).to_string();
            let property = v.get(2).map_or("", |m| m.as_str()).to_string();
            Ok(Resource { name, property })
        }
        None => {
            // Return resource with provided name
            Ok(Resource {
                name: s,
                property: String::from(""),
            })
        }
    }
}

fn check_and_match_reference(resources: &Value, reference: &str) -> Option<DockerImageForPulumi> {
    let val = resources.get(reference);
    let re = Regex::new(r"(\$\{.+\})(/)(.+)").unwrap();

    match val {
        Some(val) => {
            let has_build_context = val
                .get("properties".to_string())
                // Assert that properties is always defined ? TODO - Rework on it
                .unwrap()
                .get("build".to_string());
            if has_build_context.is_some() {
                let a = re
                    .captures(
                        has_build_context
                            .unwrap()
                            .get("context".to_string())
                            .unwrap()
                            .as_str()
                            .unwrap(),
                    )
                    .unwrap();

                let image_name = a.get(3).map_or("", |m| m.as_str());
                let context_path = a.get(1).map_or("", |m| m.as_str());

                if image_name.is_empty() | context_path.is_empty() {
                    return None;
                }

                Some(DockerImageForPulumi {
                    name: None,
                    path: Some(format!(
                        "{}/{}",
                        context_path.replace("${pulumi.cwd}", "."),
                        image_name
                    )),
                    is_context: true,
                })
            } else {
                // No build context
                Some(DockerImageForPulumi {
                    name: Some("nginx".to_string()),
                    path: None,
                    is_context: false,
                })
            }
        }
        None => {
            // No reference context
            None
        }
    }
}

fn parse_app_configuration(
    resources: &Value,
    container: &Value,
    dapr_configuration: Option<&Value>,
) -> Vec<ContainerAppConfiguration> {
    // Handle build  context
    let image = match container.get("image") {
        Some(name) => {
            let resource =
                extract_and_parse_resource_name(name.as_str().unwrap_or_default().to_string())
                    .expect("Should contains name property");

            // Need to check if it's a reference or not
            let image = match check_and_match_reference(resources, &resource.name) {
                Some(v) => v,
                None => DockerImageForPulumi {
                    name: Some(resource.name),
                    is_context: false,
                    path: None,
                },
            };

            image
        }
        // Fallback image name: Empty String
        None => DockerImageForPulumi {
            name: None,
            path: None,
            is_context: false,
        },
    };

    let name = match container.get("name") {
        Some(name) => name.as_str().unwrap_or_default().to_string(),
        // TODO: define fallback value for name, should be yaml service name
        None => String::from(""),
    };

    if dapr_configuration.is_some() {
        // Push DaprContainerAppConfig too
        vec![
            ContainerAppConfiguration {
                // Get container image
                image: match image.name {
                    Some(v) => Some(v),
                    None => None,
                },
                build: if image.is_context {
                    Some(BuildContext {
                        context: image.path.unwrap_or_default(),
                    })
                } else {
                    None
                },
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
                image: Some(String::from("daprio/daprd:edge")),
                // Get container name
                name: format!("{}_dapr", String::from(&name)),
                depends_on: Some(vec![String::from(&name)]),
                network_mode: Some(format!("service:{}", String::from(&name))),
                // TODO
                environment: None,
                ports: None,
                networks: None,
                build: None,
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
            image: match image.name {
                Some(v) => Some(v),
                None => None,
            },
            build: if image.is_context {
                Some(BuildContext {
                    context: image.path.unwrap(),
                })
            } else {
                None
            },
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

pub fn deserialize_yaml(input: &str) -> Option<Vec<ContainerAppConfiguration>> {
    let deserialized_map = serde_yaml::Deserializer::from_str(input);
    let value = Value::deserialize(deserialized_map);

    match value {
        Ok(v) => {
            // Check if a resources property exists
            let resources = v.get("resources")?;

            // If resources exists, then iterate over containersApp applications
            let as_mapping = resources.as_mapping()?;

            fn filter_by_type(val: &&Value, resource_type: &str) -> bool {
                match val.get("type") {
                    Some(x) => x.as_str() == Some(resource_type),
                    None => false,
                }
            }

            let container_apps = as_mapping
                .values()
                .filter(|x| filter_by_type(x, "azure-native:app:ContainerApp"));

            let mut services: Vec<ContainerAppConfiguration> = Vec::new();

            for app in container_apps {
                let containers = app
                    .get("properties")?
                    .get("template")?
                    .get("containers")?
                    .as_sequence()?;

                let dapr_configuration = app.get("properties")?.get("configuration")?.get("dapr");

                let mut a: Vec<ContainerAppConfiguration> = containers
                    .iter()
                    .flat_map(|val| parse_app_configuration(resources, val, dapr_configuration))
                    .collect();

                services.append(&mut a);
            }

            Some(services)
        }

        Err(_e) => None,
    }
}
