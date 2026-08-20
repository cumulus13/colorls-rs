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
const DEFAULT_EXTENSION_COLORS: &str = include_str!("defaults/extension_colors.yaml");

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
    /// Direct extension -> color-name overrides, checked before falling
    /// back to the extension's category color (see `aliases`).
    pub extension_colors: RawMap,
}

fn parse_map(src: &str) -> Result<RawMap> {
    serde_yaml::from_str(src).context("failed to parse embedded YAML asset")
}

/// Merge a user-provided YAML override file into `base`.
///
/// Parses leniently as `serde_yaml::Value` rather than straight into
/// `RawMap`, specifically to handle a sharp YAML edge case that a hex-color
/// feature runs straight into: an *unquoted* `#` starts a YAML comment, so
/// `zip: #FF0000` silently parses as `zip: null` (empty value), not the
/// string `"#FF0000"`. A strict `HashMap<String, String>` deserialization
/// would hard-fail the entire file (and therefore theme loading, and
/// therefore the whole program) on that one line. Instead: skip just that
/// key, print a one-line hint explaining exactly what happened and how to
/// fix it, and keep going with everything else in the file intact.
fn merge_user_file(base: &mut RawMap, dir: &Path, filename: &str) -> Result<()> {
    let path = dir.join(filename);
    if !path.is_file() {
        return Ok(());
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(());
    }

    let value: serde_yaml::Value = serde_yaml::from_str(&content)
        .with_context(|| format!("parsing {} as YAML", path.display()))?;

    let mapping = match value {
        serde_yaml::Value::Mapping(m) => m,
        serde_yaml::Value::Null => return Ok(()), // whole file was comments/blank
        other => anyhow::bail!(
            "{} must be a YAML mapping of `key: value` pairs, found {}",
            path.display(),
            yaml_kind(&other)
        ),
    };

    for (k, v) in mapping {
        let key = match k {
            serde_yaml::Value::String(s) => s.to_lowercase(),
            other => {
                warn_user_config(
                    &path,
                    &format!("non-string key {} ignored", yaml_kind(&other)),
                );
                continue;
            }
        };

        match v {
            serde_yaml::Value::String(s) => {
                base.insert(key, s);
            }
            serde_yaml::Value::Bool(b) => {
                base.insert(key, b.to_string());
            }
            serde_yaml::Value::Number(n) => {
                base.insert(key, n.to_string());
            }
            serde_yaml::Value::Null => {
                warn_user_config(
                    &path,
                    &format!(
                        "`{key}:` has no value — if you meant a hex color, an unquoted `#` \
                         starts a YAML comment. Use `{key}: \"#RRGGBB\"` (quoted) or drop the \
                         `#` entirely: `{key}: RRGGBB`. This key was left at its default."
                    ),
                );
            }
            other => {
                warn_user_config(
                    &path,
                    &format!(
                        "`{key}:` has an unsupported value ({}), left at its default",
                        yaml_kind(&other)
                    ),
                );
            }
        }
    }

    Ok(())
}

fn yaml_kind(v: &serde_yaml::Value) -> &'static str {
    match v {
        serde_yaml::Value::Null => "null",
        serde_yaml::Value::Bool(_) => "a boolean",
        serde_yaml::Value::Number(_) => "a number",
        serde_yaml::Value::String(_) => "a string",
        serde_yaml::Value::Sequence(_) => "a list",
        serde_yaml::Value::Mapping(_) => "a nested mapping",
        serde_yaml::Value::Tagged(_) => "a tagged value",
    }
}

fn warn_user_config(path: &Path, message: &str) {
    eprintln!(
        "{}: warning: {}: {}",
        crate::util::PROG_NAME.as_str(),
        path.display(),
        message
    );
}

/// Parse a color name (as written in any theme YAML file) into a
/// `colored::Color`. Exposed crate-wide so per-extension overrides
/// (`extension_colors.yaml`) can resolve colors the same way category
/// colors do.
///
/// Accepts the built-in named colors (`bright_cyan`, `red`, ...) *and*
/// 24-bit hex colors: `#00FFFF`, `00FFFF`, or the 3-digit shorthand `#0FF`
/// (each digit doubled, same convention as CSS). An unrecognized name that
/// also isn't valid hex falls back to plain white rather than erroring, so
/// a typo in a config file degrades gracefully instead of failing to
/// start.
pub(crate) fn color_from_name(name: &str) -> Color {
    let trimmed = name.trim();

    if let Some(hex) = trimmed.strip_prefix('#').and_then(parse_hex_color) {
        return hex;
    }

    let lower = trimmed.to_lowercase();
    match lower.as_str() {
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
        other => parse_hex_color(other).unwrap_or(Color::White),
    }
}

