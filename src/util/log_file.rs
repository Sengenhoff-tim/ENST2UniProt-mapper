//! One log file per run, named after the run's start time (UTC).

use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

/// Creates `dir` (with parents) and a new, empty log file in it named after
/// `started` in UTC, e.g. `2026-11-16_14-03-12.log`. If that name is taken
/// (two runs in the same second), a suffix is added: `..._14-03-12_2.log`.
/// Existing files are never overwritten.
pub(crate) fn create_run_log(dir: &Path, started: SystemTime) -> Result<(File, PathBuf)> {
    fs::create_dir_all(dir)
        .with_context(|| format!("failed to create log directory '{}'", dir.display()))?;
    let stem = utc_timestamp(started);
    for n in 1u32.. {
        let name = if n == 1 {
            format!("{stem}.log")
        } else {
            format!("{stem}_{n}.log")
        };
        let path = dir.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((file, path)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(e)
                    .with_context(|| format!("failed to create log file '{}'", path.display()));
            }
        }
    }
    unreachable!("u32 suffixes exhausted")
}

/// `time` in UTC as `YYYY-MM-DD_HH-MM-SS`. Times before 1970 are clamped to
/// the epoch.
fn utc_timestamp(time: SystemTime) -> String {
    let secs = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}_{:02}-{:02}-{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Converts days since 1970-01-01 to a (year, month, day) date in the
/// proleptic Gregorian calendar (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    // Shift the epoch to 0000-03-01, so leap days fall at the end of a year.
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153; // month, counted from March
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
#[path = "tests/test_log_file.rs"]
mod tests;
