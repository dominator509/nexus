//! EP-025 dependency direction: the contract crate depends only on
//! nexus-domain, serde, and serde_json. Provider behavior lives in
//! adapters, never in the domain contracts.

#[test]
fn ep025_unit_dependency_direction() {
    // The manifest declares exactly: nexus-domain, serde, serde_json.
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
                // M4 directive T/24: sha256 digest for TranscriptArtifact
                // evidence (never raw transcripts). Recorded in the
                // ExecPlan Decision Log; MIT/Apache-2.0, audit-gated.
                || name.starts_with("sha2"),
            "unexpected dependency {name:?} in nexus-telephony"
        );
    }
}
