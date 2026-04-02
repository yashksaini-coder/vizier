//! List block: items list, dependencies list, installed crate items list.

use crate::analyzer::Visibility;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget,
    },
};

use super::types::{Focus, Tab};
use super::VizierUi;

impl<'a> VizierUi<'a> {
    pub(super) fn render_list(&self, area: Rect, buf: &mut Buffer) {
        if self.current_tab == Tab::Crates {
            if self.selected_installed_crate.is_some() {
                self.render_installed_crates_list(area, buf);
            } else {
                self.render_dependencies_list(area, buf);
            }
            return;
        }

        let selected = self.list_selected;
        let highlight_intensity = self.animation.map(|a| a.selection_highlight).unwrap_or(1.0);
        let visible_height = area.height.saturating_sub(2) as usize;
        let total_items = self.filtered_items.len();
        let scroll_offset = if let Some(sel) = selected {
            if visible_height == 0 {
                0
            } else if sel >= visible_height {
                sel.saturating_sub(visible_height - 1)
            } else {
                0
            }
        } else {
            0
        };

        let items: Vec<ListItem> = self
            .filtered_items
            .iter()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|(idx, item)| {
                let kind_style = match item.kind() {
                    "fn" => self.theme.style_function(),
                    "struct" | "enum" | "type" => self.theme.style_type(),
                    "trait" => self.theme.style_keyword(),
                    "mod" => self.theme.style_accent(),
                    "const" | "static" => self.theme.style_string(),
                    _ => self.theme.style_dim(),
                };
                let is_selected = Some(idx) == selected;
                let base_style = if is_selected {
                    if highlight_intensity < 1.0 {
                        self.theme.style_selected().add_modifier(Modifier::BOLD)
                    } else {
                        self.theme.style_selected()
                    }
                } else {
                    Style::default()
                };
                let prefix = if is_selected { "▸ " } else { "  " };
                let vis = item
                    .visibility()
                    .map(|v| match v {
                        Visibility::Public => "●",
                        Visibility::Crate => "◐",
                        _ => "○",
                    })
                    .unwrap_or("○");
                let display_name = item.name().to_string();
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, self.theme.style_accent()),
                    Span::styled(vis, self.theme.style_dim()),
                    Span::raw(" "),
                    Span::styled(format!("{:6} ", item.kind()), kind_style),
                    Span::styled(display_name, self.theme.style_normal()),
                ]))
                .style(base_style)
            })
            .collect();

        let border_style = if self.focus == Focus::List {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };
        let scroll_indicator = if total_items > visible_height {
            let pos = selected.unwrap_or(0) + 1;
            format!(" [{}/{}]", pos, total_items)
        } else {
            String::new()
        };
        let title = if self.search_input.is_empty() {
            format!(
                " Items ({}){} ",
                self.filtered_items.len(),
                scroll_indicator
            )
        } else {
            format!(
                " Items ({}/{}){} ",
                self.filtered_items.len(),
                self.items.len(),
                scroll_indicator
            )
        };
        let list_area = Rect {
            width: area.width.saturating_sub(1),
            ..area
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .style(Style::default().bg(self.theme.bg_panel))
                    .title(title),
            )
            .highlight_style(self.theme.style_selected())
            .highlight_symbol("▸ ");
        Widget::render(list, list_area, buf);

        if total_items > visible_height {
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y,
                width: 1,
                height: area.height,
            };
            let mut state = ScrollbarState::new(total_items).position(scroll_offset);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut state);
        }
    }

    pub(super) fn render_dependencies_list(&self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focus == Focus::List {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };
        let visible_height = area.height.saturating_sub(2) as usize;
        let selected = self.list_selected.unwrap_or(0);
        let (items_slice, total) =
            if self.dependency_tree.is_empty() || self.filtered_dependency_indices.is_empty() {
                (&[][..], 1usize)
            } else {
                let indices = self.filtered_dependency_indices;
                (indices, indices.len())
            };
        let total = total.max(1);
        let scroll_offset = if visible_height == 0 {
            0
        } else if selected >= visible_height {
            selected.saturating_sub(visible_height - 1)
        } else {
            0
        };

        let items: Vec<ListItem> = if self.dependency_tree.is_empty() {
            let is_selected = selected == 0;
            let style = if is_selected {
                self.theme.style_selected()
            } else {
                Style::default()
            };
            vec![ListItem::new(Line::from(vec![
                Span::styled(
                    if is_selected { "▸ " } else { "  " },
                    self.theme.style_accent(),
                ),
                Span::styled("○ ", self.theme.style_muted()),
                Span::styled("No Cargo project", self.theme.style_dim()),
            ]))
            .style(style)]
        } else if items_slice.is_empty() {
            let is_selected = selected == 0;
            let style = if is_selected {
                self.theme.style_selected()
            } else {
                Style::default()
            };
            vec![ListItem::new(Line::from(vec![
                Span::styled(
                    if is_selected { "▸ " } else { "  " },
                    self.theme.style_accent(),
                ),
                Span::styled("○ ", self.theme.style_muted()),
                Span::styled("No matches for search", self.theme.style_dim()),
            ]))
            .style(style)]
        } else {
            items_slice
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .take(visible_height)
                .map(|(display_idx, &tree_idx)| {
                    let (name, _) = &self.dependency_tree[tree_idx];
                    let is_selected = Some(display_idx) == self.list_selected;
                    let style = if is_selected {
                        self.theme.style_selected()
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            if is_selected { "▸ " } else { "  " },
                            self.theme.style_accent(),
                        ),
                        Span::styled("📦 ", self.theme.style_dim()),
                        Span::styled(name.clone(), self.theme.style_normal()),
                    ]))
                    .style(style)
                })
                .collect()
        };

        let scroll_info = if total > visible_height {
            format!(" [{}/{}]", selected + 1, total)
        } else {
            String::new()
        };
        let title = format!(" Crates ({}){} ", total, scroll_info);
        let list_area = Rect {
            width: area.width.saturating_sub(1),
            ..area
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .style(Style::default().bg(self.theme.bg_panel))
                    .title(title),
            )
            .highlight_style(self.theme.style_selected())
            .highlight_symbol("▸ ");
        Widget::render(list, list_area, buf);

        if total > visible_height {
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y,
                width: 1,
                height: area.height,
            };
            let mut state = ScrollbarState::new(total).position(selected);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut state);
        }
    }

    pub(super) fn render_installed_crates_list(&self, area: Rect, buf: &mut Buffer) {
        let selected = self.list_selected;
        let border_style = if self.focus == Focus::List {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };
        let visible_height = area.height.saturating_sub(2) as usize;

        if let Some(crate_info) = self.selected_installed_crate {
            let total_items = self.installed_crate_items.len();
            let scroll_offset = if let Some(sel) = selected {
                if visible_height == 0 {
                    0
                } else if sel >= visible_height {
                    sel.saturating_sub(visible_height - 1)
                } else {
                    0
                }
            } else {
                0
            };

            let items: Vec<ListItem> = self
                .installed_crate_items
                .iter()
                .enumerate()
                .skip(scroll_offset)
                .take(visible_height)
                .map(|(idx, item)| {
                    let kind_style = match item.kind() {
                        "fn" => self.theme.style_function(),
                        "struct" | "enum" | "type" => self.theme.style_type(),
                        "trait" => self.theme.style_keyword(),
                        "mod" => self.theme.style_accent(),
                        "const" | "static" => self.theme.style_string(),
                        _ => self.theme.style_dim(),
                    };
                    let is_selected = Some(idx) == selected;
                    let base_style = if is_selected {
                        self.theme.style_selected()
                    } else {
                        Style::default()
                    };
                    let prefix = if is_selected { "▸ " } else { "  " };
                    let vis = item
                        .visibility()
                        .map(|v| match v {
                            Visibility::Public => "●",
                            Visibility::Crate => "◐",
                            _ => "○",
                        })
                        .unwrap_or("○");
                    let module_path = item.module_path();
                    let display_name = if module_path.len() > 2 {
                        let last_mod = &module_path[module_path.len() - 1];
                        format!("{}::{}", last_mod, item.name())
                    } else if module_path.len() == 2 {
                        format!("{}::{}", module_path[1], item.name())
                    } else {
                        item.name().to_string()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(prefix, self.theme.style_accent()),
                        Span::styled(vis, self.theme.style_dim()),
                        Span::raw(" "),
                        Span::styled(format!("{:6} ", item.kind()), kind_style),
                        Span::styled(display_name, self.theme.style_normal()),
                    ]))
                    .style(base_style)
                })
                .collect();

            let scroll_info = if total_items > visible_height {
                format!(" [{}/{}]", selected.unwrap_or(0) + 1, total_items)
            } else {
                String::new()
            };
            let title = format!(
                " 📦 {} v{} ({} items){} [Esc] ",
                crate_info.name, crate_info.version, total_items, scroll_info
            );
            let list_area = Rect {
                width: area.width.saturating_sub(1),
                ..area
            };
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_style)
                        .style(Style::default().bg(self.theme.bg_panel))
                        .title(title),
                )
                .highlight_style(self.theme.style_selected());
            Widget::render(list, list_area, buf);

            if total_items > visible_height {
                let scrollbar_area = Rect {
                    x: area.x + area.width.saturating_sub(1),
                    y: area.y,
                    width: 1,
                    height: area.height,
                };
                let mut state = ScrollbarState::new(total_items).position(scroll_offset);
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));
                StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut state);
            }
        }
    }
}
