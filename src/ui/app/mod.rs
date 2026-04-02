//! Main Vizier TUI application — composed from blocks (header, list, status, overlays, right_panel).

mod header;
mod layout;
mod list;
mod overlays;
mod right_panel;
mod status;
mod types;

pub use layout::tabs_rect_for_area;
pub use types::{Focus, Tab};

use crate::analyzer::AnalyzedItem;
use crate::analyzer::CrateInfo;
use crate::crates_io::CrateDocInfo;
use crate::ui::animation::AnimationState;
use crate::ui::components::TabBar;
use crate::ui::search::{CompletionCandidate, SearchBar, SearchCompletion};
use crate::ui::theme::Theme;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{block::BorderType, Block, Borders, Widget},
};

/// Main Vizier UI widget — data and builder; rendering is delegated to block modules.
pub struct VizierUi<'a> {
    // Data
    pub(super) items: &'a [AnalyzedItem],
    pub(super) all_items_impl_lookup: Option<&'a [AnalyzedItem]>,
    pub(super) filtered_items: &'a [&'a AnalyzedItem],
    pub(super) candidates: &'a [CompletionCandidate],
    pub(super) crate_info: Option<&'a CrateInfo>,
    pub(super) dependency_tree: &'a [(String, usize)],
    pub(super) filtered_dependency_indices: &'a [usize],
    pub(super) crate_doc: Option<&'a CrateDocInfo>,
    pub(super) crate_doc_loading: bool,
    pub(super) crate_doc_failed: bool,
    pub(super) selected_installed_crate: Option<&'a crate::analyzer::InstalledCrate>,
    pub(super) installed_crate_items: &'a [&'a AnalyzedItem],
    pub(super) target_size_bytes: Option<u64>,
    // UI state
    pub(super) search_input: &'a str,
    pub(super) current_tab: Tab,
    pub(super) focus: Focus,
    pub(super) list_selected: Option<usize>,
    pub(super) selected_item: Option<&'a AnalyzedItem>,
    pub(super) completion_selected: usize,
    pub(super) show_completion: bool,
    pub(super) show_help: bool,
    pub(super) show_settings: bool,
    pub(super) status_message: &'a str,
    pub(super) inspector_scroll: usize,
    pub(super) animation: Option<&'a AnimationState>,
    pub(super) theme: &'a Theme,
    // Copilot in-TUI chat
    pub(super) show_copilot_chat: bool,
    pub(super) copilot_chat_messages: &'a [(String, String)],
    pub(super) copilot_chat_input: &'a str,
    pub(super) copilot_chat_loading: bool,
    pub(super) copilot_chat_scroll: usize,
}

