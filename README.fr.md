[English](README.md) · **Français**

> [!NOTE]
> **Réservé · futur foyer de Model Policy** — reconstruit dans le dépôt de base canonique [`libre-ai/libre-ai`](https://github.com/libre-ai/libre-ai) ([topologie multi-dépôts, ADR-0008](https://github.com/libre-ai/libre-ai/blob/main/docs/adr/0008-multi-repo-target-topology-and-brand.md)).
> Ce dépôt rouvrira comme dépôt produit réel lorsque le propriétaire l'activera, consommant la base comme dépendance versionnée. Les fondations décrites ci-dessous sont **en cours de construction** — avec des liens vers le code qui existe déjà.

# Model Policy

**Habilitation de sécurité pour les modèles d'IA.** Confrontez un besoin métier — type de tâche, finalité de traitement, sensibilité des données — à la politique de sécurité de votre organisation — pays bannis, juridictions, licences, exigences d'hébergement, prix — et obtenez un **verdict explicable, règle par règle** pour chaque modèle : `eligible`, `ineligible` ou `indeterminate`. Jamais un défaut silencieux.

Le cas canonique auquel il répond : _« pas les US, pas la Chine, mais l'auto-hébergé convient »_ — exprimé comme une contrainte sur **où circulent les données d'inférence** (juridiction, conscient du CLOUD Act), indépendamment de **qui a créé le modèle** (origine). Les deux dimensions sont de premier plan et contraignables séparément.

## Ce qui le distingue

- **Explicable, pas un score.** Chaque verdict est traçable règle par règle — vous voyez _quelle_ règle a échoué et _pourquoi_, sur quel fait sourcé. Un modèle non conforme est écarté, avec son raisonnement consultable — jamais listé « avec un avertissement ».
- **Refus par défaut.** Un modèle inconnu, un fait manquant sur une dimension requise, ou un chemin d'hébergement non documenté n'est jamais `eligible`.
- **Déterministe et rejouable.** Une même politique, un même instantané et un même besoin produisent toujours une preuve octet-pour-octet identique. Les verdicts sont reproductibles et auditables, pas l'opinion d'un modèle.
- **Filtrer, puis classer.** L'éligibilité (domaine de la sécurité) est strictement séparée du classement par benchmark/prix (domaine du métier).
- **Faits sourcés uniquement.** Les politiques reposent sur un socle de règles sourcé ; chaque dérogation est nommée et tracée. Les faits sur les modèles portent leur provenance.

## État — spécifié publiquement, fondations en construction

Model Policy est reconstruit à partir de contrats verrouillés. Il **n'est pas encore publié** ; le cœur d'évaluation déterministe vient d'abord, et une bonne partie existe déjà et est prouvée dans le dépôt de base :

| Fondation                                                          | État                      | Preuve                                                                                                                                                  |
| ------------------------------------------------------------------ | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`policy-core`** — moteur d'évaluation Rust déterministe          | ✅ construit, octet-exact | Conformité 20/20 sur les vecteurs golden face au `SEMANTICS.md` normatif ([#212](https://github.com/libre-ai/libre-ai/pull/212))                        |
| **`policy-core` → WASM** — composant sans capacité                 | ✅ construit              | Aucun import hôte — ni horloge, ni réseau, ni système de fichiers, ni aléa, ni identité ([#214](https://github.com/libre-ai/libre-ai/pull/214))         |
| **Évaluation côté serveur** — l'app consomme le composant          | ✅ câblé                  | L'hôte Bun instancie le WASM et évalue ; 20/20 octet-exact en direct ([#215](https://github.com/libre-ai/libre-ai/pull/215))                            |
| **`policy-core-ref`** — évaluateur de référence TypeScript         | ✅ publié                 | Sémantique octet-identique, conformité 144 vecteurs ([#207](https://github.com/libre-ai/libre-ai/pull/207))                                             |
| Validateurs d'édition — politique / instantané / besoin            | ✅ construits             | Fermés par défaut, conformes aux contrats ([#169](https://github.com/libre-ai/libre-ai/pull/169)–[#181](https://github.com/libre-ai/libre-ai/pull/181)) |
| Surface de commandes — autoriser, persister, exporter, UI de trace | ⏳ suite                  | Autorisation Biscuit, isolation par locataire, preuve de rejeu                                                                                          |

Ce dépôt est `private` jusqu'à ce qu'un audit de secrets autorise sa réouverture publique (vague 4). **Cible de référence :** l'outillage de gouvernance de registres/cartes de modèles (p. ex. Hugging Face Hub) — atteinte par une habilitation explicable et refusée par défaut plutôt que par la découverte.

## Comment ça fonctionne

1. **Éditer** — les éditeurs écrivent une politique versionnée de règles d'éligibilité sourcées sur les faits des modèles ; les approbateurs acceptent des versions de politique **immuables** (un proposeur ne peut approuver sa propre version).
2. **Instantané** — importer des faits sourcés sur le modèle/fournisseur, valider provenance et licence, et figer un instantané adressé par contenu.
3. **Évaluer** — déclarer un besoin borné, lancer une évaluation **locale et déterministe**, et inspecter le verdict avec ses règles échouées et inconnues et leur preuve. La révocation bloque toute nouvelle évaluation mais ne réécrit jamais la preuve passée.

## Architecture — assemblé à partir de briques interopérables

Model Policy est un produit assemblé à partir de briques versionnées indépendamment ; chacune est utilisable et testable seule, et le produit est leur composition (la cible multi-dépôts de [l'ADR-0008](https://github.com/libre-ai/libre-ai/blob/main/docs/adr/0008-multi-repo-target-topology-and-brand.md)).

| Brique                                       | Rôle                                                | Interface exposée / consommée                                                                                                    |
| -------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **`policy-core`** (Rust → composant WASM)    | Le moteur d'évaluation déterministe                 | Monde WIT `policy-core` : `evaluate(policy, snapshot, need, evaluated-at) → evaluation`, sans capacité                           |
| **`@libre-ai/policy-core-ref`** (TypeScript) | Évaluateur de référence, sémantique octet-identique | Même contrat d'évaluation, pour recoupement et usage côté JS                                                                     |
| **`@libre-ai/web-platform`**                 | Fondation SSR / BFF Bun                             | Gestionnaire de requêtes, document accessible rendu côté serveur                                                                 |
| **Contrats**                                 | Surface d'interopérabilité verrouillée              | Schémas `policy-definition.v2`, `model-snapshot.v2`, `policy-need.v2`, `policy-evaluation.v2` + vecteurs golden + `SEMANTICS.md` |

L'hôte qui autorise passe au moteur les octets canoniques de politique/instantané/besoin ; le moteur ne détient aucun jeton et n'atteint aucune capacité. Tout consommateur qui parle les mêmes contrats peut piloter la même évaluation.

## Où se déroule le travail

Tout le développement actif est dans le dépôt de base, sous :

- `apps/model-policy` — l'hôte produit (cockpit SSR, évaluation côté serveur)
- `crates/policy-core` — le moteur Rust et son composant WASM
- `packages/policy-core-ref` — l'évaluateur de référence TypeScript
- `contracts/` — les schémas verrouillés, le monde WIT et les vecteurs golden
- [`docs/apps/model-policy.md`](https://github.com/libre-ai/libre-ai/blob/main/docs/apps/model-policy.md) — le cahier des charges produit complet

Pour suivre l'avancement ou contribuer, ouvrez issues et pull requests dans [`libre-ai/libre-ai`](https://github.com/libre-ai/libre-ai). Ce dépôt reste réservé jusqu'à son activation.

## Licence

EUPL-1.2.
