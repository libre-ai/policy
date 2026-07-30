import { afterEach, describe, expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, readFile, rename, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  checkProductPhaseFiles,
  DEFAULT_REPO_ROOT,
  extractGateDefinition,
  extractGateIds,
  isAllowedEvidenceRecordPath,
  type ProductPhaseRoadmap,
  type ReviewRole,
  replaceReadmeProjection,
  validateRoadmapSemantics,
} from "./check-product-phases";

const temporaryDirectories: string[] = [];
const FIXTURE_EVIDENCE_PATH = "distribution/evidence/model-policy/mp-p0-g01-fixture.json";
const CANONICAL_JSON_MAX_BYTES = 1_048_576;

interface MutableEvidenceFixture {
  assertion: string;
  reviewBindings: Array<{ role: ReviewRole; sha256: string }>;
  serviceObservation?: {
    deploymentIdentity: string;
    windowStartedAt: string;
    windowEndedAt: string;
    authorizationEvidencePath: string;
    authorizationEvidenceSha256: string;
    smokeEvidencePath: string;
    smokeEvidenceSha256: string;
    rollbackEvidencePath: string;
    rollbackEvidenceSha256: string;
    incidentState: "none_observed" | "incidents_bound_in_artifacts";
    incidentEvidence: Array<{ path: string; sha256: string }>;
  };
}

function validRoadmap(): ProductPhaseRoadmap {
  return {
    schemaVersion: "libre-ai.model-policy-phases.v1",
    documentStatus: "draft",
    statusAuthorities: { program: "GOALS.md", execution: "STATUS.md" },
    updatedAt: "2026-07-28T00:00:00Z",
    phases: [
      {
        id: "MP-P0",
        title: "Foundation",
        outcome: "Establish the deterministic authority boundary.",
        document: "docs/apps/model-policy/phases/00-foundation.md",
        dependsOn: [],
        activationPrerequisites: ["Owner selection is recorded by GOALS.md"],
        requiredIndependentReviewRoles: ["architecture", "security", "technical"],
        gates: [
          {
            id: "MP-P0-G01",
            requiredEvidenceLevel: "qualified",
            evidence: [],
          },
          {
            id: "MP-P0-G02",
            requiredEvidenceLevel: "verified",
            evidence: [],
          },
        ],
      },
      {
        id: "MP-P1",
        title: "Deterministic tunnel",
        outcome: "Qualify a use case without an LLM.",
        document: "docs/apps/model-policy/phases/01-deterministic-qualification.md",
        dependsOn: ["MP-P0"],
        activationPrerequisites: ["MP-P0 evidence is owner-accepted"],
        requiredIndependentReviewRoles: ["business", "privacy"],
        gates: [
          {
            id: "MP-P1-G01",
            requiredEvidenceLevel: "qualified",
            evidence: [],
          },
        ],
      },
    ],
  };
}

async function runGit(repoRoot: string, args: string[]): Promise<void> {
  const process = Bun.spawn(["git", ...args], {
    cwd: repoRoot,
    stdout: "ignore",
    stderr: "pipe",
  });
  const stderr = await new Response(process.stderr).text();
  const exitCode = await process.exited;
  if (exitCode !== 0) throw new Error(`git ${args.join(" ")} failed: ${stderr}`);
}

async function runGitOutput(repoRoot: string, args: string[]): Promise<string> {
  const process = Bun.spawn(["git", ...args], {
    cwd: repoRoot,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
    process.exited,
  ]);
  if (exitCode !== 0) throw new Error(`git ${args.join(" ")} failed: ${stderr}`);
  return stdout.trim();
}

function sha256(value: string | Uint8Array): string {
  return `sha256:${new Bun.CryptoHasher("sha256").update(value).digest("hex")}`;
}

function withMalformedUtf8(value: Uint8Array, needle: string): Uint8Array {
  const result = value.slice();
  const needleBytes = new TextEncoder().encode(needle);
  for (let offset = 0; offset <= result.length - needleBytes.length; offset += 1) {
    if (needleBytes.every((byte, index) => result[offset + index] === byte)) {
      result[offset] = 0xff;
      return result;
    }
  }
  throw new Error(`fixture text is missing: ${needle}`);
}

function jsonDocumentWithByteLength(byteLength: number): string {
  const prefix = '{"padding":"';
  const suffix = '"}';
  const paddingLength = byteLength - prefix.length - suffix.length;
  if (paddingLength < 0) throw new Error("requested JSON fixture is too small");
  return `${prefix}${"a".repeat(paddingLength)}${suffix}`;
}

async function createFixture(
  roadmap: unknown,
  firstPhaseDocument = [
    "# MP-P0 — Foundation",
    "",
    "### MP-P0-G01 — First gate",
    "",
    "### MP-P0-G02 — Second gate",
    "",
  ].join("\n"),
): Promise<string> {
  const repoRoot = await mkdtemp(join(tmpdir(), "model-policy-phases-"));
  temporaryDirectories.push(repoRoot);
  const directories = [
    "apps/model-policy",
    "distribution/evidence/model-policy/operations",
    "distribution/evidence/model-policy/reviews",
    "docs/apps/model-policy/phases",
    "docs/apps/model-policy",
    "docs/reviews/model-policy",
  ];
  for (const directory of directories) await mkdir(join(repoRoot, directory), { recursive: true });

  const sourceFiles = [
    "docs/apps/model-policy/phases.v1.schema.json",
    "docs/apps/model-policy/evidence-record.v1.schema.json",
    "docs/apps/model-policy/review-attestation.v1.schema.json",
    "docs/apps/model-policy/operational-evidence.v1.schema.json",
  ];
  for (const sourceFile of sourceFiles) {
    await writeFile(
      join(repoRoot, sourceFile),
      await Bun.file(join(DEFAULT_REPO_ROOT, sourceFile)).text(),
    );
  }
  await writeFile(
    join(repoRoot, "docs/apps/model-policy/phases.v1.json"),
    `${JSON.stringify(roadmap, null, 2)}\n`,
  );
  await writeFile(
    join(repoRoot, "docs/apps/model-policy/phases/00-foundation.md"),
    firstPhaseDocument,
  );
  await writeFile(
    join(repoRoot, "docs/apps/model-policy/phases/01-deterministic-qualification.md"),
    "# MP-P1 — Tunnel\n\n### MP-P1-G01 — Gate\n",
  );
  const reviewRoles: ReviewRole[] = [
    "accessibility",
    "architecture",
    "business",
    "legal",
    "operations",
    "performance",
    "privacy",
    "security",
    "technical",
  ];
  for (const role of reviewRoles) {
    await writeFile(
      join(repoRoot, `docs/reviews/model-policy/${role}.md`),
      `# Independent ${role} review\n\nVerdict: approve.\n`,
    );
  }
  await writeFile(
    join(repoRoot, "apps/model-policy/README.md"),
    "# App\n\n<!-- model-policy-phases:start -->\nstale\n<!-- model-policy-phases:end -->\n",
  );
  await writeFile(
    join(repoRoot, "docs/apps/model-policy/README.md"),
    "# Plan\n\n<!-- model-policy-plan:start -->\nstale\n<!-- model-policy-plan:end -->\n",
  );
  await runGit(repoRoot, ["init", "-q"]);
  await runGit(repoRoot, ["add", "."]);
  return repoRoot;
}

