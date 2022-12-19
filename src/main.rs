pub mod pulumi;
pub mod serializer;

use clap::{Parser, ValueEnum};
use pulumi::Pulumi;
use serializer::{serialize_to_compose, Extension, Serializer};

use std::{fs, path::Path};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Convertor type (eg: pulumi, azure, terraform)
    #[arg(value_enum)]
    provider: Provider,

    /// input file to convert
    #[arg(short, long)]
    input: String,

    /// Output folder
    #[arg(short, long)]
    output: String,
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

fn parse_extension(filename: &str) -> Result<Extension, Extension> {
    let extension = Path::new(filename).extension().and_then(|val| val.to_str());

    match extension {
        Some("yml" | "yaml") => Ok(Extension::Yaml),
        Some("ts") => Ok(Extension::Typescript),
        Some("bicep") => Ok(Extension::Bicep),
        Some("json") => Ok(Extension::Json),
        _ => Err(Extension::NotSupported),
    }
}

fn main() -> Result<(), ()> {
    let args = Args::parse();

    let file = fs::read_to_string(&args.input);

    match file {
        Ok(file) => {
            let extension = match parse_extension(&args.input) {
                Ok(r) => r,
                Err(e) => e,
            };

            match args.provider {
                Provider::Pulumi => {
                    let a = Pulumi::new(args.output, &extension);
                    match a.deserialize_value(&file) {
                        Ok(value) => {
                            let a = match serialize_to_compose(value) {
                                Ok(v) => v,
                                Err(v) => vec![],
                            };
                            fs::write("docker-compose.yml", a).expect("Should write file");
                            // println!("{:?}", a)
                        }
                        Err(e) => todo!(),
                    };
                }
                Provider::Azure => todo!(),
                Provider::Terraform => todo!(),
            }
        }
        Err(err) => println!("Pas ok"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}
