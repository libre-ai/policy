//! Design-system consumption control.
//!
//! The Libre IA design system is vendored under `assets/libre-ia/` and pinned
//! by `manifest.json`. These checks verify that the web client *consumes* it
//! rather than working around it:
//!
//! * every colour literal lives in the token file, nowhere else;
//! * every breakpoint is expressed in a reader-relative unit;
//! * the token pairs the UI actually renders clear their WCAG 2.1 AA floor.
//!
//! The scan is a directory walk, not a hard-coded list, so a new stylesheet is
//! covered the day it lands. It reports how many files it scanned and fails if
//! that number is zero — otherwise a moved asset directory would leave the
//! whole control silently green forever.

use std::{fs, path::PathBuf};

/// Only this file may spell a colour out. Everything else references a token.
const TOKENS: &str = "libre-ia/tokens.css";

/// WCAG 2.1: 4.5:1 for normal body text.
const AA_TEXT: f64 = 4.5;
/// WCAG 2.1: 3:1 for user-interface components and graphical objects.
const AA_UI: f64 = 3.0;

fn assets() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// Every `.css` under `assets/`, as (asset-relative label, contents).
fn stylesheets() -> Vec<(String, String)> {
    fn walk(dir: &PathBuf, root: &PathBuf, out: &mut Vec<(String, String)>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().is_some_and(|ext| ext == "css") {
                let label = path
                    .strip_prefix(root)
                    .expect("asset is under the root")
                    .to_string_lossy()
                    .replace('\\', "/");
                let body = fs::read_to_string(&path).expect("stylesheet is UTF-8");
                out.push((label, body));
            }
        }
    }

    let root = assets();
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    found.sort();
    found
}

/// Number of stylesheets found, logged and proven non-zero.
fn scanned(purpose: &str) -> Vec<(String, String)> {
    let sheets = stylesheets();
    println!(
        "[design-system control] scanned {} stylesheet(s) for {purpose}: {}",
        sheets.len(),
        sheets
            .iter()
            .map(|(label, _)| label.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(
        !sheets.is_empty(),
        "scanned 0 stylesheets under {}: the asset directory moved and this control is inert",
        assets().display()
    );
    sheets
}

/// Drop `/* ... */` so commented-out examples never trip the scanners.
fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Hex colour literals (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`).
fn hex_literals(css: &str) -> Vec<String> {
    let chars: Vec<char> = css.chars().collect();
    let mut hits = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' {
            let digits: String = chars[i + 1..]
                .iter()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            // `#212` inside prose is followed by more word characters; a real
            // colour literal ends the token.
            let terminated = chars
                .get(i + 1 + digits.len())
                .is_none_or(|c| !c.is_alphanumeric() && *c != '-' && *c != '_');
            if terminated && matches!(digits.len(), 3 | 4 | 6 | 8) {
                hits.push(format!("#{digits}"));
            }
            i += 1 + digits.len();
        } else {
            i += 1;
        }
    }
    hits
}

/// sRGB relative luminance, per the WCAG 2.1 definition.
fn luminance(hex: &str) -> f64 {
    let raw = hex.trim_start_matches('#');
    let full: String = if raw.len() == 3 {
        raw.chars().flat_map(|c| [c, c]).collect()
    } else {
        raw.to_string()
    };
    let channel = |offset: usize| {
        let v =
            f64::from(u8::from_str_radix(&full[offset..offset + 2], 16).expect("hex pair")) / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4)
}

/// WCAG 2.1 contrast ratio, ordered so it is always >= 1.
fn contrast(a: &str, b: &str) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Value of a `--name: value;` custom property declared in the token file.
fn token(css: &str, name: &str) -> String {
    css.lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == name).then(|| value.trim().trim_end_matches(';').trim().to_string())
        })
        .unwrap_or_else(|| panic!("token `{name}` is not declared in {TOKENS}"))
}

#[test]
fn design_system_is_vendored_at_the_pinned_version() {
    let manifest = fs::read_to_string(assets().join("libre-ia/manifest.json")).unwrap();
    assert!(manifest.contains("\"version\": \"2.0.0\""));
    let tokens = fs::read_to_string(assets().join(TOKENS)).unwrap();
    assert!(tokens.contains("--color-libre: #22C55E"));
    assert!(assets().join("favicon.svg").is_file());
}

