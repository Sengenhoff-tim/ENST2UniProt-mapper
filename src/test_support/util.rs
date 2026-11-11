//! Small assertion and file helpers.

use std::path::PathBuf;

/// The items of `items` as string slices, for compact assertions.
pub(crate) fn strs<T: AsRef<str>>(items: &[T]) -> Vec<&str> {
    items.iter().map(AsRef::as_ref).collect()
}

/// Writes `content` to `name` inside `dir` and returns its path.
pub(crate) fn write_file(dir: &tempfile::TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("write test file");
    path
}
