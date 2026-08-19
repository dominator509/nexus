//! EP-027 dependency direction: the contract crate depends only on
//! nexus-domain, serde, serde_json, and sha2 (digest evidence).
//! Provider behavior lives in adapters (connectors/*), never in the
//! domain contracts.

#[test]
fn ep027_unit_dependency_direction() {
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("Cargo.toml readable");
    let section = manifest
        .split("[dependencies]")
        .nth(1)
        .expect("dependencies section");
    let dep_section = section.split("\n\n").next().unwrap_or(section);
    for line in dep_section.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let name = line.split(' ').next().unwrap_or("").trim();
        assert!(
            name.starts_with("nexus-domain")
                || name.starts_with("serde")
                || name.starts_with("serde_json")
                // sha256 digest for FaxDocument artifact evidence
                // (SPEC-014 behavior 6; nexus-email precedent).
                // MIT/Apache-2.0, audit-gated.
                || name.starts_with("sha2"),
            "unexpected dependency {name:?} in nexus-fax"
        );
    }
}
