//! Vizier - Rust Code Inspector
//!
//! A terminal-based Rust code inspector with beautiful TUI.

use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::layout::Rect;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{env, io, path::PathBuf, time::Duration};
use vizier_lib::{
    app::App,
    ui::{app::tabs_rect_for_area, app::Focus, app::Tab, AnimationState, VizierUi},
};

fn main() -> Result<()> {
    // Load .env so GITHUB_TOKEN etc. are available (cwd first, then project path overrides)
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().collect();
    let mut project_path = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap_or(PathBuf::from(".")));
    // Resolve to absolute path so we always analyze the directory the user expects
    if project_path.exists() {
        if let Ok(canon) = std::fs::canonicalize(&project_path) {
            project_path = canon;
        }
    }
    let _ = dotenvy::from_path(project_path.join(".env"));

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run app
    let mut app = App::new();

    // Try to load settings (ignore errors, use defaults)
    let _ = app.load_settings();

    // Analyze the project
    if let Err(e) = app.analyze_project(project_path.as_path()) {
        app.status_message = format!("Analysis failed: {}", e);
    }

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let mut animation = AnimationState::new();
    let mut inspector_scroll: usize = 0;
    let mut last_selected: Option<usize> = None;

    loop {
        // Update animations
        animation.update();

        // Reset inspector scroll on selection change
        let current_selected = app.list_state.selected();
        if current_selected != last_selected {
            inspector_scroll = 0;
            animation.on_selection_change();
            last_selected = current_selected;
        }

        // Poll Copilot chat response (from background thread)
        if let Ok(response) = app.copilot_rx.try_recv() {
            app.copilot_chat_messages
                .push(("assistant".to_string(), response));
            app.copilot_chat_loading = false;
        }

        // Poll crate docs channel and maybe start fetch for selected dependency
        app.poll_crate_docs_rx();
        app.maybe_start_crate_doc_fetch();

        // Draw UI
        let selected_dep_name = app.selected_dependency_name();
        let crate_doc = selected_dep_name
            .as_ref()
            .and_then(|n| app.crate_docs_cache.get(n));
        let crate_doc_loading = app.crate_docs_loading.as_deref() == selected_dep_name.as_deref();
        let crate_doc_failed = selected_dep_name
            .as_ref()
            .is_some_and(|n| app.crate_docs_failed.contains(n));
        terminal.draw(|frame| {
            let filtered = app.get_filtered_items();
            let selected = app.list_state.selected();

            let installed_items: Vec<&vizier_lib::analyzer::AnalyzedItem> = app
                .installed_crate_filtered
                .iter()
                .filter_map(|&i| app.installed_crate_items.get(i))
                .collect();

            let all_items_impl =
                if app.current_tab == Tab::Crates && app.selected_installed_crate.is_some() {
                    Some(app.installed_crate_items.as_slice())
                } else {
                    Some(app.items.as_slice())
                };
            let ui = VizierUi::new(&app.theme)
                .items(&app.items)
                .all_items_impl_lookup(all_items_impl)
                .filtered_items(&filtered)
                .list_selected(selected)
                .candidates(&app.filtered_candidates)
                .crate_info(app.crate_info.as_ref())
                .dependency_tree(&app.dependency_tree)
                .filtered_dependency_indices(&app.filtered_dependency_indices)
                .crate_doc(crate_doc)
                .crate_doc_loading(crate_doc_loading)
                .crate_doc_failed(crate_doc_failed)
                .selected_installed_crate(app.selected_installed_crate.as_ref())
                .installed_crate_items(&installed_items)
                .target_size_bytes(app.target_size_bytes)
                .search_input(&app.search_input)
                .current_tab(app.current_tab)
                .focus(app.focus)
                .selected_item(app.selected_item())
                .completion_selected(app.completion_selected)
                .show_completion(app.show_completion)
                .show_help(app.show_help)
                .show_settings(app.show_settings)
                .status_message(&app.status_message)
                .inspector_scroll(inspector_scroll)
                .animation_state(&animation)
                .show_copilot_chat(app.copilot_chat_open)
                .copilot_chat_messages(&app.copilot_chat_messages)
                .copilot_chat_input(&app.copilot_chat_input)
                .copilot_chat_loading(app.copilot_chat_loading)
                .copilot_chat_scroll(app.copilot_chat_scroll);

            frame.render_widget(ui, frame.area());
        })?;

        if app.should_quit {
            break;
        }

        // Handle events with shorter poll time when animating
        let poll_duration = if animation.is_animating() {
            Duration::from_millis(16) // ~60fps when animating
        } else {
            Duration::from_millis(50)
        };

        if event::poll(poll_duration)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key_event(
                        app,
                        key.code,
                        key.modifiers,
                        &mut inspector_scroll,
                        &mut animation,
                    );
                }
                Event::Mouse(mouse) => {
                    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                        if let Ok(size) = terminal.size() {
                            let area = Rect::new(0, 0, size.width, size.height);
                            if let Some(tabs_rect) = tabs_rect_for_area(area) {
                                let col = mouse.column;
                                let row = mouse.row;
                                if col >= tabs_rect.x
                                    && col < tabs_rect.x + tabs_rect.width
                                    && row >= tabs_rect.y
                                    && row < tabs_rect.y + tabs_rect.height
                                {
                                    let tab_count = 4u16;
                                    let inner_w = tabs_rect.width.saturating_sub(2);
                                    if inner_w >= tab_count {
                                        let tab_width = inner_w / tab_count;
                                        let inner_x = tabs_rect.x + 1;
                                        let rel = col.saturating_sub(inner_x);
                                        let idx = (rel / tab_width).min(3) as usize;
                                        let new_tab = Tab::from_index(idx);
                                        if app.current_tab != new_tab {
                                            app.current_tab = new_tab;
                                            app.list_state.select(Some(0));
                                            if app.current_tab == Tab::Crates
                                                && app.installed_crates_list.is_empty()
                                            {
                                                let _ = app.scan_installed_crates();
                                            }
                                            app.filter_items();
                                            animation.on_tab_change();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_copilot_chat_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Esc => {
            app.toggle_copilot_chat();
        }
        KeyCode::Enter if modifiers.is_empty() => {
            app.submit_copilot_message();
        }
        KeyCode::Backspace if modifiers.is_empty() => {
            app.copilot_chat_input.pop();
        }
        KeyCode::Char(c) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
            app.copilot_chat_input.push(c);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.copilot_chat_scroll = app.copilot_chat_scroll.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.copilot_chat_scroll = app.copilot_chat_scroll.saturating_sub(1);
        }
        KeyCode::PageDown => {
            app.copilot_chat_scroll = app.copilot_chat_scroll.saturating_add(10);
        }
        KeyCode::PageUp => {
            app.copilot_chat_scroll = app.copilot_chat_scroll.saturating_sub(10);
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.copilot_chat_scroll = 0;
        }
        KeyCode::Tab if modifiers.is_empty() => {
            app.next_focus();
        }
        KeyCode::BackTab => {
            app.prev_focus();
        }
        _ => {}
    }
}

fn handle_key_event(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    inspector_scroll: &mut usize,
    animation: &mut AnimationState,
) {
    use vizier_lib::ui::app::Tab;

    // When Copilot chat panel is open: PgDn/PgUp/arrows/Home/End always scroll the chat (no need to focus chat first)
    if app.copilot_chat_open {
        match code {
            KeyCode::PageDown => {
                app.copilot_chat_scroll = app.copilot_chat_scroll.saturating_add(10);
                return;
            }
            KeyCode::PageUp => {
                app.copilot_chat_scroll = app.copilot_chat_scroll.saturating_sub(10);
                return;
            }
            KeyCode::Down => {
                app.copilot_chat_scroll = app.copilot_chat_scroll.saturating_add(1);
                return;
            }
            KeyCode::Up => {
                app.copilot_chat_scroll = app.copilot_chat_scroll.saturating_sub(1);
                return;
            }
            KeyCode::Home => {
                app.copilot_chat_scroll = 0;
                return;
            }
            KeyCode::End => {
                app.copilot_chat_scroll = app.copilot_chat_scroll.saturating_add(9999);
                return;
            }
            KeyCode::Char(c) => {
                if modifiers == KeyModifiers::SHIFT && c == 'C' {
                    // Let Shift+C fall through to toggle panel
                } else {
                    app.focus = Focus::CopilotChat;
                    app.copilot_chat_input.push(c);
                    return;
                }
            }
            KeyCode::Backspace => {
                app.focus = Focus::CopilotChat;
                app.copilot_chat_input.pop();
                return;
            }
            KeyCode::Enter if modifiers.is_empty() => {
                app.focus = Focus::CopilotChat;
                app.submit_copilot_message();
                return;
            }
            _ => {}
        }
    }

    // Global shortcuts — never run when focus is CopilotChat
    let in_copilot_chat = app.focus == Focus::CopilotChat;
    match code {
        KeyCode::Char('q')
            if modifiers.is_empty() && !in_copilot_chat && app.focus != Focus::Search =>
        {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('?')
            if modifiers.is_empty() && !in_copilot_chat && app.focus != Focus::Search =>
        {
            app.toggle_help();
            return;
        }
        KeyCode::Char('t')
            if modifiers.is_empty() && !in_copilot_chat && app.focus != Focus::Search =>
        {
            app.cycle_theme();
            return;
        }
        KeyCode::Char('S')
            if modifiers.contains(KeyModifiers::SHIFT)
                && !in_copilot_chat
                && app.focus != Focus::Search =>
        {
            app.toggle_settings();
            return;
        }
        KeyCode::Char('g')
            if modifiers.is_empty() && !in_copilot_chat && app.focus != Focus::Search =>
        {
            let _ = webbrowser::open("https://github.com/yashksaini-coder/vizier");
            return;
        }
        KeyCode::Char('C')
            if modifiers.contains(KeyModifiers::SHIFT)
                && !in_copilot_chat
                && app.focus != Focus::Search =>
        {
            if app.selected_item().is_some() {
                app.toggle_copilot_chat();
            } else {
                app.status_message = "Select an item in the list to ask Copilot about it".into();
            }
            return;
        }
        KeyCode::Char('s')
            if modifiers.is_empty() && !in_copilot_chat && app.focus != Focus::Search =>
        {
            let _ = webbrowser::open("https://github.com/sponsors/yashksaini-coder");
            return;
        }
        KeyCode::Esc => {
            if app.show_settings {
                app.toggle_settings();
            } else if app.show_help {
                app.show_help = false;
            } else if app.show_completion {
                app.show_completion = false;
            } else if app.focus == Focus::CopilotChat {
                app.toggle_copilot_chat();
            } else if app.current_tab == Tab::Crates && app.selected_installed_crate.is_some() {
                app.clear_installed_crate();
            } else if !app.search_input.is_empty() {
                app.clear_search();
            } else {
                app.should_quit = true;
            }
            return;
        }
        _ => {}
    }

    // Settings overlay: t cycle theme
    if app.show_settings {
        if let KeyCode::Char('t') = code {
            app.cycle_theme();
        }
        return;
    }

    // Help is open - any key closes it
    if app.show_help {
        app.show_help = false;
        return;
    }

    // Tab switching with number keys (not when typing in Copilot chat)
    match code {
        KeyCode::Char('1')
            if modifiers.is_empty() && !in_copilot_chat && app.focus != Focus::Search =>
        {
            app.current_tab = Tab::Types;
            app.list_state.select(Some(0));
            app.filter_items();
            animation.on_tab_change();
            return;
        }
        KeyCode::Char('2')
            if modifiers.is_empty() && !in_copilot_chat && app.focus != Focus::Search =>
        {
            app.current_tab = Tab::Functions;
            app.list_state.select(Some(0));
            app.filter_items();
            animation.on_tab_change();
            return;
        }
        KeyCode::Char('3')
            if modifiers.is_empty() && !in_copilot_chat && app.focus != Focus::Search =>
        {
            app.current_tab = Tab::Modules;
            app.list_state.select(Some(0));
            app.filter_items();
            animation.on_tab_change();
            return;
        }
        KeyCode::Char('4')
            if modifiers.is_empty() && !in_copilot_chat && app.focus != Focus::Search =>
        {
            app.current_tab = Tab::Crates;
            app.list_state.select(Some(0));
            if app.installed_crates_list.is_empty() {
                let _ = app.scan_installed_crates();
            }
            app.filter_items();
            animation.on_tab_change();
            return;
        }
        _ => {}
    }

    // Focus-specific handling
    match app.focus {
        Focus::Search => handle_search_input(app, code, modifiers),
        Focus::List => handle_list_input(app, code, modifiers),
        Focus::Inspector => handle_inspector_input(app, code, modifiers, inspector_scroll),
        Focus::CopilotChat => handle_copilot_chat_input(app, code, modifiers),
    }
}

fn handle_search_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Char(c) => {
            app.on_char(c);
        }
        KeyCode::Backspace => {
            app.on_backspace();
        }
        KeyCode::Down => {
            if app.show_completion {
                app.next_completion();
            } else {
                app.focus = Focus::List;
            }
        }
        KeyCode::Up if app.show_completion => {
            app.prev_completion();
        }
        KeyCode::Tab | KeyCode::BackTab if modifiers.is_empty() => {
            if code == KeyCode::Tab {
                if app.show_completion {
                    app.select_completion();
                }
                app.next_focus(); // Tab: search -> list -> inspector
            } else {
                app.prev_focus(); // BackTab: search -> inspector -> list
            }
        }
        KeyCode::Enter => {
            if app.show_completion {
                app.select_completion();
            } else {
                // Dependencies tab (inside a crate): try qualified path search
                if app.current_tab == Tab::Crates && app.selected_installed_crate.is_some() {
                    app.search_qualified_path();
                }
                app.filter_items();
                app.focus = Focus::List;
            }
        }
        _ => {}
    }
}

fn handle_list_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    use vizier_lib::ui::app::Tab;

    match code {
        KeyCode::Down | KeyCode::Char('j') => {
            app.next_item();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.prev_item();
        }
        KeyCode::Tab if modifiers.is_empty() => {
            app.next_focus();
        }
        KeyCode::BackTab => {
            app.prev_focus();
        }
        KeyCode::Char('/') => {
            app.focus = Focus::Search;
        }
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            // Dependencies: Enter on a dep opens that crate's items (from registry)
            if app.current_tab == Tab::Crates && app.selected_installed_crate.is_none() {
                if let Some(name) = app.selected_dependency_name() {
                    if app.dependency_root_name() != Some(name.as_str()) {
                        let _ = app.select_installed_crate(&name);
                        app.list_state.select(Some(0));
                    } else {
                        app.focus = Focus::Inspector;
                    }
                } else {
                    app.focus = Focus::Inspector;
                }
            } else {
                app.focus = Focus::Inspector;
            }
        }
        KeyCode::Char('o' | 'c') if modifiers.is_empty() && app.current_tab == Tab::Crates => {
            if let Some(name) = app.selected_crate_name_for_display() {
                let url = if code == KeyCode::Char('c') {
                    format!("https://crates.io/crates/{}", name)
                } else {
                    format!("https://docs.rs/{}", name)
                };
                if webbrowser::open(&url).is_ok() {
                    app.status_message = format!("Opened {} in browser", name);
                } else {
                    app.status_message = format!("Failed to open {}", url);
                }
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.current_tab == Tab::Crates && app.selected_installed_crate.is_some() {
                app.clear_installed_crate();
            } else {
                app.focus = Focus::Search;
            }
        }
        KeyCode::Home | KeyCode::Char('g') => {
            let len = app.get_current_list_len();
            if len > 0 {
                app.list_state.select(Some(0));
            }
        }
        KeyCode::End | KeyCode::Char('G') => {
            let len = app.get_current_list_len();
            if len > 0 {
                app.list_state.select(Some(len - 1));
            }
        }
        KeyCode::PageDown => {
            // Jump 10 items
            for _ in 0..10 {
                app.next_item();
            }
        }
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.prev_item();
            }
        }
        _ => {}
    }
}

