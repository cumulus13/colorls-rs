//! Cross-platform config directory resolution and loading of `config.yaml`.
//!
//! Resolution order (first match wins):
//!   1. `--config <path>` CLI flag (may point at a directory or a single
//!      `config.yaml` file)
//!   2. `$COLORLS_CONFIG` environment variable (same rules as above)
//!   3. `<platform config dir>/colorls`
//!      - Linux:   `~/.config/colorls`
//!      - macOS:   `~/Library/Application Support/colorls`
//!      - Windows: `%APPDATA%\colorls`
//!
//! This mirrors the *purpose* of upstream colorls' `~/.config/colorls`
//! override directory (drop YAML files in, they get merged over the
//! built-in defaults) without requiring the same file layout.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::theme::UserConfig;

pub struct ResolvedConfig {
    pub dir: Option<PathBuf>,
    pub settings: UserConfig,
}

pub fn resolve_config_dir(cli_override: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = cli_override {
        return Some(normalize_dir(p));
    }
    if let Ok(env_path) = std::env::var("COLORLS_CONFIG") {
        if !env_path.trim().is_empty() {
            return Some(normalize_dir(Path::new(&env_path)));
        }
    }
    if let Some(d) = dirs::config_dir() {
        return Some(d.join("colorls"));
    }
    // `dirs::config_dir()` deliberately returns `None` on Android, since
    // the crate can't assume a conventional filesystem layout there
    // without JNI calls into the app sandbox. Termux (which builds/runs
    // `android`-target binaries too) *does* have a normal, writable
    // `$HOME` though, so fall back to `$HOME/.config/colorls` directly.
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return Some(PathBuf::from(home).join(".config").join("colorls"));
        }
    }
    None
}

fn normalize_dir(p: &Path) -> PathBuf {
    // If the caller pointed straight at a config.yaml file, use its parent.
    if p.is_file() {
        return p
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| p.to_path_buf());
    }
    p.to_path_buf()
}

pub fn load_settings(dir: Option<&Path>) -> Result<UserConfig> {
    let Some(dir) = dir else {
        return Ok(UserConfig::default());
    };
    let path = dir.join("config.yaml");
    if !path.is_file() {
        return Ok(UserConfig::default());
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let settings: UserConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("parsing {} as YAML", path.display()))?;
    Ok(settings)
}

pub fn load(cli_override: Option<&Path>) -> Result<ResolvedConfig> {
    let dir = resolve_config_dir(cli_override);
    let settings = load_settings(dir.as_deref())?;
    Ok(ResolvedConfig { dir, settings })
}
