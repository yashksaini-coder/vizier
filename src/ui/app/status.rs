//! Status bar (footer) block: context-specific commands and counts.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::types::{Focus, Tab};
use super::VizierUi;

impl<'a> VizierUi<'a> {
    pub(super) fn render_status(&self, area: Rect, buf: &mut Buffer) {
        let focus_indicator = match self.focus {
            Focus::Search => ("🔍", "Search"),
            Focus::List => ("📋", "List"),
            Focus::Inspector => ("🔬", "Inspector"),
            Focus::CopilotChat => ("💬", "Copilot"),
        };

        let status_line =
            if self.current_tab == Tab::Crates && self.selected_installed_crate.is_some() {
                if let Some(crate_info) = self.selected_installed_crate {
                    let selection_info = if let Some(selected) = self.list_selected {
                        format!("[{}/{}]", selected + 1, self.installed_crate_items.len())
                    } else {
                        format!("[0/{}]", self.installed_crate_items.len())
                    };
                    Line::from(vec![
                        Span::styled(" 📦 ", self.theme.style_accent()),
                        Span::styled(&crate_info.name, self.theme.style_normal()),
                        Span::styled(format!(" v{}", crate_info.version), self.theme.style_dim()),
                        Span::styled(" │ ", self.theme.style_muted()),
                        Span::styled(selection_info, self.theme.style_muted()),
                        Span::styled(" │ ", self.theme.style_muted()),
                        Span::styled("Tab", self.theme.style_accent()),
                        Span::styled(" focus ", self.theme.style_muted()),
                        Span::styled("↑/↓ j/k", self.theme.style_accent()),
                        Span::styled(" list ", self.theme.style_muted()),
                        Span::styled("Enter →", self.theme.style_accent()),
                        Span::styled(" details ", self.theme.style_muted()),
                        Span::styled("/", self.theme.style_accent()),
                        Span::styled(" search ", self.theme.style_muted()),
                        Span::styled("Esc", self.theme.style_accent()),
                        Span::styled(" back ", self.theme.style_muted()),
                        Span::styled("│ ", self.theme.style_dim()),
                        Span::styled(" [g] ", self.theme.style_accent()),
                        Span::styled("GitHub ", self.theme.style_muted()),
                        Span::styled("[s] ", self.theme.style_accent()),
                        Span::styled("Sponsor", self.theme.style_muted()),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(" 📦 Crates ", self.theme.style_accent()),
                        Span::styled(focus_indicator.0, self.theme.style_accent()),
                        Span::styled(format!(" {} ", focus_indicator.1), self.theme.style_dim()),
                        Span::styled(" │ Tab ↑/↓ Enter / Esc back ", self.theme.style_muted()),
                        Span::styled("│ ", self.theme.style_dim()),
                        Span::styled(" [g] ", self.theme.style_accent()),
                        Span::styled("GitHub ", self.theme.style_muted()),
                        Span::styled("[s] ", self.theme.style_accent()),
                        Span::styled("Sponsor", self.theme.style_muted()),
                    ])
                }
            } else if self.current_tab == Tab::Crates {
                Line::from(vec![
                    Span::styled("Commands: ", self.theme.style_dim()),
                    Span::styled("Tab", self.theme.style_accent()),
                    Span::styled(" focus ", self.theme.style_muted()),
                    Span::styled("↑/↓ j/k", self.theme.style_accent()),
                    Span::styled(" list ", self.theme.style_muted()),
                    Span::styled("Enter →", self.theme.style_accent()),
                    Span::styled(" open ", self.theme.style_muted()),
                    Span::styled("/", self.theme.style_accent()),
                    Span::styled(" search ", self.theme.style_muted()),
                    Span::styled("[o]", self.theme.style_accent()),
                    Span::styled(" docs ", self.theme.style_muted()),
                    Span::styled("[c]", self.theme.style_accent()),
                    Span::styled(" crates.io ", self.theme.style_muted()),
                    Span::styled("│ ", self.theme.style_dim()),
                    Span::styled("📦", self.theme.style_accent()),
                    Span::styled(
                        format!(" Crates ({}) ", self.filtered_dependency_indices.len()),
                        self.theme.style_normal(),
                    ),
                    Span::styled("│ ", self.theme.style_dim()),
                    Span::styled(" [g] ", self.theme.style_accent()),
                    Span::styled("GitHub ", self.theme.style_muted()),
                    Span::styled("[s] ", self.theme.style_accent()),
                    Span::styled("Sponsor", self.theme.style_muted()),
                ])
            } else if !self.status_message.is_empty() {
                Line::from(vec![
                    Span::styled(
                        format!(" {} ", self.status_message),
                        self.theme.style_string(),
                    ),
                    Span::styled(" │ ", self.theme.style_muted()),
                    Span::styled("Tab", self.theme.style_accent()),
                    Span::styled(" focus ", self.theme.style_muted()),
                    Span::styled("↑/↓ Enter / ", self.theme.style_accent()),
                    Span::styled("? help q quit ", self.theme.style_muted()),
                    Span::styled("│ ", self.theme.style_dim()),
                    Span::styled(" [g] ", self.theme.style_accent()),
                    Span::styled("GitHub ", self.theme.style_muted()),
                    Span::styled("[s] ", self.theme.style_accent()),
                    Span::styled("Sponsor", self.theme.style_muted()),
                ])
            } else {
                let (fn_count, struct_count, _enum_count, _trait_count) = self.items.iter().fold(
                    (0usize, 0usize, 0usize, 0usize),
                    |(f, s, e, t), item| match item.kind() {
                        "fn" => (f + 1, s, e, t),
                        "struct" => (f, s + 1, e, t),
                        "enum" => (f, s, e + 1, t),
                        "trait" => (f, s, e, t + 1),
                        _ => (f, s, e, t),
                    },
                );
                let selection_info = if let Some(selected) = self.list_selected {
                    format!("[{}/{}]", selected + 1, self.filtered_items.len())
                } else {
                    format!("[0/{}]", self.filtered_items.len())
                };
                Line::from(vec![
                    Span::styled("Commands: ", self.theme.style_dim()),
                    Span::styled("Tab", self.theme.style_accent()),
                    Span::styled(" focus ", self.theme.style_muted()),
                    Span::styled("↑/↓ j/k", self.theme.style_accent()),
                    Span::styled(" list ", self.theme.style_muted()),
                    Span::styled("Enter →", self.theme.style_accent()),
                    Span::styled(" open ", self.theme.style_muted()),
                    Span::styled("/", self.theme.style_accent()),
                    Span::styled(" search ", self.theme.style_muted()),
                    Span::styled("1-4", self.theme.style_accent()),
                    Span::styled(" tabs ", self.theme.style_muted()),
                    Span::styled("? ", self.theme.style_accent()),
                    Span::styled("help ", self.theme.style_muted()),
                    Span::styled("q ", self.theme.style_accent()),
                    Span::styled("quit ", self.theme.style_muted()),
                    Span::styled("│ ", self.theme.style_dim()),
                    Span::styled("fn:", self.theme.style_function()),
                    Span::styled(format!("{} ", fn_count), self.theme.style_normal()),
                    Span::styled("struct:", self.theme.style_type()),
                    Span::styled(format!("{} ", struct_count), self.theme.style_normal()),
                    Span::styled("selection ", self.theme.style_muted()),
                    Span::styled(selection_info, self.theme.style_dim()),
                    Span::styled("│ ", self.theme.style_dim()),
                    Span::styled(" [g] ", self.theme.style_accent()),
                    Span::styled("GitHub ", self.theme.style_muted()),
                    Span::styled("[s] ", self.theme.style_accent()),
                    Span::styled("Sponsor", self.theme.style_muted()),
                ])
            };

        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(self.theme.style_border())
            .style(Style::default().bg(self.theme.bg_panel));
        let inner = block.inner(area);
        block.render(area, buf);
        Paragraph::new(status_line)
            .alignment(Alignment::Left)
            .render(inner, buf);
    }
}
