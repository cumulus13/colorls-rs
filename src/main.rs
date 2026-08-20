//! File: src\main.rs
//! Author: Hadi Cahyadi <cumulus13@gmail.com>
//! Date: 2026-08-20
//! Description:
//! License: MIT

mod cli;
mod colors;
mod config;
mod entry;
mod git;
mod icons;
mod render;
mod sorter;
mod theme;
mod util;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{CommandFactory, FromArgMatches};
use clap_version_flag::colorful_version;
use cli::{Cli, ColorWhen};
use entry::{read_dir_entries, FileEntry};
use git::{git_available, status_for_dir, GitRepoInfo};
use render::RenderCtx;
use theme::{init_config_dir, Theme};
use util::PROG_NAME;

/// Parse CLI args with the `Command`'s name/bin_name set to whichever
/// executable name we were actually invoked as (`colorls` or `lls`), so
/// `--help` and `--version` reflect that instead of always saying
/// "colorls" regardless of how the user launched it.
fn parse_cli() -> Cli {
    let name = PROG_NAME.as_str();
    let cmd = Cli::command().name(name).bin_name(name);
    let matches = cmd.get_matches();
    match Cli::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && (args[1] == "-V" || args[1] == "--version") {
        let version = colorful_version!();
        version.print_and_exit();
    }

    #[cfg(windows)]
    {
        let _ = colored::control::set_virtual_terminal(true);
    }

    let cli = parse_cli();

    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{}: error: {:#}", PROG_NAME.as_str(), e);
            ExitCode::FAILURE
        }
    }
}

fn run(mut cli: Cli) -> Result<ExitCode> {
    let resolved = config::load(cli.config_path.as_deref())?;

    if cli.print_config_dir {
        match &resolved.dir {
            Some(d) => crate::oprintln!("{}", d.display()),
            None => crate::oprintln!("(no config directory could be resolved on this platform)"),
        }
        return Ok(ExitCode::SUCCESS);
    }

    if cli.init_config {
        let dir = resolved
            .dir
            .clone()
            .context("could not resolve a config directory on this platform")?;
        let written = init_config_dir(&dir)?;
        if written.is_empty() {
            crate::oprintln!(
                "{}: config already present at {} (nothing overwritten)",
                PROG_NAME.as_str(),
                dir.display()
            );
        } else {
            crate::oprintln!(
                "{}: wrote default config to {}",
                PROG_NAME.as_str(),
                dir.display()
            );
            for f in written {
                crate::oprintln!("  {}", f);
            }
        }
        return Ok(ExitCode::SUCCESS);
    }

    // Layer config.yaml settings under any flags the user didn't explicitly
    // pass, so CLI flags always win.
    let settings = resolved.settings.clone();
    if !cli.light && !cli.dark {
        if let Some(theme_name) = &settings.theme {
            if theme_name.eq_ignore_ascii_case("light") {
                cli.light = true;
            }
        }
    }
    if !cli.git_status {
        cli.git_status = settings.git_status.unwrap_or(false);
    }
    if !cli.group_directories_first {
        cli.group_directories_first = settings.group_directories_first.unwrap_or(false);
    }
    if !cli.sort_files_first {
        cli.sort_files_first = settings.sort_files_first.unwrap_or(false);
    }
    if !cli.report {
        cli.report = settings.report.unwrap_or(false);
    }
    if !cli.all && !cli.almost_all && settings.all.unwrap_or(false) {
        cli.all = true;
    }
    if !cli.long {
        cli.long = settings.long.unwrap_or(false);
    }
    if let Some(depth) = cli.tree {
        // `0` is clap's sentinel for "bare `--tree` with no explicit
        // depth" (see cli.rs) — resolve it against config.yaml's
        // `tree_depth`, falling back to 3 if that's unset too. An
        // explicit `--tree=N` from the user always wins outright.
        if depth == 0 {
            cli.tree = Some(settings.tree_depth.unwrap_or(3).max(1));
        }
    }

    let icons_enabled = if cli.no_icons {
        false
    } else if cli.icons {
        true
    } else {
        settings.icons.unwrap_or(true)
    };

    let color_enabled = match cli.color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => {
            use std::io::IsTerminal;
            std::io::stdout().is_terminal()
        }
    };

    // `colored` auto-detects TTY-ness internally and will otherwise ignore
    // our own --color=always/never decision when stdout is piped (e.g. into
    // `head` or a file); force it to always respect our resolved choice.
    colored::control::set_override(color_enabled);

    let theme = Theme::load(resolved.dir.as_deref(), cli.light)
        .context("failed to load color/icon theme")?;

    let paths: Vec<PathBuf> = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths.clone()
    };

    let mut any_error = false;
    let show_headers = paths.len() > 1;

    for (idx, path) in paths.iter().enumerate() {
        if idx > 0 {
            crate::oprintln!();
        }
        if let Err(e) = list_one(
            path,
            &cli,
            &theme,
            color_enabled,
            icons_enabled,
            show_headers,
        ) {
            eprintln!("{}: {}: {:#}", PROG_NAME.as_str(), path.display(), e);
            any_error = true;
        }
    }

    if any_error {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn list_one(
    path: &Path,
    cli: &Cli,
    theme: &Theme,
    color_enabled: bool,
    icons_enabled: bool,
    show_headers: bool,
) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("no such file or directory");
    }

    let git_available_flag = if cli.git_status {
        let avail = git_available();
        if !avail && !cli.quiet {
            eprintln!(
                "{}: warning: `git` executable not found; --gs disabled",
                PROG_NAME.as_str()
            );
        }
        avail
    } else {
        false
    };

    let git_info: Option<GitRepoInfo> = if git_available_flag {
        let target_dir = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or_else(|| Path::new("."))
        };
        status_for_dir(target_dir)
    } else {
        None
    };

    let ctx = RenderCtx {
        cli,
        theme,
        color_enabled,
        icons_enabled,
        git: git_info.as_ref(),
    };

    if let Some(info) = &git_info {
        if let Some(branch) = &info.branch {
            let text = if icons_enabled {
                format!("\u{e0a0} {}", branch)
            } else {
                branch.clone()
            };
            let label = crate::colors::paint(&text, "git_branch", theme, color_enabled);
            crate::oprintln!("{}", label);
        }
    }

    if path.is_file() {
        if show_headers {
            crate::oprintln!("{}:", path.display());
        }
        let entry = FileEntry::from_path(path).with_context(|| "reading metadata")?;
        if cli.long {
            render::long::render(std::slice::from_ref(&entry), &ctx);
        } else {
            render::grid::render(std::slice::from_ref(&entry), &ctx);
        }
        return Ok(());
    }

    if show_headers {
        crate::oprintln!("{}:", path.display());
    }

    if cli.tree.is_some() {
        render::tree::render(path, cli, &ctx, git_info.as_ref());
        return Ok(());
    }

    if cli.recursive {
        list_dir_recursive(path, cli, &ctx, theme, color_enabled, true)?;
        return Ok(());
    }

    let entries = list_dir_flat(path, cli)?;

    if cli.long {
        render::long::render(&entries, &ctx);
    } else {
        render::grid::render(&entries, &ctx);
    }

    if cli.report {
        print_report(&entries, theme, color_enabled);
    }

    Ok(())
}

