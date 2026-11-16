use tracing::info;

use super::Variant;

/// The pipeline area a variant event came from. Rendered as the uppercase
/// first word of every variant log message.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stage {
    DuplicateReconciliation,
    VariantValidation,
    StopCodonValidation,
    MissingSequenceMapping,
    StopCodonResolution,
    VariantResolution,
    ValidationSafetyNet,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Stage::DuplicateReconciliation => "DUPLICATE RECONCILIATION",
            Stage::VariantValidation => "VARIANT VALIDATION",
            Stage::StopCodonValidation => "STOP CODON VALIDATION",
            Stage::MissingSequenceMapping => "MISSING SEQUENCE MAPPING",
            Stage::StopCodonResolution => "STOP CODON RESOLUTION",
            Stage::VariantResolution => "VARIANT RESOLUTION",
            Stage::ValidationSafetyNet => "VALIDATION SAFETY NET",
        }
    }
}

/// Emits one `info!` event for a variant.
pub(crate) fn log_variant_event(stage: Stage, variant: &Variant, outcome: &str) {
    emit(stage, variant, outcome, false);
}

/// Like `log_variant_event`, for a variant that was rejected; counted in the
/// final rejection summary.
pub(crate) fn log_variant_rejection(stage: Stage, variant: &Variant, outcome: &str) {
    emit(stage, variant, outcome, true);
}

fn emit(stage: Stage, variant: &Variant, outcome: &str, rejected: bool) {
    info!(
        stage = stage.label(),
        variant = %variant,
        outcome,
        rejected,
        "{}: {}",
        stage.label(),
        outcome,
    );
}
