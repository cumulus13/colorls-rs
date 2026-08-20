use std::io::Write;
use std::time::SystemTime;

/// Write a line to stdout, exiting quietly (code 0) on a broken pipe (e.g.
/// `colorls | head`) instead of panicking the way a bare `println!` would.
/// Any other write failure is reported and exits with a failure code.
pub fn safe_println(s: &str) {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    if let Err(e) = writeln!(lock, "{}", s) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("colorls: error writing output: {}", e);
        std::process::exit(1);
    }
}

#[macro_export]
macro_rules! oprintln {
    () => {
        $crate::util::safe_println("")
    };
    ($($arg:tt)*) => {
        $crate::util::safe_println(&format!($($arg)*))
    };
}

use chrono::{DateTime, Local};
use unicode_width::UnicodeWidthStr;

/// Format a byte count the way `ls -lh` / colorls do: one decimal place
/// past the first unit, no decimal for bytes.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    if bytes == 0 {
        return "0B".to_string();
    }
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{}{}", bytes, UNITS[0])
    } else {
        format!("{:.1}{}", size, UNITS[unit_idx])
    }
}

pub fn format_time(t: SystemTime) -> String {
    let dt: DateTime<Local> = t.into();
    dt.format("%d %b %H:%M").to_string()
}
/// Detect terminal width, falling back to 80 columns when not attached to a
/// TTY (pipes, redirected output, CI, etc.) or when detection fails.
pub fn terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .filter(|w| *w > 0)
        .unwrap_or(80)
}

/// Visible column width of a string, accounting for wide (CJK) characters.
/// Nerd-font icon glyphs are typically reported as width 1 or 2 depending
/// on the font; we treat the whole private-use-area icon range as width 2
/// to match how most nerd fonts actually render them in a terminal grid.
pub fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let cp = c as u32;
            if (0xE000..=0xF8FF).contains(&cp) || (0xF0000..=0xFFFFD).contains(&cp) {
                2
            } else {
                UnicodeWidthStr::width(c.to_string().as_str())
            }
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_bytes() {
        assert_eq!(human_size(0), "0B");
        assert_eq!(human_size(512), "512B");
        assert_eq!(human_size(1023), "1023B");
    }

    #[test]
    fn human_size_kilobytes_and_up() {
        assert_eq!(human_size(1024), "1.0K");
        assert_eq!(human_size(1536), "1.5K");
        assert_eq!(human_size(1024 * 1024), "1.0M");
        assert_eq!(human_size(1024 * 1024 * 1024), "1.0G");
    }

    #[test]
    fn display_width_ascii() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn display_width_icon_glyph_counts_as_two() {
        // A Nerd Font private-use-area glyph should be treated as width 2,
        // matching how these fonts actually render in a terminal grid.
        let icon = "\u{f15b}";
        assert_eq!(display_width(icon), 2);
    }
}