impl<'a> VizierUi<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            items: &[],
            all_items_impl_lookup: None,
            filtered_items: &[],
            candidates: &[],
            crate_info: None,
            dependency_tree: &[],
            filtered_dependency_indices: &[],
            crate_doc: None,
            crate_doc_loading: false,
            crate_doc_failed: false,
            selected_installed_crate: None,
            installed_crate_items: &[],
            target_size_bytes: None,
            search_input: "",
            current_tab: Tab::default(),
            focus: Focus::default(),
            list_selected: None,
            selected_item: None,
            completion_selected: 0,
            show_completion: false,
            show_help: false,
            show_settings: false,
            status_message: "",
            inspector_scroll: 0,
            animation: None,
            theme,
            show_copilot_chat: false,
            copilot_chat_messages: &[],
            copilot_chat_input: "",
            copilot_chat_loading: false,
            copilot_chat_scroll: 0,
        }
    }

    #[must_use]
    pub fn items(mut self, items: &'a [AnalyzedItem]) -> Self {
        self.items = items;
        self
    }
    #[must_use]
    pub fn all_items_impl_lookup(mut self, items: Option<&'a [AnalyzedItem]>) -> Self {
        self.all_items_impl_lookup = items;
        self
    }
    #[must_use]
    pub fn filtered_items(mut self, items: &'a [&'a AnalyzedItem]) -> Self {
        self.filtered_items = items;
        self
    }
    #[must_use]
    pub fn selected_installed_crate(
        mut self,
        crate_info: Option<&'a crate::analyzer::InstalledCrate>,
    ) -> Self {
        self.selected_installed_crate = crate_info;
        self
    }
    #[must_use]
    pub fn installed_crate_items(mut self, items: &'a [&'a AnalyzedItem]) -> Self {
        self.installed_crate_items = items;
        self
    }
    #[must_use]
    pub fn target_size_bytes(mut self, bytes: Option<u64>) -> Self {
        self.target_size_bytes = bytes;
        self
    }
    #[must_use]
    pub fn list_selected(mut self, selected: Option<usize>) -> Self {
        self.list_selected = selected;
        self
    }
    #[must_use]
    pub fn candidates(mut self, candidates: &'a [CompletionCandidate]) -> Self {
        self.candidates = candidates;
        self
    }
    #[must_use]
    pub fn crate_info(mut self, info: Option<&'a CrateInfo>) -> Self {
        self.crate_info = info;
        self
    }
    #[must_use]
    pub fn dependency_tree(mut self, tree: &'a [(String, usize)]) -> Self {
        self.dependency_tree = tree;
        self
    }
    #[must_use]
    pub fn filtered_dependency_indices(mut self, indices: &'a [usize]) -> Self {
        self.filtered_dependency_indices = indices;
        self
    }
    #[must_use]
    pub fn crate_doc(mut self, doc: Option<&'a CrateDocInfo>) -> Self {
        self.crate_doc = doc;
        self
    }
    #[must_use]
    pub fn crate_doc_loading(mut self, loading: bool) -> Self {
        self.crate_doc_loading = loading;
        self
    }
    #[must_use]
    pub fn crate_doc_failed(mut self, failed: bool) -> Self {
        self.crate_doc_failed = failed;
        self
    }
    #[must_use]
    pub fn search_input(mut self, input: &'a str) -> Self {
        self.search_input = input;
        self
    }
    #[must_use]
    pub fn current_tab(mut self, tab: Tab) -> Self {
        self.current_tab = tab;
        self
    }
    #[must_use]
    pub fn focus(mut self, focus: Focus) -> Self {
        self.focus = focus;
        self
    }
    #[must_use]
    pub fn selected_item(mut self, item: Option<&'a AnalyzedItem>) -> Self {
        self.selected_item = item;
        self
    }
    #[must_use]
    pub fn completion_selected(mut self, index: usize) -> Self {
        self.completion_selected = index;
        self
    }
    #[must_use]
    pub fn show_completion(mut self, show: bool) -> Self {
        self.show_completion = show;
        self
    }
    #[must_use]
    pub fn show_help(mut self, show: bool) -> Self {
        self.show_help = show;
        self
    }
    #[must_use]
    pub fn show_settings(mut self, show: bool) -> Self {
        self.show_settings = show;
        self
    }
    #[must_use]
    pub fn status_message(mut self, msg: &'a str) -> Self {
        self.status_message = msg;
        self
    }
    #[must_use]
    pub fn inspector_scroll(mut self, scroll: usize) -> Self {
        self.inspector_scroll = scroll;
        self
    }
    #[must_use]
    pub fn animation_state(mut self, animation: &'a AnimationState) -> Self {
        self.animation = Some(animation);
        self
    }
    #[must_use]
    pub fn show_copilot_chat(mut self, show: bool) -> Self {
        self.show_copilot_chat = show;
        self
    }
    #[must_use]
    pub fn copilot_chat_messages(mut self, messages: &'a [(String, String)]) -> Self {
        self.copilot_chat_messages = messages;
        self
    }
    #[must_use]
    pub fn copilot_chat_input(mut self, input: &'a str) -> Self {
        self.copilot_chat_input = input;
        self
    }
    #[must_use]
    pub fn copilot_chat_loading(mut self, loading: bool) -> Self {
        self.copilot_chat_loading = loading;
        self
    }
    #[must_use]
    pub fn copilot_chat_scroll(mut self, scroll: usize) -> Self {
        self.copilot_chat_scroll = scroll;
        self
    }

    fn render_search(&self, area: Rect, buf: &mut Buffer) {
        let placeholder = match self.current_tab {
            Tab::Types => "Search types... (struct, enum, type)",
            Tab::Functions => "Search functions...",
            Tab::Modules => "Search modules...",
            Tab::Crates => {
                if self.selected_installed_crate.is_some() {
                    "Filter items... (e.g., de::Deserialize)"
                } else {
                    "Search crates... (filter by name)"
                }
            }
        };
        let search = SearchBar::new(self.search_input, self.theme)
            .focused(self.focus == Focus::Search)
            .placeholder(placeholder);
        search.render(area, buf);
    }

    fn render_completion(&self, search_area: Rect, buf: &mut Buffer) {
        if !self.show_completion || self.candidates.is_empty() {
            return;
        }
        let max_height = 12.min(self.candidates.len() as u16 + 2);
        let dropdown_area = Rect {
            x: search_area.x + 2,
            y: search_area.y + search_area.height,
            width: search_area.width.saturating_sub(4).min(60),
            height: max_height,
        };
        let completion = SearchCompletion::new(self.candidates, self.theme)
            .selected(self.completion_selected)
            .filter(self.search_input)
            .max_visible(10);
        completion.render(dropdown_area, buf);
    }

    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles: Vec<&str> = Tab::all().iter().map(|t| t.title()).collect();
        let tab_bar = TabBar::new(titles, self.theme)
            .select(self.current_tab.index())
            .focused(self.focus == Focus::Inspector);
        tab_bar.render(area, buf);
    }
}

