use serde::{Deserialize, Serialize};
use serde_yaml::Mapping;

#[derive(Debug, Clone, Copy)]
pub enum Language {
    Yaml,
    Typescript,
    Javascript,
    Json,
    Bicep,
    NotSupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildContext {
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DaprBluePrint {
    pub app_port: Option<u32>,
    pub enabled: Option<bool>,
    pub app_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IngressBluePrint {
    pub external: Option<bool>,
    pub target_port: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigurationBluePrint {
    pub ingress: Option<IngressBluePrint>,
    pub dapr: Option<DaprBluePrint>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateBluePrint {
    pub containers: Option<Vec<ContainerBluePrint>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerBluePrint {
    pub image: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerAppBluePrint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<ConfigurationBluePrint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateBluePrint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildContextBluePrint {
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerImageBluePrint {
    pub name: Option<String>,
    pub build: BuildContextBluePrint,
    pub reference_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    type Output;
    fn deserialize_value(&mut self, input: &str) -> Result<&Self::Output, String>;
    fn serialize_value(
        &self,
        services: &Vec<ContainerAppConfiguration>,
    ) -> Result<Vec<u8>, serde_yaml::Error> {
        let as_value = vec![services.clone(), vec![default_configuration()]]
            .concat()
            .iter()
            .fold(Mapping::new(), |acc, x| cast_struct_as_value(acc, &x));

        let configuration = merge_configuration_with_networks(Mapping::new(), as_value);

        match serde_yaml::to_string(&configuration) {
            Ok(v) => Ok(v.as_bytes().to_vec()),
            Err(e) => Err(e),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestSerializer {}

    impl Serializer for TestSerializer {
        type Output = TestSerializer;
        fn deserialize_value(&mut self, _input: &str) -> Result<&Self, String> {
            Ok(self)
        }
    }

    #[test]
    fn test_default_configuration() {
        let expected = ContainerAppConfiguration {
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
        };

        let output = default_configuration();

        assert_eq!(expected, output)
    }

    #[test]
    fn test_merge_configuration_with_networks() {
        let mut expected = Mapping::new();
        expected.insert(
            serde_yaml::to_value("version").unwrap(),
            serde_yaml::to_value("3.9").unwrap(),
        );

        expected.insert(
            serde_yaml::to_value("services").unwrap(),
            serde_yaml::to_value(Mapping::new()).unwrap(),
        );

        let dapr_network = Mapping::new();

        let mut networks = Mapping::new();

        networks.insert(
            serde_yaml::to_value("dapr-network").unwrap(),
            serde_yaml::to_value(dapr_network).unwrap(),
        );

        expected.insert(
            serde_yaml::to_value("networks").unwrap(),
            serde_yaml::to_value(networks).unwrap(),
        );
        let output = merge_configuration_with_networks(Mapping::new(), Mapping::new());

        assert_eq!(expected, output)
    }

    #[test]
    fn test_serializer() {
        let serializer = TestSerializer {};

        let input = vec![
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

        let expected = r#"version: '3.9'
services:
  myapp:
    depends_on:
    - placement
    networks:
    - dapr-network
    build:
      context: ./node-app
  myapp_dapr:
    depends_on:
    - myapp
    image: daprio/daprd:edge
    command:
    - ./daprd
    - -app-id
    - myapp
    - -app-port
    - '3000'
    - -placement-host-address
    - placement:50006
    - air
    network_mode: service:myapp
  placement:
    networks:
    - dapr-network
    image: daprio/dapr
    ports:
    - 50006:50006
    command:
    - ./placement
    - -port
    - '50006'
networks:
  dapr-network: {}
"#
        .as_bytes()
        .to_vec();

        let output = serializer.serialize_value(&input).unwrap();

        assert_eq!(expected, output);
    }
}
