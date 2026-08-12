//! Generated Nexus contracts.
//!
//! This crate is generated from `schemas/` by
//! `packages/contracts/scripts/generate.py`. Do not hand-edit `generated.rs`;
//! regenerate it and commit the result. `validated.rs` is a handwritten typed
//! layer that converts generated wire DTOs into domain-typed IDs and
//! vocabulary enums (EP-002).

#![forbid(unsafe_code)]

mod generated;
mod validated;

pub use generated::*;
pub use validated::*;
