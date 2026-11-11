use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};
use flate2::read::MultiGzDecoder;

/// Opens `path` for buffered reading, transparently decompressing it when
/// the name ends in `.gz` (UniProt and Ensembl both ship gzipped dumps).
pub(crate) fn open_maybe_gz(path: &Path) -> Result<Box<dyn BufRead + Send>> {
    let file = File::open(path).with_context(|| format!("failed to open '{}'", path.display()))?;
    let gz = path.extension().is_some_and(|ext| ext == "gz");
    Ok(if gz {
        Box::new(BufReader::new(MultiGzDecoder::new(file)))
    } else {
        Box::new(BufReader::new(file))
    })
}

#[cfg(test)]
#[path = "tests/test_input.rs"]
mod tests;
