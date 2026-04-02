//! Right panel: vertical divider, inspector (item details / dependency view / crate info).

use crate::ui::dependency_view::{self, DependencyDocView, DependencyView};
use crate::ui::inspector::InspectorPanel;
use crate::ui::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        block::BorderType, Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};

use super::types::{Focus, Tab};
use super::RustlensUi;

/// Parse a line of markdown into styled spans: **bold**, `code`, ## header.
fn markdown_line_to_spans(line: &str, theme: &Theme, base_style: Style) -> Vec<Span<'static>> {
    let mut spans: Vec<Span> = Vec::new();
    let bold = base_style.add_modifier(Modifier::BOLD);
    let code_style = theme.style_type();
    let header_style = theme.style_accent().add_modifier(Modifier::BOLD);

    let mut s = line;
    if s.starts_with("## ") {
        spans.push(Span::styled("  ", theme.style_dim()));
        spans.push(Span::styled(
            s.trim_start_matches("## ").to_string(),
            header_style,
        ));
        return spans;
    }
    if s.starts_with("# ") {
        spans.push(Span::styled("  ", theme.style_dim()));
        spans.push(Span::styled(
            s.trim_start_matches("# ").to_string(),
            header_style,
        ));
        return spans;
    }

    while !s.is_empty() {
        if let Some(rest) = s.strip_prefix("**") {
            if let Some(end) = rest.find("**") {
                spans.push(Span::styled(rest[..end].to_string(), bold));
                s = &rest[end + 2..];
                continue;
            }
        }
        if s.starts_with('`') {
            if let Some(end) = s[1..].find('`') {
                let code = &s[1..=end];
                spans.push(Span::styled(code.to_string(), code_style));
                s = &s[end + 2..];
                continue;
            }
        }
        let next_bold = s.find("**");
        let next_code = s.find('`');
        let next = match (next_bold, next_code) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        if let Some(i) = next {
            spans.push(Span::styled(s[..i].to_string(), base_style));
            s = &s[i..];
        } else {
            spans.push(Span::styled(s.to_string(), base_style));
            break;
        }
    }
    spans
}

impl<'a> RustlensUi<'a> {
    pub(super) fn render_vertical_divider(&self, area: Rect, buf: &mut Buffer) {
        let style = self.theme.style_border();
        let symbol = "│";
        for y in area.top()..area.bottom() {
            if area.width > 0 {
                if let Some(cell) = buf.cell_mut((area.x, y)) {
                    cell.set_symbol(symbol).set_style(style);
                }
            }
        }
    }

    pub(super) fn render_inspector(&self, area: Rect, buf: &mut Buffer) {
        if self.current_tab == Tab::Crates && self.selected_installed_crate.is_some() {
            if self.selected_item.is_none() {
                self.render_installed_crate_info(area, buf);
            } else {
                let inspector = InspectorPanel::new(self.theme)
                    .item(self.selected_item)
                    .all_items(self.all_items_impl_lookup)
                    .focused(self.focus == Focus::Inspector)
                    .scroll(self.inspector_scroll);
                inspector.render(area, buf);
            }
        } else if self.current_tab == Tab::Crates {
            let root_name = self.dependency_tree.first().map(|(n, _)| n.as_str());
            let selected_name = self
                .list_selected
                .and_then(|i| self.filtered_dependency_indices.get(i).copied())
                .and_then(|tree_idx| self.dependency_tree.get(tree_idx))
                .map(|(n, _)| n.as_str());
            let showing_root = root_name
                .zip(selected_name)
                .map(|(r, s)| r == s)
                .unwrap_or(true);
            if showing_root {
                let dep_view = DependencyView::new(self.theme)
                    .crate_info(self.crate_info)
                    .focused(self.focus == Focus::Inspector)
                    .scroll(self.inspector_scroll)
                    .show_browser_hint(true);
                dep_view.render(area, buf);
            } else if let Some(name) = selected_name {
                if self.crate_doc_loading {
                    dependency_view::render_doc_loading(self.theme, area, buf, name);
                } else if self.crate_doc_failed {
                    dependency_view::render_doc_failed(self.theme, area, buf, name);
                } else if let Some(doc) = self.crate_doc {
                    let doc_view = DependencyDocView::new(self.theme, doc)
                        .focused(self.focus == Focus::Inspector)
                        .scroll(self.inspector_scroll)
                        .show_browser_hint(true);
                    doc_view.render(area, buf);
                } else {
                    dependency_view::render_doc_loading(self.theme, area, buf, name);
                }
            } else {
                let dep_view = DependencyView::new(self.theme)
                    .crate_info(self.crate_info)
                    .focused(self.focus == Focus::Inspector)
                    .scroll(self.inspector_scroll)
                    .show_browser_hint(true);
                dep_view.render(area, buf);
            }
        } else {
            let inspector = InspectorPanel::new(self.theme)
                .item(self.selected_item)
                .all_items(self.all_items_impl_lookup)
                .focused(self.focus == Focus::Inspector)
                .scroll(self.inspector_scroll);
            inspector.render(area, buf);
        }
    }

