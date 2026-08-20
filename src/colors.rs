use colored::{Color, Colorize};

use crate::theme::Theme;

/// Colorize `text` according to `category`, or return it unmodified when
/// `enabled` is false (i.e. `--color=never` or output isn't a TTY).
pub fn paint(text: &str, category: &str, theme: &Theme, enabled: bool) -> String {
    if !enabled {
        return text.to_string();
    }
    text.color(theme.color_for(category)).to_string()
}

/// Colorize `text` with an already-resolved `Color` directly, bypassing
/// the category lookup. Used for per-extension overrides where the color
/// was resolved via `icons::entry_color` rather than a category name.
pub fn paint_color(text: &str, color: Color, enabled: bool) -> String {
    if !enabled {
        return text.to_string();
    }
    text.color(color).to_string()
}
