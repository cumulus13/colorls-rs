use crate::entry::FileEntry;
use crate::theme::{Theme, DEFAULT_FILE_ICON, DEFAULT_FOLDER_ICON, SYMLINK_ICON};

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