    pub(super) fn render_installed_crate_info(&self, area: Rect, buf: &mut Buffer) {
        let crate_info = match self.selected_installed_crate {
            Some(c) => c,
            None => return,
        };
        let border_style = if self.focus == Focus::Inspector {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };
        let mut lines = vec![
            Line::from(vec![
                Span::styled("📦 ", Style::default()),
                Span::styled(
                    &crate_info.name,
                    self.theme
                        .style_accent_bold()
                        .add_modifier(Modifier::UNDERLINED),
                ),
                Span::raw(" "),
                Span::styled(format!("v{}", crate_info.version), self.theme.style_muted()),
            ]),
            Line::from(""),
        ];
        if let Some(ref desc) = crate_info.description {
            lines.push(Line::from(vec![
                Span::styled("━━━ ", self.theme.style_muted()),
                Span::styled("Description", self.theme.style_accent()),
                Span::styled(" ━━━", self.theme.style_muted()),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(desc.clone(), self.theme.style_normal()),
            ]));
            lines.push(Line::from(""));
        }
        if let Some(ref license) = crate_info.license {
            lines.push(Line::from(vec![
                Span::styled("  License: ", self.theme.style_dim()),
                Span::styled(license.clone(), self.theme.style_normal()),
            ]));
        }
        if !crate_info.authors.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  Authors: ", self.theme.style_dim()),
                Span::styled(crate_info.authors.join(", "), self.theme.style_normal()),
            ]));
        }
        if let Some(ref repo) = crate_info.repository {
            lines.push(Line::from(vec![
                Span::styled("  Repository: ", self.theme.style_dim()),
                Span::styled(repo.clone(), self.theme.style_accent()),
            ]));
        }
        if let Some(ref docs) = crate_info.documentation {
            lines.push(Line::from(vec![
                Span::styled("  Docs: ", self.theme.style_dim()),
                Span::styled(docs.clone(), self.theme.style_accent()),
            ]));
        }
        if !crate_info.keywords.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("━━━ ", self.theme.style_muted()),
                Span::styled("Keywords", self.theme.style_accent()),
                Span::styled(" ━━━", self.theme.style_muted()),
            ]));
            lines.push(Line::from(""));
            let keywords: Vec<Span> = crate_info
                .keywords
                .iter()
                .map(|k| Span::styled(format!(" {} ", k), self.theme.style_keyword()))
                .collect();
            lines.push(Line::from(vec![Span::raw("  ")]).patch_style(Style::default()));
            for kw in keywords {
                lines.push(Line::from(vec![Span::raw("  "), kw]));
            }
        }
        if !crate_info.categories.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("━━━ ", self.theme.style_muted()),
                Span::styled("Categories", self.theme.style_accent()),
                Span::styled(" ━━━", self.theme.style_muted()),
            ]));
            lines.push(Line::from(""));
            for cat in &crate_info.categories {
                lines.push(Line::from(vec![
                    Span::raw("  • "),
                    Span::styled(cat.clone(), self.theme.style_type()),
                ]));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("━━━ ", self.theme.style_muted()),
            Span::styled("Location", self.theme.style_accent()),
            Span::styled(" ━━━", self.theme.style_muted()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  📁 "),
            Span::styled(
                crate_info.path.display().to_string(),
                self.theme.style_muted(),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("━━━ ", self.theme.style_muted()),
            Span::styled("Analysis", self.theme.style_accent()),
            Span::styled(" ━━━", self.theme.style_muted()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{} items found", self.installed_crate_items.len()),
                self.theme.style_normal(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Select an item to view details", self.theme.style_muted()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" [o] ", self.theme.style_accent()),
            Span::styled("docs.rs  ", self.theme.style_dim()),
            Span::styled(" [c] ", self.theme.style_accent()),
            Span::styled("crates.io", self.theme.style_dim()),
        ]));
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(" ◇ Crate Info ");
        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        paragraph.render(area, buf);
    }

    pub(super) fn render_copilot_chat(&self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }
        let border_style = if self.focus == Focus::CopilotChat {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(" ◇ Copilot ");
        let inner = block.inner(area);
        block.render(area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(2), Constraint::Length(1)])
            .split(inner);
        let messages_area = chunks[0];
        let input_area = chunks[1];

        let mut lines: Vec<Line<'_>> = Vec::new();
        if self.copilot_chat_loading
            && self
                .copilot_chat_messages
                .last()
                .is_some_and(|(r, _)| r == "user")
        {
            lines.push(Line::from(Span::styled(
                "  … Copilot is thinking…",
                self.theme.style_muted(),
            )));
        }
        for (role, content) in self.copilot_chat_messages {
            let label = if role == "user" { "You" } else { "Copilot" };
            let base_style = if role == "user" {
                self.theme.style_accent()
            } else {
                self.theme.style_normal()
            };
            lines.push(Line::from(Span::styled(
                format!("  {}: ", label),
                self.theme.style_dim().add_modifier(Modifier::BOLD),
            )));
            for raw_line in content.lines() {
                let trimmed = raw_line.trim_end();
                if role == "assistant" {
                    let mut sp = vec![Span::raw("    ")];
                    sp.extend(markdown_line_to_spans(trimmed, self.theme, base_style));
                    lines.push(Line::from(sp));
                } else {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(trimmed.to_string(), base_style),
                    ]));
                }
            }
            lines.push(Line::from(""));
        }

        let total_lines = lines.len();
        let visible_height = messages_area.height as usize;
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll = self.copilot_chat_scroll.min(max_scroll);

        // Line-based scroll: slice the content (like inspector) so scroll is in line units, not rows.
        // Paragraph::scroll() uses row offset which breaks when lines wrap.
        let content_area = Rect {
            width: messages_area.width.saturating_sub(1),
            ..messages_area
        };
        let visible_lines: Vec<Line<'_>> = lines
            .iter()
            .skip(scroll)
            .take(visible_height)
            .cloned()
            .collect();
        Paragraph::new(visible_lines)
            .wrap(Wrap { trim: false })
            .render(content_area, buf);

        if total_lines > visible_height {
            let scrollbar_area = Rect {
                x: messages_area.x + messages_area.width.saturating_sub(1),
                y: messages_area.y,
                width: 1,
                height: messages_area.height,
            };
            let mut scrollbar_state = ScrollbarState::new(total_lines)
                .position(scroll)
                .viewport_content_length(visible_height);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            scrollbar.render(scrollbar_area, buf, &mut scrollbar_state);
        }

        // Input row: distinct background so the input area is clearly visible
        let input_display: &str = if self.copilot_chat_input.is_empty() {
            "Ask about this item… (Enter to send, Esc to close)"
        } else {
            self.copilot_chat_input
        };
        let input_style = if self.focus == Focus::CopilotChat {
            self.theme.style_accent()
        } else {
            self.theme.style_muted()
        };
        let input_block = Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(self.theme.bg_highlight));
        let input_inner = input_block.inner(input_area);
        input_block.render(input_area, buf);
        let input_line = Paragraph::new(Line::from(vec![
            Span::styled(" ▸ ", self.theme.style_dim()),
            Span::styled(input_display, input_style),
        ]));
        input_line.render(input_inner, buf);
    }
}