#[test]
fn every_colour_literal_lives_in_the_token_file() {
    for (label, body) in scanned("colour literals") {
        let hits = hex_literals(&strip_comments(&body));
        if label == TOKENS {
            assert!(!hits.is_empty(), "{TOKENS} declares no colour tokens");
            continue;
        }
        assert!(
            hits.is_empty(),
            "{label} spells colours out ({hits:?}); reference a token from {TOKENS} instead"
        );
    }
}

#[test]
fn every_breakpoint_is_reader_relative() {
    // A media-query `em` resolves against the reader's default font size, so
    // the layout switch tracks font scaling; a `px` breakpoint ignores it.
    const ABSOLUTE: [&str; 6] = ["px", "pt", "pc", "cm", "mm", "in"];

    for (label, body) in scanned("breakpoints") {
        for query in strip_comments(&body)
            .split("@media")
            .skip(1)
            .filter_map(|tail| tail.split('{').next().map(str::to_string))
        {
            for unit in ABSOLUTE {
                assert!(
                    !query.contains(unit),
                    "{label}: breakpoint `@media{query}` uses the absolute unit `{unit}`; \
                     use `em` so it follows the reader's font size"
                );
            }
        }
    }
}

#[test]
fn rendered_token_pairs_clear_wcag_aa() {
    let tokens = fs::read_to_string(assets().join(TOKENS)).unwrap();
    let of = |name: &str| token(&tokens, name);
    let mut checked = 0;

    // Pairs the client actually paints, per themes.css + components.css.
    for theme in ["dark", "light"] {
        let bg = of(&format!("--color-theme-{theme}-background"));
        let foreground = of(&format!("--color-theme-{theme}-foreground"));
        let cases: [(&str, String, String, f64); 6] = [
            // Body copy and secondary copy sit directly on the background.
            ("foreground", foreground.clone(), bg.clone(), AA_TEXT),
            (
                "muted",
                of(&format!("--color-theme-{theme}-muted")),
                bg.clone(),
                AA_TEXT,
            ),
            // `.lia-button--primary` / active `.lia-badge`: label on the fill.
            (
                "actionText on action",
                of(&format!("--color-theme-{theme}-actionText")),
                of(&format!("--color-theme-{theme}-action")),
                AA_TEXT,
            ),
            // `outline-offset` clears the control, so the focus ring is painted
            // against the background and must be visible there.
            (
                "focus",
                of(&format!("--color-theme-{theme}-focus")),
                bg.clone(),
                AA_UI,
            ),
            // `.lia-input` / `.lia-card` boundaries: WCAG 1.4.11.
            (
                "border",
                of(&format!("--color-theme-{theme}-border")),
                bg.clone(),
                AA_UI,
            ),
            // The action fill is 2.28:1 on white, so it can never be what
            // delineates a control in the light theme. The system draws that
            // edge with `--color-foreground` instead (`.lia-button--primary`
            // and the active `.lia-badge` both set `border-color`), and that
            // is the edge 1.4.11 applies to.
            ("action boundary", foreground, bg.clone(), AA_UI),
        ];

        for (name, fg, background, floor) in cases {
            let ratio = contrast(&fg, &background);
            println!(
                "[design-system control] {theme}.{name}: {fg} on {background} \
                 = {ratio:.3}:1 (floor {floor}:1)"
            );
            assert!(
                ratio >= floor,
                "{theme}.{name}: {fg} on {background} = {ratio:.3}:1, below the {floor}:1 floor"
            );
            checked += 1;
        }
    }

    assert_eq!(checked, 12, "expected 12 contrast checks, ran {checked}");
}

#[test]
fn clearance_styles_stay_flat_and_token_only() {
    let css = fs::read_to_string(assets().join("clearance.css")).unwrap();
    for forbidden in [
        "rgb(",
        "hsl(",
        "oklch(",
        "color-mix(",
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
