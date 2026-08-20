//! Embedded default theme (colors + icons + extension aliases) and the
//! merge logic that layers user overrides from the config directory on top.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use colored::Color;
use serde::Deserialize;

const DEFAULT_DARK_COLORS: &str = include_str!("defaults/dark_colors.yaml");
const DEFAULT_LIGHT_COLORS: &str = include_str!("defaults/light_colors.yaml");
const DEFAULT_ICONS: &str = include_str!("defaults/icons.yaml");
const DEFAULT_FILENAMES: &str = include_str!("defaults/filenames.yaml");
const DEFAULT_FOLDERS: &str = include_str!("defaults/folders.yaml");
const DEFAULT_ALIASES: &str = include_str!("defaults/aliases.yaml");

pub const DEFAULT_FILE_ICON: &str = "\u{f15b}";
pub const DEFAULT_FOLDER_ICON: &str = "\u{f07b}";
pub const SYMLINK_ICON: &str = "\u{f481}";

/// Raw string->string map as stored in every YAML asset file.
type RawMap = HashMap<String, String>;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UserConfig {
    /// General behaviour defaults, all optional and overridable by CLI flags.
    #[serde(default)]
    pub theme: Option<String>, // "dark" | "light"
    #[serde(default)]
    pub icons: Option<bool>,
    #[serde(default)]
    pub git_status: Option<bool>,
    #[serde(default)]
    pub group_directories_first: Option<bool>,
    #[serde(default)]
    pub sort_files_first: Option<bool>,
    #[serde(default)]
    pub report: Option<bool>,
    #[serde(default)]
    pub tree_depth: Option<usize>,
    #[serde(default)]
    pub long: Option<bool>,
    #[serde(default)]
    pub all: Option<bool>,
}

/// Fully resolved theme: colors, icons and category aliases, after merging
/// built-in defaults with anything the user placed in the config directory.
pub struct Theme {
    pub colors: HashMap<String, Color>,
    pub icons_by_ext: RawMap,
    pub icons_by_filename: RawMap,
    pub icons_by_folder: RawMap,
    pub aliases: RawMap,
}

fn parse_map(src: &str) -> Result<RawMap> {
    serde_yaml::from_str(src).context("failed to parse embedded YAML asset")
}

fn merge_user_file(base: &mut RawMap, dir: &Path, filename: &str) -> Result<()> {
    let path = dir.join(filename);
    if path.is_file() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        if content.trim().is_empty() {
            return Ok(());
        }
        let user: RawMap = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing {} as YAML map", path.display()))?;
        for (k, v) in user {
            base.insert(k.to_lowercase(), v);
        }
    }
    Ok(())
}

fn color_from_name(name: &str) -> Color {
    match name.trim().to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "bright_black" | "brightblack" | "gray" | "grey" => Color::BrightBlack,
        "bright_red" | "brightred" => Color::BrightRed,
        "bright_green" | "brightgreen" => Color::BrightGreen,
        "bright_yellow" | "brightyellow" => Color::BrightYellow,
        "bright_blue" | "brightblue" => Color::BrightBlue,
        "bright_magenta" | "brightmagenta" => Color::BrightMagenta,
        "bright_cyan" | "brightcyan" => Color::BrightCyan,
        "bright_white" | "brightwhite" => Color::BrightWhite,
        _ => Color::White,
    }
}

impl Theme {
    /// Build the theme by loading embedded defaults, then overlaying any
    /// matching files found in `config_dir` (if it exists).
    pub fn load(config_dir: Option<&Path>, light: bool) -> Result<Theme> {
        let base_colors_src = if light {
            DEFAULT_LIGHT_COLORS
        } else {
            DEFAULT_DARK_COLORS
        };
        let mut colors_raw = parse_map(base_colors_src)?;
        let mut icons_by_ext = parse_map(DEFAULT_ICONS)?;
        let mut icons_by_filename = parse_map(DEFAULT_FILENAMES)?;
        let mut icons_by_folder = parse_map(DEFAULT_FOLDERS)?;
        let mut aliases = parse_map(DEFAULT_ALIASES)?;

        if let Some(dir) = config_dir {
            let color_file = if light {
                "light_colors.yaml"
            } else {
                "dark_colors.yaml"
            };
            merge_user_file(&mut colors_raw, dir, color_file)?;
            merge_user_file(&mut icons_by_ext, dir, "icons.yaml")?;
            merge_user_file(&mut icons_by_filename, dir, "filenames.yaml")?;
            merge_user_file(&mut icons_by_folder, dir, "folders.yaml")?;
            merge_user_file(&mut aliases, dir, "aliases.yaml")?;
        }

        let colors = colors_raw
            .into_iter()
            .map(|(k, v)| (k, color_from_name(&v)))
            .collect();

        Ok(Theme {
            colors,
            icons_by_ext,
            icons_by_filename,
            icons_by_folder,
            aliases,
        })
    }

    pub fn color_for(&self, category: &str) -> Color {
        self.colors.get(category).copied().unwrap_or(Color::White)
    }
}

/// Writes every default asset (including a top-level config.yaml) into
/// `dir`, without overwriting files that already exist. Returns the list of
/// files actually written.
pub fn init_config_dir(dir: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let assets: &[(&str, &str)] = &[
        ("dark_colors.yaml", DEFAULT_DARK_COLORS),
        ("light_colors.yaml", DEFAULT_LIGHT_COLORS),
        ("icons.yaml", DEFAULT_ICONS),
        ("filenames.yaml", DEFAULT_FILENAMES),
        ("folders.yaml", DEFAULT_FOLDERS),
        ("aliases.yaml", DEFAULT_ALIASES),
        ("config.yaml", DEFAULT_CONFIG_YAML),
    ];
    let mut written = Vec::new();
    for (name, content) in assets {
        let path = dir.join(name);
        if !path.exists() {
            std::fs::write(&path, content)
                .with_context(|| format!("writing {}", path.display()))?;
            written.push(name.to_string());
        }
    }
    Ok(written)
}

const DEFAULT_CONFIG_YAML: &str = r#"# colorls configuration
# Every key is optional; CLI flags always take precedence over this file.
theme: dark              # "dark" or "light"
icons: true              # show nerd-font icons
git_status: false        # show per-entry git status by default
group_directories_first: false
sort_files_first: false
report: false
tree_depth: 3
long: false
all: false
"#;
