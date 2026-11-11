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
