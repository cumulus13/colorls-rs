use crate::entry::FileEntry;
use crate::icons::{icon_for, resolved_category};
use crate::render::RenderCtx;
use crate::util::{display_width, terminal_width};

const SPACING: usize = 2;

struct Cell {
    rendered: String,
    width: usize,
}

fn build_cell(entry: &FileEntry, ctx: &RenderCtx) -> Cell {
    let icon = if ctx.icons_enabled {
        format!("{} ", icon_for(entry, ctx.theme))
    } else {
        String::new()
    };

    let mut label = entry.name.clone();
    if entry.is_dir {
        label.push('/');
    }

    let category = resolved_category(entry, ctx.theme);
    let colored_label = crate::colors::paint(&label, &category, ctx.theme, ctx.color_enabled);

    let git = ctx.git_status_glyph(&entry.path);
    let git_raw_width = if ctx.cli.git_status { 2 } else { 0 };

    let rendered = match git {
        Some(g) => format!("{}{}{} ", g, icon, colored_label),
        None => format!("{}{}", icon, colored_label),
    };

    let icon_width = if ctx.icons_enabled { 2 } else { 0 };
    let width = display_width(&label) + icon_width + git_raw_width;

    Cell { rendered, width }
}

/// Render `entries` as a multi-column grid sized to the terminal width, or
/// one entry per line if `oneline` is set or output width can't fit more
/// than a single column.
pub fn render(entries: &[FileEntry], ctx: &RenderCtx) {
    if entries.is_empty() {
        return;
    }

    let cells: Vec<Cell> = entries.iter().map(|e| build_cell(e, ctx)).collect();

    if ctx.cli.oneline {
        for cell in &cells {
            crate::oprintln!("{}", cell.rendered);
        }
        return;
    }

    let term_width = terminal_width();
    let n = cells.len();
    let max_width = cells.iter().map(|c| c.width).max().unwrap_or(1);

    // Fast path: nothing fits side by side.
    if max_width + SPACING > term_width {
        for cell in &cells {
            crate::oprintln!("{}", cell.rendered);
        }
        return;
    }

    let mut chosen_cols = 1usize;
    let mut chosen_rows = n;
    let mut chosen_col_widths: Vec<usize> = vec![max_width];

    let upper_bound = (term_width / (1 + SPACING)).max(1).min(n);
    for cols in (1..=upper_bound).rev() {
        let rows = n.div_ceil(cols);
        let mut col_widths = vec![0usize; cols];
        for (i, cell) in cells.iter().enumerate() {
            let col = i / rows;
            if cell.width > col_widths[col] {
                col_widths[col] = cell.width;
            }
        }
        let total: usize = col_widths.iter().sum::<usize>() + SPACING * (cols.saturating_sub(1));
        if total <= term_width {
            chosen_cols = cols;
            chosen_rows = rows;
            chosen_col_widths = col_widths;
            break;
        }
    }

    for row in 0..chosen_rows {
        let mut line = String::new();
        #[allow(clippy::needless_range_loop)]
        // `col` is used for lookahead into the next column, not just indexing
        for col in 0..chosen_cols {
            let idx = col * chosen_rows + row;
            if idx >= n {
                continue;
            }
            let cell = &cells[idx];
            line.push_str(&cell.rendered);
            let pad = chosen_col_widths[col].saturating_sub(cell.width);
            if col + 1 < chosen_cols && (col + 1) * chosen_rows + row < n {
                line.push_str(&" ".repeat(pad + SPACING));
            }
        }
        crate::oprintln!("{}", line.trim_end());
    }
}
