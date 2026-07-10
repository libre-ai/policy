//! Curated governance data: parsing and validation of the shipped file.

use rumble_ai_clearance_dataset::{DatasetError, parse_governance};

const SHIPPED_PROVIDERS: &str = include_str!("../../../content/governance/providers.yaml");

#[test]
fn shipped_governance_file_parses_and_validates() {
    let governance = parse_governance(SHIPPED_PROVIDERS).expect("shipped governance parses");

    assert!(
        governance.providers().len() >= 30,
        "expected a substantial initial curation, got {}",
        governance.providers().len()
    );

    let mistral = governance
        .provider("mistralai")
        .expect("mistralai is curated");
    assert_eq!(mistral.origin().to_owned(), "FR".to_owned());
}

#[test]
fn duplicate_provider_ids_are_rejected() {
    let yaml = r#"
version: 1
providers:
  - id: acme
    name: Acme
    origin: FR
    source: "https://acme.example"
  - id: acme
    name: Acme Again
    origin: DE
    source: "https://acme.example"
"#;
    assert!(matches!(
        parse_governance(yaml),
        Err(DatasetError::DuplicateProviderId(id)) if id == "acme"
    ));
}

#[test]
fn invalid_country_code_is_rejected() {
    let yaml = r#"
version: 1
providers:
  - id: acme
    name: Acme
    origin: France
    source: "https://acme.example"
"#;
    assert!(matches!(
        parse_governance(yaml),
        Err(DatasetError::InvalidCountryCode { provider, .. }) if provider == "acme"
    ));
}

#[test]
fn alias_lookup_maps_hf_handles_to_providers() {
    let governance = parse_governance(SHIPPED_PROVIDERS).expect("shipped governance parses");

    let meta = governance
        .provider_for_alias("meta-llama")
        .expect("meta-llama alias is mapped");
    assert_eq!(meta.id(), "meta");
}
