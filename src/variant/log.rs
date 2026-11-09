use tracing::info;

use super::Variant;

/// The pipeline area a variant event came from. Rendered as the uppercase
/// first word of every variant log message.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stage {
    DuplicateReconciliation,
    VariantValidation,
    StopCodonValidation,
    EbiNormalization,
    StopCodonResolution,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Stage::DuplicateReconciliation => "DUPLICATE RECONCILIATION",
            Stage::VariantValidation => "VARIANT VALIDATION",
            Stage::StopCodonValidation => "STOP CODON VALIDATION",
            Stage::EbiNormalization => "EBI NORMALIZATION",
            Stage::StopCodonResolution => "STOP CODON RESOLUTION",
        }
    }
}

/// Emits one `info!` event for a variant.
pub(crate) fn log_variant_event(stage: Stage, variant: &Variant, outcome: &str) {
    info!(
        stage = stage.label(),
        variant = %variant,
        outcome,
        "{}: {}",
        stage.label(),
        outcome,
    );
}