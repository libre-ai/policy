//! Policy loading: default rulebook ⊕ org policy merge, fail-closed validation.
//!
//! An invalid policy never degrades into a partial one — compilation either
//! yields a complete effective policy or refuses to evaluate.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::Deserialize;

use rumble_ai_clearance_domain::{
    BenchDimension, Constraint, CountryCode, Openness, Policy, ProviderId, Purpose, RankingSpec,
    Rule, Sensitivity, Task,
};

/// Why a policy document was refused. Fail-closed: any error means no
/// effective policy at all.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("invalid policy YAML: {source}")]
    Yaml {
        #[from]
        source: yaml_serde::Error,
    },
    #[error("unsupported policy version {0} (expected 1)")]
    UnsupportedVersion(u32),
    #[error("disable_rules references unknown rule id `{0}`")]
    UnknownDisabledRule(String),
    #[error("duplicate rule id `{0}`")]
    DuplicateRuleId(String),
    #[error("invalid rule `{id}`: {message}")]
    InvalidRule { id: String, message: String },
    #[error("a rulebook must not disable rules (disable_rules belongs to the org policy)")]
    RulebookDisablesRules,
    #[error(
        "ranking for task `{0}` has an empty dimension list; drop the entry instead \
         (an empty list would silently degrade ranking to price only)"
    )]
    EmptyRankingDimensions(String),
}

