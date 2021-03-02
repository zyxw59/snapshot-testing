use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

use difference::Changeset;
use thiserror::Error;

const UPDATE_SNAPSHOTS_VAR: &str = "UPDATE_SNAPSHOTS";

#[derive(Debug, Error)]
pub enum Error {
    #[error("Created new snapshot")]
    Created,
    #[error("Updated snapshot")]
    Updated,
    #[error("Difference between actual and expected")]
    Difference,
    #[error("Error opening file: {0}")]
    File(#[source] io::Error),
    #[error("Error reading file: {0}")]
    Read(#[source] io::Error),
    #[error("Error writing file: {0}")]
    Write(#[source] io::Error),
}

pub fn check_snapshot(actual: &str, snapshot: impl AsRef<Path>) -> Result<(), Error> {
    check_snapshot_diff_flag(actual, snapshot, true)
}

pub fn check_snapshot_no_diff(actual: &str, snapshot: impl AsRef<Path>) -> Result<(), Error> {
    check_snapshot_diff_flag(actual, snapshot, false)
}

fn check_snapshot_diff_flag(actual: &str, snapshot: impl AsRef<Path>, show_diff: bool) -> Result<(), Error> {
    if !snapshot.as_ref().exists() {
        create(actual, snapshot, show_diff)
    } else if std::env::var(UPDATE_SNAPSHOTS_VAR).is_ok() {
        check_and_update(actual, snapshot, show_diff)
    } else {
        check(actual, snapshot, show_diff)
    }
}

fn check(actual: &str, snapshot: impl AsRef<Path>, show_diff: bool) -> Result<(), Error> {
    let mut file = File::open(snapshot).map_err(Error::File)?;
    let expected = read_to_string(&mut file)?;

    compare(actual, &expected, show_diff)
}

fn create(actual: &str, snapshot: impl AsRef<Path>, show_diff: bool) -> Result<(), Error> {
    let mut file = File::create(snapshot).map_err(Error::File)?;
    file.write(actual.as_bytes()).map_err(Error::Write)?;

    let _ = compare(actual, "", show_diff);
    Err(Error::Created)
}

fn check_and_update(actual: &str, snapshot: impl AsRef<Path>, show_diff: bool) -> Result<(), Error> {
    if check(actual, &snapshot, show_diff).is_err() {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(snapshot)
            .map_err(Error::File)?;
        file.write(actual.as_bytes()).map_err(Error::Write)?;
        Err(Error::Updated)
    } else {
        Ok(())
    }
}

fn compare(actual: &str, expected: &str, show_diff: bool) -> Result<(), Error> {
    let diff = Changeset::new(expected, actual, "");
    if diff.distance == 0 {
        Ok(())
    } else {
        if show_diff {
            eprintln!("{}", diff);
        }
        Err(Error::Difference)
    }
}

fn read_to_string(file: &mut File) -> Result<String, Error> {
    let buffer_len = file.metadata().map(|m| m.len() as usize + 1).unwrap_or(0);
    let mut buffer = String::with_capacity(buffer_len);
    file.read_to_string(&mut buffer).map_err(Error::Read)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_compare() {
        super::compare("hello world", "hello, world!", false).unwrap_err();
        super::compare("hello world", "hello world", false).unwrap();
        super::compare(
            "this string\nhas multiple\nline",
            "this string\nhas multiple\nlines",
            false,
        )
        .unwrap_err();
    }

    #[test]
    fn snapshot() {
        use super::Error;
        std::env::remove_var(super::UPDATE_SNAPSHOTS_VAR);
        let create_file = std::path::Path::new("snapshots/create.snap");
        if create_file.exists() {
            std::fs::remove_file(create_file).unwrap();
        }
        match super::check_snapshot("hello world", create_file) {
            Err(Error::Created) => {}
            other => panic!("Expected `Err(Created)`, got `{:?}`", other),
        }
        super::check_snapshot("hello world", create_file).unwrap();

        match super::check_snapshot("hello world", "snapshots/difference.snap") {
            Err(Error::Difference) => {}
            other => panic!("Expected `Err(Difference)`, got `{:?}`", other),
        }

        std::env::set_var(super::UPDATE_SNAPSHOTS_VAR, "1");
        match super::check_snapshot("hello world!", create_file) {
            Err(Error::Updated) => {}
            other => panic!("Expected `Err(Updated)`, got `{:?}`", other),
        }
        super::check_snapshot("hello world!", create_file).unwrap();
        std::fs::remove_file(create_file).unwrap();
    }
}
