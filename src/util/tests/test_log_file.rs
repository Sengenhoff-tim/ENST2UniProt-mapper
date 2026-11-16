use std::time::Duration;

use super::*;

fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

#[test]
fn timestamps_are_utc_dates() {
    assert_eq!(utc_timestamp(at(0)), "1970-01-01_00-00-00");
    assert_eq!(utc_timestamp(at(1_700_000_000)), "2023-11-14_22-13-20");
    // leap day, and the last second of a year
    assert_eq!(utc_timestamp(at(951_782_400)), "2000-02-29_00-00-00");
    assert_eq!(utc_timestamp(at(1_735_689_599)), "2024-12-31_23-59-59");
}

#[test]
fn creates_the_directory_and_a_file_named_after_the_start() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("nested/logs");
    let (_, path) = create_run_log(&dir, at(1_700_000_000)).unwrap();
    assert_eq!(path, dir.join("2023-11-14_22-13-20.log"));
    assert!(path.is_file());
}

#[test]
fn runs_in_the_same_second_get_a_suffix_instead_of_overwriting() {
    let tmp = tempfile::tempdir().unwrap();
    let (_, first) = create_run_log(tmp.path(), at(0)).unwrap();
    fs::write(&first, "first run").unwrap();
    let (_, second) = create_run_log(tmp.path(), at(0)).unwrap();
    let (_, third) = create_run_log(tmp.path(), at(0)).unwrap();

    assert_eq!(second, tmp.path().join("1970-01-01_00-00-00_2.log"));
    assert_eq!(third, tmp.path().join("1970-01-01_00-00-00_3.log"));
    assert_eq!(fs::read_to_string(&first).unwrap(), "first run");
}
