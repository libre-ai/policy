import { type JsonRecord, type RawDefect, StrictJsonError } from "./types";

// RFC 8785 JSON Canonicalization + SEMANTICS.md §2 strict parsing.
// Rejects: BOM, invalid UTF-8, duplicate member names, unpaired surrogates,
// nesting >64, non-JSON numbers. Allows fractional IEEE-754.
export class StrictJsonParser {
  private index = 0;

  constructor(
    private readonly input: string,
    private readonly maxDepth: number = 64,
  ) {}

  parse(): JsonRecord {
    this.skipWhitespace();
    const value = this.parseValue(0);
    this.skipWhitespace();
    if (this.index !== this.input.length) {
      this.fail("invalid JSON trailing bytes");
    }
    if (!isRecord(value)) {
      this.fail("root JSON value must be object");
    }
    return value;
  }

  private parseValue(depth: number): unknown {
    if (depth > this.maxDepth) {
      this.fail("maximum JSON depth exceeded", "max-depth");
    }
    this.skipWhitespace();
    const current = this.input[this.index];
    if (current === "{") {
      return this.parseObject(depth);
    }
    if (current === "[") {
      return this.parseArray(depth);
    }
    if (current === '"') {
      return this.parseString();
    }
    if (current === "t") {
      return this.parseLiteral("true", true);
    }
    if (current === "f") {
      return this.parseLiteral("false", false);
    }
    if (current === "n") {
      return this.parseLiteral("null", null);
    }
    if (current === "-" || (current !== undefined && current >= "0" && current <= "9")) {
      return this.parseNumber();
    }
    if (current === "+" || current === "N" || current === "I") {
      this.fail("invalid JSON number", "invalid-number");
    }
    this.fail("invalid JSON value");
  }

  private parseObject(depth: number): JsonRecord {
    this.index += 1;
    this.skipWhitespace();
    const result: JsonRecord = {};
    const keys = new Set<string>();

    if (this.input[this.index] === "}") {
      this.index += 1;
      return result;
    }

    while (true) {
      if (this.input[this.index] !== '"') {
        this.fail("object member name must be a string");
      }
      const key = this.parseString();
      if (keys.has(key)) {
        this.fail("duplicate JSON object member", "duplicate-member");
      }
      keys.add(key);
      this.skipWhitespace();
      if (this.input[this.index] !== ":") {
        this.fail("missing object member colon");
      }
      this.index += 1;
      result[key] = this.parseValue(depth + 1);
      this.skipWhitespace();
      const delimiter = this.input[this.index];
      if (delimiter === "}") {
        this.index += 1;
        return result;
      }
      if (delimiter !== ",") {
        this.fail("missing object member delimiter");
      }
      this.index += 1;
      this.skipWhitespace();
    }
  }

  private parseArray(depth: number): unknown[] {
    this.index += 1;
    this.skipWhitespace();
    const result: unknown[] = [];

    if (this.input[this.index] === "]") {
      this.index += 1;
      return result;
    }

    while (true) {
      result.push(this.parseValue(depth + 1));
      this.skipWhitespace();
      const delimiter = this.input[this.index];
      if (delimiter === "]") {
        this.index += 1;
        return result;
      }
      if (delimiter !== ",") {
        this.fail("missing array item delimiter");
      }
      this.index += 1;
      this.skipWhitespace();
    }
  }

