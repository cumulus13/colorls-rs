use std::path::Path;

use crate::cli::Cli;
use crate::entry::{read_dir_entries, FileEntry};
use crate::git::GitRepoInfo;
use crate::icons::{icon_for, resolved_category};
use crate::render::RenderCtx;
use crate::sorter::sort_entries;

const BRANCH: &str = "\u{251c}\u{2500}\u{2500} "; // "├── "
const LAST_BRANCH: &str = "\u{2514}\u{2500}\u{2500} "; // "└── "
const PIPE: &str = "\u{2502}   "; // "│   "
const SPACE: &str = "    ";

pub fn render(root: &Path, cli: &Cli, ctx: &RenderCtx, git: Option<&GitRepoInfo>) {
    let max_depth = cli.tree.unwrap_or(3);
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());
    crate::oprintln!(
        "{}",
        crate::colors::paint(&name, "dir", ctx.theme, ctx.color_enabled)
    );
    walk(root, "", 1, max_depth, cli, ctx, git);
}

fn walk(
    dir: &Path,
    prefix: &str,
    depth: usize,
    max_depth: usize,
    cli: &Cli,
    ctx: &RenderCtx,
    git: Option<&GitRepoInfo>,
) {
    if depth > max_depth {
        return;
    }

    let mut entries: Vec<FileEntry> = match read_dir_entries(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("colorls: {}: {}", dir.display(), e);
            return;
        }
    };

    if !cli.show_hidden() {
        entries.retain(|e| !e.is_hidden());
    }

    sort_entries(&mut entries, cli);

    let last_idx = entries.len().saturating_sub(1);
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == last_idx;
        let connector = if is_last { LAST_BRANCH } else { BRANCH };

        let icon = if ctx.icons_enabled {
            format!("{} ", icon_for(entry, ctx.theme))
        } else {
            String::new()
        };

        let mut label = entry.name.clone();
        if entry.is_dir {
            label.push('/');
        }
        if let Some(target) = &entry.symlink_target {
            label.push_str(" -> ");
            label.push_str(&target.to_string_lossy());
        }

        let category = resolved_category(entry, ctx.theme);
        let colored_label = crate::colors::paint(&label, &category, ctx.theme, ctx.color_enabled);

        let git_glyph = if cli.git_status {
            let status = git.map(|g| g.status_for(&entry.path));
            match status {
                Some(s) => format!(
                    "{} ",
                    crate::colors::paint(s.glyph(), s.category(), ctx.theme, ctx.color_enabled)
                ),
                None => String::new(),
            }
        } else {
            String::new()
        };

        let branch = crate::colors::paint(connector, "tree_branch", ctx.theme, ctx.color_enabled);
        crate::oprintln!("{}{}{}{}{}", prefix, branch, git_glyph, icon, colored_label);

        if entry.is_dir && !entry.is_symlink {
            let next_prefix = format!("{}{}", prefix, if is_last { SPACE } else { PIPE });
            walk(
                &entry.path,
                &next_prefix,
                depth + 1,
                max_depth,
                cli,
                ctx,
                git,
            );
        }
    }
}
