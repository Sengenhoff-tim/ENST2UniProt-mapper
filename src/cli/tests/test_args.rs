use clap::Parser;

use super::*;

fn parse(extra: &[&str]) -> Cli {
    let args = ["enst2uniprot-mapper", "--output", "out.txt"];
    Cli::try_parse_from(args.iter().chain(extra)).expect("valid arguments")
}

#[test]
fn defaults_keep_apis_and_ebi_off() {
    let cli = parse(&[]);
    assert!(!cli.uniprot_api_fallback);
    assert!(!cli.ensembl_api_fallback);
    assert!(!cli.ebi_variants);
    assert!(cli.include_uniprot_variants);
    assert_eq!(cli.log_file.to_str(), Some("exceptions.log"));
    assert_eq!(cli.api_max_attempts, DEFAULT_MAX_ATTEMPTS);
}

#[test]
fn boolean_options_work_as_bare_flags() {
    let cli = parse(&[
        "--uniprot-api-fallback",
        "--ensembl-api-fallback",
        "--ebi-variants",
        "--include-uniprot-variants",
    ]);
    assert!(
        cli.uniprot_api_fallback
            && cli.ensembl_api_fallback
            && cli.ebi_variants
            && cli.include_uniprot_variants
    );
}

#[test]
fn boolean_options_take_explicit_values() {
    let cli = parse(&[
        "--include-uniprot-variants=false",
        "--uniprot-api-fallback=true",
    ]);
    assert!(!cli.include_uniprot_variants);
    assert!(cli.uniprot_api_fallback);
}

#[test]
fn input_files_are_optional_paths() {
    let cli = parse(&["--uniprot-xml", "sprot.xml.gz", "--ensembl-fasta", "pep.fa"]);
    assert_eq!(cli.uniprot_xml.unwrap().to_str(), Some("sprot.xml.gz"));
    assert_eq!(cli.ensembl_fasta.unwrap().to_str(), Some("pep.fa"));
}

#[test]
fn only_the_output_is_required() {
    let cli = parse(&[]);
    assert_eq!(cli.output.to_str(), Some("out.txt"));
    assert!(cli.ensure_accessions.is_none() && cli.ensembl_idmapping.is_none());
    assert!(cli.uniprot_xml.is_none() && cli.sample_variants.is_none());
    assert!(Cli::try_parse_from(["enst2uniprot-mapper"]).is_err());
}
