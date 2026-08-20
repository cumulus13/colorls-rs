use colored::Color;

use crate::entry::FileEntry;
use crate::theme::{color_from_name, Theme, DEFAULT_FILE_ICON, DEFAULT_FOLDER_ICON, SYMLINK_ICON};

/// Resolve the glyph to render for `entry`.
pub fn icon_for(entry: &FileEntry, theme: &Theme) -> String {
    if entry.is_broken_symlink {
        return SYMLINK_ICON.to_string();
    }

    if entry.is_dir {
        let key = entry.name.to_lowercase();
        return theme
            .icons_by_folder
            .get(&key)
            .cloned()
            .unwrap_or_else(|| DEFAULT_FOLDER_ICON.to_string());
    }

    let name_key = entry.name.to_lowercase();
    if let Some(icon) = theme.icons_by_filename.get(&name_key) {
        return icon.clone();
    }

    if let Some(ext) = entry.extension() {
        if let Some(icon) = theme.icons_by_ext.get(&ext) {
            return icon.clone();
        }
    }

    if entry.is_symlink {
        return SYMLINK_ICON.to_string();
    }

    DEFAULT_FILE_ICON.to_string()
}

/// Resolve the color *category* (a key into `theme.colors`) for `entry`.
/// This mirrors upstream colorls' notion of "what kind of thing is this,
/// for coloring purposes" independent of which literal icon glyph is used.
pub fn category_for(entry: &FileEntry) -> &'static str {
    if entry.is_broken_symlink {
        return "dead_link";
    }
    if entry.is_symlink {
        return "symlink";
    }
    if entry.is_dir {
        return "dir";
    }
    if entry.is_executable() {
        return "executable_file";
    }
    "recognized_file"
}

/// Look up a finer-grained category (source_code, image, document, ...)
/// via the extension-alias table, falling back to the coarse category from
/// `category_for`.
pub fn resolved_category(entry: &FileEntry, theme: &Theme) -> String {
    if !entry.is_dir && !entry.is_symlink {
        if let Some(ext) = entry.extension() {
            if let Some(cat) = theme.aliases.get(&ext) {
                return cat.clone();
            }
        }
        if entry.is_executable() {
            return "executable_file".to_string();
        }
        return "unrecognized_file".to_string();
    }
    category_for(entry).to_string()
}

/// Resolve the actual `Color` to paint an entry's name with. Checks
/// `extension_colors.yaml` first for a direct per-extension override (e.g.
/// giving `.zip` and `.jar` different colors even though both are
/// "compressed"), then falls back to the entry's category color exactly as
/// `resolved_category` + `Theme::color_for` would.
pub fn entry_color(entry: &FileEntry, theme: &Theme) -> Color {
    if !entry.is_dir && !entry.is_symlink {
        if let Some(ext) = entry.extension() {
            if let Some(color_name) = theme.extension_colors.get(&ext) {
                return color_from_name(color_name);
            }
        }
    }
    theme.color_for(&resolved_category(entry, theme))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn file_entry(name: &str) -> FileEntry {
        #[cfg(unix)]
        {
            FileEntry {
                path: PathBuf::from(name),
                name: name.to_string(),
                is_dir: false,
                is_symlink: false,
                symlink_target: None,
                is_broken_symlink: false,
                size: 0,
                modified: Some(SystemTime::now()),
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
                is_dir: false,
                is_symlink: false,
                symlink_target: None,
                is_broken_symlink: false,
                size: 0,
                modified: Some(SystemTime::now()),
                readonly: false,
            }
        }
    }

    #[test]
    fn distinct_archive_extensions_get_distinct_colors_by_default() {
        let theme = Theme::load(None, false).unwrap();
        let zip = entry_color(&file_entry("a.zip"), &theme);
        let jar = entry_color(&file_entry("a.jar"), &theme);
        let bz2 = entry_color(&file_entry("a.tar.bz2"), &theme);
        let tar = entry_color(&file_entry("a.tar"), &theme);

        // All four share the "compressed" category, but extension_colors.yaml
        // gives each a distinct color, so none of these should collide.
        assert_ne!(zip, jar);
        assert_ne!(zip, bz2);
        assert_ne!(zip, tar);
        assert_ne!(jar, bz2);
    }

    #[test]
    fn unmapped_extension_falls_back_to_category_color() {
        let theme = Theme::load(None, false).unwrap();
        // .rs has no entry in extension_colors.yaml, so it should fall back
        // to the "source_code" category color, not panic or default white.
        let rs = entry_color(&file_entry("main.rs"), &theme);
        assert_eq!(rs, theme.color_for("source_code"));
    }
}
