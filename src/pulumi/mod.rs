pub mod js;
pub mod yaml;
use log::error;

use crate::serializer::{ContainerAppConfiguration, Language, Serializer};

pub struct Pulumi {
    language: Language,
    pub resources: Option<Vec<ContainerAppConfiguration>>,
}

impl Pulumi {
    pub fn new(language: Language) -> Option<Pulumi> {
        match language {
            Language::Yaml | Language::Typescript | Language::Javascript => Some(Pulumi {
                language,
                resources: None,
            }),
            _ => None,
        }
    }
}

impl Serializer for Pulumi {
    type Output = Pulumi;
    fn deserialize_value(&mut self, input: &str) -> Option<&Self> {
        match self.language {
            Language::Yaml => match yaml::deserialize(input) {
                Some(value) => {
                    self.resources = Some(value);
                    Some(self)
                }
                None => None,
            },
            Language::Typescript | Language::Javascript => match js::deserialize(input) {
                Some(value) => {
                    self.resources = Some(value);
                    Some(self)
                }
                None => None,
            },
            _ => {
                error!("Language not supported");
                None
            }
        }
    }
}
