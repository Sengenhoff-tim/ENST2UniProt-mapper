use tracing::{info, warn};

use super::types::EbiFeature;

/// The pipeline area an EBI feature event came from. Rendered as the
/// uppercase first word of every EBI feature log message.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stage {
    EbiConversion,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Stage::EbiConversion => "EBI CONVERSION",
        }
    }
}

/// Emits one `info!` event for an EBI feature.
pub(crate) fn log_feature_event(stage: Stage, accession: &str, feature: &EbiFeature, outcome: &str) {
    info!(
        stage = stage.label(),
        accession,
        feature = %feature,
        outcome,
        "{}: {}",
        stage.label(),
        outcome,
    );
}

/// Emits one `warn!` event for an EBI feature.
pub(crate) fn warn_feature_event(stage: Stage, accession: &str, feature: &EbiFeature, outcome: &str) {
    warn!(
        stage = stage.label(),
        accession,
        feature = %feature,
        outcome,
        "{}: {}",
        stage.label(),
        outcome,
    );
}
