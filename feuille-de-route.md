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
- créer le CLI en utilisant le crate suivant https://docs.rs/clap/latest/clap/ 
- extraire l'api Pulumi de container apps => https://www.pulumi.com/registry/packages/azure-native/api-docs/app/containerapp/ => créer les structs equivalents 
- extraire l'api Bicep de container apps => https://learn.microsoft.com/en-us/azure/templates/microsoft.app/containerapps?pivots=deployment-language-bicep => créer les struct equivalents
- créer le parser capable de translate les structs pulumi/bicep vers les structs compose
- créer la méthode permettant de convertir les structs compose dans leur equivalent yaml pour pouvoir ecrire dans le dossier d'output les fichiers docker-compose.yaml


## TODO
- [ ] Parser la propriété `dapr`, vérifier si `enabled` est a true, si c'est le cas, alors en déduire les ports à mapper dans le fichier compose dans le service dapr afférent (parser le targetPort)
- [ ] Parser la propriété `ingress` pour savoir si le service doit etre exposé via des ports publiques dans le fichier compose (parser le targetPort) 
- [ ] Gérer correctement les erreurs
- [ ] Mettre en place un système de warning pour les valeurs non-déductibles comme:
  - [ ] Lorsque une référence est faite au registry pour le nom des images
  - [ ] A incrémenter selon