impl Widget for VizierUi<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use layout::{BODY_MARGIN, HEADER_HEIGHT, STATUS_HEIGHT};

        let outer = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.theme.style_border_glow())
            .style(Style::default().bg(self.theme.bg));
        let content_area = outer.inner(area);
        outer.render(area, buf);

        let padded = Rect {
            x: content_area.x + BODY_MARGIN,
            y: content_area.y + BODY_MARGIN,
            width: content_area.width.saturating_sub(2 * BODY_MARGIN),
            height: content_area.height.saturating_sub(2 * BODY_MARGIN),
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(HEADER_HEIGHT),
                Constraint::Min(12),
                Constraint::Length(STATUS_HEIGHT),
            ])
            .split(padded);

        self.render_header(chunks[0], buf);

        let body = chunks[1];
        let left_div_right = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, 3),
                Constraint::Length(1),
                Constraint::Ratio(2, 3),
            ])
            .split(body);
        let left_column = left_div_right[0];
        let div_rect = left_div_right[1];
        let right_column = left_div_right[2];

        let left_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(6)])
            .split(left_column);
        let search_rect = left_split[0];
        let list_rect = left_split[1];

        let right_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(6)])
            .split(right_column);
        let tabs_rect = right_split[0];
        let right_content = right_split[1];

        let (inspector_rect, chat_rect) = if self.show_copilot_chat {
            let horz = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(right_content);
            (horz[0], horz[1])
        } else {
            (right_content, right_content) // chat_rect unused
        };

        self.render_search(search_rect, buf);
        self.render_list(list_rect, buf);
        self.render_vertical_divider(div_rect, buf);
        self.render_tabs(tabs_rect, buf);
        self.render_inspector(inspector_rect, buf);
        if self.show_copilot_chat {
            self.render_copilot_chat(chat_rect, buf);
        }
        self.render_status(chunks[2], buf);
        self.render_completion(search_rect, buf);
        self.render_settings_overlay(area, buf);
        self.render_help_overlay(area, buf);
    }
}
