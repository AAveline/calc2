pub mod pulumi;
pub mod serializer;

use clap::{Parser, ValueEnum};

use log::{error, info};
use pulumi::Pulumi;
use serializer::{Language, Serializer};
use std::{fs, path::Path};

const FILENAME: &str = "docker-compose.yml";
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Convertor type (eg: pulumi, azure, terraform)
    #[arg(value_enum)]
    provider: Provider,

    /// input file to convert
    #[arg(short, long)]
    input: String,
    // Output folder
    //#[arg(short, long)]
    //output: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, ValueEnum)]
enum Provider {
    /// Provider for Pulumi
    Pulumi,
    /// Provider for Azure
    Azure,
    /// Provider for Terraform
    Terraform,
}

fn parse_language(filename: &str) -> Language {
    let language = Path::new(filename).extension().and_then(|val| val.to_str());

    match language {
        Some("yml" | "yaml") => Language::Yaml,
        Some("ts") => Language::Typescript,
        Some("bicep") => Language::Bicep,
        Some("json") => Language::Json,
        _ => Language::NotSupported,
    }
}

fn main() {
    simple_logger::init().unwrap();
    let args = Args::parse();

    info!("Starting...");

    let file = fs::read_to_string(&args.input);

    match file {
        Ok(file) => {
            let language = parse_language(&args.input);

            match args.provider {
                Provider::Pulumi => {
                    let mut provider =
                        Pulumi::new(language).expect("Language is not supported for this provider");

                    let value = provider
                        .deserialize_value(&file)
                        .expect("Deserialiazed value is defined");

                    match value.serialize_value(&value.resources.as_ref().unwrap()) {
                        Ok(v) => {
                            if Path::new(FILENAME).exists() {
                                let old_file = fs::read_to_string(Path::new(FILENAME));

                                match fs::write("docker-compose.old.yml", old_file.unwrap()) {
                                    Ok(_r) => {
                                        info!("Previous compose file dumped to >> docker-compose.old.yml")
                                    }
                                    Err(e) => error!("{}", e),
                                };
                            }

                            fs::write(FILENAME, v).unwrap();

                            info!("Completed!")
                        }
                        Err(e) => error!("{}", e),
                    }
                }
                Provider::Azure => todo!(),
                Provider::Terraform => todo!(),
            }
        }
        Err(e) => error!("{}", e),
    }
}

#[cfg(test)]
mod tests {
    #[warn(dead_code)]
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}