fn handle_inspector_input(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    inspector_scroll: &mut usize,
) {
    match code {
        KeyCode::Tab if modifiers.is_empty() => {
            app.next_focus();
        }
        KeyCode::BackTab => {
            app.prev_focus();
        }
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Esc => {
            app.focus = Focus::List;
        }
        KeyCode::Char('/') => {
            app.focus = Focus::Search;
        }
        // Scroll the inspector content
        KeyCode::Down | KeyCode::Char('j') => {
            *inspector_scroll = inspector_scroll.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            *inspector_scroll = inspector_scroll.saturating_sub(1);
        }
        KeyCode::PageDown => {
            *inspector_scroll = inspector_scroll.saturating_add(10);
        }
        KeyCode::PageUp => {
            *inspector_scroll = inspector_scroll.saturating_sub(10);
        }
        KeyCode::Home | KeyCode::Char('g') => {
            *inspector_scroll = 0;
        }
        KeyCode::Char('o' | 'c') if modifiers.is_empty() && app.current_tab == Tab::Crates => {
            if let Some(name) = app.selected_crate_name_for_display() {
                let url = if code == KeyCode::Char('c') {
                    format!("https://crates.io/crates/{}", name)
                } else {
                    format!("https://docs.rs/{}", name)
                };
                if webbrowser::open(&url).is_ok() {
                    app.status_message = format!("Opened {} in browser", name);
                } else {
                    app.status_message = format!("Failed to open {}", url);
                }
            }
        }
        _ => {}
    }
}
