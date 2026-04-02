//! Application state management

use crate::analyzer::{
    AnalyzedItem, CrateInfo, CrateRegistry, DependencyAnalyzer, InstalledCrate, RustAnalyzer,
};
use crate::config::Settings;
use crate::crates_io::CrateDocInfo;
use crate::error::Result;
use crate::ui::theme::Theme;
use crate::ui::{filter_candidates, CandidateKind, CompletionCandidate, Focus, Tab};
use crate::utils::dir_size;

use ratatui::widgets::ListState;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;

/// Main application state
pub struct App {
    // Analysis data
    pub items: Vec<AnalyzedItem>,
    pub filtered_items: Vec<usize>,
    pub crate_info: Option<CrateInfo>,
    pub dependency_tree: Vec<(String, usize)>,
    /// Indices into dependency_tree for Crates tab list (filtered by search). Empty = not computed.
    pub filtered_dependency_indices: Vec<usize>,

    // Installed crates registry
    pub crate_registry: CrateRegistry,
    pub installed_crates_list: Vec<String>,
    pub selected_installed_crate: Option<InstalledCrate>,
    pub installed_crate_items: Vec<AnalyzedItem>,
    pub installed_crate_filtered: Vec<usize>,

    // UI state
    pub search_input: String,
    pub current_tab: Tab,
    pub focus: Focus,
    pub list_state: ListState,
    pub completion_selected: usize,
    pub show_completion: bool,
    pub show_help: bool,
    pub show_settings: bool,
    pub status_message: String,

    // Search
    pub candidates: Vec<CompletionCandidate>,
    pub filtered_candidates: Vec<CompletionCandidate>,

    // Config
    pub settings: Settings,
    pub theme: Theme,

    // Control
    pub should_quit: bool,
    pub project_path: Option<PathBuf>,

    // In-TUI Copilot chat (panel to the right of inspector)
    pub copilot_chat_open: bool,
    /// (role, content) with role "user" or "assistant"
    pub copilot_chat_messages: Vec<(String, String)>,
    pub copilot_chat_input: String,
    pub copilot_chat_loading: bool,
    pub copilot_chat_scroll: usize,
    /// Size of target/ directory in bytes (build artifacts), if computed.
    pub target_size_bytes: Option<u64>,

    // Dependency tab: fetched docs from crates.io (background thread, bounded cache)
    pub crate_docs_cache: HashMap<String, CrateDocInfo>,
    pub crate_docs_loading: Option<String>,
    pub crate_docs_failed: HashSet<String>,
    crate_docs_tx: mpsc::Sender<(String, Option<CrateDocInfo>)>,
    pub crate_docs_rx: mpsc::Receiver<(String, Option<CrateDocInfo>)>,

    pub copilot_tx: mpsc::Sender<String>,
    pub copilot_rx: mpsc::Receiver<String>,
}

/// Max crates to keep in docs cache (memory bound).
const CRATE_DOCS_CACHE_MAX: usize = 50;

impl App {
    pub fn new() -> Self {
        let (crate_docs_tx, crate_docs_rx) = mpsc::channel();
        let (copilot_tx, copilot_rx) = mpsc::channel();
        Self {
            items: Vec::new(),
            filtered_items: Vec::new(),
            crate_info: None,
            dependency_tree: Vec::new(),
            filtered_dependency_indices: Vec::new(),
            crate_registry: CrateRegistry::new(),
            installed_crates_list: Vec::new(),
            selected_installed_crate: None,
            installed_crate_items: Vec::new(),
            installed_crate_filtered: Vec::new(),
            search_input: String::new(),
            current_tab: Tab::default(),
            focus: Focus::default(),
            list_state: ListState::default(),
            completion_selected: 0,
            show_completion: false,
            show_help: false,
            show_settings: false,
            status_message: String::from("Ready"),
            candidates: Vec::new(),
            filtered_candidates: Vec::new(),
            settings: Settings::default(),
            theme: Theme::default(),
            should_quit: false,
            project_path: None,
            target_size_bytes: None,
            copilot_chat_open: false,
            copilot_chat_messages: Vec::new(),
            copilot_chat_input: String::new(),
            copilot_chat_loading: false,
            copilot_chat_scroll: 0,
            crate_docs_cache: HashMap::new(),
            crate_docs_loading: None,
            crate_docs_failed: HashSet::new(),
            crate_docs_tx,
            crate_docs_rx,
            copilot_tx,
            copilot_rx,
        }
    }

