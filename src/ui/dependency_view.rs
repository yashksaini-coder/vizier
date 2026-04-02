//! Dependency list and docs view (root crate info or crates.io doc for a dependency).

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{
        block::BorderType, Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
};

use crate::analyzer::{CrateInfo, DependencyKind};
use crate::crates_io::CrateDocInfo;
use crate::ui::theme::Theme;

/// View for displaying dependency information (scrollable). No tree chart; list is in the list panel.
pub struct DependencyView<'a> {
    crate_info: Option<&'a CrateInfo>,
    theme: &'a Theme,
    focused: bool,
    scroll_offset: usize,
    show_browser_hint: bool,
}

impl<'a> DependencyView<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            crate_info: None,
            theme,
            focused: false,
            scroll_offset: 0,
            show_browser_hint: false,
        }
    }

    pub fn scroll(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn crate_info(mut self, info: Option<&'a CrateInfo>) -> Self {
        self.crate_info = info;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn show_browser_hint(mut self, show: bool) -> Self {
        self.show_browser_hint = show;
        self
    }

    /// Number of lines this view would render (for scroll clamping).
    pub fn content_height(&self) -> usize {
        match self.crate_info {
            None => 10,
            Some(info) => self.build_crate_info_lines(info).len(),
        }
    }

    fn render_empty(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.style_border())
            .title(" ◇ Crates ");

        let inner = block.inner(area);
        block.render(area, buf);

        let mut help = vec![
            Line::from(""),
            Line::from(Span::styled(
                "This tab lists your project's dependencies. Select the root crate or a dependency to view its info.",
                self.theme.style_dim(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Open a Cargo project (directory with Cargo.toml) to see:",
                self.theme.style_muted(),
            )),
            Line::from(Span::styled("  • List of dependencies (left)", self.theme.style_muted())),
            Line::from(Span::styled("  • Root crate metadata or fetched docs from crates.io (right)", self.theme.style_muted())),
            Line::from(""),
            Line::from(Span::styled(
                "Run: rustlens /path/to/your/crate",
                self.theme.style_accent(),
            )),
        ];
        if self.show_browser_hint {
            help.push(Line::from(""));
            help.push(Line::from(vec![
                Span::styled(" [o] ", self.theme.style_accent()),
                Span::styled("docs.rs  ", self.theme.style_dim()),
                Span::styled(" [c] ", self.theme.style_accent()),
                Span::styled("crates.io", self.theme.style_dim()),
            ]));
        }
        Paragraph::new(help)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }

    fn build_crate_info_lines(&self, info: &CrateInfo) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Header (use owned strings so return type is independent of info lifetime)
        lines.push(Line::from(vec![
            Span::styled(
                info.name.clone(),
                self.theme
                    .style_accent_bold()
                    .add_modifier(Modifier::UNDERLINED),
            ),
            Span::raw(" "),
            Span::styled(format!("v{}", info.version), self.theme.style_dim()),
        ]));
        lines.push(Line::from(""));

        // Description
        if let Some(ref desc) = info.description {
            lines.push(Line::from(Span::styled(
                desc.clone(),
                self.theme.style_normal(),
            )));
            lines.push(Line::from(""));
        }

        // Metadata
        if let Some(ref license) = info.license {
            lines.push(Line::from(vec![
                Span::styled("License: ", self.theme.style_dim()),
                Span::raw(license.clone()),
            ]));
        }

        if !info.authors.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Authors: ", self.theme.style_dim()),
                Span::raw(info.authors.join(", ")),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled("Edition: ", self.theme.style_dim()),
            Span::raw(info.edition.clone()),
        ]));

        if let Some(ref rust_ver) = info.rust_version {
            lines.push(Line::from(vec![
                Span::styled("MSRV: ", self.theme.style_dim()),
                Span::raw(rust_ver.clone()),
            ]));
        }

        // Links
        lines.push(Line::from(""));
        if let Some(ref repo) = info.repository {
            lines.push(Line::from(vec![
                Span::styled("Repository: ", self.theme.style_dim()),
                Span::styled(repo.clone(), self.theme.style_accent()),
            ]));
        }
        if let Some(ref docs) = info.documentation {
            lines.push(Line::from(vec![
                Span::styled("Documentation: ", self.theme.style_dim()),
                Span::styled(docs.clone(), self.theme.style_accent()),
            ]));
        }

        // Features
        if !info.features.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Features ({}):", info.features.len()),
                self.theme.style_dim(),
            )));

            for feature in info.features.iter().take(10) {
                let is_default = info.default_features.contains(feature);
                let marker = if is_default { " [default]" } else { "" };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(feature.clone(), self.theme.style_string()),
                    Span::styled(marker, self.theme.style_muted()),
                ]));
            }

            if info.features.len() > 10 {
                lines.push(Line::from(Span::styled(
                    format!("  ... and {} more", info.features.len() - 10),
                    self.theme.style_muted(),
                )));
            }
        }

        // Dependencies summary
        lines.push(Line::from(""));
        let normal_deps = info
            .dependencies
            .iter()
            .filter(|d| d.kind == DependencyKind::Normal)
            .count();
        let dev_deps = info
            .dependencies
            .iter()
            .filter(|d| d.kind == DependencyKind::Dev)
            .count();
        let build_deps = info
            .dependencies
            .iter()
            .filter(|d| d.kind == DependencyKind::Build)
            .count();

        lines.push(Line::from(Span::styled(
            "Dependencies:",
            self.theme.style_dim(),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{}", normal_deps), self.theme.style_accent()),
            Span::raw(" normal, "),
            Span::styled(format!("{}", dev_deps), self.theme.style_accent()),
            Span::raw(" dev, "),
            Span::styled(format!("{}", build_deps), self.theme.style_accent()),
            Span::raw(" build"),
        ]));

        // List direct dependencies
        lines.push(Line::from(""));
        for dep in info
            .dependencies
            .iter()
            .filter(|d| d.kind == DependencyKind::Normal)
            .take(15)
        {
            let optional = if dep.optional { " (optional)" } else { "" };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(dep.name.clone(), self.theme.style_type()),
                Span::styled(format!(" {}", dep.version), self.theme.style_muted()),
                Span::styled(optional, self.theme.style_dim()),
            ]));
        }

        lines
    }

    fn render_crate_info(&self, info: &CrateInfo, area: Rect, buf: &mut Buffer) {
        let lines = self.build_crate_info_lines(info);
        let total_lines = lines.len();
        let inner = Block::default().inner(area);
        let viewport_height = inner.height as usize;
        let max_scroll = total_lines.saturating_sub(viewport_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.focused {
                self.theme.style_border_focused()
            } else {
                self.theme.style_border()
            })
            .title(" ◇ Crates ");

        let inner = block.inner(area);
        block.render(area, buf);

        let visible_lines: Vec<Line> = lines.into_iter().skip(scroll_offset).collect();
        Paragraph::new(visible_lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);

        if total_lines > inner.height as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state = ScrollbarState::new(total_lines).position(scroll_offset);
            StatefulWidget::render(scrollbar, inner, buf, &mut scrollbar_state);
        }

        if self.show_browser_hint && inner.height > 0 {
            let hint_y = inner.y + inner.height - 1;
            let hint_line = Line::from(vec![
                Span::styled(" [o] ", self.theme.style_accent()),
                Span::styled("docs.rs  ", self.theme.style_dim()),
                Span::styled(" [c] ", self.theme.style_accent()),
                Span::styled("crates.io", self.theme.style_dim()),
            ]);
            Paragraph::new(hint_line).render(
                Rect {
                    y: hint_y,
                    height: 1,
                    ..inner
                },
                buf,
            );
        }
    }
}

