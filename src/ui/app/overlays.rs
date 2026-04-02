//! Overlay blocks: settings popup, help popup.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use super::VizierUi;

impl<'a> VizierUi<'a> {
    pub(super) fn render_settings_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.show_settings {
            return;
        }
        let w = 48.min(area.width.saturating_sub(4));
        let h = 10.min(area.height.saturating_sub(4));
        let settings_area = Rect {
            x: area.x + (area.width - w) / 2,
            y: area.y + (area.height - h) / 2,
            width: w,
            height: h,
        };
        Clear.render(settings_area, buf);
        let text = vec![
            Line::from(Span::styled(" Settings ", self.theme.style_accent_bold())),
            Line::from(""),
            Line::from(Span::styled("Theme", self.theme.style_dim())),
            Line::from(vec![
                Span::raw("  Press "),
                Span::styled("t", self.theme.style_accent()),
                Span::raw(" to cycle theme"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press Esc or S to close",
                self.theme.style_muted(),
            )),
        ];
        let block = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.style_border_focused())
                .title(" Settings ")
                .style(Style::default().bg(self.theme.bg_panel)),
        );
        block.render(settings_area, buf);
    }

    pub(super) fn render_help_overlay(&self, area: Rect, buf: &mut Buffer) {
        if !self.show_help {
            return;
        }
        let help_width = 62.min(area.width.saturating_sub(4));
        let help_height = 30.min(area.height.saturating_sub(4));
        let help_area = Rect {
            x: area.x + (area.width - help_width) / 2,
            y: area.y + (area.height - help_height) / 2,
            width: help_width,
            height: help_height,
        };
        Clear.render(help_area, buf);
        let help_text = vec![
            Line::from(Span::styled(
                "⌨️  Keyboard Shortcuts",
                self.theme.style_accent_bold(),
            )),
            Line::from(""),
            Line::from(Span::styled("Focus & panels", self.theme.style_dim())),
            Line::from(vec![
                Span::styled("  Tab        ", self.theme.style_accent()),
                Span::raw("Next panel (search → list → inspector)"),
            ]),
            Line::from(vec![
                Span::styled("  Shift+Tab  ", self.theme.style_accent()),
                Span::raw("Previous panel"),
            ]),
            Line::from(vec![
                Span::styled("  /          ", self.theme.style_accent()),
                Span::raw("Focus search"),
            ]),
            Line::from(vec![
                Span::styled("  Esc        ", self.theme.style_accent()),
                Span::raw("Clear search / Back / Close popup"),
            ]),
            Line::from(""),
            Line::from(Span::styled("List & inspector", self.theme.style_dim())),
            Line::from(vec![
                Span::styled("  ↑/↓  j/k   ", self.theme.style_accent()),
                Span::raw("Move selection / Scroll inspector"),
            ]),
            Line::from(vec![
                Span::styled("  Enter  →  l ", self.theme.style_accent()),
                Span::raw("Open item / Focus inspector"),
            ]),
            Line::from(vec![
                Span::styled("  ←  h       ", self.theme.style_accent()),
                Span::raw("Back to list (e.g. exit crate view)"),
            ]),
            Line::from(vec![
                Span::styled("  Home       ", self.theme.style_accent()),
                Span::raw("First item"),
            ]),
            Line::from(vec![
                Span::styled("  G  End     ", self.theme.style_accent()),
                Span::raw("Last item"),
            ]),
            Line::from(vec![
                Span::styled("  PgUp  PgDn  ", self.theme.style_accent()),
                Span::raw("Page up / down"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Tabs", self.theme.style_dim())),
            Line::from(vec![
                Span::styled("  1  2  3  4  ", self.theme.style_accent()),
                Span::raw("Types · Functions · Modules · Crates"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Crates tab only", self.theme.style_dim())),
            Line::from(vec![
                Span::styled("  [o]        ", self.theme.style_accent()),
                Span::raw("Open docs.rs in browser"),
            ]),
            Line::from(vec![
                Span::styled("  [c]        ", self.theme.style_accent()),
                Span::raw("Open crates.io in browser"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Other", self.theme.style_dim())),
            Line::from(vec![
                Span::styled("  C          ", self.theme.style_accent()),
                Span::raw("Open Copilot chat (ask about current item)"),
            ]),
            Line::from(vec![
                Span::styled("  t          ", self.theme.style_accent()),
                Span::raw("Cycle theme"),
            ]),
            Line::from(vec![
                Span::styled("  S          ", self.theme.style_accent()),
                Span::raw("Settings overlay"),
            ]),
            Line::from(vec![
                Span::styled("  ?          ", self.theme.style_accent()),
                Span::raw("Toggle this help"),
            ]),
            Line::from(vec![
                Span::styled("  q  Esc     ", self.theme.style_accent()),
                Span::raw("Quit"),
            ]),
            Line::from(""),
            Line::from(Span::styled("Links", self.theme.style_dim())),
            Line::from(vec![
                Span::styled("  g          ", self.theme.style_accent()),
                Span::raw("Open GitHub repo in browser"),
            ]),
            Line::from(vec![
                Span::styled("  s          ", self.theme.style_accent()),
                Span::raw("Open Sponsor page in browser"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to close",
                self.theme.style_muted(),
            )),
        ];
        let help = Paragraph::new(help_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.style_border_focused())
                .title(" Help ")
                .style(Style::default().bg(self.theme.bg_panel)),
        );
        help.render(help_area, buf);
    }
}