    /// Load settings from config file
    pub fn load_settings(&mut self) -> Result<()> {
        self.settings = Settings::load()?;
        self.theme = Theme::from_name(&self.settings.ui.theme);
        Ok(())
    }

    /// Cycle to the next theme and persist to config
    pub fn cycle_theme(&mut self) {
        let next = self.theme.kind().next();
        self.theme = Theme::from_kind(next);
        self.settings.ui.theme = next.name().to_string();
        self.status_message = format!("Theme: {}", next.display_name());
        let _ = self.settings.save();
    }

    pub fn toggle_settings(&mut self) {
        self.show_settings = !self.show_settings;
    }

    /// Analyze a Rust project
    pub fn analyze_project(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(crate::error::RustlensError::Other(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }
        self.project_path = Some(path.to_path_buf());
        self.status_message = format!("Analyzing {}...", path.display());

        // Try to analyze Cargo.toml for dependencies
        let manifest_path = path.join("Cargo.toml");
        if manifest_path.exists() {
            match DependencyAnalyzer::from_manifest(&manifest_path) {
                Ok(analyzer) => {
                    if let Some(root) = analyzer.root_package() {
                        self.dependency_tree = analyzer.dependency_tree(&root.name);
                        self.crate_info = Some(root);
                    }
                }
                Err(e) => {
                    self.status_message = format!("Cargo analysis failed: {e}");
                }
            }
        }

        // Analyze Rust source files
        let analyzer = RustAnalyzer::new().with_private(self.settings.analyzer.include_private);

        let src_path = path.join("src");
        if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            self.items = analyzer.analyze_file(path)?;
        } else if src_path.exists() {
            self.analyze_directory(&analyzer, &src_path)?;
        } else if path.is_dir() {
            // No src/ (e.g. flat layout): analyze directory for .rs files
            self.analyze_directory(&analyzer, &path.to_path_buf())?;
        }

        self.update_candidates();
        self.filter_items();
        self.status_message = if self.items.is_empty() {
            format!("No Rust files found in {}", path.display())
        } else {
            format!("Found {} items", self.items.len())
        };

        // Best-effort target/ directory size (non-blocking, ignore errors)
        let target_dir = path.join("target");
        if target_dir.is_dir() {
            self.target_size_bytes = dir_size(&target_dir);
        } else {
            self.target_size_bytes = None;
        }

        Ok(())
    }

