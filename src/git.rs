use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStatus {
    Staged,
    Modified,
    Untracked,
    Unmerged,
    Ignored,
    None,
}

impl GitStatus {
    pub fn category(self) -> &'static str {
        match self {
            GitStatus::Staged => "git_status_staged",
            GitStatus::Modified => "git_status_modified",
            GitStatus::Untracked => "git_status_untracked",
            GitStatus::Unmerged => "git_status_unmerged",
            GitStatus::Ignored => "git_status_ignored",
            GitStatus::None => "git_status_none",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            GitStatus::Staged => "+",
            GitStatus::Modified => "!",
            GitStatus::Untracked => "?",
            GitStatus::Unmerged => "U",
            GitStatus::Ignored => "-",
            GitStatus::None => " ",
        }
    }
}

pub struct GitRepoInfo {
    pub branch: Option<String>,
    pub status: HashMap<PathBuf, GitStatus>,
}

/// Whether the `git` binary is on PATH at all. Checked once by the caller
/// so we only emit a single "git not found" warning per run.
pub fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Collects git status for `dir`, returning `None` if `dir` isn't inside a
/// git working tree (this is not an error condition).
pub fn status_for_dir(dir: &Path) -> Option<GitRepoInfo> {
    let inside = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !inside.status.success() {
        return None;
    }

    let branch_out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output()
        .ok();
    let branch = branch_out.and_then(|o| {
        if o.status.success() {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() || s == "HEAD" {
                None
            } else {
                Some(s)
            }
        } else {
            None
        }
    });

    let status_out = Command::new("git")
        .args(["status", "--porcelain=v1", "--ignored"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !status_out.status.success() {
        return Some(GitRepoInfo {
            branch,
            status: HashMap::new(),
        });
    }

    let text = String::from_utf8_lossy(&status_out.stdout);
    let mut status = HashMap::new();
    for line in text.lines() {
        if line.len() < 4 {
            continue;
        }
        let index_status = line.as_bytes()[0] as char;
        let worktree_status = line.as_bytes()[1] as char;
        let raw_path = &line[3..];
        // Renames come as "old -> new"; take the new path.
        let path_part = raw_path.split(" -> ").last().unwrap_or(raw_path);
        let full = dir.join(path_part);

        let parsed = classify(index_status, worktree_status);
        status.insert(full, parsed);

        // Also mark the immediate parent directories as "modified" so the
        // change is visible when listing the containing folder without
        // recursing, mirroring how upstream colorls surfaces `--gs`.
        if let Some(parent) = Path::new(path_part).parent() {
            let mut cur = PathBuf::new();
            for comp in parent.components() {
                cur.push(comp);
                let full_parent = dir.join(&cur);
                status
                    .entry(full_parent)
                    .and_modify(|e| {
                        if *e == GitStatus::None {
                            *e = parsed;
                        }
                    })
                    .or_insert(parsed);
            }
        }
    }

    Some(GitRepoInfo { branch, status })
}

fn classify(index: char, worktree: char) -> GitStatus {
    if index == '?' && worktree == '?' {
        return GitStatus::Untracked;
    }
    if index == '!' && worktree == '!' {
        return GitStatus::Ignored;
    }
    if index == 'U' || worktree == 'U' {
        return GitStatus::Unmerged;
    }
    if worktree != ' ' && worktree != '?' {
        return GitStatus::Modified;
    }
    if index != ' ' && index != '?' {
        return GitStatus::Staged;
    }
    GitStatus::None
}

impl GitRepoInfo {
    pub fn status_for(&self, path: &Path) -> GitStatus {
        self.status.get(path).copied().unwrap_or(GitStatus::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_untracked() {
        assert_eq!(classify('?', '?'), GitStatus::Untracked);
    }

    #[test]
    fn classify_ignored() {
        assert_eq!(classify('!', '!'), GitStatus::Ignored);
    }

    #[test]
    fn classify_unmerged_takes_priority() {
        assert_eq!(classify('U', ' '), GitStatus::Unmerged);
        assert_eq!(classify(' ', 'U'), GitStatus::Unmerged);
    }

    #[test]
    fn classify_worktree_modified() {
        assert_eq!(classify(' ', 'M'), GitStatus::Modified);
    }

    #[test]
    fn classify_index_staged() {
        assert_eq!(classify('M', ' '), GitStatus::Staged);
    }

    #[test]
    fn classify_clean() {
        assert_eq!(classify(' ', ' '), GitStatus::None);
    }

    #[test]
    fn status_for_dir_outside_repo_is_none() {
        // /tmp is (almost certainly) not itself a git work tree in CI or
        // locally, so this should resolve to `None` rather than error.
        let result = status_for_dir(std::path::Path::new(std::env::temp_dir().as_path()));
        assert!(result.is_none() || result.unwrap().status.is_empty());
    }
}
