//! Policy loading: default rulebook ⊕ org policy merge, fail-closed validation.
//!
//! An invalid policy never degrades into a partial one — compilation either
//! yields a complete effective policy or refuses to evaluate.
#![forbid(unsafe_code)]
