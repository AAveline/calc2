use crate::serializer::ContainerAppConfiguration;
use serde_yaml::Mapping;

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
        ports: Some(vec!["5006:5006".to_string()]),
        networks: Some(vec!["dapr-network".to_string()]),
        image: "daprio/dapr".to_string(),
        command: Some(vec![
            "./placement".to_string(),
            "-port".to_string(),
            "50006".to_string(),
        ]),
        depends_on: None,
        environment: None,
        network_mode: None,
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

    let mut dapr_network = Mapping::new();

    dapr_network.insert(
        serde_yaml::to_value("driver").unwrap(),
        serde_yaml::to_value("default").unwrap(),
    );

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