    fn analyze_directory(&mut self, analyzer: &RustAnalyzer, dir: &PathBuf) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.analyze_directory(analyzer, &path)?;
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                match analyzer.analyze_file(&path) {
                    Ok(items) => self.items.extend(items),
                    Err(e) => {
                        // Log but continue
                        eprintln!("Warning: Failed to analyze {}: {}", path.display(), e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Update completion candidates from analyzed items
    pub fn update_candidates(&mut self) {
        self.candidates = self
            .items
            .iter()
            .map(|item| {
                let kind = match item {
                    AnalyzedItem::Function(_) => CandidateKind::Function,
                    AnalyzedItem::Struct(_) => CandidateKind::Struct,
                    AnalyzedItem::Enum(_) => CandidateKind::Enum,
                    AnalyzedItem::Trait(_) => CandidateKind::Trait,
                    AnalyzedItem::Module(_) => CandidateKind::Module,
                    AnalyzedItem::TypeAlias(_) => CandidateKind::Type,
                    AnalyzedItem::Const(_) | AnalyzedItem::Static(_) => CandidateKind::Const,
                    _ => CandidateKind::Other,
                };

                let secondary = item.documentation().map(|d| {
                    let first_line = d.lines().next().unwrap_or("");
                    if first_line.len() > 40 {
                        format!("{}...", &first_line[..37])
                    } else {
                        first_line.to_string()
                    }
                });

                CompletionCandidate {
                    primary: item.name().to_string(),
                    secondary,
                    kind,
                    score: 0,
                }
            })
            .collect();

        self.filtered_candidates = self.candidates.clone();
    }

    /// Filter items based on search input and current tab
    pub fn filter_items(&mut self) {
        let query = self.search_input.to_lowercase();

        // Crates tab: when inside a crate, filter its items
        if self.current_tab == Tab::Crates && self.selected_installed_crate.is_some() {
            self.filter_installed_crates();
            return;
        }

        // Crates tab (top level): filter crate list by name, keep alphabetical order
        if self.current_tab == Tab::Crates {
            let mut indices: Vec<usize> = self
                .dependency_tree
                .iter()
                .enumerate()
                .filter(|(_, (name, _))| {
                    query.is_empty()
                        || name.to_lowercase().contains(&query)
                        || name.to_lowercase().replace('-', "_").contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
            indices.sort_by(|&a, &b| {
                self.dependency_tree[a]
                    .0
                    .to_lowercase()
                    .cmp(&self.dependency_tree[b].0.to_lowercase())
            });
            self.filtered_dependency_indices = indices;
            if self
                .list_state
                .selected()
                .is_some_and(|s| s >= self.filtered_dependency_indices.len())
            {
                self.list_state.select(Some(0));
            }
            self.filtered_candidates = Vec::new();
            self.completion_selected = 0;
            return;
        }

        self.filtered_items = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // Filter by tab
                let tab_match = match self.current_tab {
                    Tab::Types => matches!(
                        item,
                        AnalyzedItem::Struct(_)
                            | AnalyzedItem::Enum(_)
                            | AnalyzedItem::TypeAlias(_)
                    ),
                    Tab::Functions => matches!(item, AnalyzedItem::Function(_)),
                    Tab::Modules => matches!(item, AnalyzedItem::Module(_)),
                    Tab::Crates => true, // Handled by crate list or filter_installed_crates
                };

                // Filter by search
                let search_match = query.is_empty() || item.name().to_lowercase().contains(&query);

                tab_match && search_match
            })
            .map(|(i, _)| i)
            .collect();

        // Reset selection if out of bounds
        if self
            .list_state
            .selected()
            .is_some_and(|s| s >= self.filtered_items.len())
        {
            self.list_state.select(Some(0));
        }

        // Update completion candidates; only show candidates relevant to the active tab
        let matched = filter_candidates(&self.candidates, &self.search_input);
        self.filtered_candidates = match self.current_tab {
            Tab::Types => matched
                .into_iter()
                .filter(|c| {
                    matches!(
                        c.kind,
                        CandidateKind::Struct | CandidateKind::Enum | CandidateKind::Type
                    )
                })
                .collect(),
            Tab::Functions => matched
                .into_iter()
                .filter(|c| c.kind == CandidateKind::Function)
                .collect(),
            Tab::Modules => matched
                .into_iter()
                .filter(|c| c.kind == CandidateKind::Module)
                .collect(),
            Tab::Crates => Vec::new(),
        };
        self.completion_selected = 0;
    }

    /// Scan for installed crates
    pub fn scan_installed_crates(&mut self) -> Result<()> {
        self.status_message = "Scanning installed crates...".to_string();
        self.crate_registry.scan()?;
        self.installed_crates_list = self
            .crate_registry
            .crate_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        self.status_message = format!(
            "Found {} installed crates",
            self.installed_crates_list.len()
        );
        Ok(())
    }

    /// Filter installed crates based on search
    /// Supports qualified path search like "serde::de::Deserialize"
    fn filter_installed_crates(&mut self) {
        let query = self.search_input.to_lowercase();

        if self.selected_installed_crate.is_some() {
            // Filter items within selected crate by qualified path or name
            self.installed_crate_filtered = self
                .installed_crate_items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    if query.is_empty() {
                        return true;
                    }
                    // Check if query contains :: for path matching
                    if query.contains("::") {
                        // Match against qualified path
                        item.qualified_name().to_lowercase().contains(&query) ||
                        // Or match partial module path
                        item.module_path().iter()
                            .any(|p| p.to_lowercase().contains(&query.replace("::", "")))
                    } else {
                        // Simple name match
                        item.name().to_lowercase().contains(&query)
                    }
                })
                .map(|(i, _)| i)
                .collect();
        }

        // Reset selection if out of bounds
        if self
            .list_state
            .selected()
            .is_some_and(|s| s >= self.get_current_list_len())
        {
            self.list_state.select(Some(0));
        }
    }

    /// Parse qualified path and navigate to crate + filter items
    /// E.g., "serde::de::Deserialize" -> select serde crate, filter for de::Deserialize
    pub fn search_qualified_path(&mut self) -> bool {
        let query = self.search_input.clone();
        let query = query.trim();

        // Check for qualified path (contains ::)
        if !query.contains("::") {
            return false;
        }

        let parts: Vec<&str> = query.split("::").collect();
        if parts.is_empty() {
            return false;
        }

        let crate_name = parts[0].to_string();

        // Check if crate exists
        let crate_exists = self.installed_crates_list.iter().any(|name| {
            name.to_lowercase() == crate_name.to_lowercase()
                || name.to_lowercase().replace('-', "_") == crate_name.to_lowercase()
        });

        if !crate_exists {
            self.status_message = format!("Crate '{}' not found", crate_name);
            return false;
        }

        // Find actual crate name (might have hyphens)
        let actual_name = self
            .installed_crates_list
            .iter()
            .find(|name| {
                name.to_lowercase() == crate_name.to_lowercase()
                    || name.to_lowercase().replace('-', "_") == crate_name.to_lowercase()
            })
            .cloned();

        // Select the crate if not already selected
        let already_selected = self
            .selected_installed_crate
            .as_ref()
            .map(|c| c.name.to_lowercase() == crate_name.to_lowercase())
            .unwrap_or(false);

        if !already_selected {
            if let Some(name) = actual_name {
                let _ = self.select_installed_crate(&name);
            }
        }

        // Set search to remaining path for filtering
        if parts.len() > 1 {
            // Keep the module path part for filtering
            self.search_input = parts[1..].join("::");
            self.filter_installed_crates();
        }

        true
    }

    /// Select an installed crate and analyze it
    pub fn select_installed_crate(&mut self, name: &str) -> Result<()> {
        if let Some(crate_info) = self.crate_registry.latest(name) {
            self.selected_installed_crate = Some(crate_info.clone());
            self.status_message = format!("Analyzing {}...", name);

            match self.crate_registry.analyze_crate(name, None) {
                Ok(items) => {
                    self.installed_crate_items = items;
                    self.installed_crate_filtered = (0..self.installed_crate_items.len()).collect();
                    self.status_message =
                        format!("{}: {} items", name, self.installed_crate_items.len());
                }
                Err(e) => {
                    self.status_message = format!("Analysis failed: {e}");
                }
            }
        }
        Ok(())
    }

    /// Clear selected installed crate (go back to list)
    pub fn clear_installed_crate(&mut self) {
        self.selected_installed_crate = None;
        self.installed_crate_items.clear();
        self.installed_crate_filtered.clear();
        self.list_state.select(Some(0));
    }

    /// Crates to show in Crates tab: project dependencies when we have a Cargo project, else all installed.
    pub fn installed_crates_display_list(&self) -> Vec<String> {
        let project_dep_names: HashSet<String> = self
            .dependency_tree
            .iter()
            .filter(|(_, depth)| *depth > 0)
            .map(|(name, _)| name.clone())
            .collect();
        if project_dep_names.is_empty() {
            self.installed_crates_list.clone()
        } else {
            self.installed_crates_list
                .iter()
                .filter(|n| project_dep_names.contains(*n))
                .cloned()
                .collect()
        }
    }

    /// Crate name for "open in browser" (o key): current crate when inside one, or selected dep from list.
    pub fn selected_crate_name_for_display(&self) -> Option<String> {
        if self.current_tab != Tab::Crates {
            return None;
        }
        if let Some(ref c) = self.selected_installed_crate {
            return Some(c.name.clone());
        }
        self.selected_dependency_name()
    }

    /// Selected crate name in Crates tab (root or a dep). None if inside a crate, empty list, or wrong tab.
    pub fn selected_dependency_name(&self) -> Option<String> {
        if self.current_tab != Tab::Crates
            || self.selected_installed_crate.is_some()
            || self.dependency_tree.is_empty()
        {
            return None;
        }
        let list_idx = self.list_state.selected().unwrap_or(0);
        let tree_idx = self.filtered_dependency_indices.get(list_idx).copied()?;
        self.dependency_tree
            .get(tree_idx)
            .map(|(name, _)| name.clone())
    }

    /// Root crate name in dependency tree (first entry, depth 0). None if no tree.
    pub fn dependency_root_name(&self) -> Option<&str> {
        self.dependency_tree.first().map(|(n, _)| n.as_str())
    }

    /// Process any received crate doc fetch results (call each frame).
    pub fn poll_crate_docs_rx(&mut self) {
        while let Ok((name, doc)) = self.crate_docs_rx.try_recv() {
            if self.crate_docs_loading.as_deref() == Some(name.as_str()) {
                self.crate_docs_loading = None;
            }
            if let Some(info) = doc {
                if self.crate_docs_cache.len() >= CRATE_DOCS_CACHE_MAX {
                    if let Some(key) = self.crate_docs_cache.keys().next().cloned() {
                        self.crate_docs_cache.remove(&key);
                    }
                }
                self.crate_docs_cache.insert(name.clone(), info);
            } else {
                self.crate_docs_failed.insert(name);
            }
        }
    }

    /// If on Crates tab and selected crate is not root and not cached/loading/failed, start fetch in background.
    pub fn maybe_start_crate_doc_fetch(&mut self) {
        if self.current_tab != Tab::Crates {
            return;
        }
        let Some(name) = self.selected_dependency_name() else {
            return;
        };
        if self.dependency_root_name() == Some(name.as_str()) {
            return; // selected root: show local crate_info, no fetch
        }
        if self.crate_docs_cache.contains_key(&name)
            || self.crate_docs_loading.as_deref() == Some(name.as_str())
            || self.crate_docs_failed.contains(&name)
        {
            return;
        }
        self.crate_docs_loading = Some(name.clone());
        let tx = self.crate_docs_tx.clone();
        thread::spawn(move || {
            let result = crate::crates_io::fetch_crate_docs(&name);
            let _ = tx.send((name, result));
        });
    }

    /// Get current list length based on tab and selection state
    pub fn get_current_list_len(&self) -> usize {
        if self.current_tab == Tab::Crates {
            if self.selected_installed_crate.is_some() {
                self.installed_crate_filtered.len()
            } else {
                let n = self.filtered_dependency_indices.len();
                if self.dependency_tree.is_empty() || n == 0 {
                    1
                } else {
                    n
                }
            }
        } else {
            self.filtered_items.len()
        }
    }

    /// Get the currently selected item
    pub fn selected_item(&self) -> Option<&AnalyzedItem> {
        if self.current_tab == Tab::Crates && self.selected_installed_crate.is_some() {
            return self
                .list_state
                .selected()
                .and_then(|i| self.installed_crate_filtered.get(i))
                .and_then(|&idx| self.installed_crate_items.get(idx));
        }
        if self.current_tab == Tab::Crates {
            return None; // Inspector shows root/crate docs, not an item
        }
        self.list_state
            .selected()
            .and_then(|i| self.filtered_items.get(i))
            .and_then(|&idx| self.items.get(idx))
    }

    /// Get filtered items as references
    pub fn get_filtered_items(&self) -> Vec<&AnalyzedItem> {
        if self.current_tab == Tab::Crates && self.selected_installed_crate.is_some() {
            self.installed_crate_filtered
                .iter()
                .filter_map(|&i| self.installed_crate_items.get(i))
                .collect()
        } else {
            self.filtered_items
                .iter()
                .filter_map(|&i| self.items.get(i))
                .collect()
        }
    }

    // Navigation methods
    pub fn next_item(&mut self) {
        let len = self.get_current_list_len();
        if len == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % len,
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn prev_item(&mut self) {
        let len = self.get_current_list_len();
        if len == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => i.checked_sub(1).unwrap_or(len - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn next_tab(&mut self) {
        self.current_tab = self.current_tab.next();
        self.list_state.select(Some(0));
        self.show_completion = false; // Hide completions when switching tabs
        self.filter_items();

        // Scan crates if switching to installed crates tab
        if self.current_tab == Tab::Crates && self.installed_crates_list.is_empty() {
            let _ = self.scan_installed_crates();
        }
    }

    pub fn prev_tab(&mut self) {
        self.current_tab = self.current_tab.prev();
        self.list_state.select(Some(0));
        self.show_completion = false; // Hide completions when switching tabs
        self.filter_items();

        if self.current_tab == Tab::Crates && self.installed_crates_list.is_empty() {
            let _ = self.scan_installed_crates();
        }
    }

    pub fn next_focus(&mut self) {
        self.focus = self.focus.next(self.copilot_chat_open);
    }

    pub fn prev_focus(&mut self) {
        self.focus = self.focus.prev(self.copilot_chat_open);
    }

    pub fn next_completion(&mut self) {
        if !self.filtered_candidates.is_empty() {
            self.completion_selected =
                (self.completion_selected + 1) % self.filtered_candidates.len();
        }
    }

    pub fn prev_completion(&mut self) {
        if !self.filtered_candidates.is_empty() {
            self.completion_selected = self
                .completion_selected
                .checked_sub(1)
                .unwrap_or(self.filtered_candidates.len() - 1);
        }
    }

    pub fn select_completion(&mut self) {
        if let Some(candidate) = self.filtered_candidates.get(self.completion_selected) {
            self.search_input = candidate.primary.clone();
            self.show_completion = false;
            self.filter_items();
        }
    }

    // Input handling
    pub fn on_char(&mut self, c: char) {
        self.search_input.push(c);
        self.filter_items();
        // Don't show completions in Crates tab - use direct qualified path search
        self.show_completion = self.search_input.len() >= 2
            && !(self.current_tab == Tab::Crates && self.selected_installed_crate.is_some());
    }

    pub fn on_backspace(&mut self) {
        self.search_input.pop();
        self.filter_items();
        self.show_completion = self.search_input.len() >= 2
            && !(self.current_tab == Tab::Crates && self.selected_installed_crate.is_some());
    }

    pub fn clear_search(&mut self) {
        self.search_input.clear();
        self.show_completion = false;
        self.filter_items();
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Build context string for the currently selected item (for Copilot).
    pub fn build_copilot_context(&self) -> Option<String> {
        let item = self.selected_item()?;
        let loc = item
            .source_location()
            .and_then(|l| l.file.as_ref())
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let line = item
            .source_location()
            .and_then(|l| l.line)
            .map(|n| format!(":{}", n))
            .unwrap_or_default();
        let mut ctx = format!(
            "I'm inspecting this Rust item in Rustlens TUI. Use it as context.\n\n\
             **Item:** {} {}\n**Location:** {}{}\n**Definition:**\n```rust\n{}\n```\n",
            item.kind(),
            item.qualified_name(),
            loc,
            line,
            item.definition(),
        );
        if let Some(doc) = item.documentation() {
            let doc = doc.lines().take(10).collect::<Vec<_>>().join("\n");
            ctx.push_str("\n**Docs:**\n");
            ctx.push_str(&doc);
            ctx.push('\n');
        }
        ctx.push_str("\n---\nAnswer the user's question about this item.");
        Some(ctx)
    }

    /// Submit the current chat input to Copilot (spawns thread, sets loading).
    pub fn submit_copilot_message(&mut self) {
        let input = self.copilot_chat_input.trim().to_string();
        if input.is_empty() {
            return;
        }
        self.copilot_chat_input.clear();
        self.copilot_chat_messages
            .push(("user".to_string(), input.clone()));

        let context = if let Some(c) = self.build_copilot_context() {
            c
        } else {
            self.copilot_chat_messages
                .push(("assistant".to_string(), "No item selected.".to_string()));
            return;
        };

        let mut full_prompt = context;
        full_prompt.push_str("\n\n**Conversation:**\n");
        for (role, content) in &self.copilot_chat_messages {
            let label = if role == "user" { "User" } else { "Assistant" };
            let _ = writeln!(full_prompt, "{}: {}", label, content);
        }
        full_prompt.push_str("\nRespond to the user's latest message above.");

        let tx = self.copilot_tx.clone();
        let project_path = self.project_path.clone();
        thread::spawn(move || {
            let mut cmd = Command::new("copilot");
            cmd.arg("-p").arg(&full_prompt).arg("--allow-all").arg("-s");
            if let Some(ref p) = project_path {
                cmd.arg("--add-dir").arg(p);
            }
            let output = cmd.output();
            let response = match output {
                Ok(o) if o.status.success() => {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                }
                Ok(o) => format!(
                    "Copilot error (exit {}): {}",
                    o.status,
                    String::from_utf8_lossy(&o.stderr)
                ),
                Err(e) => format!("Failed to run copilot: {}", e),
            };
            let _ = tx.send(response);
        });
        self.copilot_chat_loading = true;
    }

    /// Toggle Copilot chat panel; when opening with an item selected, focus chat.
    pub fn toggle_copilot_chat(&mut self) {
        self.copilot_chat_open = !self.copilot_chat_open;
        if self.copilot_chat_open && self.selected_item().is_some() {
            self.focus = Focus::CopilotChat;
        } else if !self.copilot_chat_open && self.focus == Focus::CopilotChat {
            self.focus = Focus::Inspector;
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::RustAnalyzer;

    fn make_app_with_items() -> App {
        let source = r#"
            pub struct Foo {}
            pub fn bar() {}
            pub mod baz {}
        "#;
        let items = RustAnalyzer::new().analyze_source(source).unwrap();
        let mut app = App::new();
        app.items = items;
        app.filtered_items = vec![0, 1, 2];
        app.list_state.select(Some(0));
        app
    }

    #[test]
    fn test_get_current_list_len_types_tab() {
        let mut app = make_app_with_items();
        app.current_tab = Tab::Types;
        app.filter_items();
        assert_eq!(app.get_current_list_len(), 1);
    }

    #[test]
    fn test_get_current_list_len_functions_tab() {
        let mut app = make_app_with_items();
        app.current_tab = Tab::Functions;
        app.filter_items();
        assert_eq!(app.get_current_list_len(), 1);
    }

    #[test]
    fn test_get_current_list_len_crates_tab_empty_tree() {
        let mut app = App::new();
        app.current_tab = Tab::Crates;
        app.dependency_tree = vec![];
        app.filtered_dependency_indices = vec![];
        assert_eq!(app.get_current_list_len(), 1);
    }

    #[test]
    fn test_get_current_list_len_crates_tab_with_deps() {
        let mut app = App::new();
        app.current_tab = Tab::Crates;
        app.dependency_tree = vec![
            ("rustlens".to_string(), 0),
            ("serde".to_string(), 1),
            ("ratatui".to_string(), 1),
        ];
        app.filtered_dependency_indices = vec![0, 1, 2];
        assert_eq!(app.get_current_list_len(), 3);
    }

    #[test]
    fn test_selected_dependency_name_none_when_wrong_tab() {
        let mut app = App::new();
        app.current_tab = Tab::Types;
        app.dependency_tree = vec![("rustlens".to_string(), 0)];
        app.filtered_dependency_indices = vec![0];
        app.list_state.select(Some(0));
        assert!(app.selected_dependency_name().is_none());
    }

    #[test]
    fn test_selected_dependency_name_returns_selected() {
        let mut app = App::new();
        app.current_tab = Tab::Crates;
        app.dependency_tree = vec![("rustlens".to_string(), 0), ("serde".to_string(), 1)];
        app.filtered_dependency_indices = vec![0, 1];
        app.list_state.select(Some(1));
        assert_eq!(app.selected_dependency_name(), Some("serde".to_string()));
    }

    #[test]
    fn test_dependency_root_name() {
        let mut app = App::new();
        app.dependency_tree = vec![("rustlens".to_string(), 0), ("serde".to_string(), 1)];
        assert_eq!(app.dependency_root_name(), Some("rustlens"));
        app.dependency_tree.clear();
        assert!(app.dependency_root_name().is_none());
    }

    #[test]
    fn test_selected_item_types_tab() {
        let mut app = make_app_with_items();
        app.current_tab = Tab::Types;
        app.filter_items();
        app.list_state.select(Some(0));
        let item = app.selected_item().unwrap();
        assert_eq!(item.name(), "Foo");
    }

    #[test]
    fn test_get_filtered_items() {
        let mut app = make_app_with_items();
        app.current_tab = Tab::Types;
        app.filter_items();
        let filtered = app.get_filtered_items();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name(), "Foo");
    }

    #[test]
    fn test_installed_crates_display_list_empty_tree_returns_all_installed() {
        let mut app = App::new();
        app.dependency_tree = vec![];
        app.installed_crates_list = vec!["foo".into(), "bar".into()];
        let list = app.installed_crates_display_list();
        assert_eq!(list, vec!["foo", "bar"]);
    }

    #[test]
    fn test_installed_crates_display_list_filters_by_project_deps() {
        let mut app = App::new();
        app.dependency_tree = vec![("rustlens".to_string(), 0), ("serde".to_string(), 1)];
        app.installed_crates_list = vec!["serde".into(), "other".into()];
        let list = app.installed_crates_display_list();
        assert_eq!(list, vec!["serde"]);
    }
}
