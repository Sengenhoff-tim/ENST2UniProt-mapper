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
mod tests {
    use std::io::{Read, Write};

    use flate2::Compression;
    use flate2::write::GzEncoder;

    use super::*;
    use crate::test_support::write_file;

    fn read_all(path: &Path) -> String {
        let mut content = String::new();
        open_maybe_gz(path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        content
    }

    #[test]
    fn reads_plain_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(&dir, "plain.txt", "hello\n");
        assert_eq!(read_all(&path), "hello\n");
    }

    #[test]
    fn decompresses_gz_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.txt.gz");
        let mut encoder = GzEncoder::new(File::create(&path).unwrap(), Compression::default());
        encoder.write_all(b"hello\n").unwrap();
        encoder.finish().unwrap();

        assert_eq!(read_all(&path), "hello\n");
    }

    #[test]
    fn missing_file_is_an_error_naming_the_path() {
        let err = open_maybe_gz(Path::new("/nonexistent/input.txt"))
            .err()
            .unwrap();
        assert!(err.to_string().contains("/nonexistent/input.txt"), "{err}");
    }
}
