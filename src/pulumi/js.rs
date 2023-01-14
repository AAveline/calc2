use crate::pulumi;
use regex::Regex;
use std::panic;

use crate::serializer::{
    BuildContextBluePrint, ContainerAppBluePrint, ContainerAppConfiguration,
    ContainerImageBluePrint,
};

fn parse_line(line: &str) -> String {
    let a = line.replace(" ", "");
    let re = Regex::new(r####"([a-zA-Z"]+)(:)([a-zA-Z0-9-:.`'/"\{}\[\]]+)?"####).unwrap();

    let captures = re.captures(&a);

    let computed = match captures {
        Some(c) => {
            let key = c.get(1).unwrap().as_str();
            let value = if c.get(3).is_some() {
                let mut computed = c.get(3).unwrap().as_str();
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
            let value = value
                .replace("\"\"", "\"")
                .replace("`", "")
                .replace("'", "");

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
    use crate::serializer::{
        ConfigurationBluePrint, ContainerBluePrint, DaprBluePrint, IngressBluePrint,
        TemplateBluePrint,
    };

    use super::*;

    #[test]
    fn test_parse_line() {
        let output = parse_line("key: ");
        assert_eq!("\"key\":", output);

        let output = parse_line("key: value");
        assert_eq!("\"key\":\"value\",", output);

        let output = parse_line("\"key\":\"value\"");
        assert_eq!("\"key\":\"value\",", output);

        let output = parse_line("key:\"value\"");
        assert_eq!("\"key\":\"value\",", output);

        let output = parse_line("\"key\":value");
        assert_eq!("\"key\":\"value\",", output);

        let output = parse_line("\"key\":value,");
        assert_eq!("\"key\":\"value\",", output);

        let output = parse_line("\"key\":'value',");
        assert_eq!("\"key\":\"value\",", output);

        let output = parse_line("\"key\":`value`,");
        assert_eq!("\"key\":\"value\",", output);

        let output = parse_line("\"key\":\"node-12\",");
        assert_eq!("\"key\":\"node-12\",", output);

        let output = parse_line("\"key\":\"node:12\",");
        assert_eq!("\"key\":\"node:12\",", output);

        let output = parse_line("\"key\":\"node:12.4\",");
        assert_eq!("\"key\":\"node:12.4\",", output);
    }

    #[test]
    fn test_get_images() {
        // No valid resource
        let data = r####"
        const test = new NoResource() {}
        "####;
        let output = get_images(data);
        let expected: Vec<ContainerImageBluePrint> = vec![];
        assert_eq!(expected, output);

        // Valid resource name and context with reference
        let data = r####"
        const remixImage = new docker.Image("remix", {
            imageName: pulumi.interpolate`${registry.loginServer}/remix:v1`,
            build: { 
                context: "../frontend",
            },
        });"####;

        let output = get_images(data);
        let expected = vec![ContainerImageBluePrint {
            name: Some("remixImage".to_string()),
            build: BuildContextBluePrint {
                context: "../frontend".to_string(),
            },
            reference_name: Some("remixImage.imageName".to_string()),
        }];

        assert_eq!(expected, output);

        // Valid resource name and context without reference
        let data = r####"
        const remixImage = new docker.Image("remix", {
            imageName: "node-18",
            build: { 
                context: "../frontend",
            },
        });"####;

        let output = get_images(data);
        let expected = vec![ContainerImageBluePrint {
            name: Some("remixImage".to_string()),
            build: BuildContextBluePrint {
                context: "../frontend".to_string(),
            },
            reference_name: Some("remixImage.imageName".to_string()),
        }];

        assert_eq!(expected, output);

        // TODO
        // Valid resource name and context with direct string path
        /*let data = r####"
              const remixImage = new docker.Image("remix", {
                  imageName: "node-18",
                  build: "",
              });"####;

        let output = get_images(data);
        let expected = vec![ContainerImageBluePrint {
            name: Some("remixImage".to_string()),
            build: BuildContextBluePrint {
                context: "../frontend".to_string(),
            },
            reference_name: Some("remixImage.imageName".to_string()),
        }];

        assert_eq!(expected, output);*/

        // Invalid build context
        let data = r####"
                const remixImage = new docker.Image("remix", {
                    imageName: "node-18",
                    
                });"####;

        let expected = panic::catch_unwind(|| get_images(data));

        assert!(expected.is_err());
    }

    #[test]
    fn test_get_apps() {
        // No valid resource
        let data = r####"
                const test = new NoResource() {}
                "####;
        let output = get_apps(data);
        let expected: Vec<ContainerAppBluePrint> = vec![];
        assert_eq!(expected, output);

        // TODO
        // Valid resource name and context with reference
        /*
        let data = r####"
                const frontendApp = new app.ContainerApp("frontend", {
                    resourceGroupName: resourceGroup.name,
                    managedEnvironmentId: managedEnv.id,
                    configuration: {
                        dapr: {
                            enabled: true,
                            appPort: 8000,
                            appId: "remix"
                        },
                        ingress: {
                            external: true,
                            targetPort: 8000,
                        },
                    },
                    template: {
                        containers: [{
                            name: "remix",
                            image: "node:12",
                        }],
                    },
                });"####;

        let output = get_apps(data);
        let expected = vec![ContainerAppBluePrint {
            configuration: ConfigurationBluePrint {
                dapr: Some(DaprBluePrint {
                    app_id: Some("remix".to_string()),
                    app_port: Some(8000),
                    enabled: Some(true),
                }),
                ingress: Some(IngressBluePrint {
                    external: Some(true),
                    target_port: Some(8000),
                }),
            },
            template: TemplateBluePrint {
                containers: Some(vec![ContainerBluePrint {
                    image: "node:12".to_string(),
                    name: "remix".to_string(),
                }]),
            },
        }];

        assert_eq!(expected, output);
        */
    }
}
