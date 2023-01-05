use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::serializer::ContainerAppConfiguration;

#[derive(Serialize, Deserialize, Debug)]
struct Image {
    imageName: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Dapr {
    enabled: String,
    appPort: String,
    appId: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct Ingress {
    external: String,
    targetPort: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Configuration {
    dapr: Dapr,
}
#[derive(Serialize, Deserialize, Debug)]
struct Container {
    resourceGroupName: String,
    managedEnvironmentId: String,
    configuration: Configuration,
}

fn parse_line(line: &str) -> String {
    let a = line.replace(" ", "");
    let re = Regex::new(r####"([a-zA-Z"]+)(:)([a-zA-Z0-9.`$"\{}\[\]]+)?"####).unwrap();

    // println!("{a}");
    let captures = re.captures(&a);
    let computed = match captures {
        Some(c) => {
            let key = c.get(1).unwrap().as_str();
            let value = if c.get(3).is_some() {
                let computed = c.get(3).unwrap().as_str();
                let with_quotes = format!("\"{}\",", computed);
                let tokens = ["{", "[{"];
                let has_token = tokens.contains(&computed);

                if has_token {
                    computed.to_string()
                } else {
                    with_quotes
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

pub fn deserialize(input: &str) -> Option<Vec<ContainerAppConfiguration>> {
    let images_services: Vec<(String, String)> =
        Regex::new(r####"new docker.Image\("(?P<name>.+)",( ?)(?P<value>\{(\n.+)+[^;s"\n.+])"####)
            .unwrap()
            .captures_iter(&input)
            .map(|container| (container["name"].to_owned(), container["value"].to_owned()))
            .collect();

    for (_image_name, image) in images_services {
        let mut s = String::from("");

        for line in image.trim().lines() {
            println!("{line}");
            let parsed_line = parse_line(line);
            s.push_str(&parsed_line);
        }

        s = s.replace("})", "}").replace(",}", "}");

        let to_json: Image = serde_json::from_str(&s).unwrap();
        let to_yaml = serde_yaml::to_value(to_json);
        println!("{:?}", to_yaml);
    }

    let container_app_services: Vec<(String, String)> = Regex::new(
        r####"new app.ContainerApp\("(?P<name>.+)",( ?)(?P<value>\{(\n.+)+[^;s"\n.+])"####,
    )
    .unwrap()
    .captures_iter(&input)
    .map(|container| (container["name"].to_owned(), container["value"].to_owned()))
    .collect();

    for (_container_name, container) in container_app_services {
        let mut s = String::from("");

        for line in container.trim().lines() {
            let parsed_line = parse_line(line);
            s.push_str(&parsed_line);
        }

        s = s.replace("})", "}").replace(",}", "}");

        let to_json: Container = serde_json::from_str(&s).unwrap();
        let to_yaml = serde_yaml::to_value(to_json);
        println!("{:?}", to_yaml);
    }

    None
}
