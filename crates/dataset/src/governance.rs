//! Curated governance data: model creators and their country of origin.

use serde::Deserialize;

use crate::error::DatasetError;

/// One curated model creator.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderInfo {
    id: String,
    name: String,
    origin: String,
    /// Citation backing the entry (official company page).
    source: String,
    /// Hugging Face organisation handles mapped onto this provider.
    #[serde(default)]
    aliases: Vec<String>,
}

impl ProviderInfo {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// ISO 3166-1 alpha-2 country of the company's headquarters.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct GovernanceDoc {
    version: u32,
    providers: Vec<ProviderInfo>,
}

/// Validated curated governance data.
#[derive(Debug, Clone, PartialEq)]
pub struct Governance {
    providers: Vec<ProviderInfo>,
}

impl Governance {
    pub fn providers(&self) -> &[ProviderInfo] {
        &self.providers
    }

    pub fn provider(&self, id: &str) -> Option<&ProviderInfo> {
        self.providers.iter().find(|provider| provider.id == id)
    }

    /// Resolve a Hugging Face organisation handle to its curated provider.
    pub fn provider_for_alias(&self, alias: &str) -> Option<&ProviderInfo> {
        self.providers
            .iter()
            .find(|provider| provider.aliases.iter().any(|known| known == alias))
    }
}

/// Parse and validate the curated governance YAML. Fail-closed: duplicate
/// ids and malformed country codes refuse the whole document.
pub fn parse_governance(yaml: &str) -> Result<Governance, DatasetError> {
    let doc: GovernanceDoc = yaml_serde::from_str(yaml)?;
    if doc.version != 1 {
        return Err(DatasetError::UnsupportedGovernanceVersion(doc.version));
    }

    let mut seen: Vec<&str> = Vec::new();
    for provider in &doc.providers {
        if seen.contains(&provider.id.as_str()) {
            return Err(DatasetError::DuplicateProviderId(provider.id.clone()));
        }
        seen.push(&provider.id);

        let code_ok =
            provider.origin.len() == 2 && provider.origin.chars().all(|c| c.is_ascii_uppercase());
        if !code_ok {
            return Err(DatasetError::InvalidCountryCode {
                provider: provider.id.clone(),
                code: provider.origin.clone(),
            });
        }
    }

    Ok(Governance {
        providers: doc.providers,
    })
}
