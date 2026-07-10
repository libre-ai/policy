# rumble-ai-clearance — Design

Statut : validé (brainstorming section par section, 2026-07-10).

## Problème

Une équipe (persona entreprise) doit savoir **quel modèle IA utiliser selon son
besoin métier** — type de tâche, finalité de traitement, sensibilité des
données — sous les **contraintes de sa sécurité groupe** : pays interdits,
open/closed source, auto-hébergement, prix. Exemple canonique : « 0 USA et
Chine, mais auto-hébergé OK ». Les sources croisées : benchmarks
[Artificial Analysis](https://artificialanalysis.ai/) (AA) et catalogue
[Hugging Face](https://huggingface.co/models) (HF).

## Décisions structurantes

| Décision    | Choix                                                                                                      |
| ----------- | ---------------------------------------------------------------------------------------------------------- |
| Forme       | Core Rust + CLI + web UI + serveur axum dès v1                                                             |
| Périmètre   | LLM d'abord, modèle de données extensible (embeddings/ASR/image non peuplés en v1)                         |
| Données     | Snapshot versionné, provenance par champ ; toute décision cite sa version de dataset                       |
| Déploiement | Auto-déployé par org ; politique = YAML versionné chez l'org ; zéro multi-tenant, zéro auth applicative v1 |
| Taxonomie   | Rulebook défaut sourcé + overrides org (désactivations explicites et tracées)                              |

**Contrainte juridique** (vérifiée 2026-07-10,
[data-api](https://artificialanalysis.ai/data-api)) : le tier gratuit AA est
« _Internal use only; no redistribution_ », 100 req/jour, attribution
obligatoire. Le produit livre **le pipeline, jamais les données AA** : chaque
org synchronise avec sa propre clé (usage interne conforme). Les métadonnées HF
(publiques) et le dataset de gouvernance curaté sont librement versionnables.
La démo publique n'expose aucune donnée AA.

## 1. Trois datasets à régimes juridiques distincts

| Dataset                  | Contenu                                                                                                                              | Source                                                | Public ?              |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------- | --------------------- |
| **Catalogue**            | Identité modèles, licence, poids ouverts, contexte, modalités, gated                                                                 | API HF + AA (identités)                               | Oui                   |
| **Gouvernance** (curaté) | Provider → pays siège ; classification `closed`/`open_weight`/`open_source` ; auto-hébergeabilité ; restrictions licence commerciale | Curation manuelle **sourcée** (`content/governance/`) | Oui                   |
| **Benchmarks**           | Indices AA (Intelligence, Coding, Agentic, Math, Multilingual), prix $/Mtok, vitesse                                                 | API AA, clé de l'org                                  | **Non** — interne org |

Snapshot org = fusion des trois + provenance par champ (source, date, version
schéma) + manifest daté. Écriture atomique : un sync partiellement échoué ne
remplace pas le snapshot précédent.

## 2. Origine ≠ hébergement

- `origin` : pays du créateur du modèle (Meta→US, Mistral→FR, DeepSeek→CN).
  Curaté par provider.
- `hosting` : où passent les données à l'inférence — `self_hosted` /
  `eu_sovereign_api` / `provider_api` / `hyperscaler_api` — avec pays **et
  juridiction applicable** (un provider US hébergé en UE reste soumis au CLOUD
  Act ; la donnée porte la nuance, la politique décide).

« Données sensibles : aucun flux US/CN mais Llama auto-hébergé OK » =
contrainte sur `hosting.jurisdiction`. Une org plus stricte peut aussi bannir
par `origin`.

## 3. Moteur d'éligibilité (`crates/domain`)

Fonctions pures, zéro I/O. Entrées : snapshot + politique compilée + profil de
besoin. Sortie par modèle : `Eligible` / `Ineligible { règles déclenchées }` /
`Indeterminate { données manquantes }` — verdict explicable règle par règle.

**Deny-by-default** : modèle inconnu → inéligible ; donnée manquante sur une
dimension requise → `Indeterminate`, traité non-éligible (fail-closed).

**Filtrer puis classer, jamais mélanger** : éligibilité = AND de toutes les
contraintes actives (domaine sécurité) ; classement = tri des seuls éligibles
par dimensions bench de la tâche puis prix (domaine métier). Un modèle non
conforme n'apparaît jamais en liste ; son verdict reste consultable.

## 4. Politique : rulebook ⊕ org ⊕ besoin

1. **Rulebook défaut** (`content/rulebook/`, YAML) : tâches → dimensions bench
   - exigences capacitaires ; sensibilités → exigences d'hébergement ;
     finalités → contraintes légales. Chaque règle est sourcée.
2. **Politique org** (YAML chez l'org) : bans origin/juridiction/provider/
   licence, seuils par sensibilité, overrides et désactivations **nommées et
   tracées** de règles défaut.
3. **Profil de besoin** (UI ou fichier) : tâche + finalité + sensibilité.

Politique effective compilée et validée fail-closed (invalide → refus
d'évaluer).

Taxonomie v1 : tâches = {génération de code, agentique/outils,
résumé/extraction, classification, rédaction, traduction/multilingue,
raisonnement/analyse, chat général} ; sensibilité = C0 public → C3 restreint
(labels renommables ; défauts : C2 → juridiction UE ou self-host, C3 →
self-host uniquement) ; finalités RGPD-alignées (PII, décision automatisée,
santé, contenu public…).

## 5. Composants

```
crates/domain      types + moteur pur (natif + wasm)
crates/policy      schémas + parsing + merge rulebook⊕org + validation fail-closed
crates/dataset     schéma snapshot, manifest, provenance, chargement
crates/sync        connecteurs AA / HF / curaté → snapshot atomique
crates/cli         sync | validate | evaluate --need | explain <model> | check <model> (exit 0/1)
crates/api         axum read-only
apps/web           Dioxus WASM double mode
content/           rulebook défaut + gouvernance curatée
schemas/           JSON Schema : policy, need, snapshot, governance
```

- **API axum read-only par construction** : aucune mutation HTTP — politique et
  snapshot ne changent que par fichier + redéploiement (pas d'authz applicative
  v1 ; l'exposition réseau relève de l'org). Refuse de démarrer si politique ou
  snapshot invalides. Endpoints (envelope `{data, meta}`, pagination cursor) :
  `GET /api/v1/models`, `POST /api/v1/evaluations`,
  `GET /api/v1/models/:id/verdict`, `GET /api/v1/dataset`,
  `GET /api/v1/policy`. Zéro PII en logs.
- **Web UI double mode** : local (moteur WASM, fichiers chargés dans le
  navigateur, rien ne sort — mode de la démo publique) ; serveur (client de
  l'API org). Vues : explorateur filtrable (pays, juridiction, openness,
  licence, hébergement, prix, indices), assistant besoin, fiche modèle
  (verdict + provenance), bandeau version snapshot + attribution AA. Responsive
  en unités relatives.
- **Sync** : connecteurs testés sur fixtures HTTP enregistrées, jamais d'appel
  live en CI. Budget AA ≤ 100 req/jour largement suffisant.

## 6. Erreurs — fail-closed systématique

Politique invalide → refus d'évaluer. Donnée manquante → `Indeterminate`.
Modèle inconnu → inéligible. Sync en échec → snapshot précédent conservé.
Jamais de dégradé silencieux.

## 7. Tests

- Unit `domain` + property-based (proptest) : monotonicité (une contrainte
  ajoutée ne peut que réduire l'ensemble éligible), deny-by-default,
  déterminisme.
- Golden tests de verdicts sur fixtures de snapshot.
- Contrats API : assertions explicites par endpoint (intégration axum).
- e2e CLI (assert_cmd) ; e2e web `playwright test` versionné (chromium +
  firefox + webkit).
- CI : clippy zéro warnings, cargo-deny (AGPL/SSPL bannies), couverture à
  seuil bloquant, garde-fous hygiène (secrets, chemins machine-locaux, aucune
  donnée AA committée).
