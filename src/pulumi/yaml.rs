use log::error;
use serde::Deserialize;
use serde_yaml::Value;

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

            let services = pulumi::build_configuration(apps, images);

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
    fn test_deserialize() {
        /*
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
        }*/
    }
}
