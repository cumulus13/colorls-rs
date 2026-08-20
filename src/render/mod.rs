pub mod grid;
pub mod long;
pub mod tree;

use crate::cli::Cli;
use crate::git::GitRepoInfo;
use crate::theme::Theme;

pub struct RenderCtx<'a> {
    pub cli: &'a Cli,
    pub theme: &'a Theme,
    pub color_enabled: bool,
    pub icons_enabled: bool,
    pub git: Option<&'a GitRepoInfo>,
}

impl<'a> RenderCtx<'a> {
    pub fn git_status_glyph(&self, path: &std::path::Path) -> Option<String> {
        if !self.cli.git_status {
            return None;
        }
        let info = self.git?;
        let status = info.status_for(path);
        let glyph = status.glyph();
        Some(crate::colors::paint(
            glyph,
            status.category(),
            self.theme,
            self.color_enabled,
        ))
    }
}