  private parseString(): string {
    this.index += 1;
    let decoded = "";

    while (this.index < this.input.length) {
      const current = this.input[this.index] ?? "";
      const code = current.charCodeAt(0);

      if (current === '"') {
        this.index += 1;
        return decoded;
      }
      if (code < 0x20) {
        this.fail("unescaped control character in JSON string");
      }
      if (current === "\\") {
        this.index += 1;
        const escapeCode = this.input[this.index];
        if (escapeCode === undefined) {
          this.fail("unterminated JSON escape");
        }
        const simpleEscapes: Record<string, string> = {
          '"': '"',
          "\\": "\\",
          "/": "/",
          b: "\b",
          f: "\f",
          n: "\n",
          r: "\r",
          t: "\t",
        };
        if (escapeCode !== "u") {
          const value = simpleEscapes[escapeCode];
          if (value === undefined) {
            this.fail("invalid JSON escape");
          }
          decoded += value;
          this.index += 1;
          continue;
        }
        this.index += 1;
        const first = this.readHexCodeUnit();
        if (first >= 0xd800 && first <= 0xdbff) {
          if (this.input.slice(this.index, this.index + 2) !== "\\u") {
            this.fail("unpaired high surrogate", "unpaired-surrogate");
          }
          this.index += 2;
          const second = this.readHexCodeUnit();
          if (second < 0xdc00 || second > 0xdfff) {
            this.fail("unpaired low surrogate", "unpaired-surrogate");
          }
          decoded += String.fromCodePoint(0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00));
        } else if (first >= 0xdc00 && first <= 0xdfff) {
          this.fail("unpaired low surrogate", "unpaired-surrogate");
        } else {
          decoded += String.fromCharCode(first);
        }
      } else {
        decoded += current;
        this.index += 1;
      }
    }
    this.fail("unterminated JSON string");
  }

  private readHexCodeUnit(): number {
    const hex = this.input.slice(this.index, this.index + 4);
    this.index += 4;
    if (hex.length < 4) {
      this.fail("incomplete JSON unicode escape");
    }
    const code = parseInt(hex, 16);
    if (!Number.isInteger(code) || code < 0 || code > 0xffff) {
      this.fail("invalid JSON unicode escape");
    }
    return code;
  }

  private parseNumber(): number {
    const start = this.index;
    if (this.input[this.index] === "-") {
      this.index += 1;
    }
    if (this.input[this.index] === "0") {
      this.index += 1;
    } else {
      const digit = this.input[this.index];
      if (digit && digit >= "1" && digit <= "9") {
        while (this.index < this.input.length) {
          const d = this.input[this.index];
          if (!d || d < "0" || d > "9") break;
          this.index += 1;
        }
      } else {
        this.fail("invalid JSON number");
      }
    }

    if (this.input[this.index] === ".") {
      this.index += 1;
      const frac = this.input[this.index];
      if (!frac || frac < "0" || frac > "9") {
        this.fail("invalid JSON number fraction");
      }
      while (this.index < this.input.length) {
        const d = this.input[this.index];
        if (!d || d < "0" || d > "9") break;
        this.index += 1;
      }
    }

    if (this.input[this.index] === "e" || this.input[this.index] === "E") {
      this.index += 1;
      if (this.input[this.index] === "+" || this.input[this.index] === "-") {
        this.index += 1;
      }
      const exp = this.input[this.index];
      if (!exp || exp < "0" || exp > "9") {
        this.fail("invalid JSON number exponent");
      }
      while (this.index < this.input.length) {
        const d = this.input[this.index];
        if (!d || d < "0" || d > "9") break;
        this.index += 1;
      }
    }

    const numStr = this.input.slice(start, this.index);
    const value = Number(numStr);
    if (!Number.isFinite(value)) {
      this.fail("invalid JSON number value", "invalid-number");
    }
    return value;
  }

  private parseLiteral(expected: string, value: unknown): unknown {
    if (this.input.slice(this.index, this.index + expected.length) === expected) {
      this.index += expected.length;
      return value;
    }
    this.fail("invalid JSON literal");
  }

  private skipWhitespace(): void {
    while (
      this.index < this.input.length &&
      (this.input[this.index] === " " ||
        this.input[this.index] === "\t" ||
        this.input[this.index] === "\n" ||
        this.input[this.index] === "\r")
    ) {
      this.index += 1;
    }
  }

  private fail(message: string, defect: RawDefect = "invalid-json"): never {
    throw new StrictJsonError(defect, `${message} at index ${this.index}`);
  }
}

export function parseStrictJson(bytes: Uint8Array, maxDepth: number = 64): JsonRecord {
  // Check for BOM
  if (bytes.length >= 3 && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
    throw new StrictJsonError("bom", "JSON must not start with BOM");
  }

  // Decode UTF-8
  let text: string;
  try {
    const decoder = new TextDecoder("utf-8", { fatal: true });
    text = decoder.decode(bytes);
  } catch {
    throw new StrictJsonError("invalid-utf8", "invalid UTF-8 sequence");
  }

  // Parse with strict rules
  return new StrictJsonParser(text, maxDepth).parse();
}

function isRecord(value: unknown): value is JsonRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
