pub mod pulumi;
pub mod serializer;
pub mod utils;

use clap::{Parser, ValueEnum};
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

fn parse_language(filename: &str) -> Result<Language, Language> {
    let language = Path::new(filename).extension().and_then(|val| val.to_str());

    match language {
        Some("yml" | "yaml") => Ok(Language::Yaml),
        Some("ts") => Ok(Language::Typescript),
        Some("bicep") => Ok(Language::Bicep),
        Some("json") => Ok(Language::Json),
        _ => Err(Language::NotSupported),
    }
}

fn main() {
    let args = Args::parse();

    let file = fs::read_to_string(&args.input);

    match file {
        Ok(file) => {
            let language = match parse_language(&args.input) {
                Ok(r) => r,
                Err(e) => e,
            };

            match args.provider {
                Provider::Pulumi => {
                    let mut provider = Pulumi::new(language).expect("Language is not supported");

                    let value = provider
                        .deserialize_value(&file)
                        .expect("Deserialiazed value is defined");

                    match value.serialize_value(&value.resources.as_ref().unwrap()) {
                        Ok(v) => {
                            if Path::new(FILENAME).exists() {
                                let old_file = fs::read_to_string(Path::new(FILENAME));

                                match fs::write("docker-compose.old.yml", old_file.unwrap()) {
                                    Ok(_r) => utils::write("Old file saved".to_string()),
                                    Err(e) => utils::write_err(e.to_string()),
                                };
                            }

                            fs::write(FILENAME, v).unwrap();
                        }
                        Err(e) => utils::write_err(e.to_string()).unwrap(),
                    }
                }
                Provider::Azure => todo!(),
                Provider::Terraform => todo!(),
            }
        }
        Err(e) => println!("{}", e),
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