/// Parse a bare hex color string (no leading `#`) — either 6 hex digits
/// (`00FFFF`) or the 3-digit shorthand (`0FF`, each digit doubled) — into
/// a 24-bit `Color::TrueColor`. Returns `None` for anything else, so
/// callers can safely try this after (or instead of) named-color lookup.
fn parse_hex_color(s: &str) -> Option<Color> {
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |c: char| c.to_digit(16).map(|d| d as u8);
    match s.len() {
        6 => {
            let mut chars = s.chars();
            let r = byte(chars.next()?)? * 16 + byte(chars.next()?)?;
            let g = byte(chars.next()?)? * 16 + byte(chars.next()?)?;
            let b = byte(chars.next()?)? * 16 + byte(chars.next()?)?;
            Some(Color::TrueColor { r, g, b })
        }
        3 => {
            let mut chars = s.chars();
            let r = byte(chars.next()?)?;
            let g = byte(chars.next()?)?;
            let b = byte(chars.next()?)?;
            Some(Color::TrueColor {
                r: r * 16 + r,
                g: g * 16 + g,
                b: b * 16 + b,
            })
        }
        _ => None,
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
        let mut extension_colors = parse_map(DEFAULT_EXTENSION_COLORS)?;

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
            merge_user_file(&mut extension_colors, dir, "extension_colors.yaml")?;
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
            extension_colors,
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
        ("extension_colors.yaml", DEFAULT_EXTENSION_COLORS),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_colors_still_work() {
        assert_eq!(color_from_name("bright_cyan"), Color::BrightCyan);
        assert_eq!(color_from_name("Red"), Color::Red);
        assert_eq!(color_from_name("  yellow  "), Color::Yellow);
    }

    #[test]
    fn unknown_name_falls_back_to_white() {
        assert_eq!(color_from_name("not_a_real_color"), Color::White);
    }

    #[test]
    fn hex_with_hash_prefix() {
        assert_eq!(
            color_from_name("#00FFFF"),
            Color::TrueColor {
                r: 0,
                g: 255,
                b: 255
            }
        );
    }

    #[test]
    fn hex_without_hash_prefix() {
        // Recommended form in docs, since it sidesteps the YAML
        // unquoted-`#`-starts-a-comment gotcha entirely.
        assert_eq!(
            color_from_name("FF8800"),
            Color::TrueColor {
                r: 255,
                g: 136,
                b: 0
            }
        );
    }

    #[test]
    fn hex_is_case_insensitive() {
        assert_eq!(color_from_name("#ff8800"), color_from_name("#FF8800"));
    }

    #[test]
    fn hex_three_digit_shorthand_expands_each_digit() {
        // #0FF -> #00FFFF (each digit doubled), same convention as CSS.
        assert_eq!(
            color_from_name("#0FF"),
            Color::TrueColor {
                r: 0,
                g: 255,
                b: 255
            }
        );
    }

    #[test]
    fn invalid_hex_length_falls_back_to_white() {
        assert_eq!(color_from_name("#1234"), Color::White);
        assert_eq!(color_from_name("#12"), Color::White);
    }

    #[test]
    fn invalid_hex_digits_fall_back_to_white() {
        assert_eq!(color_from_name("#GGGGGG"), Color::White);
    }

    #[test]
    fn merge_user_file_skips_unquoted_hash_gracefully_instead_of_erroring() {
        let dir = std::env::temp_dir().join(format!("colorls_theme_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // `custom: #00FFFF` (unquoted) is the classic YAML gotcha: the `#`
        // starts a comment, so this key parses as null. It must not fail
        // the whole file/theme load — `zip` on the next line should still
        // come through fine.
        std::fs::write(
            dir.join("extension_colors.yaml"),
            "custom: #00FFFF\nzip: red\n",
        )
        .unwrap();

        let mut base: RawMap = HashMap::new();
        let result = merge_user_file(&mut base, &dir, "extension_colors.yaml");

        assert!(result.is_ok());
        assert!(!base.contains_key("custom")); // left unset, not crashed
        assert_eq!(base.get("zip"), Some(&"red".to_string()));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn merge_user_file_accepts_quoted_hash_hex() {
        let dir = std::env::temp_dir().join(format!("colorls_theme_test2_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("extension_colors.yaml"), "custom: \"#00FFFF\"\n").unwrap();

        let mut base: RawMap = HashMap::new();
        merge_user_file(&mut base, &dir, "extension_colors.yaml").unwrap();

        assert_eq!(base.get("custom"), Some(&"#00FFFF".to_string()));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn merge_user_file_accepts_bare_hex_without_hash() {
        let dir = std::env::temp_dir().join(format!("colorls_theme_test3_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("extension_colors.yaml"), "custom: 00FFFF\n").unwrap();

        let mut base: RawMap = HashMap::new();
        merge_user_file(&mut base, &dir, "extension_colors.yaml").unwrap();

        assert_eq!(base.get("custom"), Some(&"00FFFF".to_string()));

        std::fs::remove_dir_all(&dir).ok();
    }
}
