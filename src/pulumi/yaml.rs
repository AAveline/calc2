use log::error;
use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use crate::pulumi;
use crate::serializer::{
    ContainerAppBluePrint, ContainerAppConfiguration, ContainerImageBluePrint,
};

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

fn filter_by_type(val: &&Value, resource_type: &str) -> bool {
    match val.get("type") {
        Some(x) => x.as_str() == Some(resource_type),
        None => false,
    }
}

fn get_images(mapping: &Mapping) -> Vec<ContainerImageBluePrint> {
    mapping
        .keys()
        .map(|key| match mapping.get(key) {
            Some(resource) => {
                if filter_by_type(&resource, "docker:RegistryImage") {
                    let mut image: ContainerImageBluePrint =
                        serde_yaml::from_value(resource.get("properties").unwrap().to_owned())
                            .unwrap();
                    image.reference_name = Some(key.as_str().unwrap().to_string());

                    Some(image)
                } else {
                    None
                }
            }
            None => None,
        })
        .flatten()
        .collect()
}

fn get_apps(mapping: &Mapping) -> Vec<ContainerAppBluePrint> {
    mapping
        .values()
        .filter(|x| filter_by_type(x, "azure-native:app:ContainerApp"))
        .map(|container| {
            serde_yaml::from_value(container.get("properties").unwrap().to_owned()).unwrap()
        })
        .collect()
}

pub fn deserialize(input: &str) -> Result<Vec<ContainerAppConfiguration>, String> {
    let deserialized_map = serde_yaml::Deserializer::from_str(input);
    let value = Value::deserialize(deserialized_map);

    match value {
        Ok(v) => {
            // If resources exists, then iterate over containersApp applications
            let as_mapping = v
                .get("resources")
                .expect("Resources need to be defined")
                .as_mapping()
                .expect("A mapping need to be generated");

            let images: Vec<ContainerImageBluePrint> = get_images(as_mapping);
            let apps: Vec<ContainerAppBluePrint> = get_apps(as_mapping);

            let services = pulumi::build_configuration(apps, images);

            Ok(services)
        }

        Err(e) => {
            error!("{}", e);
            // TODO refacto this
            Err("An error occured".to_string())
        }
    }
}

mod tests {
    use crate::serializer::{
        BuildContextBluePrint, ConfigurationBluePrint, ContainerBluePrint, DaprBluePrint,
        IngressBluePrint, TemplateBluePrint,
    };

    use super::*;

    #[test]
    fn test_get_images() {
        let images = r#"
      resources:
        myImage:
          type: docker:RegistryImage
          properties:
            name: ${registry.loginServer}/node-app:v1.0.0
            build:
              context: ${pulumi.cwd}/node-app
          options:
            provider: ${provider}
        myImageNotWorking:
          type: docker:RegistryImageNotWorking
          properties:
            name: ${registry.loginServer}/node-app:v1.0.0
            build:
              context: ${pulumi.cwd}/node-app
          options:
            provider: ${provider}
      "#;
        let deserialized_map = serde_yaml::Deserializer::from_str(images);
        let value = Value::deserialize(deserialized_map).unwrap();

        let as_mapping = &value
            .get("resources")
            .expect("Resources need to be defined")
            .as_mapping()
            .expect("A mapping need to be generated");

        let output = get_images(as_mapping);

        let expected = vec![ContainerImageBluePrint {
            reference_name: Some("myImage".to_string()),
            name: "${registry.loginServer}/node-app:v1.0.0".to_string(),
            build: BuildContextBluePrint {
                context: "${pulumi.cwd}/node-app".to_string(),
            },
        }];

        assert_eq!(expected, output);
    }

    #[test]
    fn test_get_apps() {
        let apps = r#"
      resources:
        containerapp:
          type: azure-native:app:ContainerApp
          properties:
            configuration:
              ingress:
                external: true
                targetPort: 80
              dapr:
                appPort: 3000
                enabled: true
                appId: myapp
            template:
              containers:
                - image: ${myImage.name}
                  name: myapp
        containerappnotworking:
          type: azure-native:app:ContainerAppNotWorking
          properties:
            configuration:
              ingress:
                external: true
                targetPort: 80
              dapr:
                appPort: 3000
                enabled: true
                appId: myapp
            template:
              containers:
                - image: ${myImage.name}
                  name: myapp
      "#;

        let deserialized_map = serde_yaml::Deserializer::from_str(apps);
        let value = Value::deserialize(deserialized_map).unwrap();

        let as_mapping = &value
            .get("resources")
            .expect("Resources need to be defined")
            .as_mapping()
            .expect("A mapping need to be generated");

        let output = get_apps(as_mapping);

        let expected = vec![ContainerAppBluePrint {
            configuration: ConfigurationBluePrint {
                ingress: Some(IngressBluePrint {
                    external: Some(true),
                    target_port: Some(80),
                }),
                dapr: Some(DaprBluePrint {
                    app_id: Some("myapp".to_string()),
                    app_port: Some(3000),
                    enabled: Some(true),
                }),
            },
            template: TemplateBluePrint {
                containers: Some(vec![ContainerBluePrint {
                    name: "myapp".to_string(),
                    image: "${myImage.name}".to_string(),
                }]),
            },
        }];

        assert_eq!(expected, output);
    }

    #[test]
    fn test_deserialize() {
        let wrong_format = r#"
          resources:
               containerapp:
              type: azure-native:app:ContainerApp
              properties:
                configuration:
                  ingress:
                    external: true
                    target_port: 80
                  dapr:
                    app_port: 8000
                    enabled: true
                    app_id: myapp
                template:
                  containers:
                    - image: ${myImage.name}
                      name: myapp
          "#;

        let output = deserialize(wrong_format);

        assert_eq!(Err("An error occured".to_string()), output);
    }
}
