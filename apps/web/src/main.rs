//! Application entrypoint. The body is cfg-gated on the `web` feature; host
//! builds (default features, used by CI) compile an empty `main` with no
//! renderer dependencies.
#![forbid(unsafe_code)]

#[cfg(feature = "web")]
fn main() {
    dioxus::launch(rumble_ai_clearance_web::App);
}

#[cfg(not(feature = "web"))]
fn main() {
    // Host build: the UI is exercised through SSR tests instead.
}