interface EvidenceFixtureOptions {
  readonly achievedEvidenceLevel?: "verified" | "qualified" | "in_service";
  readonly commands?: string[];
  readonly harnessIdentifiers?: string[];
  readonly emptyToolVersions?: boolean;
  readonly emptyInputIdentities?: boolean;
  readonly roles?: ReviewRole[];
  readonly producerRef?: string;
  readonly sharedReviewerRef?: string;
  readonly sharedReviewId?: boolean;
  readonly verdict?: "approve" | "approve_with_minor_reservations" | "reject" | "not_applicable";
  readonly majorFindings?: string[];
  readonly attestationVerdict?: "approve" | "approve_with_minor_reservations" | "reject";
  readonly attestationMajorFindings?: string[];
  readonly inputDigest?: string;
  readonly includeServiceObservation?: boolean;
  readonly includeIncidentEvidence?: boolean;
  readonly phaseId?: string;
  readonly gateId?: string;
  readonly sourceCommit?: string;
  readonly referenceDigest?: string;
  readonly artifactPath?: string;
  readonly invalidAuthorizationDigest?: boolean;
  readonly invalidServiceDigest?: boolean;
  readonly serviceDeploymentIdentity?: string;
  readonly incidentState?: "none_observed" | "incidents_bound_in_artifacts";
}

async function rewriteEvidenceFixture(
  repoRoot: string,
  mutate: (record: MutableEvidenceFixture) => void,
): Promise<void> {
  const evidencePath = join(repoRoot, FIXTURE_EVIDENCE_PATH);
  const record = JSON.parse(await readFile(evidencePath, "utf8")) as MutableEvidenceFixture;
  mutate(record);
  const evidenceText = `${JSON.stringify(record, null, 2)}\n`;
  await writeFile(evidencePath, evidenceText);

  const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
  const roadmap = JSON.parse(await readFile(roadmapPath, "utf8")) as ProductPhaseRoadmap;
  const evidenceReference = roadmap.phases[0]?.gates[0]?.evidence[0] as
    | { sha256: string }
    | undefined;
  if (!evidenceReference) throw new Error("fixture evidence reference is missing");
  evidenceReference.sha256 = sha256(evidenceText);
  await writeFile(roadmapPath, `${JSON.stringify(roadmap, null, 2)}\n`);
  await runGit(repoRoot, ["add", "."]);
}

async function writeRawEvidenceFixture(
  repoRoot: string,
  evidenceContent: string | Uint8Array,
): Promise<void> {
  await writeFile(join(repoRoot, FIXTURE_EVIDENCE_PATH), evidenceContent);
  const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
  const roadmap = JSON.parse(await readFile(roadmapPath, "utf8")) as ProductPhaseRoadmap;
  const evidenceReference = roadmap.phases[0]?.gates[0]?.evidence[0] as
    | { sha256: string }
    | undefined;
  if (!evidenceReference) throw new Error("fixture evidence reference is missing");
  evidenceReference.sha256 = sha256(evidenceContent);
  await writeFile(roadmapPath, `${JSON.stringify(roadmap, null, 2)}\n`);
  await runGit(repoRoot, ["add", "."]);
}

