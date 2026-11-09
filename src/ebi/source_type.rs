use anyhow::{anyhow, Result};

const ALLOWED_SOURCE_TYPES: [&str; 10] = [
    "uniprot",
    "large scale study",
    "mixed",
    "clinvar",
    "nci-tcga",
    "cosmic curated",
    "ensembl",
    "gnomad",
    "topmed",
    "exac",
];

pub(super) fn validate_source_types(source_types: &[&str]) -> Result<()> {
    if source_types.len() > 2 {
        return Err(anyhow!(
            "at most 2 sourcetype values accepted, got {}",
            source_types.len()
        ));
    }
    for st in source_types {
        if !ALLOWED_SOURCE_TYPES
            .iter()
            .any(|a| a.eq_ignore_ascii_case(st))
        {
            return Err(anyhow!(
                "invalid sourcetype '{}', must be one of {:?}",
                st,
                ALLOWED_SOURCE_TYPES
            ));
        }
    }
    Ok(())
}
