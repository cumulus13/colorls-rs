use colored::Colorize;

use crate::theme::Theme;

/// Colorize `text` according to `category`, or return it unmodified when
/// `enabled` is false (i.e. `--color=never` or output isn't a TTY).
pub fn paint(text: &str, category: &str, theme: &Theme, enabled: bool) -> String {
    if !enabled {
        return text.to_string();
    }
    text.color(theme.color_for(category)).to_string()
}
