use crate::pulumi;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::serializer::{
    ContainerAppBluePrint, ContainerAppConfiguration, ContainerImageBluePrint,
};

fn parse_line(line: &str) -> String {
    let a = line.replace(" ", "");
    let re = Regex::new(r####"([a-zA-Z"]+)(:)([a-zA-Z0-9.`$"\{}\[\]]+)?"####).unwrap();

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
                    let re = Regex::new(r"^[0-9]+").unwrap().is_match(computed);

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
            let value = value.replace("\"\"", "\"");

            let computed = format!("{key}:{value}");

            computed
        }
        None => a.to_string(),
    };
    computed
}

fn get_images(input: &str) -> Vec<ContainerImageBluePrint> {
    let images_services: Vec<(String, String)> =
        Regex::new(r####"new docker.Image\("(?P<name>.+)",( ?)(?P<value>\{(\n.+)+[^;s"\n.+])"####)
            .unwrap()
            .captures_iter(&input)
            .map(|container| (container["name"].to_owned(), container["value"].to_owned()))
            .collect();
    let mut images: Vec<ContainerImageBluePrint> = vec![];

    for (image_name, image) in images_services {
        let mut s = String::from("");

        for line in image.trim().lines() {
            let parsed_line = parse_line(line);
            s.push_str(&parsed_line);
        }

        s = s.replace("})", "}").replace(",}", "}");

        let mut serialized: ContainerImageBluePrint = serde_json::from_str(&s).unwrap();

        serialized.reference_name = Some(image_name);
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

        s = s.replace("})", "}").replace(",}", "}");

        let serialized: ContainerAppBluePrint = serde_json::from_str(&s).unwrap();

        containers.push(serialized);
    }

    containers
}

pub fn deserialize(input: &str) -> Result<Vec<ContainerAppConfiguration>, String> {
    let images = get_images(&input);
    let apps = get_apps(&input);

    let services = pulumi::build_configuration(apps, images);

    Ok(services)

    // TODO: Refacto this
    // Err("An error occured".to_string())
}
