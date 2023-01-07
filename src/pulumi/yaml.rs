use log::error;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::serializer::{BuildContext, ContainerAppConfiguration};

#[derive(Debug, PartialEq)]
struct Resource {
    name: String,
    property: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct DockerImageForPulumi {
    name: Option<String>,
    path: Option<String>,
    is_context: bool,
}

#[derive(Debug)]
pub struct AppConfiguration {
    pub container: ContainerBluePrint,
    pub dapr_configuration: Option<DaprBluePrint>,
    pub ingress_configuration: Option<IngressBluePrint>,
}

fn extract_and_parse_resource_name(s: String) -> Result<Resource, ()> {
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

fn check_and_match_reference(
    images: &Vec<ContainerImageBluePrint>,
    reference: &str,
) -> Option<DockerImageForPulumi> {
    let val = images
        .iter()
        .find(|image| image.referenceName.clone().unwrap() == reference);
    let re = Regex::new(r"(\$\{.+\})(/)(.+)").unwrap();

    match val {
        Some(val) => {
            let has_build_context = &val.build;

            let a = re.captures(&has_build_context.context).unwrap();

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
        }
        None => None,
    }
}

fn build_image_for_serialization(
    images: &Vec<ContainerImageBluePrint>,
    container: ContainerBluePrint,
) -> DockerImageForPulumi {
    let resource =
        extract_and_parse_resource_name(container.name).expect("Should contains name property");

    // Need to check if it's a reference or not
    let image = match check_and_match_reference(images, &resource.name) {
        Some(v) => v,
        None => DockerImageForPulumi {
            name: Some(resource.name),
            is_context: false,
            path: None,
        },
    };

    image
}

fn build_ports_mapping_for_serialization(
    configuration: AppConfiguration,
) -> (Option<u32>, Option<Vec<String>>) {
    let dapr_configuration = configuration.dapr_configuration;
    let ingress_configuration = configuration.ingress_configuration;
    let container_name = configuration.container.name;

    let has_dapr_enabled = dapr_configuration.is_some();
    let has_ingress_exposed = ingress_configuration.is_some();

    let dapr_app_port = match dapr_configuration.clone() {
        Some(val) => val.appPort,
        None => None,
    };

    let dapr_app_id = match dapr_configuration.clone() {
        Some(val) => val.appId,
        None => None,
    };

    let ingress_app_port = match ingress_configuration {
        Some(val) => val.targetPort,
        None => None,
    };

    let mut ports: Vec<String> = vec![];
    // TODO: Assert for now than source and target ports are sames (container name and dapr target)

    if has_dapr_enabled && has_ingress_exposed {
        let has_right_target = container_name == dapr_app_id.unwrap_or_default();

        if has_right_target {
            ports.push(format!(
                "{}:{}",
                ingress_app_port.unwrap_or_default().to_string(),
                dapr_app_port.unwrap_or_default().to_string()
            ))
        }
    }

    if (!has_dapr_enabled) && has_ingress_exposed {
        ports.push(format!(
            "{}:{}",
            ingress_app_port.unwrap_or_default().to_string(),
            ingress_app_port.unwrap_or_default().to_string()
        ))
    }

    (
        dapr_app_port,
        if !ports.is_empty() { Some(ports) } else { None },
    )
}

fn parse_app_configuration(
    images: &Vec<ContainerImageBluePrint>,
    configuration: AppConfiguration,
) -> Vec<ContainerAppConfiguration> {
    let container = configuration.container.clone();
    let dapr_configuration = configuration.dapr_configuration.clone();

    let image = build_image_for_serialization(images, container);
    let name = configuration.container.name.clone();
    let (dapr_app_port, ports) = build_ports_mapping_for_serialization(configuration);

    let has_dapr_enabled = match dapr_configuration {
        Some(v) => v.enabled.unwrap(),
        None => false,
    };

    if has_dapr_enabled {
        vec![
            ContainerAppConfiguration {
                image: image.name,
                build: image.is_context.then(|| BuildContext {
                    context: image.path.unwrap(),
                }),
                name: name.clone(),
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
                name: format!("{}_dapr", name.clone()),
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
                    format!("{}", dapr_app_port.unwrap_or_default()),
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
            ports: ports.clone(),
            command: None,
        }]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaprBluePrint {
    appPort: Option<u32>,
    enabled: Option<bool>,
    appId: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngressBluePrint {
    external: Option<bool>,
    targetPort: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationBluePrint {
    ingress: Option<IngressBluePrint>,
    dapr: Option<DaprBluePrint>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateBluePrint {
    containers: Option<Vec<ContainerBluePrint>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerBluePrint {
    image: String,
    name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ContainerAppBluePrint {
    configuration: ConfigurationBluePrint,
    template: TemplateBluePrint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct BuildContextBluePrint {
    context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ContainerImageBluePrint {
    name: String,
    build: BuildContextBluePrint,
    referenceName: Option<String>,
}

pub fn deserialize(input: &str) -> Option<Vec<ContainerAppConfiguration>> {
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

            let images: Vec<ContainerImageBluePrint> = as_mapping
                .keys()
                .map(|key| match as_mapping.get(key) {
                    Some(resource) => {
                        if filter_by_type(&resource, "docker:RegistryImage") {
                            let mut image: ContainerImageBluePrint = serde_yaml::from_value(
                                resource.get("properties").unwrap().to_owned(),
                            )
                            .unwrap();
                            image.referenceName = Some(key.as_str().unwrap().to_string());

                            Some(image)
                        } else {
                            None
                        }
                    }
                    None => None,
                })
                .flatten()
                .collect();

            let apps: Vec<ContainerAppBluePrint> = as_mapping
                .values()
                .filter(|x| filter_by_type(x, "azure-native:app:ContainerApp"))
                .map(|container| {
                    serde_yaml::from_value(container.get("properties").unwrap().to_owned()).unwrap()
                })
                .collect();

            let mut services: Vec<ContainerAppConfiguration> = Vec::new();

            for app in apps {
                let containers = app.template.containers;
                let dapr_configuration = app.configuration.dapr;
                let ingress_configuration = app.configuration.ingress;

                let mut a: Vec<ContainerAppConfiguration> = containers
                    .unwrap()
                    .iter()
                    .flat_map(|container| {
                        parse_app_configuration(
                            &images,
                            AppConfiguration {
                                container: container.to_owned(),
                                dapr_configuration: dapr_configuration.clone(),
                                ingress_configuration: ingress_configuration.clone(),
                            },
                        )
                    })
                    .collect();

                services.append(&mut a);
            }

            Some(services)
        }

        Err(e) => {
            error!("{}", e);
            None
        }
    }
}

mod tests {
    use super::*;
    #[test]
    fn test_extract_and_parse_resource_name() {
        let input1 = "${resource.property}".to_string();
        let expected = Ok(Resource {
            name: "resource".to_string(),
            property: Some("property".to_string()),
        });
        let output = extract_and_parse_resource_name(input1);
        assert_eq!(expected, output);

        let input2 = "resource".to_string();
        let expected = Ok(Resource {
            name: "resource".to_string(),
            property: None,
        });
        let output = extract_and_parse_resource_name(input2);
        assert_eq!(expected, output);
    }

    #[test]
    fn test_build_image_for_serialization() {
        let containers = r#"
        with_reference:
          image: ${myImage.name}
          name: myapp
        without_reference:
          image: node-12
          name: myapp
        "#;

        let input_without_resource_reference = r#"
        resources:
          containerapp:
            type: azure-native:app:ContainerApp
            properties:
              configuration:
                ingress:
                  external: true
                  targetPort: 80
                dapr:
                  appPort: 8000
                  enabled: true
                  appId: myapp
              template:
                containers:
                  - image: ${myImage.name}
                    name: myapp
        "#;

        let deserialized_containers = serde_yaml::Deserializer::from_str(containers);
        let containers_value = Value::deserialize(deserialized_containers).unwrap();

        let container_with_reference = containers_value.get("with_reference").unwrap();
        let container_without_reference = containers_value.get("without_reference").unwrap();

        let input = serde_yaml::Deserializer::from_str(input_without_resource_reference);
        let value = Value::deserialize(input);

        let output = build_image_for_serialization(
            value.unwrap().get("resources").unwrap(),
            container_with_reference,
        );

        let expected = DockerImageForPulumi {
            name: Some("myImage".to_string()),
            path: None,
            is_context: false,
        };

        assert_eq!(expected, output);

        let input_with_context = r#"
        resources:
          myImage:
            type: docker:RegistryImage
            properties:
              name: ${registry.loginServer}/node-app:v1.0.0
              build:
                context: ${pulumi.cwd}/node-app
            options:
              provider: ${provider}
          containerapp:
            type: azure-native:app:ContainerApp
            properties:
              configuration:
                ingress:
                  external: true
                  targetPort: 80
                dapr:
                  appPort: 8000
                  enabled: true
                  appId: myapp
              template:
                containers:
                  - image: ${myImage.name}
                    name: myapp
        "#;
        let deserialized_map = serde_yaml::Deserializer::from_str(input_with_context);
        let value = Value::deserialize(deserialized_map);
        let output = build_image_for_serialization(
            value.unwrap().get("resources").unwrap(),
            container_with_reference,
        );

        let expected = DockerImageForPulumi {
            name: None,
            path: Some("./node-app".to_string()),
            is_context: true,
        };

        assert_eq!(expected, output);

        let input_without_context = r#"
        resources:
          containerapp:
            type: azure-native:app:ContainerApp
            properties:
              configuration:
                ingress:
                  external: true
                  targetPort: 80
                dapr:
                  appPort: 8000
                  enabled: true
                  appId: myapp
              template:
                containers:
                  - image: node-12
                    name: myapp
        "#;

        let deserialized_map = serde_yaml::Deserializer::from_str(input_without_context);
        let value = Value::deserialize(deserialized_map);
        let output = build_image_for_serialization(
            value.unwrap().get("resources").unwrap(),
            container_without_reference,
        );

        let expected = DockerImageForPulumi {
            name: Some("node-12".to_string()),
            path: None,
            is_context: false,
        };

        assert_eq!(expected, output);
    }

    #[test]
    fn test_build_ports_mapping_for_serialization() {
        let input_without_dapr = r#"
      configuration:
        ingress:
          external: false
          targetPort: 3000
        dapr:
          appPort: 3000
          enabled: false
          appId: some-app
      template:
        containers:
          - image: ${myImage.name}
            name: some-app
      "#;
        let deserialized_map = serde_yaml::Deserializer::from_str(input_without_dapr);
        let value = Value::deserialize(deserialized_map).unwrap();

        let container = &value
            .get("template")
            .unwrap()
            .get("containers")
            .unwrap()
            .as_sequence()
            .unwrap()[0];
        let dapr_configuration = value.get("configuration").unwrap().get("dapr");
        let ingress_configuration = value.get("configuration").unwrap().get("ingress");

        let configuration = AppConfiguration {
            container,
            dapr_configuration,
            ingress_configuration,
        };

        let (dapr_app_port, ports) = build_ports_mapping_for_serialization(configuration);

        assert_eq!(dapr_app_port, &Value::from(3000));
        assert_eq!(ports, None);

        let input_with_ingress = r#"
        configuration:
          ingress:
            external: true
            targetPort: 3000
          dapr:
            appPort: 3000
            enabled: false
            appId: some-app
        template:
          containers:
            - image: ${myImage.name}
              name: some-app
        "#;
        let deserialized_map = serde_yaml::Deserializer::from_str(input_with_ingress);
        let value = Value::deserialize(deserialized_map).unwrap();

        let container = &value
            .get("template")
            .unwrap()
            .get("containers")
            .unwrap()
            .as_sequence()
            .unwrap()[0];
        let dapr_configuration = value.get("configuration").unwrap().get("dapr");
        let ingress_configuration = value.get("configuration").unwrap().get("ingress");

        let configuration = AppConfiguration {
            container,
            dapr_configuration,
            ingress_configuration,
        };

        let (dapr_app_port, ports) = build_ports_mapping_for_serialization(configuration);

        assert_eq!(dapr_app_port, &Value::from(3000));
        assert_eq!(ports, Some(vec!["3000:3000".to_string()]));

        let input_with_dapr = r#"
        configuration:
          ingress:
            external: false
          dapr:
            appPort: 3000
            enabled: true
            appId: some-app
        template:
          containers:
            - image: ${myImage.name}
              name: some-app
        "#;
        let deserialized_map = serde_yaml::Deserializer::from_str(input_with_dapr);
        let value = Value::deserialize(deserialized_map).unwrap();

        let container = &value
            .get("template")
            .unwrap()
            .get("containers")
            .unwrap()
            .as_sequence()
            .unwrap()[0];
        let dapr_configuration = value.get("configuration").unwrap().get("dapr");
        let ingress_configuration = value.get("configuration").unwrap().get("ingress");

        let configuration = AppConfiguration {
            container,
            dapr_configuration,
            ingress_configuration,
        };

        let (dapr_app_port, ports) = build_ports_mapping_for_serialization(configuration);

        assert_eq!(dapr_app_port, &Value::from(3000));
        assert_eq!(ports, None);

        let input_with_dapr_and_ingress = r#"
        configuration:
          ingress:
            external: true
            targetPort: 80
          dapr:
            appPort: 3000
            enabled: true
            appId: some-app
        template:
          containers:
            - image: ${myImage.name}
              name: some-app
        "#;
        let deserialized_map = serde_yaml::Deserializer::from_str(input_with_dapr_and_ingress);
        let value = Value::deserialize(deserialized_map).unwrap();

        let container = &value
            .get("template")
            .unwrap()
            .get("containers")
            .unwrap()
            .as_sequence()
            .unwrap()[0];
        let dapr_configuration = value.get("configuration").unwrap().get("dapr");
        let ingress_configuration = value.get("configuration").unwrap().get("ingress");

        let configuration = AppConfiguration {
            container,
            dapr_configuration,
            ingress_configuration,
        };

        let (dapr_app_port, ports) = build_ports_mapping_for_serialization(configuration);

        assert_eq!(dapr_app_port, &Value::from(3000));
        assert_eq!(ports, Some(vec!["80:3000".to_string()]));
    }

    #[test]
    fn test_parse_app_configuration() {
        let value = r#"
        resources:
          myImage:
            type: docker:RegistryImage
            properties:
              name: ${registry.loginServer}/node-app:v1.0.0
              build:
                context: ${pulumi.cwd}/node-app
            options:
              provider: ${provider}
          containerapp:
            type: azure-native:app:ContainerApp
            properties:
              configuration:
                ingress:
                  external: true
                  targetPort: 80
                dapr:
                  appPort: 8000
                  enabled: true
                  appId: myapp
              template:
                containers:
                  - image: ${myImage.name}
                    name: myapp
        "#;
        let deserialized_map = serde_yaml::Deserializer::from_str(value);
        let resources = Value::deserialize(deserialized_map).unwrap();

        let input_with_dapr = r#"
        configuration:
          ingress:
            external: false
            targetPort: 3000
          dapr:
            appPort: 3000
            enabled: true
            appId: myapp
        template:
          containers:
            - image: ${myImage.name}
              name: myapp
        "#;
        let deserialized_map = serde_yaml::Deserializer::from_str(input_with_dapr);
        let value = Value::deserialize(deserialized_map).unwrap();

        let container = &value
            .get("template")
            .unwrap()
            .get("containers")
            .unwrap()
            .as_sequence()
            .unwrap()[0];
        let dapr_configuration = value.get("configuration").unwrap().get("dapr");
        let ingress_configuration = value.get("configuration").unwrap().get("ingress");

        let configuration = AppConfiguration {
            container,
            dapr_configuration,
            ingress_configuration,
        };

        let output = parse_app_configuration(resources.get("resources").unwrap(), configuration);
        let expected = vec![
            ContainerAppConfiguration {
                image: None,
                build: Some(BuildContext {
                    context: "./node-app".to_string(),
                }),
                name: "myapp".to_string(),
                depends_on: Some(vec!["placement".to_string()]),
                networks: Some(vec![String::from("dapr-network")]),
                network_mode: None,
                environment: None,
                ports: None,
                command: None,
            },
            ContainerAppConfiguration {
                image: Some(String::from("daprio/daprd:edge")),
                name: format!("myapp_dapr"),
                depends_on: Some(vec![String::from("myapp")]),
                network_mode: Some(format!("service:{}", String::from("myapp"))),
                environment: None,
                ports: None,
                networks: None,
                build: None,
                command: Some(vec![
                    "./daprd".to_string(),
                    "-app-id".to_string(),
                    String::from("myapp"),
                    "-app-port".to_string(),
                    "3000".to_string(),
                    "-placement-host-address".to_string(),
                    "placement:50006".to_string(),
                    "air".to_string(),
                ]),
            },
        ];

        assert_eq!(expected, output);

        let input_without_dapr = r#"
        configuration:
          ingress:
            external: false
            targetPort: 3000
          dapr:
            appPort: 3000
            enabled: false
            appId: myapp
        template:
          containers:
            - image: node-12
              name: myapp
        "#;
        let deserialized_map = serde_yaml::Deserializer::from_str(input_without_dapr);
        let value = Value::deserialize(deserialized_map).unwrap();

        let container = &value
            .get("template")
            .unwrap()
            .get("containers")
            .unwrap()
            .as_sequence()
            .unwrap()[0];
        let dapr_configuration = value.get("configuration").unwrap().get("dapr");
        let ingress_configuration = value.get("configuration").unwrap().get("ingress");

        let configuration = AppConfiguration {
            container,
            dapr_configuration,
            ingress_configuration,
        };

        let output = parse_app_configuration(resources.get("resources").unwrap(), configuration);
        let expected = vec![ContainerAppConfiguration {
            image: Some("node-12".to_string()),
            build: None,
            name: "myapp".to_string(),
            depends_on: None,
            networks: None,
            network_mode: None,
            environment: None,
            ports: None,
            command: None,
        }];

        assert_eq!(expected, output)
    }

    #[test]
    fn test_deserialize_yaml() {
        let wrong_format = r#"
      resources:
           containerapp:
          type: azure-native:app:ContainerApp
          properties:
            configuration:
              ingress:
                external: true
                targetPort: 80
              dapr:
                appPort: 8000
                enabled: true
                appId: myapp
            template:
              containers:
                - image: ${myImage.name}
                  name: myapp
      "#;

        let output = deserialize(wrong_format);

        assert_eq!(None, output);
    }
}
