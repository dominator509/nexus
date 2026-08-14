//! Provider manifests (SPEC-009; EP-013 M3).
//!
//! A manifest records the exact component identity and replacement
//! contract for a provider. Credentials are referenced by id, never
//! stored. The manifest is loadable from `config/models/providers/`.

use serde::{Deserialize, Serialize};

/// Provider kind (mirrors SPEC-009 vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManifestProviderKind {
    Bifrost,
    Deepseek,
    OpenaiCompatible,
}

impl ManifestProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bifrost => "BIFROST",
            Self::Deepseek => "DEEPSEEK",
            Self::OpenaiCompatible => "OPENAI_COMPATIBLE",
        }
    }
}

/// A provider manifest: exact identity, transport, and replacement
/// contract (EP-013 M3 requirement 6: exact component version,
/// digest, license, source, replacement contract).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderManifest {
    pub provider_id: String,
    pub kind: ManifestProviderKind,
    /// Base URL of the OpenAI-compatible chat completions surface.
    pub base_url: String,
    /// Exact provider version.
    pub version: String,
    /// Artifact digest when available (empty when the provider is a
    /// commercial API with no distributable artifact).
    pub digest: String,
    /// License or commercial terms.
    pub license: String,
    /// Source reference (registry or vendor documentation).
    pub source: String,
    /// Credential reference id; the value is resolved by the caller.
    pub credential_ref: Option<String>,
    /// Replacement contract.
    pub replacement_contract: String,
}

impl ProviderManifest {
    pub fn new(
        provider_id: impl Into<String>,
        kind: ManifestProviderKind,
        base_url: impl Into<String>,
        version: impl Into<String>,
        license: impl Into<String>,
        source: impl Into<String>,
        replacement_contract: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            kind,
            base_url: base_url.into(),
            version: version.into(),
            digest: String::new(),
            license: license.into(),
            source: source.into(),
            credential_ref: None,
            replacement_contract: replacement_contract.into(),
        }
    }

    pub fn with_digest(mut self, digest: impl Into<String>) -> Self {
        self.digest = digest.into();
        self
    }

    pub fn with_credential_ref(mut self, credential_ref: impl Into<String>) -> Self {
        self.credential_ref = Some(credential_ref.into());
        self
    }
}

/// An ordered set of provider manifests (deterministic: sorted by
/// provider id on construction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderManifestSet {
    pub providers: Vec<ProviderManifest>,
}

impl ProviderManifestSet {
    pub fn new(mut providers: Vec<ProviderManifest>) -> Self {
        providers.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
        Self { providers }
    }

    pub fn get(&self, provider_id: &str) -> Option<&ProviderManifest> {
        self.providers.iter().find(|p| p.provider_id == provider_id)
    }

    pub fn provider_ids(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|p| p.provider_id.clone())
            .collect()
    }

    /// The preferred provider (Bifrost) when present.
    pub fn preferred(&self) -> Option<&ProviderManifest> {
        self.providers
            .iter()
            .find(|p| p.kind == ManifestProviderKind::Bifrost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bifrost_manifest() -> ProviderManifest {
        ProviderManifest::new(
            "bifrost",
            ManifestProviderKind::Bifrost,
            "http://127.0.0.1:8000/v1",
            "0.1.0",
            "Apache-2.0",
            "https://github.com/example/bifrost",
            "ModelGateway contract; Bifrost preferred but replaceable",
        )
        .with_credential_ref("secret/bifrost/key")
    }

    #[test]
    fn ep013_unit_manifest_round_trip() {
        let m = bifrost_manifest();
        let v = serde_json::to_value(&m).unwrap();
        let back: ProviderManifest = serde_json::from_value(v).unwrap();
        assert_eq!(back.provider_id, "bifrost");
        assert_eq!(back.kind, ManifestProviderKind::Bifrost);
        assert_eq!(back.credential_ref.as_deref(), Some("secret/bifrost/key"));
    }

    #[test]
    fn ep013_unit_manifest_set_deterministic_order() {
        let set = ProviderManifestSet::new(vec![
            ProviderManifest::new(
                "deepseek-v4-flash",
                ManifestProviderKind::Deepseek,
                "https://api.deepseek.com/v1",
                "v4-flash",
                "commercial API terms",
                "DeepSeek API docs",
                "ReflexProvider contract; OpenAI or Gemini fast provider fallback",
            ),
            bifrost_manifest(),
        ]);
        // Sorted: bifrost before deepseek-v4-flash.
        assert_eq!(
            set.provider_ids(),
            vec!["bifrost".to_string(), "deepseek-v4-flash".to_string()]
        );
        assert_eq!(set.preferred().unwrap().provider_id, "bifrost");
        assert!(set.get("missing").is_none());
    }

    #[test]
    fn ep013_unit_manifest_kind_round_trip() {
        assert_eq!(
            ManifestProviderKind::OpenaiCompatible.as_str(),
            "OPENAI_COMPATIBLE"
        );
        assert_eq!(
            serde_json::to_value(ManifestProviderKind::Deepseek).unwrap(),
            serde_json::json!("DEEPSEEK")
        );
    }
}
