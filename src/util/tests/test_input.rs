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
