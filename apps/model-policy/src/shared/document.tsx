import type { DocumentDescriptor } from "@libre-ai/web-platform";
import type { PolicyDefinition } from "../domain/policy-definition";
import { ModelPolicyCockpit } from "../ui/model-policy-cockpit";

// The read-only cockpit is server-rendered and works without JavaScript, so no
// client module is declared; interactivity (authoring, approval journeys) arrives
// with a later increment.
export function modelPolicyCockpitDocument(
  policies: readonly PolicyDefinition[],
): DocumentDescriptor {
  return {
    app: <ModelPolicyCockpit policies={policies} />,
    description: "Cockpit humain des politiques de modèle approuvées de Libre AI.",
    lang: "fr",
    title: "Libre AI — Politiques de modèle",
  };
}
