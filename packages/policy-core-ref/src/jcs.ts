// RFC 8785 JSON Canonicalization Scheme
// Produces byte-identical output for identical semantic inputs.
// SEMANTICS.md §9: JCS(x) with SHA-256 digest over label || 0x00 || JCS(normalized(x))

import type { JsonRecord } from "./types";

export function jcs(value: unknown): Uint8Array {
  const canonical = sortObjectMembers(value);
  const json = stringify(canonical);
  return new TextEncoder().encode(json);
}

export function digest(label: string, value: unknown): string {
  const hasher = new Bun.CryptoHasher("sha256");
  hasher.update(label);
  hasher.update(new Uint8Array([0])); // 0x00 separator
  hasher.update(jcs(value));
  return hasher.digest("hex");
}

// Sort all object members by key (ascending UTF-8 order).
// Recursively sort nested structures.
function sortObjectMembers(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(sortObjectMembers);
  }
  if (isObject(value)) {
    const sorted: JsonRecord = {};
    const keys = Object.keys(value).sort();
    for (const key of keys) {
      sorted[key] = sortObjectMembers((value as JsonRecord)[key]);
    }
    return sorted;
  }
  return value;
}

// RFC 8785 canonicalization: serialize value to JSON string with canonical number format.
function stringify(value: unknown): string {
  if (value === null) {
    return "null";
  }
  if (typeof value === "boolean") {
    return value ? "true" : "false";
  }
  if (typeof value === "number") {
    return stringifyNumber(value);
  }
  if (typeof value === "string") {
    return stringifyString(value);
  }
  if (Array.isArray(value)) {
    const items = value.map(stringify);
    return `[${items.join(",")}]`;
  }
  if (isObject(value)) {
    const keys = Object.keys(value).sort();
    const pairs = keys.map(
      (key) => `${stringifyString(key)}:${stringify((value as JsonRecord)[key])}`,
    );
    return `{${pairs.join(",")}}`;
  }
  throw new TypeError(`cannot stringify ${typeof value}`);
}

// RFC 8785 requires careful handling of floating-point numbers.
// -0 and 0 are equal, and numbers must serialize in a canonical form.
function stringifyNumber(value: number): string {
  if (!Number.isFinite(value)) {
    throw new TypeError("non-finite number in JSON");
  }
  // -0 === 0 in IEEE-754 equality, but they serialize differently.
  // Canonical form: return "0" for both.
  if (Object.is(value, -0)) {
    return "0";
  }
  // For finite numbers, use JSON.stringify which produces IEEE-754 canonical form.
  return JSON.stringify(value);
}

// RFC 8785: escape all control characters and quotes/backslashes.
function stringifyString(value: string): string {
  let result = '"';
  for (let i = 0; i < value.length; i++) {
    const char = value[i];
    if (!char) return `${result}"`; // Safety check, should never occur
    const code = char.charCodeAt(0);
    switch (char) {
      case '"':
      case "\\":
        result += `\\${char}`;
        break;
      case "\b":
        result += "\\b";
        break;
      case "\f":
        result += "\\f";
        break;
      case "\n":
        result += "\\n";
        break;
      case "\r":
        result += "\\r";
        break;
      case "\t":
        result += "\\t";
        break;
      default:
        if (code < 0x20) {
          result += `\\u${code.toString(16).padStart(4, "0")}`;
        } else {
          result += char;
        }
    }
  }
  result += '"';
  return result;
}

function isObject(value: unknown): value is JsonRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
