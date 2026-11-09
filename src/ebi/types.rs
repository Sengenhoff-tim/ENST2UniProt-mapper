use std::fmt;

// Only fields read elsewhere in this module are kept
#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct ProteinFeatureInfo {
    pub(crate) features: Vec<EbiFeature>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct EbiFeature {
    #[serde(rename = "ftId")]
    pub(super) ft_id: Option<String>,
    pub(super) begin: String,
    #[serde(rename = "wildType")]
    pub(super) wild_type: Option<String>,
    #[serde(rename = "mutatedType")]
    pub(super) mutated_type: Option<String>,
}

/// Compact one-line form used in logs, e.g. `rs123 (1929 G>W)`.
/// A `-` stands for a `None` field.
impl fmt::Display for EbiFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} {}>{})",
            self.ft_id.as_deref().unwrap_or("-"),
            self.begin,
            self.wild_type.as_deref().unwrap_or("-"),
            self.mutated_type.as_deref().unwrap_or("-"),
        )
    }
}
