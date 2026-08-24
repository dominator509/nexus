//! SPDX license expression parsing and classification (SPEC-019;
//! LICENSE_POLICY.md; EP-039 M3).
//!
//! External license strings arrive as SPDX expressions. This module is
//! the transport boundary that parses them and produces a canonical
//! Nexus license class. It distinguishes (directive I):
//!
//! - `MIT`                         -> GREEN
//! - `MIT OR Apache-2.0`           -> GREEN (both branches GREEN)
//! - `MIT AND GPL-3.0`             -> SIDECAR (copyleft applies, NOT
//!   green merely because MIT appears)
//! - `GPL-3.0-only` / `-or-later`  -> SIDECAR
//! - `LicenseRef-*`                -> UNKNOWN (fail closed)
//! - unknown aliases               -> UNKNOWN (fail closed)
//!
//! Combination semantics (fail-closed):
//! - leaf: policy-file mapping (allowlist/classes), else M1 classifier,
//!   else UNKNOWN
//! - AND: every branch applies -> most restrictive branch wins
//! - OR: a choice is offered -> most restrictive branch wins (a grant
//!   that includes a copyleft or unknown option is never auto-approved)
//! - WITH: exception adds permission; base license class governs; an
//!   unknown exception makes the leaf UNKNOWN
//! - `/` is the deprecated OR separator; it is normalized to OR
//! - parentheses group
//!
//! Any unknown leaf fails the whole expression closed. No expression is
//! ever approved merely because it contains a permissive id.

use nexus_supply_chain::vocabulary::LicenseClass;

use crate::policy_files::PolicyFiles;

/// Result of parsing + classifying an SPDX expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpdxClassification {
    /// Canonical class of the whole expression.
    pub class: Option<LicenseClass>,
    /// Deterministic human-safe reason.
    pub reason: String,
    /// True when the expression references at least one id outside the
    /// canonical Nexus policy tables.
    pub has_unknown_branch: bool,
}

/// One leaf license id inside an expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Expr {
    Leaf(String),
    With { base: String, exception: String },
    And(Vec<Expr>),
    Or(Vec<Expr>),
}

/// Tokenize and parse an SPDX expression into a tree.
///
/// Grammar handled: ids (SPDX id chars + `-`/`.`/`+`), `OR`, `AND`,
/// `WITH`, `(`, `)`, and `/` as deprecated OR. `LicenseRef-*` ids are
/// parsed as ordinary leaves and fail closed at classification because
/// no policy table can know them.
pub(crate) fn parse_expression(input: &str) -> Result<Expr, String> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    let expr = parse_or(&tokens, &mut pos)?;
    if pos != tokens.len() {
        return Err(format!(
            "trailing tokens in SPDX expression: {:?}",
            &tokens[pos..]
        ));
    }
    Ok(expr)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Id(String),
    Or,
    And,
    With,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let mut rest = input.trim();
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix('(') {
            out.push(Tok::LParen);
            rest = r.trim_start();
            continue;
        }
        if let Some(r) = rest.strip_prefix(')') {
            out.push(Tok::RParen);
            rest = r.trim_start();
            continue;
        }
        if let Some(r) = rest.strip_prefix('/') {
            // Deprecated OR separator.
            out.push(Tok::Or);
            rest = r.trim_start();
            continue;
        }
        if let Some(r) = rest.strip_prefix('"') {
            // Quoted license ref (rare); consume to closing quote.
            let end = r
                .find('"')
                .ok_or_else(|| "unterminated quoted id".to_string())?;
            let id = &r[..end];
            out.push(Tok::Id(id.to_string()));
            rest = r[end + 1..].trim_start();
            continue;
        }
        // Match keyword or id. Keywords are case-insensitive.
        let (word, r) = take_word(rest);
        let upper = word.to_ascii_uppercase();
        match upper.as_str() {
            "OR" => out.push(Tok::Or),
            "AND" => out.push(Tok::And),
            "WITH" => out.push(Tok::With),
            _ => out.push(Tok::Id(word.to_string())),
        }
        rest = r.trim_start();
    }
    Ok(out)
}

fn take_word(s: &str) -> (&str, &str) {
    let end = s
        .find(|c: char| {
            !(c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '+' || c == '_')
        })
        .unwrap_or(s.len());
    (&s[..end], &s[end..])
}

fn parse_or(tokens: &[Tok], pos: &mut usize) -> Result<Expr, String> {
    let mut branches = vec![parse_and(tokens, pos)?];
    while *pos < tokens.len() && tokens[*pos] == Tok::Or {
        *pos += 1;
        branches.push(parse_and(tokens, pos)?);
    }
    if branches.len() == 1 {
        Ok(branches.remove(0))
    } else {
        Ok(Expr::Or(branches))
    }
}

