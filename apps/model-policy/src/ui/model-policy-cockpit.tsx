// Read-only Model Policy cockpit view. Accessibility first: an ordered
// textual/table view of approved policies; conveyed as text, never colour.
// Server-rendered and usable without JavaScript. The deterministic rule
// evaluation is the deferred Rust/WASM boundary — this view authors nothing and
// evaluates nothing; it lists the approved policy definitions and their metadata.

import type { PolicyDefinition } from "../domain/policy-definition";

export function ModelPolicyCockpit({
  policies,
}: {
  readonly policies: readonly PolicyDefinition[];
}) {
  return (
    <>
      <a className="skip-link" href="#policies">
        Aller à la liste des politiques
      </a>
      <header>
        <h1>Politiques de modèle</h1>
        <p>
          Autoriser, approuver et appliquer des politiques déterministes qui décident si un
          instantané de modèle satisfait un besoin, avec des verdicts explicables. L'évaluation
          reste déterministe ; l'approbation humaine borne chaque politique appliquée.
        </p>
      </header>
      <main id="policies">
        <h2 id="policies-heading">Politiques approuvées</h2>
        <p>{`${policies.length} politique(s).`}</p>
        <table aria-labelledby="policies-heading">
          <caption>
            Liste des politiques approuvées : identifiant, version, nombre de règles, auteur de la
            proposition et date d'approbation.
          </caption>
          <thead>
            <tr>
              <th scope="col">Politique</th>
              <th scope="col">Version</th>
              <th scope="col">Règles</th>
              <th scope="col">Proposée par</th>
              <th scope="col">Approuvée le</th>
            </tr>
          </thead>
          <tbody>
            {policies.map((policy) => (
              <tr key={policy.id}>
                <th scope="row">{policy.id}</th>
                <td>{policy.version}</td>
                <td>{policy.rules.length}</td>
                <td>{policy.proposedBy}</td>
                <td>{policy.approvedAt}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </main>
    </>
  );
}