impl Widget for DependencyView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.crate_info {
            Some(info) => self.render_crate_info(info, area, buf),
            None => self.render_empty(area, buf),
        }
    }
}

/// View for a dependency's docs from crates.io (scrollable).
pub struct DependencyDocView<'a> {
    doc: &'a CrateDocInfo,
    theme: &'a Theme,
    focused: bool,
    scroll_offset: usize,
    show_browser_hint: bool,
}

impl<'a> DependencyDocView<'a> {
    pub fn new(theme: &'a Theme, doc: &'a CrateDocInfo) -> Self {
        Self {
            doc,
            theme,
            focused: false,
            scroll_offset: 0,
            show_browser_hint: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn scroll(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    pub fn show_browser_hint(mut self, show: bool) -> Self {
        self.show_browser_hint = show;
        self
    }

    fn section_title(&self, title: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled("▸ ", self.theme.style_accent()),
            Span::styled(
                title.to_string(),
                self.theme.style_accent().add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ─────────────", self.theme.style_muted()),
        ])
    }

    fn build_lines(&self) -> Vec<Line<'_>> {
        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled(
                self.doc.name.clone(),
                self.theme
                    .style_accent_bold()
                    .add_modifier(Modifier::UNDERLINED),
            ),
            Span::raw(" "),
            Span::styled(format!("v{}", self.doc.version), self.theme.style_dim()),
        ]));
        lines.push(Line::from(""));

        if let Some(ref d) = self.doc.description {
            lines.push(self.section_title("Description"));
            lines.push(Line::from(""));
            let desc = if d.len() > 600 {
                format!("{}…", &d[..600])
            } else {
                d.clone()
            };
            for line in desc.lines() {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    self.theme.style_normal(),
                )));
            }
            lines.push(Line::from(""));
        }

        let has_links = self.doc.documentation.is_some()
            || self.doc.homepage.is_some()
            || self.doc.repository.is_some();
        if has_links {
            lines.push(self.section_title("Links"));
            lines.push(Line::from(""));
            if let Some(ref u) = self.doc.documentation {
                lines.push(Line::from(vec![
                    Span::styled("  Docs: ", self.theme.style_dim()),
                    Span::styled(u.clone(), self.theme.style_accent()),
                ]));
            }
            if let Some(ref u) = self.doc.homepage {
                lines.push(Line::from(vec![
                    Span::styled("  Home: ", self.theme.style_dim()),
                    Span::styled(u.clone(), self.theme.style_accent()),
                ]));
            }
            if let Some(ref u) = self.doc.repository {
                lines.push(Line::from(vec![
                    Span::styled("  Repo: ", self.theme.style_dim()),
                    Span::styled(u.clone(), self.theme.style_accent()),
                ]));
            }
            lines.push(Line::from(""));
        }

        let is_github_repo = self
            .doc
            .repository
            .as_ref()
            .map(|r| r.contains("github.com"))
            .unwrap_or(false);
        if let Some(ref g) = self.doc.github {
            lines.push(self.section_title("GitHub"));
            lines.push(Line::from(""));
            if let Some(n) = g.stars {
                lines.push(Line::from(vec![
                    Span::styled("  Stars:  ", self.theme.style_dim()),
                    Span::styled(format!("{}", n), self.theme.style_accent()),
                ]));
            }
            if let Some(n) = g.forks {
                lines.push(Line::from(vec![
                    Span::styled("  Forks:  ", self.theme.style_dim()),
                    Span::styled(format!("{}", n), self.theme.style_accent()),
                ]));
            }
            if let Some(ref lang) = g.language {
                lines.push(Line::from(vec![
                    Span::styled("  Lang:   ", self.theme.style_dim()),
                    Span::styled(lang.clone(), self.theme.style_type()),
                ]));
            }
            if let Some(ref updated) = g.updated_at {
                let short =
                    if updated.len() >= 10 && updated.as_bytes().get(10).copied() == Some(b'T') {
                        updated[..10].to_string()
                    } else {
                        updated.clone()
                    };
                lines.push(Line::from(vec![
                    Span::styled("  Updated:", self.theme.style_dim()),
                    Span::styled(format!(" {}", short), self.theme.style_muted()),
                ]));
            }
            if let Some(n) = g.open_issues_count {
                if n > 0 {
                    lines.push(Line::from(vec![
                        Span::styled("  Issues: ", self.theme.style_dim()),
                        Span::styled(format!("{} open", n), self.theme.style_warning()),
                    ]));
                }
            }
            lines.push(Line::from(""));
        } else if is_github_repo {
            lines.push(self.section_title("GitHub"));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("  ", self.theme.style_dim()),
                Span::styled(
                    "Unavailable (rate limit or set GITHUB_TOKEN for more)",
                    self.theme.style_muted(),
                ),
            ]));
            lines.push(Line::from(""));
        }
        lines
    }
}

