use std::borrow::Borrow;

use regex::Regex;
use serde::Deserialize;
use serde_yaml::Value;

use crate::serializer::{ContainerAppConfiguration, Extension, Serializer};

pub struct Pulumi<'a> {
    #[warn(dead_code)]
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

fn check_and_match_reference(
    resources: &Value,
    reference: String,
    properties: Vec<String>,
) -> Option<bool> {
    let mut val = resources.get(reference);

    let a = match val {
        Some(_v) => {
            let re = Regex::new(r"(?P<resource>\$\{(.+).loginServer\})(/)(.+)").unwrap();

            for property in properties {
                val = match val?.get(&property) {
                    Some(_v) => val?.get(&property).to_owned(),
                    None => None,
                };
            }

            /*
                Need the ability to parse the given resource and potentially extract the targeted property
                OR extract the nested resource with a targeted property
                Need to define the potency of this function :
                    - Where to put the recursive ability, based on what attribute
                    - How to fallback

                Maybe the potency is simply to parse the name of the image without a nested recursive link ?
                - Eg: ${registry.loginServer}/node-app:v1.0.0 => node-app:v1.0.0
                Maybe we need to cast the resource in a struct and act on it ?
                    - If it's a docker image, do something
                    - If it's anything else, do something else
                But maybe it's not required because we only need the image for the docker compose ?
            */
            match val {
                Some(val) => {
                    let val = val.as_str().unwrap();
                    let a = re.captures(val).unwrap();
                    println!("{:?}", val);

                    let nested_ref = a.get(2).map_or("", |m| m.as_str());
                    println!("{:?}", nested_ref);
                    if nested_ref.is_empty() {
                        println!("empty");
                        println!("{:?}", val)
                    } else {
                        // Do something with recursive call
                        let b = check_and_match_reference(
                            resources,
                            nested_ref.to_string(),
                            vec![nested_ref.to_string(), "properties".to_string()],
                        );
                        println!("{:?}", b);
                    }
                }
                None => (),
            }

            Some(true)
        }
        None => None,
    };

    a
}

fn parse_app_configuration(
    container: &Value,
    dapr_configuration: Option<&Value>,
) -> Vec<ContainerAppConfiguration> {
    // Handle build  context
    let image = match container.get("image") {
        Some(name) => {
            // Need to check if it's a reference or not

            // Not a reference
            name.as_str().unwrap().to_string()
        }
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

            let images = as_mapping
                .values()
                .filter(|x| filter_by_type(x, "docker:RegistryImage"));

            check_and_match_reference(
                resources,
                "myImage".to_string(),
                vec!["properties".to_string(), "name".to_string()],
            );

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
                    .flat_map(|val| parse_app_configuration(val, dapr_configuration))
                    .collect();

                services.append(&mut a);
            }

            Some(services)
        }

        Err(_e) => None,
    }
}
