//! Design tokens — centralized constants for the Rustlens UI.

use ratatui::{
    style::Modifier,
    text::{Line, Span},
};

use crate::ui::theme::Theme;

// ── Scrollbar ──
pub const SCROLLBAR_UP: &str = "↑";
pub const SCROLLBAR_DOWN: &str = "↓";

// ── Overlay dimensions ──
pub const OVERLAY_SETTINGS_W: u16 = 50;
pub const OVERLAY_SETTINGS_H: u16 = 12;
pub const OVERLAY_HELP_W: u16 = 64;
pub const OVERLAY_HELP_H: u16 = 32;

// ── Layout ──
pub const HEADER_HEIGHT: u16 = 4;
pub const STATUS_HEIGHT: u16 = 1;
pub const BODY_MARGIN: u16 = 1;
pub const MIN_BODY_HEIGHT: u16 = 10;
pub const COMPLETION_MAX_VISIBLE: usize = 10;

// ── Borders & Chrome ──
pub const SECTION_DIVIDER: &str = "─────────────────";

// ── Small terminal thresholds ──
pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 20;

/// Unified section header: "  ▸ Title ─────────────────"
pub fn section_header(title: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled("  ▸ ", theme.style_accent()),
        Span::styled(
            title.to_string(),
            theme.style_accent().add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {}", SECTION_DIVIDER), theme.style_muted()),
    ])
}