impl Widget for DependencyDocView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = self.build_lines();
        let total_lines = lines.len();
        let inner = Block::default().inner(area);
        let viewport_height = inner.height as usize;
        let max_scroll = total_lines.saturating_sub(viewport_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.focused {
                self.theme.style_border_focused()
            } else {
                self.theme.style_border()
            })
            .title(format!(" ◇ {} (docs) ", self.doc.name));

        let inner = block.inner(area);
        block.render(area, buf);

        let visible: Vec<Line> = lines.into_iter().skip(scroll_offset).collect();
        Paragraph::new(visible)
            .wrap(Wrap { trim: false })
            .render(inner, buf);

        if total_lines > inner.height as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut state = ScrollbarState::new(total_lines).position(scroll_offset);
            StatefulWidget::render(scrollbar, inner, buf, &mut state);
        }

        if self.show_browser_hint && inner.height > 0 {
            let hint_y = inner.y + inner.height - 1;
            let hint_line = Line::from(vec![
                Span::styled(" [o] ", self.theme.style_accent()),
                Span::styled("docs.rs  ", self.theme.style_dim()),
                Span::styled(" [c] ", self.theme.style_accent()),
                Span::styled("crates.io", self.theme.style_dim()),
            ]);
            Paragraph::new(hint_line).render(
                Rect {
                    y: hint_y,
                    height: 1,
                    ..inner
                },
                buf,
            );
        }
    }
}

