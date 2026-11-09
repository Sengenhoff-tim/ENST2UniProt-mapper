use tracing::info;

/// The pipeline area an isoform event came from. Rendered as the uppercase
/// first word of every isoform log message.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stage {
    IsoformReconstruction,
    NaturalVariantMapping,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Stage::IsoformReconstruction => "ISOFORM RECONSTRUCTION",
            Stage::NaturalVariantMapping => "NATURAL VARIANT MAPPING",
        }
    }
}

/// Emits one `info!` event for an isoform or feature of a UniProt entry.
/// `subject` names the isoform or feature the event is about.
pub(crate) fn log_entry_event(stage: Stage, accession: &str, subject: &str, outcome: &str) {
    emit(stage, accession, subject, outcome, false);
}

/// Like `log_entry_event`, for an isoform or feature that was rejected;
/// counted in the final rejection summary.
pub(crate) fn log_entry_rejection(stage: Stage, accession: &str, subject: &str, outcome: &str) {
    emit(stage, accession, subject, outcome, true);
}

fn emit(stage: Stage, accession: &str, subject: &str, outcome: &str, rejected: bool) {
    info!(
        stage = stage.label(),
        accession,
        subject,
        outcome,
        rejected,
        "{}: {}",
        stage.label(),
        outcome,
    );
}