fn parse_and(tokens: &[Tok], pos: &mut usize) -> Result<Expr, String> {
    let mut branches = vec![parse_with(tokens, pos)?];
    while *pos < tokens.len() && tokens[*pos] == Tok::And {
        *pos += 1;
        branches.push(parse_with(tokens, pos)?);
    }
    if branches.len() == 1 {
        Ok(branches.remove(0))
    } else {
        Ok(Expr::And(branches))
    }
}

fn parse_with(tokens: &[Tok], pos: &mut usize) -> Result<Expr, String> {
    let base = parse_atom(tokens, pos)?;
    if *pos < tokens.len() && tokens[*pos] == Tok::With {
        *pos += 1;
        let exception = match parse_atom(tokens, pos)? {
            Expr::Leaf(e) => e,
            _ => return Err("WITH must be followed by a single exception id".to_string()),
        };
        match base {
            Expr::Leaf(b) => Ok(Expr::With { base: b, exception }),
            _ => Err("WITH base must be a single license id".to_string()),
        }
    } else {
        Ok(base)
    }
}

fn parse_atom(tokens: &[Tok], pos: &mut usize) -> Result<Expr, String> {
    if *pos >= tokens.len() {
        return Err("unexpected end of SPDX expression".to_string());
    }
    match &tokens[*pos] {
        Tok::LParen => {
            *pos += 1;
            let inner = parse_or(tokens, pos)?;
            if *pos >= tokens.len() || tokens[*pos] != Tok::RParen {
                return Err("missing closing parenthesis".to_string());
            }
            *pos += 1;
            Ok(inner)
        }
        Tok::Id(id) => {
            *pos += 1;
            Ok(Expr::Leaf(id.clone()))
        }
        t => Err(format!("unexpected token in SPDX expression: {t:?}")),
    }
}

/// Well-known SPDX exceptions that ADD permission to a base license.
/// Only these are accepted for `WITH`; anything else makes the leaf
/// UNKNOWN (fail closed).
const KNOWN_EXCEPTIONS: &[&str] = &["LLVM-EXCEPTION"];

/// Classify one leaf id through the checked-in policy tables, falling
/// back to the M1 canonical classifier.
fn classify_leaf(
    id: &str,
    files: &PolicyFiles,
    canonical: &dyn Fn(&str) -> Option<LicenseClass>,
) -> Option<LicenseClass> {
    let upper = id.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return None;
    }
    // allowlist.toml -> GREEN
    if files
        .allowlist
        .allow
        .iter()
        .any(|a| a.to_ascii_uppercase() == upper)
    {
        return Some(LicenseClass::Green);
    }
    // classes.toml -> REVIEW/SIDECAR/EXTERNAL/PROHIBITED
    for (group, class) in [
        (&files.classes.review.spdx, LicenseClass::Review),
        (&files.classes.sidecar.spdx, LicenseClass::Sidecar),
        (&files.classes.external.spdx, LicenseClass::External),
        (&files.classes.prohibited.spdx, LicenseClass::Prohibited),
    ] {
        if group.iter().any(|a| a.to_ascii_uppercase() == upper) {
            return Some(class);
        }
    }
    // M1 canonical classifier fallback.
    canonical(id)
}

/// Rank of a class for fail-closed combination (higher = more
/// restrictive / less admissible).
fn rank(class: LicenseClass) -> u8 {
    match class {
        LicenseClass::Green => 0,
        LicenseClass::Review => 1,
        LicenseClass::Sidecar => 2,
        LicenseClass::External => 3,
        LicenseClass::Prohibited => 4,
    }
}

