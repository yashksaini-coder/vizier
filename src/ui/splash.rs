//! Splash screen with waves animation shown before the main TUI.

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame, Terminal,
};
use std::io;
use std::time::{Duration, Instant};

use crate::ui::theme::Theme;

const SPLASH_DURATION: Duration = Duration::from_millis(2200);
const WAVE_ROWS: usize = 8;
const WAVE_CHARS: &str = "~∿∼〜〰︴";

/// Draw a single frame of the waves animation. `phase` is in 0.0..1.0 (or more for continuous).
fn draw_waves(frame: &mut Frame, area: Rect, phase: f64, theme: &Theme) {
    let wave_len = 24usize;
    let width = area.width as usize;
    let height = area.height.saturating_sub(2) as usize;
    if height == 0 || width == 0 {
        return;
    }
    for row in 0..height.min(WAVE_ROWS) {
        let y = area.y + 2 + row as u16;
        let mut line = String::with_capacity(width);
        let row_phase = phase + row as f64 * 0.2;
        for col in 0..width {
            let t = (col as f64 / wave_len as f64) + row_phase;
            let wave = (t * std::f64::consts::TAU).sin();
            let idx = ((wave + 1.0) * 0.5 * (WAVE_CHARS.len() - 1) as f64) as usize;
            let idx = idx.min(WAVE_CHARS.len() - 1);
            let c = WAVE_CHARS.chars().nth(idx).unwrap_or('~');
            line.push(c);
        }
        let style = if row % 2 == 0 {
            theme.style_accent()
        } else {
            theme.style_dim()
        };
        let span = Span::styled(line, style);
        frame.render_widget(Paragraph::new(span), Rect::new(area.x, y, area.width, 1));
    }
}

/// Run the splash screen: waves animation + title. Returns when duration elapsed or any key pressed.
pub fn run_splash(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let theme = Theme::default();
    let start = Instant::now();

    loop {
        let elapsed = start.elapsed();
        if elapsed >= SPLASH_DURATION {
            break;
        }
        let phase = elapsed.as_secs_f64() * 2.0;

        terminal.draw(|frame| {
            let area = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(10),
                    Constraint::Length(3),
                ])
                .split(area);

            let title = Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        "RUSTLENS",
                        theme.style_accent_bold().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("  ·  ", theme.style_muted()),
                    Span::styled("Rust Code Inspector", theme.style_dim()),
                ]),
            ])
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(theme.style_border()),
            );
            title.render(chunks[0], frame.buffer_mut());

            draw_waves(frame, chunks[1], phase, &theme);

            let hint = Paragraph::new(Line::from(vec![
                Span::styled("Starting... ", theme.style_muted()),
                Span::styled("(press any key to skip)", theme.style_dim()),
            ]))
            .alignment(Alignment::Center);
            hint.render(chunks[2], frame.buffer_mut());
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    break;
                }
            }
        }
    }

    Ok(())
}