async function addEvidenceFixture(
  repoRoot: string,
  options: EvidenceFixtureOptions = {},
): Promise<void> {
  await runGit(repoRoot, ["config", "user.email", "fixture@example.invalid"]);
  await runGit(repoRoot, ["config", "user.name", "Fixture"]);
  await runGit(repoRoot, ["commit", "-q", "-m", "fixture source"]);
  const sourceCommit = await runGitOutput(repoRoot, ["rev-parse", "HEAD"]);
  const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
  const roadmap = JSON.parse(await readFile(roadmapPath, "utf8")) as ProductPhaseRoadmap;
  const phase = roadmap.phases[0];
  const gate = phase?.gates[0];
  if (!phase || !gate) throw new Error("fixture roadmap lacks MP-P0-G01");

  const phaseDocumentBytes = new Uint8Array(
    await Bun.file(join(repoRoot, phase.document)).arrayBuffer(),
  );
  const phaseDocumentDigest = sha256(phaseDocumentBytes);
  const gateDefinition = extractGateDefinition(
    new TextDecoder().decode(phaseDocumentBytes),
    gate.id,
  );
  if (!gateDefinition) throw new Error("fixture phase document lacks MP-P0-G01");
  const gateDefinitionSha256 = sha256(gateDefinition);
  const artifactPath = options.artifactPath ?? phase.document;
  const artifactDigest = sha256(
    new Uint8Array(await Bun.file(join(repoRoot, artifactPath)).arrayBuffer()),
  );
  const roles = options.roles ?? phase.requiredIndependentReviewRoles;
  const reviewBindings = await Promise.all(
    roles.map(async (role) => {
      const reportPath = `docs/reviews/model-policy/${role}.md`;
      const reviewerRef = options.sharedReviewerRef ?? `reviewer:${role}`;
      const reportSha256 = sha256(
        new Uint8Array(await Bun.file(join(repoRoot, reportPath)).arrayBuffer()),
      );
      const attestationRecord = `distribution/evidence/model-policy/reviews/mp-p0-g01-${role}.json`;
      const attestation = {
        schemaVersion: "libre-ai.model-policy-review-attestation.v1",
        reviewId: options.sharedReviewId
          ? "MP-REV-P0-G01-SHARED"
          : `MP-REV-P0-G01-${role.toUpperCase()}`,
        phaseId: phase.id,
        gateId: gate.id,
        candidateCommit: sourceCommit,
        role,
        reviewerRef,
        verdict: options.attestationVerdict ?? "approve",
        findings: {
          blocking: [],
          major: options.attestationMajorFindings ?? [],
          minor: [],
          residual: ["Fixture review attestation is not product evidence."],
        },
        reportPath,
        reportSha256,
        recordedAt: "2026-07-28T00:00:00Z",
      };
      const attestationText = `${JSON.stringify(attestation, null, 2)}\n`;
      await writeFile(join(repoRoot, attestationRecord), attestationText);
      return {
        role,
        reviewerRef,
        attestationRecord,
        sha256: sha256(attestationText),
      };
    }),
  );
  const evidenceRecord: Record<string, unknown> = {
    schemaVersion: "libre-ai.model-policy-evidence-record.v1",
    evidenceId: "MP-EVD-P0-G01-FIXTURE",
    phaseId: options.phaseId ?? phase.id,
    gateId: options.gateId ?? gate.id,
    gateDefinitionSha256,
    assertion: "The fixture gate has independently reproducible evidence.",
    achievedEvidenceLevel: options.achievedEvidenceLevel ?? "qualified",
    sourceCommit: options.sourceCommit ?? sourceCommit,
    evidenceProducerRef: options.producerRef ?? "producer:fixture",
    artifactDigests: [{ path: artifactPath, sha256: artifactDigest }],
    toolVersions: options.emptyToolVersions ? [] : [{ tool: "bun", version: Bun.version }],
    inputIdentities: options.emptyInputIdentities
      ? []
      : [
          {
            kind: "repository_fixture",
            identifier: "model-policy:phase-document:v1",
            path: phase.document,
            sha256: options.inputDigest ?? phaseDocumentDigest,
          },
          ...(options.includeServiceObservation
            ? [
                {
                  kind: "operated_environment",
                  identifier: "deployment:model-policy:fixture",
                  sha256: sha256("deployment:model-policy:fixture"),
                },
              ]
            : []),
        ],
    commands: options.commands ?? ["bun test apps/model-policy/tools/check-product-phases.test.ts"],
    expectedResults: ["The gate evidence validation passes."],
    observedResults: ["The gate evidence validation passed."],
    findings: {
      blocking: [],
      major: options.majorFindings ?? [],
      minor: [],
      residual: ["Fixture-only evidence does not activate a product phase."],
    },
    verdict: options.verdict ?? "approve",
    reviewBindings,
    harnessIdentifiers: options.harnessIdentifiers ?? ["bun:test:model-policy-phase-checker"],
    invalidationConditions: ["Any bound artifact digest changes."],
    recordedAt: "2026-07-28T00:00:00Z",
  };
  if (options.includeServiceObservation) {
    const deploymentIdentity =
      options.serviceDeploymentIdentity ?? "deployment:model-policy:fixture";
    const windowStartedAt = "2026-07-27T00:00:00Z";
    const windowEndedAt = "2026-07-28T00:00:00Z";
    const authorizationEvidencePath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json";
    const smokeEvidencePath = "distribution/evidence/model-policy/operations/mp-p0-g01-smoke.json";
    const rollbackEvidencePath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-rollback.json";
    const incidentEvidencePath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-incident.json";
    const commonOperationalBinding = {
      schemaVersion: "libre-ai.model-policy-operational-evidence.v1",
      phaseId: phase.id,
      gateId: gate.id,
      evidenceId: "MP-EVD-P0-G01-FIXTURE",
      deploymentIdentity,
      windowStartedAt,
      windowEndedAt,
    };
    const authorizationEvidence = `${JSON.stringify(
      {
        ...commonOperationalBinding,
        operationalEvidenceId: "MP-OPS-P0-G01-AUTHORIZATION",
        kind: "deployment_authorization",
        outcome: "authorized",
        authorizationRef: "approval:fixture",
        recordedAt: "2026-07-26T00:00:00Z",
      },
      null,
      2,
    )}\n`;
    const smokeEvidence = `${JSON.stringify(
      {
        ...commonOperationalBinding,
        operationalEvidenceId: "MP-OPS-P0-G01-SMOKE",
        kind: "smoke_test",
        outcome: "passed",
        observedAt: "2026-07-27T12:00:00Z",
        recordedAt: "2026-07-27T12:01:00Z",
      },
      null,
      2,
    )}\n`;
    const rollbackEvidence = `${JSON.stringify(
      {
        ...commonOperationalBinding,
        operationalEvidenceId: "MP-OPS-P0-G01-ROLLBACK",
        kind: "rollback_test",
        outcome: "passed",
        observedAt: "2026-07-27T13:00:00Z",
        recordedAt: "2026-07-27T13:01:00Z",
      },
      null,
      2,
    )}\n`;
    const incidentEvidence = `${JSON.stringify(
      {
        ...commonOperationalBinding,
        operationalEvidenceId: "MP-OPS-P0-G01-INCIDENT",
        kind: "incident_report",
        outcome: "resolved",
        observedAt: "2026-07-27T14:00:00Z",
        incidentId: "incident:fixture",
        recordedAt: "2026-07-27T15:00:00Z",
      },
      null,
      2,
    )}\n`;
    await writeFile(join(repoRoot, authorizationEvidencePath), authorizationEvidence);
    await writeFile(join(repoRoot, smokeEvidencePath), smokeEvidence);
    await writeFile(join(repoRoot, rollbackEvidencePath), rollbackEvidence);
    if (options.includeIncidentEvidence) {
      await writeFile(join(repoRoot, incidentEvidencePath), incidentEvidence);
    }
    evidenceRecord.serviceObservation = {
      deploymentIdentity,
      windowStartedAt,
      windowEndedAt,
      authorizationEvidencePath,
      authorizationEvidenceSha256: options.invalidAuthorizationDigest
        ? `sha256:${"0".repeat(64)}`
        : sha256(authorizationEvidence),
      smokeEvidencePath,
      smokeEvidenceSha256: options.invalidServiceDigest
        ? `sha256:${"0".repeat(64)}`
        : sha256(smokeEvidence),
      rollbackEvidencePath,
      rollbackEvidenceSha256: sha256(rollbackEvidence),
      incidentState: options.incidentState ?? "none_observed",
      incidentEvidence: options.includeIncidentEvidence
        ? [{ path: incidentEvidencePath, sha256: sha256(incidentEvidence) }]
        : [],
    };
  }
  const evidenceText = `${JSON.stringify(evidenceRecord, null, 2)}\n`;
  await writeFile(join(repoRoot, FIXTURE_EVIDENCE_PATH), evidenceText);
  gate.evidence.push({
    record: FIXTURE_EVIDENCE_PATH,
    sha256: options.referenceDigest ?? sha256(evidenceText),
  });
  await writeFile(roadmapPath, `${JSON.stringify(roadmap, null, 2)}\n`);
  await runGit(repoRoot, ["add", "."]);
}

afterEach(async () => {
  await Promise.all(
    temporaryDirectories
      .splice(0)
      .map((directory) => rm(directory, { recursive: true, force: true })),
  );
});

describe("validateRoadmapSemantics", () => {
  test("accepts an acyclic planning-only phase record", () => {
    expect(validateRoadmapSemantics(validRoadmap())).toEqual([]);
  });

  test("rejects duplicate evidence records", () => {
    const roadmap = validRoadmap();
    const gate = roadmap.phases[0]?.gates[0];
    if (gate) {
      const reference = {
        record: "distribution/evidence/model-policy/p0-g01.json",
        sha256: `sha256:${"a".repeat(64)}`,
      };
      gate.evidence.push(reference, reference);
    }
    expect(validateRoadmapSemantics(roadmap)).toContain("MP-P0-G01: duplicate evidence record");
  });

  test("rejects dependency cycles", () => {
    const roadmap = validRoadmap();
    const first = roadmap.phases[0];
    if (first) first.dependsOn.push("MP-P1");
    expect(validateRoadmapSemantics(roadmap)).toContain("MP-P0: dependency cycle");
  });
});

