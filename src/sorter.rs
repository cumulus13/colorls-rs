use std::cmp::Ordering;

use crate::cli::Cli;
use crate::entry::FileEntry;

fn name_cmp(a: &FileEntry, b: &FileEntry) -> Ordering {
    a.name.to_lowercase().cmp(&b.name.to_lowercase())
}

/// Sort `entries` in place according to the combination of flags on `cli`.
/// Order of precedence: primary key (time/size/extension/name) first, then
/// directories-first / files-first grouping is applied as a stable
/// secondary pass, then the whole thing is reversed if `-r` was given.
pub fn sort_entries(entries: &mut [FileEntry], cli: &Cli) {
    entries.sort_by(|a, b| {
        let primary = if cli.sort_time {
            b.modified.cmp(&a.modified) // newest first
        } else if cli.sort_size {
            b.size.cmp(&a.size) // largest first
        } else if cli.sort_extension {
            let ea = a.extension().unwrap_or_default();
            let eb = b.extension().unwrap_or_default();
            ea.cmp(&eb).then_with(|| name_cmp(a, b))
        } else {
            name_cmp(a, b)
        };
        primary.then_with(|| name_cmp(a, b))
    });

    if cli.group_directories_first {
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => Ordering::Equal,
        });
    } else if cli.sort_files_first {
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            _ => Ordering::Equal,
        });
    }

    if cli.reverse {
        entries.reverse();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn entry(name: &str, is_dir: bool, size: u64, age_secs: u64) -> FileEntry {
        #[cfg(unix)]
        {
            FileEntry {
                path: PathBuf::from(name),
                name: name.to_string(),
                is_dir,
                is_symlink: false,
                symlink_target: None,
                is_broken_symlink: false,
                size,
                modified: Some(SystemTime::now() - Duration::from_secs(age_secs)),
                readonly: false,
                mode: 0o644,
                uid: 0,
                gid: 0,
            }
        }
        #[cfg(not(unix))]
        {
            FileEntry {
                path: PathBuf::from(name),
                name: name.to_string(),
                is_dir,
                is_symlink: false,
                symlink_target: None,
                is_broken_symlink: false,
                size,
                modified: Some(SystemTime::now() - Duration::from_secs(age_secs)),
                readonly: false,
            }
        }
    }

    fn cli_with(args: &[&str]) -> Cli {
        let mut full = vec!["colorls"];
        full.extend_from_slice(args);
        Cli::parse_from(full)
    }

    #[test]
    fn sorts_by_name_case_insensitive_by_default() {
        let mut entries = vec![entry("banana", false, 1, 0), entry("Apple", false, 1, 0)];
        let cli = cli_with(&[]);
        sort_entries(&mut entries, &cli);
        assert_eq!(entries[0].name, "Apple");
        assert_eq!(entries[1].name, "banana");
    }

    #[test]
    fn group_directories_first() {
        let mut entries = vec![entry("a_file", false, 1, 0), entry("z_dir", true, 1, 0)];
        let cli = cli_with(&["--sd"]);
        sort_entries(&mut entries, &cli);
        assert!(entries[0].is_dir);
    }

    #[test]
    fn sort_by_size_largest_first() {
        let mut entries = vec![entry("small", false, 10, 0), entry("large", false, 1000, 0)];
        let cli = cli_with(&["-S"]);
        sort_entries(&mut entries, &cli);
        assert_eq!(entries[0].name, "large");
    }

    #[test]
    fn reverse_flips_final_order() {
        let mut entries = vec![entry("a", false, 1, 0), entry("b", false, 1, 0)];
        let cli = cli_with(&["-r"]);
        sort_entries(&mut entries, &cli);
        assert_eq!(entries[0].name, "b");
        assert_eq!(entries[1].name, "a");
    }
}
