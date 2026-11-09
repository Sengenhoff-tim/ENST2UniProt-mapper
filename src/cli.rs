// ============================================================================
// CLI argument definition
// ============================================================================

use crate::util::{DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BACKOFF_MS, DEFAULT_TIMEOUT_SECS};

#[derive(clap::Parser)]
#[command(about = "Map variants from UniProt/EBI onto isoform sequences")]
pub(crate) struct Cli {
    /// Path to accession list file: accessions that must be in the output in
    /// addition to every entry of --uniprot-xml. Those not in the XML are
    /// fetched via --uniprot-api-fallback if enabled, and logged otherwise.
    #[arg(long)]
    pub(crate) accessions: String,

    /// Path to UniProt/Ensembl id mapping file, used to extend the
    /// accession list with accessions derived from ENST variant ids.
    #[arg(long)]
    pub(crate) idmapping: String,

    /// Path to a file containing UniProt canonical accessions to exclude from
    /// the final accession list, one per line (applied as the last step, after
    /// id-mapping extension).
    #[arg(long, value_name = "FILE")]
    pub(crate) omit_accessions: Option<std::path::PathBuf>,

    /// Path to ENST variant file. If omitted, no ENST-derived sample variants
    /// are included (only UniProt/EBI variants, per --uniprot-variants /
    /// --ebi-variants).
    #[arg(long)]
    pub(crate) variants: Option<String>,

    /// Path to write the synthetic flat-file output to. Defaults to stdout.
    #[arg(long)]
    pub(crate) output: Option<String>,

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
    /// Can be used as flag (--uniprot-variants) or with explicit value (--uniprot-variants true/false).
    #[arg(long, default_value_t = true, value_parser = clap::value_parser!(bool), action = clap::ArgAction::Set, num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    pub(crate) uniprot_variants: bool,

    /// UniProt XML file (e.g. a UniProtKB download or `uniprot_sprot.xml.gz`;
    /// `.gz` is decompressed transparently). Every entry in it is processed
    /// (except those in --omit-accessions). Required unless
    /// --uniprot-api-fallback is set.
    #[arg(long, value_name = "FILE")]
    pub(crate) uniprot_xml: Option<std::path::PathBuf>,

    /// Query the UniProt REST API for accessions from --accessions (or derived
    /// via --idmapping) that are not in --uniprot-xml. Without it, such
    /// accessions are only logged.
    /// Can be used as flag (--uniprot-api-fallback) or with explicit value (--uniprot-api-fallback=true/false).
    #[arg(long, default_value_t = false, value_parser = clap::value_parser!(bool), action = clap::ArgAction::Set, num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    pub(crate) uniprot_api_fallback: bool,

    /// Ensembl protein FASTA (e.g. `Homo_sapiens.GRCh38.pep.all.fa.gz`;
    /// `.gz` is decompressed transparently). Source of sequences for ENSTs
    /// without a UniProt mapping. Records are keyed by the `transcript:`
    /// header field, or by the first header token if that is an ENST id.
    #[arg(long, value_name = "FILE")]
    pub(crate) ensembl_fasta: Option<std::path::PathBuf>,

    /// Query the Ensembl REST API for transcripts not found in --ensembl-fasta.
    /// Can be used as flag (--ensembl-api-fallback) or with explicit value (--ensembl-api-fallback=true/false).
    #[arg(long, default_value_t = false, value_parser = clap::value_parser!(bool), action = clap::ArgAction::Set, num_args = 0..=1, require_equals = true, default_missing_value = "true")]
    pub(crate) ensembl_api_fallback: bool,

    /// Path to write skipped/malformed-input exceptions to, as tab-separated
    /// `<identifier>\t<message>` lines.
    #[arg(long, default_value = "exceptions.log")]
    pub(crate) exceptions: String,

    /// Per-request timeout, in seconds, for outgoing REST calls (UniProt,
    /// EBI, Ensembl). Covers connect + write + read for a single attempt;
    /// each attempt is retried independently on failure.
    #[arg(long, default_value_t = DEFAULT_TIMEOUT_SECS)]
    pub(crate) timeout_secs: u64,

    /// Number of attempts (including the first) before a failed fetch is
    /// treated as fatal and the program exits.
    #[arg(long, default_value_t = DEFAULT_MAX_ATTEMPTS)]
    pub(crate) max_attempts: u32,

    /// Base backoff, in milliseconds, between retry attempts. Doubles after
    /// each failed attempt (e.g. 500ms -> 1s -> 2s -> ...).
    #[arg(long, default_value_t = DEFAULT_RETRY_BACKOFF_MS)]
    pub(crate) retry_backoff_ms: u64,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn parse(extra: &[&str]) -> Cli {
        let args = [
            "sp_embl_builder",
            "--accessions",
            "acc.txt",
            "--idmapping",
            "map.tsv",
        ];
        Cli::try_parse_from(args.iter().chain(extra)).expect("valid arguments")
    }

    #[test]
    fn defaults_keep_apis_and_ebi_off() {
        let cli = parse(&[]);
        assert!(!cli.uniprot_api_fallback);
        assert!(!cli.ensembl_api_fallback);
        assert!(!cli.ebi_variants);
        assert!(cli.uniprot_variants);
        assert_eq!(cli.exceptions, "exceptions.log");
        assert_eq!(cli.max_attempts, DEFAULT_MAX_ATTEMPTS);
    }

    #[test]
    fn boolean_options_work_as_bare_flags() {
        let cli = parse(&[
            "--uniprot-api-fallback",
            "--ensembl-api-fallback",
            "--ebi-variants",
            "--uniprot-variants",
        ]);
        assert!(
            cli.uniprot_api_fallback
                && cli.ensembl_api_fallback
                && cli.ebi_variants
                && cli.uniprot_variants
        );
    }

    #[test]
    fn boolean_options_take_explicit_values() {
        let cli = parse(&["--uniprot-variants=false", "--uniprot-api-fallback=true"]);
        assert!(!cli.uniprot_variants);
        assert!(cli.uniprot_api_fallback);
    }

    #[test]
    fn input_files_are_optional_paths() {
        let cli = parse(&["--uniprot-xml", "sprot.xml.gz", "--ensembl-fasta", "pep.fa"]);
        assert_eq!(cli.uniprot_xml.unwrap().to_str(), Some("sprot.xml.gz"));
        assert_eq!(cli.ensembl_fasta.unwrap().to_str(), Some("pep.fa"));
    }

    #[test]
    fn accessions_and_idmapping_are_required() {
        assert!(Cli::try_parse_from(["sp_embl_builder", "--accessions", "a"]).is_err());
    }
}
