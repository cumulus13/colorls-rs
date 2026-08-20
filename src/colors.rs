use colored::{Color, Colorize};

use crate::theme::Theme;

/// Render `text` in `color`, honoring `enabled` (false = plain, unmodified
/// text — used for `--color=never` or non-TTY output).
///
/// For `Color::TrueColor` (hex colors from `dark_colors.yaml`,
/// `light_colors.yaml`, or `extension_colors.yaml`), this bypasses
/// `colored`'s own automatic downgrade-to-nearest-16-color behavior and
/// always emits the exact 24-bit ANSI sequence. `colored` decides whether
/// to downgrade based on the `COLORTERM` environment variable, which is an
/// unreliable signal in practice — it's frequently missing over SSH, in
/// tmux, or in other passthrough shells even when the terminal actually
/// displaying the output supports 24-bit color just fine. A hex color the
/// user explicitly configured should render as exactly that color
/// wherever the terminal honors 24-bit ANSI (effectively universal among
/// terminals still receiving updates in 2026); silently substituting the
/// nearest of 16 named colors instead is surprising at best and can look
/// like a completely different color at worst.
fn render(text: &str, color: Color, enabled: bool) -> String {
    if !enabled {
        return text.to_string();
    }
    match color {
        Color::TrueColor { r, g, b } => format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m"),
        _ => text.color(color).to_string(),
    }
}

/// Colorize `text` according to `category`, or return it unmodified when
/// `enabled` is false (i.e. `--color=never` or output isn't a TTY).
pub fn paint(text: &str, category: &str, theme: &Theme, enabled: bool) -> String {
    render(text, theme.color_for(category), enabled)
}

/// Colorize `text` with an already-resolved `Color` directly, bypassing
/// the category lookup. Used for per-extension overrides where the color
/// was resolved via `icons::entry_color` rather than a category name.
pub fn paint_color(text: &str, color: Color, enabled: bool) -> String {
    render(text, color, enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truecolor_renders_exact_bytes_regardless_of_colorterm() {
        // Regression test for the reported bug: hex colors must render as
        // the exact configured RGB triple, not get silently downgraded to
        // the nearest of 16 named colors when COLORTERM isn't set/detected.
        std::env::remove_var("COLORTERM");
        let out = render(
            "x",
            Color::TrueColor {
                r: 170,
                g: 170,
                b: 0,
            },
            true,
        );
        assert_eq!(out, "\x1b[38;2;170;170;0mx\x1b[0m");

        std::env::set_var("COLORTERM", "truecolor");
        let out2 = render(
            "x",
            Color::TrueColor {
                r: 170,
                g: 170,
                b: 0,
            },
            true,
        );
        assert_eq!(out, out2);
    }

    #[test]
    fn truecolor_channel_order_is_never_swapped() {
        // #AABBCC -> r=0xAA g=0xBB b=0xCC, specifically checked in that
        // order since a channel-order bug is exactly what was reported.
        let out = render(
            "x",
            Color::TrueColor {
                r: 0xAA,
                g: 0xBB,
                b: 0xCC,
            },
            true,
        );
        assert_eq!(out, "\x1b[38;2;170;187;204mx\x1b[0m");
    }

    #[test]
    fn named_colors_still_use_colored_crate_path() {
        // Named (non-hex) colors are unaffected by the truecolor bypass —
        // still go through `colored`'s normal 4-bit SGR codes. Force the
        // override here since `colored` otherwise auto-detects "not a
        // TTY" during `cargo test` and silently strips color, which would
        // make this assertion fail for reasons unrelated to what it's
        // actually testing.
        colored::control::set_override(true);
        let out = render("x", Color::Red, true);
        assert!(out.contains('\x1b'));
        assert!(!out.contains("38;2;"));
    }

    #[test]
    fn disabled_returns_plain_text() {
        assert_eq!(
            render("x", Color::TrueColor { r: 1, g: 2, b: 3 }, false),
            "x"
        );
    }
}
