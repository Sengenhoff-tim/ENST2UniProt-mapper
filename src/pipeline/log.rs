use tracing::{info, warn};

/// The pipeline area an event came from. Rendered as the uppercase first
/// word of every pipeline log message.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stage {
    VariantResolution,
    VariantValidation,
    EntryProcessing,
    TranscriptMapping,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Stage::VariantResolution => "VARIANT RESOLUTION",
            Stage::VariantValidation => "VARIANT VALIDATION",
            Stage::EntryProcessing => "ENTRY PROCESSING",
            Stage::TranscriptMapping => "TRANSCRIPT MAPPING",
        }
    }
}

/// Emits one `info!` event. `subject` is the entry, transcript or variant
/// id the event is about.
pub(crate) fn log_pipeline_event(stage: Stage, subject: &str, outcome: &str) {
    info!(stage = stage.label(), subject, outcome, "{}: {}", stage.label(), outcome);
}

/// Like `log_pipeline_event`, for a record that was rejected; counted in the
/// final rejection summary.
pub(crate) fn log_pipeline_rejection(stage: Stage, subject: &str, outcome: &str) {
    info!(stage = stage.label(), subject, outcome, rejected = true, "{}: {}", stage.label(), outcome);
}

/// Like `log_pipeline_rejection`, but also shown on stderr (`warn!`).
pub(crate) fn warn_pipeline_rejection(stage: Stage, subject: &str, outcome: &str) {
    warn!(stage = stage.label(), subject, outcome, rejected = true, "{}: {}", stage.label(), outcome);
}
