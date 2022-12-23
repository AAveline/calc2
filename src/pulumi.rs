use regex::Regex;
use serde::Deserialize;
use serde_yaml::Value;

use crate::serializer::{BuildContext, ContainerAppConfiguration, Language, Serializer};

pub struct Pulumi {
    language: Language,
    pub resources: Option<Vec<ContainerAppConfiguration>>,
}

#[derive(Debug)]
pub struct DockerImageForPulumi {
    name: Option<String>,
    path: Option<String>,
    is_context: bool,
}

impl Pulumi {
    pub fn new(language: Language) -> Option<Pulumi> {
        match language {
            Language::Yaml | Language::Typescript => Some(Pulumi {
                language,
                resources: None,
            }),
            _ => None,
        }
    }
}

impl Serializer for Pulumi {
    type Output = Pulumi;
    fn deserialize_value(&mut self, input: &str) -> Option<&Self> {
        match self.language {
            Language::Yaml => match deserialize_yaml(input) {
                Some(value) => {
                    self.resources = Some(value);
                    Some(self)
                }
                None => None,
            },
            Language::Typescript => todo!(),
            // TODO: Return an error with context
            _ => None,
        }
    }
}
#[derive(Debug)]
struct Resource {
    name: String,
    property: Option<String>,
}

fn extract_and_parse_resource_name(s: String) -> Result<Resource, ()> {
    // TODO: Handle case where it's not a reference

    match Regex::new(r"\$\{(.+)\.(.+)\}")
        .expect("Should match previous regex")
        .captures(&s)
    {
        Some(v) => {
            let name = v.get(1).map_or("", |m| m.as_str()).to_string();
            let property = Some(v.get(2).map_or("", |m| m.as_str()).to_string());
            Ok(Resource { name, property })
        }
        None => Ok(Resource {
            name: s,
            property: None,
        }),
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
                Some(DockerImageForPulumi {
                    name: Some(reference.to_string()),
                    path: None,
                    is_context: false,
                })
            }
        }
        None => None,
    }
}
struct AppConfiguration<'a> {
    container: &'a Value,
    dapr_configuration: Option<&'a Value>,
    ingress_configuration: Option<&'a Value>,
}

fn build_image_for_serialization(resources: &Value, container: &Value) -> DockerImageForPulumi {
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
        None => DockerImageForPulumi {
            name: None,
            path: None,
            is_context: false,
        },
    };

    image
}

fn build_name_for_serialization(container: &Value) -> String {
    let name = match container.get("name") {
        Some(name) => name.as_str().unwrap_or_default().to_string(),
        // TODO: define fallback value for name, should be yaml service name
        None => String::from(""),
    };

    name
}

fn build_ports_mapping_for_serialization(
    configuration: AppConfiguration,
) -> (&Value, Option<Vec<String>>) {
    let dapr_configuration = configuration.dapr_configuration;
    let ingress_configuration = configuration.ingress_configuration;
    let container_name = configuration.container.get("name");

    let has_dapr_enabled = dapr_configuration
        .unwrap_or(&Value::Null)
        .get("enabled".to_string())
        .unwrap_or(&Value::Null);

    let has_ingress_exposed = ingress_configuration
        .unwrap_or(&Value::Null)
        .get("external".to_string())
        .unwrap_or(&Value::Null);

    let dapr_app_port = dapr_configuration
        .unwrap_or(&Value::Null)
        .get("appPort")
        .unwrap_or(&Value::Null);

    let ingress_app_port = ingress_configuration
        .unwrap_or(&Value::Null)
        .get("targetPort")
        .unwrap_or(&Value::Null);

    let mut ports: Vec<String> = vec![];
    // TODO: Assert for now than source and target ports are sames (container name and dapr target)

    if has_dapr_enabled.as_bool() == Some(true) && has_ingress_exposed.as_bool() == Some(true) {
        let has_right_target = container_name
            == dapr_configuration
                .unwrap_or(&Value::Null)
                .get("appId".to_string());

        if has_right_target {
            ports.push(format!(
                "{}:{}",
                ingress_app_port.as_f64().unwrap_or_default().to_string(),
                dapr_app_port.as_f64().unwrap_or_default().to_string()
            ))
        }
    }

    if (has_dapr_enabled.as_bool() == Some(false) || has_dapr_enabled.is_null())
        && has_ingress_exposed.as_bool() == Some(true)
    {
        ports.push(format!(
            "{}:{}",
            ingress_app_port.as_f64().unwrap_or_default().to_string(),
            ingress_app_port.as_f64().unwrap_or_default().to_string()
        ))
    }

    (
        dapr_app_port,
        if !ports.is_empty() { Some(ports) } else { None },
    )
}

fn parse_app_configuration(
    resources: &Value,
    configuration: AppConfiguration,
) -> Vec<ContainerAppConfiguration> {
    let container = configuration.container;
    let dapr_configuration = configuration.dapr_configuration;

    let image = build_image_for_serialization(resources, container);
    let name = build_name_for_serialization(container);
    let (dapr_app_port, ports) = build_ports_mapping_for_serialization(configuration);

    if dapr_configuration.is_some() {
        vec![
            ContainerAppConfiguration {
                image: image.name,
                build: image.is_context.then(|| BuildContext {
                    context: image.path.unwrap(),
                }),
                name: String::from(&name),
                depends_on: Some(vec!["placement".to_string()]),
                networks: Some(vec![String::from("dapr-network")]),
                network_mode: None,
                environment: None,
                ports: ports.clone(),
                command: None,
            },
            // Dapr Sidecar config
            ContainerAppConfiguration {
                image: Some(String::from("daprio/daprd:edge")),
                name: format!("{}_dapr", String::from(&name)),
                depends_on: Some(vec![String::from(&name)]),
                network_mode: Some(format!("service:{}", String::from(&name))),
                environment: None,
                // No exposed ports for dapr sidecar
                ports: None,
                networks: None,
                build: None,
                command: Some(vec![
                    "./daprd".to_string(),
                    "-app-id".to_string(),
                    String::from(&name),
                    "-app-port".to_string(),
                    format!("{}", dapr_app_port.as_f64().unwrap_or_default().to_string()),
                    "-placement-host-address".to_string(),
                    "placement:50006".to_string(),
                    "air".to_string(),
                ]),
            },
        ]
    } else {
        vec![ContainerAppConfiguration {
            image: image.name,
            build: image.is_context.then(|| BuildContext {
                context: image.path.unwrap(),
            }),
            name,
            depends_on: None,
            // No Dapr network
            networks: None,
            environment: None,
            network_mode: None,
            // TODO: Can have port if ingress defined
            ports: ports.clone(),
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
                let ingress_configuration =
                    app.get("properties")?.get("configuration")?.get("ingress");

                let mut a: Vec<ContainerAppConfiguration> = containers
                    .iter()
                    .flat_map(|container| {
                        parse_app_configuration(
                            resources,
                            AppConfiguration {
                                container,
                                dapr_configuration,
                                ingress_configuration,
                            },
                        )
                    })
                    .collect();

                services.append(&mut a);
            }

            Some(services)
        }

        Err(_e) => None,
    }
}
