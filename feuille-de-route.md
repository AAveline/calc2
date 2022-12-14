# CALC2 (ContainerApps Local Compose Convertor)

## Objectives:
Permettre, via un outil CLI, de pouvoir convertir depuis un ensemble d'outil IAC, des stacks remote vers leur contrepartie locale, émulées sur compose, dans un premier temps.

## what 
Créer un convertisseur permettant de faire le pont entre Pulumi/Bicep/Terraform vers compose. Le but serait de créer un ensemble de binding generique permettant de trouver leur contrepartie sur compose.
Exemple:
input::pulumi => struct pulumi => parse struct => struct compose => output::compose.yml
input::bicep => struct bicep => parse struct => struct compose => output::compose.yml

## how
cf parser précédent. 
- créer le CLI en utilisant le crate suivant https://docs.rs/clap/latest/clap/ acceptant les inputs suivants:
  - le type de convertisseur a utiliser (pulumi/azure/tf)
  - le langage selectionné si necessaire (pulumi => typescript, python etc...)
  - le format d'output
  - le dossier d'output
- extraire l'api Pulumi de container apps => https://www.pulumi.com/registry/packages/azure-native/api-docs/app/containerapp/ => créer les structs equivalents 
- extraire l'api Bicep de container apps => https://learn.microsoft.com/en-us/azure/templates/microsoft.app/containerapps?pivots=deployment-language-bicep => créer les struct equivalents
- créer le parser capable de translate les structs pulumi/bicep vers les structs compose
- créer la méthode permettant de convertir les structs compose dans leur equivalent yaml pour pouvoir ecrire dans le dossier d'output les fichiers docker-compose.yaml


## Structure
- lib.rs
  - pulumi
    - yaml.rs
    - typescript.rs
    - convertor.rs
  - azure
    - bicep.rs
    - arm.rs
    - convertor.rs
  - terraform
    - hcl.rs
    - typescript.rs
    - convertor.rs
  - compose
    - yaml.rs
    - translator.rs
  - bin
    - cli



# TODO:
Ignorer les champs qui sont None lors de la serialization compose
Parser Typescript pour Pulumi