/// Read, filter, sort, and (for `-a`) prepend `.`/`..` for a single
/// directory. Shared by the flat and recursive (`-R`) listing paths.
fn list_dir_flat(path: &Path, cli: &Cli) -> Result<Vec<FileEntry>> {
    let mut entries =
        read_dir_entries(path).with_context(|| format!("reading directory {}", path.display()))?;

    if !cli.show_hidden() {
        entries.retain(|e| !e.is_hidden());
    }

    sorter::sort_entries(&mut entries, cli);

    if cli.include_dot_entries() {
        if let Ok(dots) = entry::dot_entries(path) {
            let mut out = dots.to_vec();
            out.extend(entries);
            entries = out;
        }
    }

    Ok(entries)
}

/// `ls -R`-style recursive flat listing: print each directory's contents,
/// then recurse into every real (non `.`/`..`, non-symlink) subdirectory
/// with a `path:` header, depth-first.
fn list_dir_recursive(
    path: &Path,
    cli: &Cli,
    ctx: &RenderCtx,
    theme: &Theme,
    color_enabled: bool,
    is_first: bool,
) -> Result<()> {
    if !is_first {
        crate::oprintln!();
        crate::oprintln!("{}:", path.display());
    }

    let entries = list_dir_flat(path, cli)?;

    if cli.long {
        render::long::render(&entries, ctx);
    } else {
        render::grid::render(&entries, ctx);
    }

    if cli.report {
        print_report(&entries, theme, color_enabled);
    }

    let mut subdirs: Vec<PathBuf> = entries
        .iter()
        .filter(|e| e.is_dir && !e.is_symlink && e.name != "." && e.name != "..")
        .map(|e| e.path.clone())
        .collect();
    subdirs.sort();

    for sub in subdirs {
        list_dir_recursive(&sub, cli, ctx, theme, color_enabled, false)?;
    }

    Ok(())
}

fn print_report(entries: &[FileEntry], theme: &Theme, color_enabled: bool) {
    let dirs = entries.iter().filter(|e| e.is_dir).count();
    let files = entries.len() - dirs;
    let total_size: u64 = entries.iter().filter(|e| !e.is_dir).map(|e| e.size).sum();

    let line = format!(
        "\n{} directories, {} files, {} total",
        dirs,
        files,
        util::human_size(total_size)
    );
    crate::oprintln!(
        "{}",
        crate::colors::paint(&line, "report", theme, color_enabled)
    );
}