/// A parsed policy document: either the default rulebook or an org policy.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDoc {
    version: u32,
    #[serde(default)]
    #[allow(dead_code)] // Displayed by tooling; not used in compilation.
    organization: Option<String>,
    #[serde(default)]
    disable_rules: Vec<DisableDoc>,
    #[serde(default)]
    rules: Vec<RuleDoc>,
    /// Preference ordering per task (never eligibility). Org entries
    /// override rulebook entries task by task.
    #[serde(default)]
    ranking: BTreeMap<TaskDoc, Vec<BenchDimensionDoc>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisableDoc {
    id: String,
    /// Mandatory: a deactivation is named and traced, never silent.
    #[allow(dead_code)] // The reason lives in the org's versioned file.
    reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleDoc {
    id: String,
    #[serde(default)]
    #[allow(dead_code)] // Displayed by tooling; not used in compilation.
    description: Option<String>,
    #[serde(default)]
    #[allow(dead_code)] // Citation backing the rule; displayed by tooling.
    source: Option<String>,
    /// `always` as a plain string, scoped variants as a one-key map
    /// (`sensitivity_at_least: c2`).
    #[serde(with = "yaml_serde::with::singleton_map")]
    applies: AppliesDoc,
    /// One-key map, e.g. `deny_origin: [US, CN]`.
    #[serde(with = "yaml_serde::with::singleton_map")]
    constraint: ConstraintDoc,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum AppliesDoc {
    Always,
    SensitivityAtLeast(SensitivityDoc),
    Purpose(PurposeDoc),
    Task(TaskDoc),
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SensitivityDoc {
    C0,
    C1,
    C2,
    C3,
}

impl From<SensitivityDoc> for Sensitivity {
    fn from(doc: SensitivityDoc) -> Self {
        match doc {
            SensitivityDoc::C0 => Sensitivity::C0,
            SensitivityDoc::C1 => Sensitivity::C1,
            SensitivityDoc::C2 => Sensitivity::C2,
            SensitivityDoc::C3 => Sensitivity::C3,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PurposeDoc {
    PublicContent,
    PersonalData,
    AutomatedDecision,
    HealthData,
}

impl From<PurposeDoc> for Purpose {
    fn from(doc: PurposeDoc) -> Self {
        match doc {
            PurposeDoc::PublicContent => Purpose::PublicContent,
            PurposeDoc::PersonalData => Purpose::PersonalData,
            PurposeDoc::AutomatedDecision => Purpose::AutomatedDecision,
            PurposeDoc::HealthData => Purpose::HealthData,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TaskDoc {
    CodeGeneration,
    Agentic,
    SummaryExtraction,
    Classification,
    Writing,
    Translation,
    Reasoning,
    GeneralChat,
}

impl From<TaskDoc> for Task {
    fn from(doc: TaskDoc) -> Self {
        match doc {
            TaskDoc::CodeGeneration => Task::CodeGeneration,
            TaskDoc::Agentic => Task::Agentic,
            TaskDoc::SummaryExtraction => Task::SummaryExtraction,
            TaskDoc::Classification => Task::Classification,
            TaskDoc::Writing => Task::Writing,
            TaskDoc::Translation => Task::Translation,
            TaskDoc::Reasoning => Task::Reasoning,
            TaskDoc::GeneralChat => Task::GeneralChat,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OpennessDoc {
    Closed,
    OpenWeight,
    OpenSource,
}

impl From<OpennessDoc> for Openness {
    fn from(doc: OpennessDoc) -> Self {
        match doc {
            OpennessDoc::Closed => Openness::Closed,
            OpennessDoc::OpenWeight => Openness::OpenWeight,
            OpennessDoc::OpenSource => Openness::OpenSource,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BenchDimensionDoc {
    Intelligence,
    Coding,
    Agentic,
    Math,
    Multilingual,
}

impl From<BenchDimensionDoc> for BenchDimension {
    fn from(doc: BenchDimensionDoc) -> Self {
        match doc {
            BenchDimensionDoc::Intelligence => BenchDimension::Intelligence,
            BenchDimensionDoc::Coding => BenchDimension::Coding,
            BenchDimensionDoc::Agentic => BenchDimension::Agentic,
            BenchDimensionDoc::Math => BenchDimension::Math,
            BenchDimensionDoc::Multilingual => BenchDimension::Multilingual,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum ConstraintDoc {
    DenyOrigin(Vec<String>),
    DenyHostingJurisdiction(Vec<String>),
    RequireHostingJurisdictionIn(Vec<String>),
    DenyProvider(Vec<String>),
    RequireOpenness(Vec<OpennessDoc>),
    RequireSelfHostable(bool),
    MinContextWindow(u64),
    MinBench {
        dimension: BenchDimensionDoc,
        threshold: f64,
    },
}

/// Parse one policy document (rulebook or org policy). Fail-closed: any
/// unknown field, missing mandatory field or malformed value is an error.
pub fn parse_policy(yaml: &str) -> Result<PolicyDoc, PolicyError> {
    let doc = yaml_serde::from_str::<PolicyDoc>(yaml)?;
    if doc.version != 1 {
        return Err(PolicyError::UnsupportedVersion(doc.version));
    }
    for (task, dimensions) in &doc.ranking {
        if dimensions.is_empty() {
            return Err(PolicyError::EmptyRankingDimensions(format!("{task:?}")));
        }
    }
    Ok(doc)
}

/// Merge the default rulebook with the org policy into the effective policy:
/// rulebook rules minus traced deactivations, plus org rules. Duplicate ids
/// and dangling deactivations are errors.
pub fn compile(rulebook: &PolicyDoc, org: &PolicyDoc) -> Result<Policy, PolicyError> {
    if !rulebook.disable_rules.is_empty() {
        return Err(PolicyError::RulebookDisablesRules);
    }

    let known_ids: Vec<&str> = rulebook.rules.iter().map(|rule| rule.id.as_str()).collect();
    for disable in &org.disable_rules {
        if !known_ids.contains(&disable.id.as_str()) {
            return Err(PolicyError::UnknownDisabledRule(disable.id.clone()));
        }
    }

    let disabled: Vec<&str> = org
        .disable_rules
        .iter()
        .map(|disable| disable.id.as_str())
        .collect();

    let mut seen: Vec<&str> = Vec::new();
    let mut rules = Vec::new();
    let effective_docs = rulebook
        .rules
        .iter()
        .filter(|rule| !disabled.contains(&rule.id.as_str()))
        .chain(org.rules.iter());

    for doc in effective_docs {
        if seen.contains(&doc.id.as_str()) {
            return Err(PolicyError::DuplicateRuleId(doc.id.clone()));
        }
        seen.push(doc.id.as_str());
        rules.push(build_rule(doc)?);
    }

    Ok(Policy::new(rules))
}

fn build_rule(doc: &RuleDoc) -> Result<Rule, PolicyError> {
    let constraint = build_constraint(doc)?;
    let rule = match &doc.applies {
        AppliesDoc::Always => Rule::always(&doc.id, constraint),
        AppliesDoc::SensitivityAtLeast(level) => {
            Rule::when_sensitivity_at_least((*level).into(), &doc.id, constraint)
        }
        AppliesDoc::Purpose(purpose) => Rule::when_purpose((*purpose).into(), &doc.id, constraint),
        AppliesDoc::Task(task) => Rule::when_task((*task).into(), &doc.id, constraint),
    };
    Ok(rule)
}

/// A need profile input: what the business user wants to do. Deserializable
/// from YAML (files) or JSON (API bodies) with the same strict taxonomy.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedInput {
    task: TaskDoc,
    purpose: PurposeDoc,
    sensitivity: SensitivityDoc,
}

impl NeedInput {
    pub fn profile(&self) -> rumble_ai_clearance_domain::NeedProfile {
        rumble_ai_clearance_domain::NeedProfile::new(
            self.task.into(),
            self.purpose.into(),
            self.sensitivity.into(),
        )
    }
}

/// Parse a need profile (task + purpose + sensitivity) from YAML.
pub fn parse_need(yaml: &str) -> Result<rumble_ai_clearance_domain::NeedProfile, PolicyError> {
    let doc: NeedInput = yaml_serde::from_str(yaml)?;
    Ok(doc.profile())
}

/// Per-task preference ordering compiled from rulebook ⊕ org.
#[derive(Debug, Clone, PartialEq)]
pub struct RankingConfig {
    specs: Vec<(Task, Vec<BenchDimension>)>,
}

impl RankingConfig {
    /// Ranking dimensions for a task; tasks nobody mapped fall back to the
    /// Intelligence index.
    pub fn spec_for(&self, task: Task) -> RankingSpec {
        let dimensions = self
            .specs
            .iter()
            .find(|(mapped, _)| *mapped == task)
            .map(|(_, dimensions)| dimensions.clone())
            .unwrap_or_else(|| vec![BenchDimension::Intelligence]);
        RankingSpec::new(dimensions)
    }
}

/// Merge ranking sections: rulebook defaults, org overrides task by task.
pub fn compile_ranking(rulebook: &PolicyDoc, org: &PolicyDoc) -> RankingConfig {
    let mut merged: BTreeMap<TaskDoc, Vec<BenchDimensionDoc>> = rulebook.ranking.clone();
    for (task, dimensions) in &org.ranking {
        merged.insert(*task, dimensions.clone());
    }
    RankingConfig {
        specs: merged
            .into_iter()
            .map(|(task, dimensions)| {
                (
                    task.into(),
                    dimensions.into_iter().map(Into::into).collect(),
                )
            })
            .collect(),
    }
}

fn build_constraint(doc: &RuleDoc) -> Result<Constraint, PolicyError> {
    let constraint = match &doc.constraint {
        ConstraintDoc::DenyOrigin(countries) => {
            Constraint::DenyOrigin(countries.iter().map(CountryCode::new).collect())
        }
        ConstraintDoc::DenyHostingJurisdiction(countries) => {
            Constraint::DenyHostingJurisdiction(countries.iter().map(CountryCode::new).collect())
        }
        ConstraintDoc::RequireHostingJurisdictionIn(countries) => {
            Constraint::RequireHostingJurisdictionIn(
                countries.iter().map(CountryCode::new).collect(),
            )
        }
        ConstraintDoc::DenyProvider(providers) => {
            Constraint::DenyProvider(providers.iter().map(ProviderId::new).collect())
        }
        ConstraintDoc::RequireOpenness(allowed) => {
            Constraint::RequireOpenness(allowed.iter().map(|doc| (*doc).into()).collect())
        }
        ConstraintDoc::RequireSelfHostable(true) => Constraint::RequireSelfHostable,
        ConstraintDoc::RequireSelfHostable(false) => {
            return Err(PolicyError::InvalidRule {
                id: doc.id.clone(),
                message: "require_self_hostable: false is meaningless; drop the rule instead"
                    .to_string(),
            });
        }
        ConstraintDoc::MinContextWindow(minimum) => Constraint::MinContextWindow(*minimum),
        ConstraintDoc::MinBench {
            dimension,
            threshold,
        } => Constraint::MinBench {
            dimension: (*dimension).into(),
            threshold: *threshold,
        },
    };
    Ok(constraint)
}