fn most_restrictive(a: LicenseClass, b: LicenseClass) -> LicenseClass {
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

/// Classify a parsed expression.
fn classify_expr(
    expr: &Expr,
    files: &PolicyFiles,
    canonical: &dyn Fn(&str) -> Option<LicenseClass>,
    unknown_branch: &mut bool,
) -> Option<LicenseClass> {
    match expr {
        Expr::Leaf(id) => {
            let c = classify_leaf(id, files, canonical);
            if c.is_none() {
                *unknown_branch = true;
            }
            c
        }
        Expr::With { base, exception } => {
            // Exception must be known; base class governs.
            let exc = exception.trim().to_ascii_uppercase();
            if !KNOWN_EXCEPTIONS.contains(&exc.as_str()) {
                *unknown_branch = true;
                return None;
            }
            let c = classify_leaf(base, files, canonical);
            if c.is_none() {
                *unknown_branch = true;
            }
            c
        }
        Expr::And(branches) => {
            let mut acc: Option<LicenseClass> = None;
            for b in branches {
                let c = classify_expr(b, files, canonical, unknown_branch)?;
                acc = Some(match acc {
                    None => c,
                    Some(a) => most_restrictive(a, c),
                });
            }
            acc
        }
        Expr::Or(branches) => {
            let mut acc: Option<LicenseClass> = None;
            for b in branches {
                let c = classify_expr(b, files, canonical, unknown_branch)?;
                acc = Some(match acc {
                    None => c,
                    Some(a) => most_restrictive(a, c),
                });
            }
            acc
        }
    }
}

/// Canonical form of an OR expression's branches (sorted, uppercased)
/// used to normalize order-sensitive ids so the M1 classifier can match
/// e.g. `MIT OR Apache-2.0` -> `APACHE-2.0 OR MIT`.
fn canonical_or_key(branches: &[Expr]) -> Option<String> {
    let mut ids: Vec<String> = Vec::new();
    for b in branches {
        match b {
            Expr::Leaf(id) => ids.push(id.trim().to_ascii_uppercase()),
            _ => return None,
        }
    }
    ids.sort();
    Some(ids.join(" OR "))
}

/// Classify an SPDX expression string into a canonical Nexus class.
///
/// This is the M3 transport boundary: external license strings are
/// parsed here; canonical truth stays in the M1 classifier and the
/// checked-in policy files.
pub fn classify_spdx(
    input: &str,
    files: &PolicyFiles,
    canonical: &dyn Fn(&str) -> Option<LicenseClass>,
) -> SpdxClassification {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return SpdxClassification {
            class: None,
            reason: "empty license expression fails closed".to_string(),
            has_unknown_branch: true,
        };
    }
    let expr = match parse_expression(trimmed) {
        Ok(e) => e,
        Err(e) => {
            return SpdxClassification {
                class: None,
                reason: format!("malformed SPDX expression fails closed: {e}"),
                has_unknown_branch: true,
            }
        }
    };

    // Fast path: single id (the overwhelmingly common case) and exact
    // two-branch OR that M1 already knows in canonical order.
    match &expr {
        Expr::Leaf(id) => {
            let upper = id.trim().to_ascii_uppercase();
            if let Some(c) = canonical(&upper) {
                return SpdxClassification {
                    class: Some(c),
                    reason: format!("exact canonical id {upper}"),
                    has_unknown_branch: false,
                };
            }
        }
        Expr::Or(branches) if branches.len() == 2 => {
            if let Some(key) = canonical_or_key(branches) {
                if let Some(c) = canonical(&key) {
                    return SpdxClassification {
                        class: Some(c),
                        reason: format!("canonical OR expression {key}"),
                        has_unknown_branch: false,
                    };
                }
            }
        }
        _ => {}
    }

    let mut unknown_branch = false;
    let class = classify_expr(&expr, files, canonical, &mut unknown_branch);
    let reason = match class {
        Some(c) => format!("SPDX expression classified {}", c.as_str()),
        None if unknown_branch => {
            "expression contains a license id outside canonical policy - fails closed".to_string()
        }
        None => "expression cannot be classified - fails closed".to_string(),
    };
    SpdxClassification {
        class,
        reason,
        has_unknown_branch: unknown_branch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_id() {
        let e = parse_expression("MIT").unwrap();
        assert_eq!(e, Expr::Leaf("MIT".to_string()));
    }

    #[test]
    fn parses_or() {
        let e = parse_expression("MIT OR Apache-2.0").unwrap();
        assert_eq!(
            e,
            Expr::Or(vec![
                Expr::Leaf("MIT".to_string()),
                Expr::Leaf("Apache-2.0".to_string())
            ])
        );
    }

    #[test]
    fn parses_slash_as_or() {
        let e = parse_expression("MIT/Apache-2.0").unwrap();
        assert_eq!(
            e,
            Expr::Or(vec![
                Expr::Leaf("MIT".to_string()),
                Expr::Leaf("Apache-2.0".to_string())
            ])
        );
    }

    #[test]
    fn parses_and_with_parens() {
        let e = parse_expression("(MIT OR Apache-2.0) AND Unicode-3.0").unwrap();
        assert_eq!(
            e,
            Expr::And(vec![
                Expr::Or(vec![
                    Expr::Leaf("MIT".to_string()),
                    Expr::Leaf("Apache-2.0".to_string())
                ]),
                Expr::Leaf("Unicode-3.0".to_string())
            ])
        );
    }

    #[test]
    fn parses_with() {
        let e = parse_expression("Apache-2.0 WITH LLVM-exception").unwrap();
        assert_eq!(
            e,
            Expr::With {
                base: "Apache-2.0".to_string(),
                exception: "LLVM-exception".to_string()
            }
        );
    }

    #[test]
    fn malformed_fails() {
        assert!(parse_expression("MIT OR").is_err());
        assert!(parse_expression("(MIT").is_err());
    }
}
