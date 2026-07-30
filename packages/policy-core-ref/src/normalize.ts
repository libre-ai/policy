// SEMANTICS.md §9: Normalization of unordered arrays for digest computation.
// Normalizes: rules (by id), in/not-in sets (by JCS), facts (by name, type-rank, JCS value/source).

import { jcs } from "./jcs";
import type { JsonRecord } from "./types";

export function normalize(value: JsonRecord, kind: "policy" | "snapshot" | "need"): JsonRecord {
  const normalized = structuredClone(value);

  if (kind === "policy") {
    normalizePolicy(normalized);
  } else if (kind === "snapshot") {
    normalizeSnapshot(normalized);
  } else {
    normalizeNeed(normalized);
  }

  return normalized;
}

function normalizePolicy(value: JsonRecord): void {
  const rules = value.rules as JsonRecord[];
  if (!Array.isArray(rules)) {
    return;
  }

  // Sort rules by ascending id (raw ASCII)
  rules.sort((a, b) => {
    const aId = String(a.id);
    const bId = String(b.id);
    return compareBytes(new TextEncoder().encode(aId), new TextEncoder().encode(bId));
  });

  // Normalize in/not-in sets within rules
  for (const rule of rules) {
    if (rule.operator === "in" || rule.operator === "not-in") {
      const setValue = rule.value as unknown[];
      if (Array.isArray(setValue)) {
        // Sort set members by ascending JCS canonical form
        setValue.sort((a, b) => {
          return compareBytes(jcs(a), jcs(b));
        });
      }
    }
  }
}

function normalizeSnapshot(value: JsonRecord): void {
  const facts = value.facts as JsonRecord[];
  if (!Array.isArray(facts)) {
    return;
  }

  // Sort by (name, type-rank, JCS(value), JCS(source))
  facts.sort((a, b) => {
    const aName = new TextEncoder().encode(String(a.name));
    const bName = new TextEncoder().encode(String(b.name));
    const nameCmp = compareBytes(aName, bName);
    if (nameCmp !== 0) {
      return nameCmp;
    }

    const aRank = typeRank(a.value);
    const bRank = typeRank(b.value);
    if (aRank !== bRank) {
      return aRank - bRank;
    }

    const valueCmp = compareBytes(jcs(a.value), jcs(b.value));
    if (valueCmp !== 0) {
      return valueCmp;
    }

    return compareBytes(jcs(a.source), jcs(b.source));
  });
}

function normalizeNeed(value: JsonRecord): void {
  const facts = value.facts as JsonRecord[];
  if (!Array.isArray(facts)) {
    return;
  }

  // Sort by (name, type-rank, JCS(value))
  facts.sort((a, b) => {
    const aName = new TextEncoder().encode(String(a.name));
    const bName = new TextEncoder().encode(String(b.name));
    const nameCmp = compareBytes(aName, bName);
    if (nameCmp !== 0) {
      return nameCmp;
    }

    const aRank = typeRank(a.value);
    const bRank = typeRank(b.value);
    if (aRank !== bRank) {
      return aRank - bRank;
    }

    return compareBytes(jcs(a.value), jcs(b.value));
  });
}

// Type rank for sorting (boolean=0, number=1, string=2)
function typeRank(value: unknown): number {
  if (typeof value === "boolean") {
    return 0;
  }
  if (typeof value === "number") {
    return 1;
  }
  if (typeof value === "string") {
    return 2;
  }
  throw new TypeError("policy fact is not scalar");
}

// Bytewise unsigned lexicographic comparison
function compareBytes(left: Uint8Array, right: Uint8Array): number {
  for (let i = 0; i < Math.min(left.length, right.length); i++) {
    const diff = (left[i] ?? 0) - (right[i] ?? 0);
    if (diff !== 0) {
      return diff;
    }
  }
  return left.length - right.length;
}
