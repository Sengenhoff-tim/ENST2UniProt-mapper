use tracing::info;

/// The input file an event came from. Rendered as the uppercase first word
/// of every input log message.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stage {
    IdMappingParsing,
    SampleVariantParsing,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Stage::IdMappingParsing => "ID MAPPING PARSING",
            Stage::SampleVariantParsing => "SAMPLE VARIANT PARSING",
        }
    }
}

/// Emits one `info!` event about an input file. `location` is `<path>` or
/// `<path>:<line>`.
pub(crate) fn log_line_event(stage: Stage, location: &str, outcome: &str) {
    emit(stage, location, outcome, false);
}

/// Like `log_line_event`, for a line that was rejected; counted in the
/// final rejection summary.
pub(crate) fn log_line_rejection(stage: Stage, location: &str, outcome: &str) {
    emit(stage, location, outcome, true);
}

fn emit(stage: Stage, location: &str, outcome: &str, rejected: bool) {
    info!(
        stage = stage.label(),
        location,
        outcome,
        rejected,
        "{}: {}",
        stage.label(),
        outcome,
    );
}
