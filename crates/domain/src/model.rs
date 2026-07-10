//! Model identity, governance metadata and deployment paths.

use serde::{Deserialize, Serialize};

/// ISO 3166-1 alpha-2 country code, e.g. `US`, `FR`, `CN`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CountryCode(String);

impl CountryCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into())
    }
}

impl std::fmt::Display for CountryCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Kind of externally-operated inference API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKind {
    EuSovereign,
    Provider,
    Hyperscaler,
}

/// Where inference data flows for a given deployment path.
///
/// `SelfHosted` structurally implies no foreign jurisdiction: nothing leaves
/// the organisation, so jurisdiction-scoped bans are vacuously satisfied.
/// For `Api` paths, `country` is where the service is hosted and
/// `jurisdiction` the legal regime it answers to — a US provider hosted in
/// the EU still answers to US law (CLOUD Act), so the two may differ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hosting {
    SelfHosted,
    Api {
        kind: ApiKind,
        country: Option<CountryCode>,
        jurisdiction: Option<CountryCode>,
    },
}

impl Hosting {
    /// An API path whose hosting country and jurisdiction are both known.
    pub fn api(kind: ApiKind, country: CountryCode, jurisdiction: CountryCode) -> Self {
        Self::Api {
            kind,
            country: Some(country),
            jurisdiction: Some(jurisdiction),
        }
    }
}

/// Identifier of a model provider, e.g. `mistralai`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(String);

impl ProviderId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// How open a model is. `OpenWeight` means downloadable weights under a
/// possibly restrictive licence; `OpenSource` requires an OSI-approved one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Openness {
    Closed,
    OpenWeight,
    OpenSource,
}

/// Benchmark dimension, mirroring the Artificial Analysis indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchDimension {
    Intelligence,
    Coding,
    Agentic,
    Math,
    Multilingual,
}

/// API price in USD per million tokens.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Price {
    input_per_mtok: f64,
    output_per_mtok: f64,
}

impl Price {
    pub fn per_mtok(input: f64, output: f64) -> Self {
        Self {
            input_per_mtok: input,
            output_per_mtok: output,
        }
    }

    /// Blended price at the 3:1 input:output ratio used by Artificial
    /// Analysis, so rankings stay comparable with their published numbers.
    pub fn blended(&self) -> f64 {
        (3.0 * self.input_per_mtok + self.output_per_mtok) / 4.0
    }
}

/// A model as described by the org's snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub(crate) id: String,
    pub(crate) provider: ProviderId,
    pub(crate) origin: Option<CountryCode>,
    pub(crate) openness: Option<Openness>,
    pub(crate) self_hostable: Option<bool>,
    pub(crate) context_window: Option<u64>,
    pub(crate) hostings: Vec<Hosting>,
    pub(crate) bench: Vec<(BenchDimension, f64)>,
    pub(crate) price: Option<Price>,
}

impl Model {
    pub fn new(id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            provider: ProviderId::new(provider),
            origin: None,
            openness: None,
            self_hostable: None,
            context_window: None,
            hostings: Vec::new(),
            bench: Vec::new(),
            price: None,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// Score for one benchmark dimension, if the snapshot has it.
    pub fn bench_score(&self, dimension: BenchDimension) -> Option<f64> {
        self.bench
            .iter()
            .find(|(existing, _)| *existing == dimension)
            .map(|(_, score)| *score)
    }

    pub fn with_origin(mut self, origin: CountryCode) -> Self {
        self.origin = Some(origin);
        self
    }

    pub fn with_openness(mut self, openness: Openness) -> Self {
        self.openness = Some(openness);
        self
    }

    pub fn with_self_hostable(mut self, self_hostable: bool) -> Self {
        self.self_hostable = Some(self_hostable);
        self
    }

    pub fn with_context_window(mut self, tokens: u64) -> Self {
        self.context_window = Some(tokens);
        self
    }

    pub fn with_hosting(mut self, hosting: Hosting) -> Self {
        self.hostings.push(hosting);
        self
    }

    pub fn with_bench(mut self, dimension: BenchDimension, score: f64) -> Self {
        self.bench.retain(|(existing, _)| *existing != dimension);
        self.bench.push((dimension, score));
        self
    }

    pub fn with_price(mut self, price: Price) -> Self {
        self.price = Some(price);
        self
    }
}
