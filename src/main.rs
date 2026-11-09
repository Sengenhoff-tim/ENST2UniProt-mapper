// Warnings and skipped/malformed records are logged via `tracing`, writing
// to both stderr and the exceptions file given via `--exceptions` (see
// `init_logging` below). The workflow itself is described in `pipeline`.
//
// Usage: sp_embl_builder --accessions <accessions.txt> --idmapping <idmapping.tsv>
//          --uniprot-xml <uniprot.xml[.gz]> [--ensembl-fasta <pep.fa[.gz]>]
//          [--variants <variants.txt>] [--output <out.txt>]
//          [--uniprot-api-fallback] [--ensembl-api-fallback]

mod cli;
//mod ebi;
mod ensembl;
mod format;
mod isoform;
mod parsing;
mod pipeline;
mod types;
mod uniprot;
mod util;
mod variant;

use std::fs::File;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

use cli::Cli;
use util::{RejectionCounter, RetryConfig};

/// Sets up `tracing` to write every event to `exceptions_path` (for a
/// persistent, traceable record), every `warn!`/`error!` (and above)
/// to stderr (for live progress), and to count rejections in `rejections`.
///
/// Returns the `tracing-appender` worker guard, which must be kept alive
/// for the duration of `main` — the file writer is non-blocking and
/// buffers writes on a background thread; dropping the guard early can
/// lose buffered log lines.
fn init_logging(
    exceptions_path: &str,
    rejections: RejectionCounter,
) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let log_file = File::create(exceptions_path)
        .with_context(|| format!("failed to create exceptions file '{}'", exceptions_path))?;
    let (non_blocking, guard) = tracing_appender::non_blocking(log_file);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false);

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(false)
        .with_filter(tracing_subscriber::filter::LevelFilter::WARN);

    tracing_subscriber::registry()
        .with(file_layer)
        .with(stderr_layer)
        .with(rejections)
        .try_init()
        .context("failed to initialize tracing subscriber")?;

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    if args.uniprot_xml.is_none() && !args.uniprot_api_fallback {
        anyhow::bail!("no UniProt source: pass --uniprot-xml and/or --uniprot-api-fallback");
    }
    let retry_config = RetryConfig {
        timeout: Duration::from_secs(args.timeout_secs),
        max_attempts: args.max_attempts,
        backoff_base_ms: args.retry_backoff_ms,
    };

    let rejections = RejectionCounter::default();
    let _log_guard = init_logging(&args.exceptions, rejections.clone())
        .context("unable to set up exception logging")?;

    let result = pipeline::run(pipeline::RunOptions {
        accessions_path: args.accessions,
        variants_path: args.variants,
        idmapping_path: args.idmapping,
        omit_accessions: args.omit_accessions,
        output_path: args.output,
        uniprot_xml: args.uniprot_xml,
        uniprot_api_fallback: args.uniprot_api_fallback,
        ensembl_fasta: args.ensembl_fasta,
        ensembl_api_fallback: args.ensembl_api_fallback,
        //source_types: args.source_type.into_iter().filter(|s| !s.is_empty()).collect(),
        retry_config,
        //ebi_variants_enabled: args.ebi_variants,
        uniprot_variants_enabled: args.uniprot_variants,
    })
    .await;

    // Reported even if the run failed, to show what was dropped until then.
    if rejections.total() > 0 {
        tracing::warn!(
            "{} records rejected, see '{}' for details: {}",
            rejections.total(),
            args.exceptions,
            rejections.breakdown()
        );
    }

    result.context("run failed")
}

#[cfg(test)]
mod test_support;