describe("gate and path parsing", () => {
  test("extracts only exact gate headings in document order", () => {
    const document = [
      "### MP-P0-G01 — First",
      "### MP-P0-G02 - Wrong separator",
      "### MP-P0-G03 — Third",
    ].join("\n");
    expect(extractGateIds(document)).toEqual(["MP-P0-G01", "MP-P0-G03"]);
  });

  test("allows only canonical JSON evidence records", () => {
    expect(isAllowedEvidenceRecordPath("distribution/evidence/model-policy/p0-g01.json")).toBe(
      true,
    );
    expect(isAllowedEvidenceRecordPath("package.json")).toBe(false);
    expect(isAllowedEvidenceRecordPath("docs/reviews/report.md")).toBe(false);
  });
});

describe("replaceReadmeProjection", () => {
  test("replaces exactly one bounded generated section", () => {
    const source = [
      "# Product",
      "",
      "<!-- model-policy-phases:start -->",
      "old",
      "<!-- model-policy-phases:end -->",
      "",
    ].join("\n");
    const result = replaceReadmeProjection(source, "new");
    expect(result).toContain(
      "<!-- model-policy-phases:start -->\nnew\n<!-- model-policy-phases:end -->",
    );
    expect(result.startsWith("# Product")).toBe(true);
  });

  test("refuses missing markers rather than appending a second projection", () => {
    expect(() => replaceReadmeProjection("# Product\n", "new")).toThrow(
      "README phase projection markers must occur exactly once",
    );
  });
});

