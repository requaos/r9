//! Shared integration-test modules.
//!
//! `mod.rs` is required: without it Cargo would build `aead_link.rs` as its
//! own test target instead of a module of every test that says `mod common;`.

pub mod aead_link;