/// Render "Loading documentation for X..." in the inspector area.
pub fn render_doc_loading(theme: &Theme, area: Rect, buf: &mut Buffer, crate_name: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.style_border())
        .title(format!(" ◇ {} ", crate_name));

    let inner = block.inner(area);
    block.render(area, buf);
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Loading documentation for {} from crates.io…", crate_name),
            theme.style_dim(),
        )),
    ];
    Paragraph::new(text).render(inner, buf);
    if inner.height > 0 {
        let hint_y = inner.y + inner.height - 1;
        let hint_line = Line::from(vec![
            Span::styled(" [o] ", theme.style_accent()),
            Span::styled("docs.rs  ", theme.style_dim()),
            Span::styled(" [c] ", theme.style_accent()),
            Span::styled("crates.io", theme.style_dim()),
        ]);
        Paragraph::new(hint_line).render(
            Rect {
                y: hint_y,
                height: 1,
                ..inner
            },
            buf,
        );
    }
}

/// Render "Failed to load docs for X" in the inspector area.
pub fn render_doc_failed(theme: &Theme, area: Rect, buf: &mut Buffer, crate_name: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.style_border())
        .title(format!(" ◇ {} ", crate_name));

    let inner = block.inner(area);
    block.render(area, buf);
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Could not load documentation for {}.", crate_name),
            theme.style_muted(),
        )),
        Line::from(Span::styled(
            "Check network or try again later.",
            theme.style_dim(),
        )),
    ];
    Paragraph::new(text).render(inner, buf);
    if inner.height > 0 {
        let hint_y = inner.y + inner.height - 1;
        let hint_line = Line::from(vec![
            Span::styled(" [o] ", theme.style_accent()),
            Span::styled("docs.rs  ", theme.style_dim()),
            Span::styled(" [c] ", theme.style_accent()),
            Span::styled("crates.io", theme.style_dim()),
        ]);
        Paragraph::new(hint_line).render(
            Rect {
                y: hint_y,
                height: 1,
                ..inner
            },
            buf,
        );
    }
}
