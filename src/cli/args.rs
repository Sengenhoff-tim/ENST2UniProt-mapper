// ============================================================================
// CLI argument definition
// ============================================================================

use std::path::PathBuf;

use crate::util::{DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BACKOFF_MS, DEFAULT_TIMEOUT_SECS};

#[derive(clap::Parser)]
#[command(about = "Builds truncated UniProt flat files as input for ProtGraph")]
pub(crate) struct Cli {
    /// Accessions to ensure in the output, one per line, in addition to every
    /// entry of --uniprot-xml. Those not in the XML are fetched via
    /// --uniprot-api-fallback if enabled, and logged otherwise.
    #[arg(long, value_name = "FILE")]
    pub(crate) ensure_accessions: Option<PathBuf>,

    /// UniProt id mapping file as downloaded (e.g. `HUMAN_9606_idmapping.dat.gz`;
    /// `.gz` is decompressed transparently). Its `Ensembl_TRS` lines link ENSTs to
    /// UniProt accessions, used to extend --ensure-accessions with the accessions
    /// that the ENSTs of --sample-variants map to. If omitted, nothing is mapped.
    #[arg(long, value_name = "FILE")]
    pub(crate) ensembl_idmapping: Option<PathBuf>,

    /// UniProt canonical accessions to exclude from the output, one per line.
    /// Applies to --ensure-accessions (after the id-mapping extension) and to
    /// the entries of --uniprot-xml.
    #[arg(long, value_name = "FILE")]
    pub(crate) exclude_accessions: Option<PathBuf>,

    /// ENST sample variant file. If omitted, no ENST-derived sample variants
    /// are included (only UniProt/EBI variants, per --include-uniprot-variants /
    /// --ebi-variants).
    #[arg(long, value_name = "FILE")]
    pub(crate) sample_variants: Option<PathBuf>,

    /// File to write the synthetic flat-file output to.
    #[arg(long, value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Filter EBI variants by sourceType (comma-separated, up to 2).
    /// Allowed values: uniprot, large scale study, mixed, clinvar, nci-tcga,
    /// cosmic curated, ensembl, gnomad, topmed, exac.
    #[arg(long, value_delimiter = ',', default_value = "")]
    pub(crate) source_type: Vec<String>,

    /// Whether to fetch and include EBI-derived variants. Disabled by default;
    /// if false, EBI is never queried and only UniProt/ENST-derived variants are used.
    /// Can be used as flag (--ebi-variants) or with explicit value (--ebi-variants=true/false).
    #[arg(long, default_value_t = false, value_parser = clap::value_parser!(bool), action = clap::ArgAction::Set, num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    pub(crate) ebi_variants: bool,

    /// Whether to include UniProt-annotated variants. If false, UniProt's own
    /// variant annotations are excluded (ENST/EBI-derived variants from other
    /// sources are unaffected by this flag).
    /// Can be used as flag (--include-uniprot-variants) or with explicit value (--include-uniprot-variants=true/false).
    #[arg(long, default_value_t = true, value_parser = clap::value_parser!(bool), action = clap::ArgAction::Set, num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    pub(crate) include_uniprot_variants: bool,

    /// UniProt XML file (e.g. a UniProtKB download or `uniprot_sprot.xml.gz`;
    /// `.gz` is decompressed transparently). Every entry in it is processed
    /// (except those in --exclude-accessions). If omitted, UniProt entries come
    /// only from --uniprot-api-fallback.
    #[arg(long, value_name = "FILE")]
    pub(crate) uniprot_xml: Option<PathBuf>,

    /// Query the UniProt REST API for accessions from --ensure-accessions (or
    /// derived via --ensembl-idmapping) that are not in --uniprot-xml. Without it, such
    /// accessions are only logged.
    /// Can be used as flag (--uniprot-api-fallback) or with explicit value (--uniprot-api-fallback=true/false).
    #[arg(long, default_value_t = false, value_parser = clap::value_parser!(bool), action = clap::ArgAction::Set, num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    pub(crate) uniprot_api_fallback: bool,

    /// Ensembl protein FASTA (e.g. `Homo_sapiens.GRCh38.pep.all.fa.gz`;
    /// `.gz` is decompressed transparently). Source of sequences for ENSTs
    /// without a UniProt mapping. Records are keyed by the `transcript:`
    /// header field, or by the first header token if that is an ENST id.
    #[arg(long, value_name = "FILE")]
    pub(crate) ensembl_fasta: Option<PathBuf>,

    /// Query the Ensembl REST API for transcripts not found in --ensembl-fasta.
    /// Can be used as flag (--ensembl-api-fallback) or with explicit value (--ensembl-api-fallback=true/false).
    #[arg(long, default_value_t = false, value_parser = clap::value_parser!(bool), action = clap::ArgAction::Set, num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    pub(crate) ensembl_api_fallback: bool,

    /// File to write the run log to: every event, including skipped and
    /// malformed input records.
    #[arg(long, value_name = "FILE", default_value = "exceptions.log")]
    pub(crate) log_file: PathBuf,

    /// Per-request timeout, in seconds, for outgoing REST calls (UniProt,
    /// EBI, Ensembl). Covers connect + write + read for a single attempt;
    /// each attempt is retried independently on failure.
    #[arg(long, default_value_t = DEFAULT_TIMEOUT_SECS)]
    pub(crate) api_timeout_secs: u64,

    /// Number of attempts (including the first) before a failed fetch is
    /// treated as fatal and the program exits.
    #[arg(long, default_value_t = DEFAULT_MAX_ATTEMPTS)]
    pub(crate) api_max_attempts: u32,

    /// Base backoff, in milliseconds, between retry attempts. Doubles after
    /// each failed attempt (e.g. 500ms -> 1s -> 2s -> ...).
    #[arg(long, default_value_t = DEFAULT_RETRY_BACKOFF_MS)]
    pub(crate) api_retry_backoff_ms: u64,
}

#[cfg(test)]
#[path = "tests/test_args.rs"]
mod tests;
