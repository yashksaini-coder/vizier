//! Search bar and completion widgets

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Widget},
};

use crate::ui::theme::Theme;

/// A completion candidate
#[derive(Debug, Clone)]
pub struct CompletionCandidate {
    pub primary: String,
    pub secondary: Option<String>,
    pub kind: CandidateKind,
    pub score: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Type,
    Const,
    Crate,
    Other,
}

impl CandidateKind {
    pub fn icon(&self) -> &'static str {
        match self {
            CandidateKind::Function => "fn",
            CandidateKind::Struct => "st",
            CandidateKind::Enum => "en",
            CandidateKind::Trait => "tr",
            CandidateKind::Module => "md",
            CandidateKind::Type => "ty",
            CandidateKind::Const => "ct",
            CandidateKind::Crate => "cr",
            CandidateKind::Other => "  ",
        }
    }

    pub fn color(&self, theme: &Theme) -> Color {
        match self {
            CandidateKind::Function => theme.function,
            CandidateKind::Struct | CandidateKind::Enum | CandidateKind::Type => theme.type_,
            CandidateKind::Trait => theme.keyword,
            CandidateKind::Module | CandidateKind::Crate => theme.accent,
            CandidateKind::Const => theme.number,
            CandidateKind::Other => theme.fg_dim,
        }
    }
}

/// Search bar widget
pub struct SearchBar<'a> {
    input: &'a str,
    cursor_pos: usize,
    theme: &'a Theme,
    focused: bool,
    placeholder: &'a str,
}

impl<'a> SearchBar<'a> {
    pub fn new(input: &'a str, theme: &'a Theme) -> Self {
        Self {
            input,
            cursor_pos: input.len(),
            theme,
            focused: true,
            placeholder: "Search...",
        }
    }

    pub fn cursor_position(mut self, pos: usize) -> Self {
        self.cursor_pos = pos;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }
}

impl Widget for SearchBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.focused {
            self.theme.style_border_focused()
        } else {
            self.theme.style_border()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .style(Style::default().bg(self.theme.bg_panel))
            .title(" Search ");

        let inner = block.inner(area);
        block.render(area, buf);

        // Render prompt
        let prompt = Span::styled("❯ ", self.theme.style_accent_bold());

        let (input_text, input_style) = if self.input.is_empty() {
            (self.placeholder, self.theme.style_dim())
        } else {
            (self.input, self.theme.style_normal())
        };

        let cursor = if self.focused {
            Span::styled(
                "▏",
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::SLOW_BLINK),
            )
        } else {
            Span::raw("")
        };

        let line = Line::from(vec![
            prompt,
            Span::styled(input_text.to_string(), input_style),
            cursor,
        ]);

        let paragraph = Paragraph::new(line);
        paragraph.render(inner, buf);
    }
}

/// Search completion dropdown
pub struct SearchCompletion<'a> {
    candidates: &'a [CompletionCandidate],
    selected: usize,
    filter: &'a str,
    theme: &'a Theme,
    max_visible: usize,
}

impl<'a> SearchCompletion<'a> {
    pub fn new(candidates: &'a [CompletionCandidate], theme: &'a Theme) -> Self {
        Self {
            candidates,
            selected: 0,
            filter: "",
            theme,
            max_visible: 10,
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn filter(mut self, filter: &'a str) -> Self {
        self.filter = filter;
        self
    }

    pub fn max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }
}

impl Widget for SearchCompletion<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.candidates.is_empty() {
            return;
        }

        // Clear the area first
        Clear.render(area, buf);

        let visible_count = self.candidates.len().min(self.max_visible);

        let items: Vec<ListItem> = self
            .candidates
            .iter()
            .take(visible_count)
            .enumerate()
            .map(|(i, candidate)| {
                let kind_span = Span::styled(
                    format!("{} ", candidate.kind.icon()),
                    Style::default().fg(candidate.kind.color(self.theme)),
                );

                let name = highlight_fuzzy(&candidate.primary, self.filter, self.theme);

                let secondary = candidate
                    .secondary
                    .as_ref()
                    .map(|s| Span::styled(format!(" {}", s), self.theme.style_muted()))
                    .unwrap_or_else(|| Span::raw(""));

                let mut spans = vec![kind_span];
                spans.extend(name);
                spans.push(secondary);

                let line = Line::from(spans);
                let style = if i == self.selected {
                    self.theme.style_selected()
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.style_border_focused())
                .style(Style::default().bg(self.theme.bg_panel)),
        );

        list.render(area, buf);
    }
}

/// Highlight matching characters in fuzzy search
fn highlight_fuzzy<'a>(text: &'a str, query: &str, theme: &Theme) -> Vec<Span<'a>> {
    if query.is_empty() {
        return vec![Span::raw(text.to_string())];
    }

    let matcher = SkimMatcherV2::default();
    if let Some((_, indices)) = matcher.fuzzy_indices(text, query) {
        let mut spans = Vec::new();
        let mut last_end = 0;

        for &idx in &indices {
            if idx > last_end {
                spans.push(Span::raw(text[last_end..idx].to_string()));
            }
            spans.push(Span::styled(
                text[idx..idx + 1].to_string(),
                Style::default()
                    .bg(theme.accent)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ));
            last_end = idx + 1;
        }

        if last_end < text.len() {
            spans.push(Span::raw(text[last_end..].to_string()));
        }

        spans
    } else {
        vec![Span::raw(text.to_string())]
    }
}

/// Filter and sort candidates based on fuzzy matching
pub fn filter_candidates(
    candidates: &[CompletionCandidate],
    query: &str,
) -> Vec<CompletionCandidate> {
    if query.is_empty() {
        return candidates.to_vec();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<_> = candidates
        .iter()
        .filter_map(|c| {
            matcher.fuzzy_match(&c.primary, query).map(|score| {
                let mut candidate = c.clone();
                candidate.score = score;
                candidate
            })
        })
        .collect();

    scored.sort_by_key(|b| std::cmp::Reverse(b.score));
    scored
}
