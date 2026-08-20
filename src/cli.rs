// File: src\cli.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Date: 2026-08-20
// Description:
// License: MIT

use clap::{ArgAction, Parser, ValueEnum};
use clap_color_help::default_styles;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorWhen {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortBy {
    Name,
    Time,
    Size,
    Extension,
    None,
}

/// A fast, production-ready rewrite of `colorls`: a beautified `ls` with
/// nerd-font icons, colors, git status and a tree view.
///
/// Deliberately no `name`/`bin_name` override here: this binary ships as
/// both `colorls` and `lls` (see Cargo.toml), and `--help`/`--version`
/// should reflect whichever one was actually invoked. `parse_cli()` in
/// main.rs sets both dynamically from argv[0] before parsing.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None, styles=default_styles())]
pub struct Cli {
    /// Files and/or directories to list. Defaults to the current directory.
    pub paths: Vec<PathBuf>,

    /// Show hidden entries (dotfiles), including `.` and `..`.
    #[arg(short = 'a', long = "all")]
    pub all: bool,

    /// Show hidden entries, but omit `.` and `..`.
    #[arg(short = 'A', long = "almost-all")]
    pub almost_all: bool,

    /// Use the long listing format (permissions, owner, size, date).
    #[arg(short = 'l', long = "long")]
    pub long: bool,

    /// One entry per line.
    #[arg(short = '1', long = "oneline")]
    pub oneline: bool,

    /// Show a tree view. Optionally pass a max depth (default from
    /// config.yaml's `tree_depth`, or 3 if unset).
    #[arg(long = "tree", num_args = 0..=1, default_missing_value = "0")]
    pub tree: Option<usize>,

    /// Show per-entry git status (requires the entry to be inside a git
    /// repository; silently skipped otherwise).
    #[arg(long = "gs", visible_alias = "git-status")]
    pub git_status: bool,

    /// List directories before files.
    #[arg(long = "sd", visible_aliases = ["sort-dirs", "group-directories-first"])]
    pub group_directories_first: bool,

    /// List files before directories.
    #[arg(long = "sf", visible_alias = "sort-files")]
    pub sort_files_first: bool,

    /// Sort by modification time, newest first.
    #[arg(short = 't')]
    pub sort_time: bool,

    /// Sort by file size, largest first.
    #[arg(short = 'S', long = "sort-size")]
    pub sort_size: bool,

    /// Sort by file extension.
    #[arg(short = 'X', long = "sort-extension")]
    pub sort_extension: bool,

    /// Reverse whatever sort order is in effect.
    #[arg(short = 'r', long = "reverse")]
    pub reverse: bool,

    /// Use a light color scheme instead of the default dark one.
    #[arg(long = "light", conflicts_with = "dark")]
    pub light: bool,

    /// Force the dark color scheme (overrides config.yaml's `theme:`).
    #[arg(long = "dark")]
    pub dark: bool,

    /// Print a summary report (file/dir counts, total size) after listing.
    #[arg(long = "report")]
    pub report: bool,

    /// Disable nerd-font icons (plain text listing).
    #[arg(long = "no-icons")]
    pub no_icons: bool,

    /// Force-enable nerd-font icons even if config.yaml disables them.
    #[arg(long = "icons", conflicts_with = "no_icons")]
    pub icons: bool,

    /// Control when ANSI colors are used.
    #[arg(long = "color", value_enum, default_value_t = ColorWhen::Auto)]
    pub color: ColorWhen,

    /// Recurse into sub-directories (non-tree, flat long/grid listing).
    #[arg(short = 'R', long = "recursive")]
    pub recursive: bool,

    /// Path to a config directory (or a config.yaml file inside one) to use
    /// instead of the platform default.
    #[arg(long = "config", value_name = "PATH")]
    pub config_path: Option<PathBuf>,

    /// Write the default config files into the resolved config directory
    /// (without overwriting any that already exist) and exit.
    #[arg(long = "init-config")]
    pub init_config: bool,

    /// Print the resolved config directory and exit.
    #[arg(long = "print-config-dir")]
    pub print_config_dir: bool,

    /// Suppress non-fatal warnings (e.g. "git not found").
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Increase verbosity (repeatable: -v, -vv).
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    pub verbose: u8,
}

impl Cli {
    pub fn show_hidden(&self) -> bool {
        self.all || self.almost_all
    }

    /// Whether synthetic `.` and `..` entries should be added (classic
    /// `ls -a` semantics; `-A` deliberately omits them).
    pub fn include_dot_entries(&self) -> bool {
        self.all && !self.almost_all
    }
}
