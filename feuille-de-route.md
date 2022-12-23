
## TODO
- [x] Parser la propriété `dapr`, vérifier si `enabled` est a true, si c'est le cas, alors en déduire les ports à mapper dans le fichier compose dans le service dapr afférent (parser le targetPort)
- [x] Parser la propriété `ingress` pour savoir si le service doit etre exposé via des ports publiques dans le fichier compose (parser le targetPort) 
- [x] Vérifier lors de la génération du fichier `docker-compose` s'il existe déjà, si oui, générer ac son contenu un fichier docker-compose.old pour préserver le contenu et éviter une réconciliation pénible
- [ ] Gérer correctement les erreurs
- [ ] Output messages in stdout/stderr
- [ ] Implémenter tests pour le Provider Pulumi
- [ ] Mettre en place un système de warning pour les valeurs non-déductibles comme:
  - [ ] Lorsque une référence est faite au registry pour le nom des images
  - [ ] A incrémenter selon