//! Local crash-report directory support.
//!
//! This is the directory/listing foundation from upstream `crash/dir.zig` and
//! the list-only path of `cli/crash_report.zig`. It does not initialize Sentry
//! or capture crash envelopes.

use std::cmp::Ordering;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const ROASTTY_BUNDLE_ID: &str = "com.termsurf.roastty";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Report {
    pub name: OsString,
    pub modified: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CrashDir {
    path: PathBuf,
}

impl CrashDir {
    pub(crate) fn new(path: impl Into<PathBuf>) -> CrashDir {
        CrashDir { path: path.into() }
    }

    pub(crate) fn default() -> CrashDir {
        CrashDir::new(default_dir_path())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn reports(&self) -> std::io::Result<Vec<Report>> {
        let entries = match std::fs::read_dir(&self.path) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };

        let mut reports = Vec::new();
        for entry in entries {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_file() {
                continue;
            }

            let metadata = entry.metadata()?;
            reports.push(Report {
                name: entry.file_name(),
                modified: metadata.modified()?,
            });
        }

        reports.sort_by(|lhs, rhs| report_order(lhs, rhs));
        Ok(reports)
    }
}

pub(crate) fn default_dir_path() -> PathBuf {
    default_dir_path_from_home(std::env::var_os("HOME"))
}

fn default_dir_path_from_home(home: Option<OsString>) -> PathBuf {
    if let Some(home) = home.filter(|value| !value.is_empty()) {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(ROASTTY_BUNDLE_ID)
            .join("crash");
    }

    std::env::temp_dir().join("roastty").join("crash")
}

fn report_order(lhs: &Report, rhs: &Report) -> Ordering {
    rhs.modified
        .cmp(&lhs.modified)
        .then_with(|| lhs.name.cmp(&rhs.name))
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::os::temp_dir::TempDir;

    fn names(reports: &[Report]) -> Vec<String> {
        reports
            .iter()
            .map(|report| report.name.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn crash_default_path_uses_bundle_id_application_support_with_home() {
        let path = default_dir_path_from_home(Some(OsString::from("/Users/tester")));
        assert_eq!(
            path,
            PathBuf::from("/Users/tester")
                .join("Library")
                .join("Application Support")
                .join("com.termsurf.roastty")
                .join("crash")
        );
    }

    #[test]
    fn crash_default_path_falls_back_to_scoped_temp_subdir_without_home() {
        let path = default_dir_path_from_home(None);
        assert_eq!(path, std::env::temp_dir().join("roastty").join("crash"));
    }

    #[test]
    fn crash_reports_missing_directory_is_empty() {
        let temp = TempDir::new().unwrap();
        let dir = CrashDir::new(temp.path().join("missing"));
        assert!(dir.reports().unwrap().is_empty());
    }

    #[test]
    fn crash_reports_filter_non_files_and_return_basenames() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("crash");
        fs::create_dir(&dir_path).unwrap();
        File::create(dir_path.join("a.ghosttycrash")).unwrap();
        fs::create_dir(dir_path.join("nested")).unwrap();

        let dir = CrashDir::new(&dir_path);
        assert_eq!(dir.path(), dir_path.as_path());
        let reports = dir.reports().unwrap();
        assert_eq!(names(&reports), vec!["a.ghosttycrash"]);
    }

    #[test]
    fn crash_reports_sort_newest_first_with_name_tiebreak() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("crash");
        fs::create_dir(&dir_path).unwrap();

        File::create(dir_path.join("old.ghosttycrash")).unwrap();
        thread::sleep(Duration::from_millis(20));
        File::create(dir_path.join("new.ghosttycrash")).unwrap();

        let reports = CrashDir::new(&dir_path).reports().unwrap();
        assert_eq!(
            names(&reports),
            vec!["new.ghosttycrash", "old.ghosttycrash"]
        );

        let tied = [
            Report {
                name: "b.ghosttycrash".into(),
                modified: SystemTime::UNIX_EPOCH,
            },
            Report {
                name: "a.ghosttycrash".into(),
                modified: SystemTime::UNIX_EPOCH,
            },
        ];
        let mut tied = tied.to_vec();
        tied.sort_by(report_order);
        assert_eq!(names(&tied), vec!["a.ghosttycrash", "b.ghosttycrash"]);
    }
}
