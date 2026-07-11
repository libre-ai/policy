use std::{fs, path::PathBuf};

fn asset(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(path)
}

#[test]
fn uses_libre_ia_design_system_v2() {
    let manifest = fs::read_to_string(asset("libre-ia/manifest.json")).unwrap();
    assert!(manifest.contains("\"version\": \"2.0.0\""));
    let tokens = fs::read_to_string(asset("libre-ia/tokens.css")).unwrap();
    assert!(tokens.contains("--color-libre: #22C55E"));
    assert!(asset("favicon.svg").is_file());
}

#[test]
fn clearance_styles_are_flat_and_token_only() {
    let css = fs::read_to_string(asset("clearance.css")).unwrap();
    for forbidden in [
        "rgb(",
        "hsl(",
        "linear-gradient(",
        "radial-gradient(",
        "box-shadow:",
        "url(http",
    ] {
        assert!(
            !css.contains(forbidden),
            "forbidden `{forbidden}` in clearance.css"
        );
    }
}
