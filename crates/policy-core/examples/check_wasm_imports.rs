// Verifies the policy-core WASM artifact is capability-free: the core module and
// the encoded component both import nothing, and the component exports exactly the
// locked policy-core API and nothing else. This is the security invariant behind
// the WIT world `policy-core` (no clock, randomness, network, filesystem, KV,
// environment or logging capability): a host that instantiates it with an empty
// import object cannot be surprised by a hidden capability request.
//
// Unlike notebook-core, policy-core makes no crypto-qualification claim, so this
// check does not assert SIMD128 or a linear-memory ceiling — those are
// notebook-specific device-qualification concerns, not policy-core requirements.
//
// Usage: check_wasm_imports <module.wasm> [component.wasm]
//   module.wasm    — the cargo `wasm32-unknown-unknown` release artifact.
//   component.wasm — an already-encoded component (e.g. the jco build output). When
//                    omitted, the component is encoded here from the core module.

use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use wasmparser::{ComponentExternalKind, Encoding, Parser, Payload};
use wit_component::ComponentEncoder;

const EXPECTED_EXPORT: &str = "libre-ai:policy-core/api@1.0.0";

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let path = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: check_wasm_imports <module.wasm> [component.wasm]")?;
    let component_path = arguments.next().map(PathBuf::from);
    if arguments.next().is_some() {
        return Err("usage: check_wasm_imports <module.wasm> [component.wasm]".into());
    }
    let module = fs::read(&path)?;
    check_core_module(&module)?;

    let component = if let Some(component_path) = &component_path {
        fs::read(component_path)?
    } else {
        ComponentEncoder::default()
            .module(&module)?
            .validate(true)
            .encode()?
    };
    check_component(&component)?;

    println!(
        "policy-core WASM verified: 0 module imports, 0 component imports, exactly `{}` exported ({}, component={})",
        EXPECTED_EXPORT,
        path.display(),
        component_path.as_ref().map_or_else(
            || "generated".to_owned(),
            |value| value.display().to_string()
        )
    );
    Ok(())
}

fn check_core_module(module: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut imports = Vec::new();
    let mut component_metadata = false;
    let mut core_module = false;

    for payload in Parser::new(0).parse_all(module) {
        match payload? {
            Payload::Version { encoding, .. } => core_module = encoding == Encoding::Module,
            Payload::ImportSection(section) => {
                for import in section {
                    imports.push(format!("{:?}", import?));
                }
            }
            Payload::CustomSection(section) if section.name().starts_with("component-type") => {
                component_metadata = true;
            }
            _ => {}
        }
    }

    if !core_module {
        return Err("artifact is not a core WebAssembly module".into());
    }
    if !imports.is_empty() {
        return Err(format!("forbidden WebAssembly imports: {}", imports.join(", ")).into());
    }
    if !component_metadata {
        return Err("wit-bindgen component-type metadata is missing".into());
    }
    Ok(())
}

fn check_component(component: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut is_component = false;
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut depth = 0usize;

    for payload in Parser::new(0).parse_all(component) {
        match payload? {
            Payload::Version { encoding, .. } if depth == 0 && encoding == Encoding::Component => {
                is_component = true;
            }
            Payload::ComponentImportSection(section) if depth == 0 => {
                for import in section {
                    imports.push(format!("{:?}", import?));
                }
            }
            Payload::ComponentExportSection(section) if depth == 0 => {
                for export in section {
                    let export = export?;
                    exports.push((export.name.name.to_owned(), export.kind));
                }
            }
            Payload::ModuleSection { .. } | Payload::ComponentSection { .. } => depth += 1,
            Payload::End(_) if depth > 0 => depth -= 1,
            _ => {}
        }
    }

    if !is_component {
        return Err("component encoding did not produce a WebAssembly component".into());
    }
    if !imports.is_empty() {
        return Err(format!("forbidden component imports: {}", imports.join(", ")).into());
    }
    let expected = [(EXPECTED_EXPORT.to_owned(), ComponentExternalKind::Instance)];
    if exports != expected {
        return Err(format!("unexpected component exports: {exports:?}").into());
    }
    Ok(())
}
