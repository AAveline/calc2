use ccrate::pulumi;

pub fn deserialize(input: &str) -> Result<Vec<ContainerAppConfiguration>, String> {
    //let images = ;
    let apps = get_apps(&input);

    let services = pulumi::build_configuration(apps, images);

    match services {
        Some(val) => Ok(val),
        None => Err("No container to deserialize".to_string()),
    }
}
