# Model Policy

Habilitation de sécurité pour modèles d'IA — verdicts explicables contre la politique de l'organisation (couche 1).

Pour les équipes qui veulent employer des modèles d'IA en respectant leur politique de sécurité, qui rencontre des choix de modèles opaques, impossibles à justifier auprès de la sécurité ou d'un auditeur, ce projet permet de vérifier qu'un besoin métier peut employer un modèle donné, avec un verdict explicable, en produisant des verdicts d'habilitation explicables, rejouables et conformes à la référence, sans dépendre de : aucun scoring opaque, aucune donnée envoyée à des tiers.

## État du projet

<!-- libre-ai:project-status:begin -->
<!-- Section générée depuis project.v1.yaml — ne pas éditer à la main. -->

- Situation actuelle : L'application Model Policy et le crate policy-core (frontière WIT vendorée, implémentation de référence policy-core-ref exposée à la constellation) sont greffés et verts ; le plan de livraison par phases (docs/apps/model-policy) voyage avec le produit, ses phases restent à exécuter.
- Maturité : usable
- Exposition : spec-published
- Confiance : medium
- Preuves vérifiées le : 2026-07-30
- Avancement : 20 % du périmètre actuellement déclaré

<!-- libre-ai:project-status:end -->

## Vérifier

- `bun install && bun run check` — la chaîne de gates du dépôt, tests inclus.
- La fiche [`project.v1.yaml`](./project.v1.yaml) est l'autorité de l'état du projet ; la section « État du projet » ci-dessus en est générée et un gate de flotte échoue si elles divergent.
- La provenance de chaque chemin migré depuis le hub est tracée dans l'index de migration de `libre-ai/libre-ai` (`ecosystem/migration-index.v1.yaml`).
