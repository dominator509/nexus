//! EP-018 skill package errors (SPEC-010; ADR-025).
//!
//! The error type and codes live here so vocabulary parsers and the
//! manifest/package types share one error surface (single import).

pub use crate::manifest::{SkillPackage, SkillPackageError, SkillPackageErrorCode};
