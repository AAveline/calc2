use crate::pulumi;
use regex::Regex;

use crate::serializer::{
    ContainerAppBluePrint, ContainerAppConfiguration, ContainerImageBluePrint,
};

fn parse_line(line: &str) -> String {
    let a = line.replace(" ", "");
    let re = Regex::new(r####"([a-zA-Z"]+)(:)([a-zA-Z0-9.`/"\{}\[\]]+)?"####).unwrap();

    let captures = re.captures(&a);

    let computed = match captures {
        Some(c) => {
            let key = c.get(1).unwrap().as_str();
            let value = if c.get(3).is_some() {
                let computed = c.get(3).unwrap().as_str();
                let tokens = ["{", "[{"];
                let has_token = tokens.contains(&computed);

                if has_token {
                    computed.to_string()
                } else {
                    let with_quotes = format!("\"{}\",", computed);
                    // Check if it's a number or a boolean
                    let re = Regex::new(r"^[0-9]+").unwrap().is_match(computed);
                    // Need to cleanup this part
                    if re || computed == "true" || computed == "false" {
                        let a = format!("{},", computed.to_string());
                        a
                    } else {
                        with_quotes.to_string()
                    }
                }
            } else {
                "".to_string()
            };

            let key = format!("\"{}\"", key).replace("\"\"", "\"");
            let value = value.replace("\"\"", "\"").replace("`", "");

            let computed = format!("{key}:{value}");

            computed
        }
        None => a.to_string(),
    };
    computed
}

fn prune_output(output: String) -> String {
    //  s = s.replace("})", "}").replace(",}", "}");
    output.replace("})", "}").replace(",}", "}")
}

fn get_images(input: &str) -> Vec<ContainerImageBluePrint> {
    let images_services: Vec<(String, String, Option<String>)> =
        Regex::new(r####"((const|let) ?(?P<serviceName>.+) ?= ?)?new docker.Image\("(?P<name>.+)",( ?)(?P<value>\{(\n.+)+[^;s"\n.+])"####)
            .unwrap()
            .captures_iter(&input)
            .map(|container| {

                let service_name = match container.name("serviceName") {
                    Some(v) => {
                        Some(v.as_str().trim().to_string())
                    },
                    None => None,
                };

                (container["name"].to_owned(), container["value"].to_owned(), service_name)
})
            .collect();

    let mut images: Vec<ContainerImageBluePrint> = vec![];

    for (_image_name, image, service_name) in images_services {
        let mut s = String::from("");

        for line in image.trim().lines() {
            let parsed_line = parse_line(line);

            s.push_str(&parsed_line);
        }

        s = prune_output(s)
            // Add custom behavior
            .replace("imageName", "name");

        let mut serialized: ContainerImageBluePrint = serde_json::from_str(&s).unwrap();

        if service_name.is_some() {
            serialized.name = Some(service_name.clone().unwrap());
            serialized.reference_name = Some(format!("{}.imageName", service_name.unwrap()));
        }

        images.push(serialized);
    }

    images
}

fn get_apps(input: &str) -> Vec<ContainerAppBluePrint> {
    let container_app_services: Vec<(String, String)> = Regex::new(
        r####"new app.ContainerApp\("(?P<name>.+)",( ?)(?P<value>\{(\n.+)+[^;s"\n.+])"####,
    )
    .unwrap()
    .captures_iter(&input)
    .map(|container| (container["name"].to_owned(), container["value"].to_owned()))
    .collect();

    let mut containers: Vec<ContainerAppBluePrint> = vec![];

    for (_container_name, container) in container_app_services {
        let mut s = String::from("");

        for line in container.trim().lines() {
            let parsed_line = parse_line(line);
            s.push_str(&parsed_line);
        }

        s = prune_output(s);

        let serialized: ContainerAppBluePrint = serde_json::from_str(&s).unwrap();

        containers.push(serialized);
    }

    containers
}

pub fn deserialize(input: &str) -> Result<Vec<ContainerAppConfiguration>, String> {
    let images = get_images(&input);
    let apps = get_apps(&input);

    let services = pulumi::build_configuration(apps, images);

    match services {
        Some(val) => Ok(val),
        None => Err("No container to deserialize".to_string()),
    }
}

mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {}

    #[test]
    fn test_get_images() {}

    #[test]
    fn test_get_apps() {}
}