describe("checkProductPhaseFiles", () => {
  test("returns stable schema diagnostics instead of running semantics on malformed input", async () => {
    const repoRoot = await createFixture({ phases: null });
    const failures = await checkProductPhaseFiles({ repoRoot });
    expect(failures).toHaveLength(1);
    expect(failures[0]).toStartWith("Model Policy phase schema rejected:");
    expect(failures[0]).not.toContain("TypeError");
  });

  test("rejects an invalid staged roadmap even when the mutable worktree copy is valid", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
    const validRoadmapText = await readFile(roadmapPath, "utf8");
    await writeFile(roadmapPath, '{"phases":null}\n');
    await runGit(repoRoot, ["add", "--", "docs/apps/model-policy/phases.v1.json"]);
    await writeFile(roadmapPath, validRoadmapText);

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures[0]).toStartWith("Model Policy phase schema rejected:");
  });

  test("rejects malformed UTF-8 in an indexed roadmap before materialization", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
    const roadmapBytes = withMalformedUtf8(
      new Uint8Array(await Bun.file(roadmapPath).arrayBuffer()),
      "Foundation",
    );
    await Bun.write(roadmapPath, roadmapBytes);
    await runGit(repoRoot, ["add", roadmapPath]);

    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toContain(
      "Model Policy plan docs/apps/model-policy/phases.v1.json: JSON is not valid UTF-8",
    );
  });

  test("accepts the exact canonical JSON byte ceiling before schema validation", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
    await writeFile(roadmapPath, jsonDocumentWithByteLength(CANONICAL_JSON_MAX_BYTES));
    await runGit(repoRoot, ["add", roadmapPath]);

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures.some((failure) => failure.includes("JSON exceeds"))).toBe(false);
    expect(
      failures.some((failure) => failure.startsWith("Model Policy phase schema rejected")),
    ).toBe(true);
  });

  test("rejects a canonical JSON blob above the byte ceiling", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
    await writeFile(roadmapPath, jsonDocumentWithByteLength(CANONICAL_JSON_MAX_BYTES + 1));
    await runGit(repoRoot, ["add", roadmapPath]);

    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toContain(
      "Model Policy plan docs/apps/model-policy/phases.v1.json: JSON exceeds the 1048576-byte limit",
    );
  });

  test("never returns untrusted parser or schema-compiler content", async () => {
    const sensitiveMarker = ["AKI", "A1234567890ABCDEF"].join("");
    const malformedRepoRoot = await createFixture(validRoadmap());
    const malformedRoadmapPath = join(malformedRepoRoot, "docs/apps/model-policy/phases.v1.json");
    await writeFile(malformedRoadmapPath, `{"value":${sensitiveMarker}}\n`);
    await runGit(malformedRepoRoot, ["add", malformedRoadmapPath]);

    const parserFailures = await checkProductPhaseFiles({
      repoRoot: malformedRepoRoot,
      write: true,
    });
    expect(parserFailures).toEqual(["Model Policy plan JSON is malformed"]);
    expect(parserFailures.join("\n")).not.toContain(sensitiveMarker);

    const schemaRepoRoot = await createFixture(validRoadmap());
    const schemaPath = join(schemaRepoRoot, "docs/apps/model-policy/phases.v1.schema.json");
    const schema = JSON.parse(await readFile(schemaPath, "utf8")) as {
      properties: { documentStatus: Record<string, unknown> };
    };
    schema.properties.documentStatus.format = sensitiveMarker;
    await writeFile(schemaPath, `${JSON.stringify(schema, null, 2)}\n`);
    await runGit(schemaRepoRoot, ["add", schemaPath]);

    const compilerFailures = await checkProductPhaseFiles({
      repoRoot: schemaRepoRoot,
      write: true,
    });
    expect(compilerFailures).toEqual(["Model Policy schema compilation failed"]);
    expect(compilerFailures.join("\n")).not.toContain(sensitiveMarker);

    const validationRepoRoot = await createFixture(validRoadmap());
    const validationSchemaPath = join(
      validationRepoRoot,
      "docs/apps/model-policy/phases.v1.schema.json",
    );
    const validationSchema = JSON.parse(await readFile(validationSchemaPath, "utf8")) as {
      $defs: { phase: { properties: { title: Record<string, unknown> } } };
    };
    validationSchema.$defs.phase.properties.title.pattern = sensitiveMarker;
    await writeFile(validationSchemaPath, `${JSON.stringify(validationSchema, null, 2)}\n`);
    await runGit(validationRepoRoot, ["add", validationSchemaPath]);

    const validationFailures = await checkProductPhaseFiles({
      repoRoot: validationRepoRoot,
      write: true,
    });
    expect(validationFailures[0]).toBe(
      "Model Policy phase schema rejected: validation failed (pattern)",
    );
    expect(validationFailures.join("\n")).not.toContain(sensitiveMarker);
  });

  test("rejects structurally malformed indexed JSON before materialization", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
    await writeFile(roadmapPath, '{"schemaVersion" "invalid"}\n');
    await runGit(repoRoot, ["add", roadmapPath]);

    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toContain(
      "Model Policy plan JSON is malformed",
    );
  });

  test("rejects duplicate member names in an indexed roadmap", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
    const roadmapText = (await readFile(roadmapPath, "utf8")).replace(
      '  "documentStatus": "draft",',
      '  "documentStatus": "superseded",\n  "documentStatus": "draft",',
    );
    await writeFile(roadmapPath, roadmapText);
    await runGit(repoRoot, ["add", roadmapPath]);

    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toContain(
      "Model Policy plan JSON contains a duplicate member name",
    );
  });

  for (const schemaPath of [
    "docs/apps/model-policy/phases.v1.schema.json",
    "docs/apps/model-policy/evidence-record.v1.schema.json",
    "docs/apps/model-policy/review-attestation.v1.schema.json",
    "docs/apps/model-policy/operational-evidence.v1.schema.json",
  ]) {
    test(`rejects duplicate member names in indexed schema ${schemaPath}`, async () => {
      const repoRoot = await createFixture(validRoadmap());
      const schemaText = await readFile(join(repoRoot, schemaPath), "utf8");
      await writeFile(join(repoRoot, schemaPath), schemaText.replace("{", '{"probe":1,"probe":2,'));
      await runGit(repoRoot, ["add", schemaPath]);

      expect(await checkProductPhaseFiles({ repoRoot, write: true })).toContain(
        "Model Policy plan JSON contains a duplicate member name",
      );
    });
  }

  test("rejects JSON nesting above the structural bound", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
    await writeFile(roadmapPath, `${"[".repeat(65)}null${"]".repeat(65)}\n`);
    await runGit(repoRoot, ["add", roadmapPath]);

    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toContain(
      "Model Policy plan JSON is malformed",
    );
  });

  test("rejects an invalid staged schema even when the mutable worktree copy is valid", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const schemaPath = join(repoRoot, "docs/apps/model-policy/phases.v1.schema.json");
    const validSchemaText = await readFile(schemaPath, "utf8");
    await writeFile(schemaPath, '{"type":"not-a-json-schema-type"}\n');
    await runGit(repoRoot, ["add", "--", "docs/apps/model-policy/phases.v1.schema.json"]);
    await writeFile(schemaPath, validSchemaText);

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures[0]).toBe("Model Policy schema compilation failed");
  });

  test("does not write projections after a gate-definition failure", async () => {
    const repoRoot = await createFixture(
      validRoadmap(),
      "# MP-P0 — Foundation\n\n### MP-P0-G01 — First gate\n### MP-P0-G99 — Extra gate\n",
    );
    const readmePath = join(repoRoot, "apps/model-policy/README.md");
    const before = await readFile(readmePath, "utf8");
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures.some((failure) => failure.includes("gate headings must exactly match"))).toBe(
      true,
    );
    expect(await readFile(readmePath, "utf8")).toBe(before);
  });

  test("writes both projections only after full validation", async () => {
    const repoRoot = await createFixture(validRoadmap());
    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toEqual([]);
    expect(await checkProductPhaseFiles({ repoRoot })).toEqual([]);
  });

  test("reports counts from the validated indexed roadmap, not a mutable worktree reread", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const roadmapPath = join(repoRoot, "docs/apps/model-policy/phases.v1.json");
    await writeFile(roadmapPath, `${JSON.stringify({ ...validRoadmap(), phases: [] }, null, 2)}\n`);
    const summary: { value: { phaseCount: number; gateCount: number } | null } = {
      value: null,
    };

    expect(
      await checkProductPhaseFiles({
        repoRoot,
        write: true,
        onValidatedSummary: (value) => {
          summary.value = value;
        },
      }),
    ).toEqual([]);
    expect(summary.value).toEqual({ phaseCount: 2, gateCount: 3 });
  });

  test("accepts qualified evidence with versioned tools, inputs, harnesses and required reviews", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toEqual([]);
    expect(await checkProductPhaseFiles({ repoRoot })).toEqual([]);
  });

  test("rejects an untracked evidence record", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    await runGit(repoRoot, ["rm", "-q", "--cached", "--", FIXTURE_EVIDENCE_PATH]);
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: evidence: path is not tracked by git");
  });

  test("rejects a symlink evidence record from the git index", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    const evidencePath = join(repoRoot, FIXTURE_EVIDENCE_PATH);
    await rm(evidencePath);
    await symlink("../../../docs/apps/model-policy/phases.v1.json", evidencePath);
    await runGit(repoRoot, ["add", "--", FIXTURE_EVIDENCE_PATH]);
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: evidence: git index entry must be a regular non-symlink file",
    );
  });

  test("rejects a mismatched evidence-record digest", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { referenceDigest: `sha256:${"0".repeat(64)}` });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      `MP-P0-G01: evidence record digest mismatch for ${FIXTURE_EVIDENCE_PATH}`,
    );
  });

  test("rejects a credential marker inside a content-addressed evidence record", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    const credentialMarker = `AKIA${"1234567890ABCDEF"}`;
    await rewriteEvidenceFixture(repoRoot, (record) => {
      record.assertion = `Observed credential ${credentialMarker}`;
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: evidence contains a sensitive marker at line 7");
  });

  test("rejects malformed UTF-8 in an evidence record", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    const evidencePath = join(repoRoot, FIXTURE_EVIDENCE_PATH);
    const evidenceBytes = withMalformedUtf8(
      new Uint8Array(await Bun.file(evidencePath).arrayBuffer()),
      "fixture gate",
    );
    await writeRawEvidenceFixture(repoRoot, evidenceBytes);

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: evidence: JSON is not valid UTF-8");
  });

  test("rejects a sensitive marker hidden in an evidence record by JSON escaping", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    const evidencePath = join(repoRoot, FIXTURE_EVIDENCE_PATH);
    const evidenceText = (await readFile(evidencePath, "utf8")).replace(
      "The fixture gate has independently reproducible evidence.",
      "AKI\\u00411234567890ABCDEF",
    );
    await writeRawEvidenceFixture(repoRoot, evidenceText);

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: evidence contains a sensitive marker after JSON decoding",
    );
  });

  test("rejects duplicate evidence members that hide an escaped sensitive value", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    const evidencePath = join(repoRoot, FIXTURE_EVIDENCE_PATH);
    const evidenceText = (await readFile(evidencePath, "utf8")).replace(
      '  "assertion": "The fixture gate has independently reproducible evidence.",',
      '  "ass\\u0065rtion": "AKI\\u00411234567890ABCDEF",\n' +
        '  "assertion": "The fixture gate has independently reproducible evidence.",',
    );
    await writeRawEvidenceFixture(repoRoot, evidenceText);

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: evidence contains a duplicate JSON member name");
  });

  test("rejects a sensitive value hidden in a nested duplicate member", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    const evidencePath = join(repoRoot, FIXTURE_EVIDENCE_PATH);
    const evidenceText = (await readFile(evidencePath, "utf8")).replace(
      '  "findings": {',
      '  "findings": {\n    "major": ["AKI\\u00411234567890ABCDEF"],\n    "major": [],',
    );
    await writeRawEvidenceFixture(repoRoot, evidenceText);

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: evidence contains a duplicate JSON member name");
  });

  test("rejects empty and oversized evidence records", async () => {
    const emptyRepoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(emptyRepoRoot);
    await writeRawEvidenceFixture(emptyRepoRoot, "");
    expect(await checkProductPhaseFiles({ repoRoot: emptyRepoRoot, write: true })).toContain(
      "MP-P0-G01: evidence record is not valid JSON",
    );

    const oversizedRepoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(oversizedRepoRoot);
    await writeRawEvidenceFixture(
      oversizedRepoRoot,
      jsonDocumentWithByteLength(CANONICAL_JSON_MAX_BYTES + 1),
    );
    expect(await checkProductPhaseFiles({ repoRoot: oversizedRepoRoot, write: true })).toContain(
      "MP-P0-G01: evidence: JSON exceeds the 1048576-byte limit",
    );
  });

  test("rejects phase and gate identity drift inside a content-addressed record", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { phaseId: "MP-P1", gateId: "MP-P1-G01" });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: evidence record phase/gate binding does not match");
  });

  test("rejects evidence that does not digest the gate-definition document", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      artifactPath: "docs/apps/model-policy/phases.v1.json",
    });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: evidence does not bind the gate-definition document");
  });

  test("rejects evidence after the indexed gate definition changes semantically", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    const phaseDocumentPath = join(repoRoot, "docs/apps/model-policy/phases/00-foundation.md");
    const changedDocument = (await readFile(phaseDocumentPath, "utf8")).replace(
      "### MP-P0-G01 — First gate",
      "### MP-P0-G01 — Materially different gate",
    );
    await writeFile(phaseDocumentPath, changedDocument);
    await runGit(repoRoot, ["add", "--", "docs/apps/model-policy/phases/00-foundation.md"]);

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: gate definition digest does not match the current indexed document",
    );
  });

  test("uses the indexed phase document when the mutable worktree diverges", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot);
    const phaseDocumentPath = join(repoRoot, "docs/apps/model-policy/phases/00-foundation.md");
    const mutableDocument = (await readFile(phaseDocumentPath, "utf8")).replace(
      "### MP-P0-G01 — First gate",
      "### MP-P0-G99 — Unstaged mutable gate",
    );
    await writeFile(phaseDocumentPath, mutableDocument);

    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toEqual([]);
  });

  test("rejects evidence below the gate-required level", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { achievedEvidenceLevel: "verified", roles: [] });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: verified evidence is below qualified");
  });

  test("rejects an unavailable source commit", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { sourceCommit: "f".repeat(40) });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(`MP-P0-G01: source commit ${"f".repeat(40)} is unavailable`);
  });

  test("rejects qualified evidence without reproducible commands or harnesses", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      commands: [],
      harnessIdentifiers: [],
      emptyToolVersions: true,
      emptyInputIdentities: true,
    });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures.some((failure) => failure.includes("evidence schema rejected"))).toBe(true);
  });

  test("rejects not-applicable verdicts for qualified evidence", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { verdict: "not_applicable" });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures.some((failure) => failure.includes("evidence schema rejected"))).toBe(true);
  });

  test("rejects qualified evidence missing a phase-required independent review", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { roles: ["architecture", "security"] });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: missing required independent technical review");
  });

  test("rejects self-review and a shared reviewer across required roles", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      producerRef: "reviewer:self",
      sharedReviewerRef: "reviewer:self",
    });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(
      failures.some((failure) => failure.includes("reviewer must differ from evidence producer")),
    ).toBe(true);
    expect(
      failures.some((failure) => failure.includes("reviewer ref must be role-separated")),
    ).toBe(true);
  });

  test("rejects empty and oversized review attestations", async () => {
    for (const [content, expectedFailure] of [
      ["", "MP-P0-G01: architecture review attestation: record is not valid JSON"],
      [
        jsonDocumentWithByteLength(CANONICAL_JSON_MAX_BYTES + 1),
        "MP-P0-G01: architecture review attestation: JSON exceeds the 1048576-byte limit",
      ],
    ] as const) {
      const repoRoot = await createFixture(validRoadmap());
      await addEvidenceFixture(repoRoot, { achievedEvidenceLevel: "qualified" });
      const attestationPath =
        "distribution/evidence/model-policy/reviews/mp-p0-g01-architecture.json";
      await writeFile(join(repoRoot, attestationPath), content);
      await rewriteEvidenceFixture(repoRoot, (record) => {
        const binding = record.reviewBindings.find(
          (candidate) => candidate.role === "architecture",
        );
        if (!binding) throw new Error("architecture review binding is missing");
        binding.sha256 = sha256(content);
      });

      const failures = await checkProductPhaseFiles({ repoRoot, write: true });
      expect(failures).toContain(expectedFailure);
    }
  });

  test("rejects malformed UTF-8 in a review attestation", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { achievedEvidenceLevel: "qualified" });
    const attestationPath =
      "distribution/evidence/model-policy/reviews/mp-p0-g01-architecture.json";
    const attestationBytes = withMalformedUtf8(
      new Uint8Array(await Bun.file(join(repoRoot, attestationPath)).arrayBuffer()),
      "reviewer:architecture",
    );
    await writeFile(join(repoRoot, attestationPath), attestationBytes);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      const binding = record.reviewBindings.find((candidate) => candidate.role === "architecture");
      if (!binding) throw new Error("architecture review binding is missing");
      binding.sha256 = sha256(attestationBytes);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: architecture review attestation: JSON is not valid UTF-8",
    );
  });

  test("rejects a sensitive marker hidden in a review attestation by JSON escaping", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { achievedEvidenceLevel: "qualified" });
    const attestationPath =
      "distribution/evidence/model-policy/reviews/mp-p0-g01-architecture.json";
    const attestationText = (await readFile(join(repoRoot, attestationPath), "utf8")).replace(
      "reviewer:architecture",
      "reviewer\\u0040example.invalid",
    );
    await writeFile(join(repoRoot, attestationPath), attestationText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      const binding = record.reviewBindings.find((candidate) => candidate.role === "architecture");
      if (!binding) throw new Error("architecture review binding is missing");
      binding.sha256 = sha256(attestationText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: architecture review attestation contains a sensitive marker after JSON decoding",
    );
  });

  test("rejects duplicate attestation members that hide escaped personal data", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { achievedEvidenceLevel: "qualified" });
    const attestationPath =
      "distribution/evidence/model-policy/reviews/mp-p0-g01-architecture.json";
    const attestationText = (await readFile(join(repoRoot, attestationPath), "utf8")).replace(
      '  "reviewerRef": "reviewer:architecture",',
      '  "reviewerRef": "reviewer\\u0040example.invalid",\n' +
        '  "reviewerRef": "reviewer:architecture",',
    );
    await writeFile(join(repoRoot, attestationPath), attestationText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      const binding = record.reviewBindings.find((candidate) => candidate.role === "architecture");
      if (!binding) throw new Error("architecture review binding is missing");
      binding.sha256 = sha256(attestationText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: architecture review attestation contains a duplicate JSON member name",
    );
  });

  test("rejects one review identity reused by multiple attestation records", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { sharedReviewId: true });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(
      failures.some((failure) =>
        failure.includes("review id MP-REV-P0-G01-SHARED is already bound"),
      ),
    ).toBe(true);
  });

  test("rejects a qualified record with a rejecting verdict or unresolved major finding", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      verdict: "reject",
      majorFindings: ["The evidence remains incomplete."],
    });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: qualified evidence verdict does not approve the gate");
    expect(failures).toContain("MP-P0-G01: qualified evidence retains blocking or major findings");
  });

  test("rejects a favorable evidence record backed by a rejecting independent attestation", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      attestationVerdict: "reject",
      attestationMajorFindings: ["The independent review rejected the candidate."],
    });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(
      failures.some((failure) => failure.includes("review attestation: verdict does not approve")),
    ).toBe(true);
    expect(
      failures.some((failure) => failure.includes("review attestation: retains blocking or major")),
    ).toBe(true);
  });

  test("rejects a repository fixture digest not bound to the source commit", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { inputDigest: `sha256:${"0".repeat(64)}` });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: repository fixture digest mismatch for docs/apps/model-policy/phases/00-foundation.md",
    );
  });

  test("requires service identity and operational evidence for in-service claims", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, { achievedEvidenceLevel: "in_service" });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures.some((failure) => failure.includes("evidence schema rejected"))).toBe(true);
  });

  test("rejects in-service evidence with an unbound operational artifact", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
      invalidServiceDigest: true,
    });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures.some((failure) => failure.includes("smoke test: digest mismatch"))).toBe(true);
  });

  test("rejects in-service evidence without bound deployment authorization", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
      invalidAuthorizationDigest: true,
    });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: deployment authorization: digest mismatch for distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json",
    );
  });

  test("rejects an in-service deployment absent from the operated-environment inputs", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
      serviceDeploymentIdentity: "deployment:model-policy:unbound",
    });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: service observation is not bound to its operated environment",
    );
  });

  test("rejects empty and oversized operational evidence", async () => {
    for (const [content, expectedFailure] of [
      ["", "MP-P0-G01: deployment authorization: operational evidence is not valid JSON"],
      [
        jsonDocumentWithByteLength(CANONICAL_JSON_MAX_BYTES + 1),
        "MP-P0-G01: deployment authorization: JSON exceeds the 1048576-byte limit",
      ],
    ] as const) {
      const repoRoot = await createFixture(validRoadmap());
      await addEvidenceFixture(repoRoot, {
        achievedEvidenceLevel: "in_service",
        includeServiceObservation: true,
      });
      const authorizationPath =
        "distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json";
      await writeFile(join(repoRoot, authorizationPath), content);
      await rewriteEvidenceFixture(repoRoot, (record) => {
        if (!record.serviceObservation) throw new Error("service observation is missing");
        record.serviceObservation.authorizationEvidenceSha256 = sha256(content);
      });

      const failures = await checkProductPhaseFiles({ repoRoot, write: true });
      expect(failures).toContain(expectedFailure);
    }
  });

  test("rejects malformed UTF-8 in operational evidence", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const authorizationPath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json";
    const authorizationBytes = withMalformedUtf8(
      new Uint8Array(await Bun.file(join(repoRoot, authorizationPath)).arrayBuffer()),
      "approval:fixture",
    );
    await writeFile(join(repoRoot, authorizationPath), authorizationBytes);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.authorizationEvidenceSha256 = sha256(authorizationBytes);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: deployment authorization: JSON is not valid UTF-8");
  });

  test("rejects a sensitive marker hidden by JSON Unicode escaping", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const authorizationPath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json";
    const authorizationText = (await readFile(join(repoRoot, authorizationPath), "utf8")).replace(
      "approval:fixture",
      "AKI\\u00411234567890ABCDEF",
    );
    await writeFile(join(repoRoot, authorizationPath), authorizationText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.authorizationEvidenceSha256 = sha256(authorizationText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: deployment authorization contains a sensitive marker after JSON decoding",
    );
  });

  test("rejects duplicate operational members that hide an escaped credential", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const authorizationPath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json";
    const authorizationText = (await readFile(join(repoRoot, authorizationPath), "utf8")).replace(
      '  "authorizationRef": "approval:fixture",',
      '  "authorizationRef": "AKI\\u00411234567890ABCDEF",\n' +
        '  "authorizationRef": "approval:fixture",',
    );
    await writeFile(join(repoRoot, authorizationPath), authorizationText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.authorizationEvidenceSha256 = sha256(authorizationText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: deployment authorization contains a duplicate JSON member name",
    );
  });

  test("rejects deployment authorization recorded exactly when observation starts", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const authorizationPath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json";
    const authorization = JSON.parse(await readFile(join(repoRoot, authorizationPath), "utf8")) as {
      recordedAt: string;
      windowStartedAt: string;
    };
    authorization.recordedAt = authorization.windowStartedAt;
    const authorizationText = `${JSON.stringify(authorization, null, 2)}\n`;
    await writeFile(join(repoRoot, authorizationPath), authorizationText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.authorizationEvidenceSha256 = sha256(authorizationText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: deployment authorization: authorization must predate the observed window",
    );
  });

  test("rejects deployment authorization with a denying outcome", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const authorizationText = `${JSON.stringify(
      {
        schemaVersion: "libre-ai.model-policy-operational-evidence.v1",
        operationalEvidenceId: "MP-OPS-P0-G01-AUTHORIZATION",
        kind: "deployment_authorization",
        phaseId: "MP-P0",
        gateId: "MP-P0-G01",
        evidenceId: "MP-EVD-P0-G01-FIXTURE",
        deploymentIdentity: "deployment:model-policy:fixture",
        windowStartedAt: "2026-07-27T00:00:00Z",
        windowEndedAt: "2026-07-28T00:00:00Z",
        outcome: "denied",
        authorizationRef: "approval:fixture",
        recordedAt: "2026-07-26T00:00:00Z",
      },
      null,
      2,
    )}\n`;
    const authorizationPath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json";
    await writeFile(join(repoRoot, authorizationPath), authorizationText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.authorizationEvidenceSha256 = sha256(authorizationText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(
      failures.some((failure) =>
        failure.includes("deployment authorization: operational evidence schema rejected"),
      ),
    ).toBe(true);
  });

  test("rejects deployment authorization bound to another evidence record", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const authorizationPath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json";
    const authorization = JSON.parse(await readFile(join(repoRoot, authorizationPath), "utf8")) as {
      evidenceId: string;
    };
    authorization.evidenceId = "MP-EVD-P0-G01-UNRELATED";
    const authorizationText = `${JSON.stringify(authorization, null, 2)}\n`;
    await writeFile(join(repoRoot, authorizationPath), authorizationText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.authorizationEvidenceSha256 = sha256(authorizationText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: deployment authorization: evidence, gate, or deployment binding mismatch",
    );
  });

  test("rejects an empty smoke artifact even when its digest is correct", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const smokePath = "distribution/evidence/model-policy/operations/mp-p0-g01-smoke.json";
    const smokeText = "{}\n";
    await writeFile(join(repoRoot, smokePath), smokeText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.smokeEvidenceSha256 = sha256(smokeText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(
      failures.some((failure) =>
        failure.includes("smoke test: operational evidence schema rejected"),
      ),
    ).toBe(true);
  });

  test("rejects an observation outside its bound window", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const smokePath = "distribution/evidence/model-policy/operations/mp-p0-g01-smoke.json";
    const smoke = JSON.parse(await readFile(join(repoRoot, smokePath), "utf8")) as {
      observedAt: string;
    };
    smoke.observedAt = "2026-07-26T23:59:59Z";
    const smokeText = `${JSON.stringify(smoke, null, 2)}\n`;
    await writeFile(join(repoRoot, smokePath), smokeText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.smokeEvidenceSha256 = sha256(smokeText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: smoke test: observation instant is outside the bound window",
    );
  });

  test("rejects an operational artifact recorded after the evidence record", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const smokePath = "distribution/evidence/model-policy/operations/mp-p0-g01-smoke.json";
    const smoke = JSON.parse(await readFile(join(repoRoot, smokePath), "utf8")) as {
      recordedAt: string;
    };
    smoke.recordedAt = "2026-07-28T00:00:01Z";
    const smokeText = `${JSON.stringify(smoke, null, 2)}\n`;
    await writeFile(join(repoRoot, smokePath), smokeText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.smokeEvidenceSha256 = sha256(smokeText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: smoke test: artifact cannot postdate the evidence record",
    );
  });

  test("rejects an operational record predating its observation", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const rollbackPath = "distribution/evidence/model-policy/operations/mp-p0-g01-rollback.json";
    const rollback = JSON.parse(await readFile(join(repoRoot, rollbackPath), "utf8")) as {
      recordedAt: string;
    };
    rollback.recordedAt = "2026-07-27T12:59:59Z";
    const rollbackText = `${JSON.stringify(rollback, null, 2)}\n`;
    await writeFile(join(repoRoot, rollbackPath), rollbackText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.rollbackEvidenceSha256 = sha256(rollbackText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: rollback test: artifact record predates its observation",
    );
  });

  test("rejects reuse of one operational artifact for authorization and smoke", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.smokeEvidencePath =
        record.serviceObservation.authorizationEvidencePath;
      record.serviceObservation.smokeEvidenceSha256 =
        record.serviceObservation.authorizationEvidenceSha256;
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain("MP-P0-G01: operational evidence paths must be distinct");
  });

  test("rejects personal data inside an operational evidence artifact", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    const personalAddress = `alice${"@private.fr"}`;
    const authorizationText = `${JSON.stringify({ email: personalAddress }, null, 2)}\n`;
    const authorizationPath =
      "distribution/evidence/model-policy/operations/mp-p0-g01-authorization.json";
    await writeFile(join(repoRoot, authorizationPath), authorizationText);
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.authorizationEvidenceSha256 = sha256(authorizationText);
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: deployment authorization contains a sensitive marker at line 2",
    );
  });

  test("requires incident artifacts when an in-service window reports incidents", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
      incidentState: "incidents_bound_in_artifacts",
    });
    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures.some((failure) => failure.includes("evidence schema rejected"))).toBe(true);
  });

  test("accepts a resolved incident bound to the same deployment and observation window", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
      includeIncidentEvidence: true,
      incidentState: "incidents_bound_in_artifacts",
    });
    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toEqual([]);
  });

  test("accepts in-service evidence only with a bounded observation window and bound smoke/rollback artifacts", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    expect(await checkProductPhaseFiles({ repoRoot, write: true })).toEqual([]);
  });

  test("rejects an in-service observation ending after the record was produced", async () => {
    const repoRoot = await createFixture(validRoadmap());
    await addEvidenceFixture(repoRoot, {
      achievedEvidenceLevel: "in_service",
      includeServiceObservation: true,
    });
    await rewriteEvidenceFixture(repoRoot, (record) => {
      if (!record.serviceObservation) throw new Error("service observation is missing");
      record.serviceObservation.windowEndedAt = "2099-07-28T00:00:00Z";
    });

    const failures = await checkProductPhaseFiles({ repoRoot, write: true });
    expect(failures).toContain(
      "MP-P0-G01: service observation window cannot end after the evidence record",
    );
  });

  test("leaves both projections unchanged when staging the second write fails", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const appReadmePath = join(repoRoot, "apps/model-policy/README.md");
    const docsReadmePath = join(repoRoot, "docs/apps/model-policy/README.md");
    const docsDirectory = join(repoRoot, "docs/apps/model-policy");
    const beforeApp = await readFile(appReadmePath, "utf8");
    const beforeDocs = await readFile(docsReadmePath, "utf8");
    await chmod(docsDirectory, 0o500);
    try {
      const failures = await checkProductPhaseFiles({ repoRoot, write: true });
      expect(failures[0]).toStartWith("Model Policy projection write failed:");
      expect(await readFile(appReadmePath, "utf8")).toBe(beforeApp);
      expect(await readFile(docsReadmePath, "utf8")).toBe(beforeDocs);
    } finally {
      await chmod(docsDirectory, 0o700);
    }
  });

  test("rolls back the first projection when replacing the second projection fails", async () => {
    const repoRoot = await createFixture(validRoadmap());
    const appReadmePath = join(repoRoot, "apps/model-policy/README.md");
    const docsReadmePath = join(repoRoot, "docs/apps/model-policy/README.md");
    const beforeApp = await readFile(appReadmePath, "utf8");
    const beforeDocs = await readFile(docsReadmePath, "utf8");
    let renameCalls = 0;
    const failures = await checkProductPhaseFiles({
      repoRoot,
      write: true,
      projectionRename: async (oldPath, newPath) => {
        renameCalls += 1;
        if (renameCalls === 2) throw new Error("injected second replacement failure");
        await rename(oldPath, newPath);
      },
    });
    expect(failures).toContain(
      "Model Policy projection write failed: injected second replacement failure",
    );
    expect(await readFile(appReadmePath, "utf8")).toBe(beforeApp);
    expect(await readFile(docsReadmePath, "utf8")).toBe(beforeDocs);
  });

  test("accepts the repository planning record and both projections", async () => {
    expect(await checkProductPhaseFiles()).toEqual([]);
  });
});
