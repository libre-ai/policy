//! Read-only axum API: no HTTP mutation by construction — policy and snapshot
//! only change via files + redeploy. Refuses to boot on invalid inputs.
//! Envelope `{ data, meta }`, cursor pagination, zero PII in logs.
#![forbid(unsafe_code)]
