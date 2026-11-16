// Warnings and skipped/malformed records are logged via `tracing`, writing
// to both stdout and a per-run log file in `--log-dir` (see
// `init_logging` below). The workflow itself is described in `pipeline`.
//
// Usage: enst2uniprot-mapper --ensure-accessions <accessions.txt>
//          --ensembl-idmapping <idmapping.dat[.gz]> --uniprot-xml <uniprot.xml[.gz]>
//          [--ensembl-fasta <pep.fa[.gz]>] [--sample-variants <variants.txt>]
//          [--output <out.txt>] [--log-dir <dir>] [--uniprot-api-fallback] [--ensembl-api-fallback]

mod cli;
//mod ebi;
mod ensembl;
mod format;
mod isoform;
mod parsing;
mod pipeline;
mod shared_types;
mod uniprot;
mod util;
mod variant;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

use cli::Cli;
use util::{RejectionCounter, RetryConfig, create_run_log};

/// Sets up `tracing` to write every event to a new log file in `log_dir`
/// (for a persistent, traceable record of this run), every
/// `warn!`/`error!` (and above) to stdout (for live progress), and to count
/// rejections in `rejections`.
///
/// Returns the log file's path and the `tracing-appender` worker guard,
/// which must be kept alive
/// for the duration of `main` — the file writer is non-blocking and
/// buffers writes on a background thread; dropping the guard early can
/// lose buffered log lines.
fn init_logging(
    log_dir: &Path,
    rejections: RejectionCounter,
) -> Result<(PathBuf, tracing_appender::non_blocking::WorkerGuard)> {
    let (log_file, log_path) = create_run_log(log_dir, SystemTime::now())?;
    let (non_blocking, guard) = tracing_appender::non_blocking(log_file);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(false)
        .with_filter(tracing_subscriber::filter::LevelFilter::WARN);

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .with(rejections)
        .try_init()
        .context("failed to initialize tracing subscriber")?;

    Ok((log_path, guard))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let retry_config = RetryConfig {
        timeout: Duration::from_secs(args.api_timeout_secs),
        max_attempts: args.api_max_attempts,
        backoff_base_ms: args.api_retry_backoff_ms,
    };

    let rejections = RejectionCounter::default();
    let (log_path, _log_guard) = init_logging(&args.log_dir, rejections.clone())
        .context("unable to set up exception logging")?;

    let result = pipeline::run(pipeline::RunOptions {
        ensure_accessions: args.ensure_accessions,
        sample_variants: args.sample_variants,
        ensembl_idmapping: args.ensembl_idmapping,
        exclude_accessions: args.exclude_accessions,
        output: args.output,
        uniprot_xml: args.uniprot_xml,
        uniprot_api_fallback: args.uniprot_api_fallback,
        ensembl_fasta: args.ensembl_fasta,
        ensembl_api_fallback: args.ensembl_api_fallback,
        //source_types: args.source_type.into_iter().filter(|s| !s.is_empty()).collect(),
        retry_config,
        //ebi_variants_enabled: args.ebi_variants,
        uniprot_variants_enabled: args.include_uniprot_variants,
    })
    .await;

    // Reported even if the run failed, to show what was dropped until then.
    if rejections.total() > 0 {
        tracing::warn!(
            "{} records rejected, see '{}' for details: {}",
            rejections.total(),
            log_path.display(),
            rejections.breakdown()
        );
    }

    result.context("run failed")
}

#[cfg(test)]
mod test_support;
