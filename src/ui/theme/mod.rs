//! Theme system for Rustlens TUI

use ratatui::style::{Color, Modifier, Style};

/// Theme preset identifier (for config and cycling)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    DefaultDark,
    Nord,
    CatppuccinMocha,
    Dracula,
}

impl ThemeKind {
    pub const ALL: &'static [ThemeKind] = &[
        ThemeKind::DefaultDark,
        ThemeKind::Nord,
        ThemeKind::CatppuccinMocha,
        ThemeKind::Dracula,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ThemeKind::DefaultDark => "default_dark",
            ThemeKind::Nord => "nord",
            ThemeKind::CatppuccinMocha => "catppuccin_mocha",
            ThemeKind::Dracula => "dracula",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeKind::DefaultDark => "Default Dark",
            ThemeKind::Nord => "Nord",
            ThemeKind::CatppuccinMocha => "Catppuccin Mocha",
            ThemeKind::Dracula => "Dracula",
        }
    }

    pub fn from_name(name: &str) -> Self {
        let s = name.to_lowercase();
        let s = s.trim();
        match s {
            "nord" => ThemeKind::Nord,
            "catppuccin_mocha" | "catppuccin" | "mocha" | "catppuccin mocha" => {
                ThemeKind::CatppuccinMocha
            }
            "dracula" => ThemeKind::Dracula,
            "default_dark" | "default" | "default dark" => ThemeKind::DefaultDark,
            _ => ThemeKind::DefaultDark,
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&k| k == self).unwrap_or(0);
        let next = (i + 1) % Self::ALL.len();
        Self::ALL[next]
    }
}

/// Color palette for the UI
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub accent: Color,
    pub accent_dim: Color,
    pub bg: Color,
    pub bg_highlight: Color,
    pub bg_panel: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub fg_muted: Color,
    pub border: Color,
    pub border_focused: Color,
    /// Active tab background (darker tone so white text is always visible)
    pub tab_active_bg: Color,
    /// Active tab text (white for consistent contrast across themes)
    pub tab_active_fg: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,
    // Syntax colors
    pub keyword: Color,
    pub function: Color,
    pub type_: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
}

impl Theme {
    pub fn default_dark() -> Self {
        Self {
            name: "Default Dark".into(),
            accent: Color::Rgb(78, 191, 113), // #4EBF71 - Green
            accent_dim: Color::Rgb(45, 110, 65),
            bg: Color::Rgb(24, 24, 24),
            bg_highlight: Color::Rgb(45, 45, 45),
            bg_panel: Color::Rgb(32, 32, 32),
            fg: Color::Rgb(230, 230, 230),
            fg_dim: Color::Rgb(195, 195, 200),
            fg_muted: Color::Rgb(140, 140, 145),
            border: Color::Rgb(60, 60, 60),
            border_focused: Color::Rgb(78, 191, 113),
            tab_active_bg: Color::Rgb(88, 66, 134), // Darker purple: high contrast for white text
            tab_active_fg: Color::Rgb(255, 255, 255), // White: consistent across themes
            error: Color::Rgb(244, 67, 54),
            warning: Color::Rgb(255, 152, 0),
            success: Color::Rgb(76, 175, 80),
            info: Color::Rgb(33, 150, 243),
            keyword: Color::Rgb(198, 120, 221), // Purple
            function: Color::Rgb(97, 175, 239), // Blue
            type_: Color::Rgb(229, 192, 123),   // Yellow
            string: Color::Rgb(152, 195, 121),  // Green
            number: Color::Rgb(209, 154, 102),  // Orange
            comment: Color::Rgb(92, 99, 112),   // Gray
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "Nord".into(),
            accent: Color::Rgb(136, 192, 208),    // Nord8
            accent_dim: Color::Rgb(94, 129, 172), // Nord10
            bg: Color::Rgb(46, 52, 64),           // Nord0
            bg_highlight: Color::Rgb(59, 66, 82), // Nord1
            bg_panel: Color::Rgb(67, 76, 94),     // Nord2
            fg: Color::Rgb(236, 239, 244),        // Nord6
            fg_dim: Color::Rgb(229, 233, 240),    // Nord5 brighter
            fg_muted: Color::Rgb(136, 142, 156),  // Nord3 brighter
            border: Color::Rgb(76, 86, 106),      // Nord3
            border_focused: Color::Rgb(136, 192, 208),
            tab_active_bg: Color::Rgb(65, 89, 122), // Darker Nord blue: high contrast for white text
            tab_active_fg: Color::Rgb(255, 255, 255), // White: consistent across themes
            error: Color::Rgb(191, 97, 106),        // Nord11
            warning: Color::Rgb(235, 203, 139),     // Nord13
            success: Color::Rgb(163, 190, 140),     // Nord14
            info: Color::Rgb(129, 161, 193),        // Nord9
            keyword: Color::Rgb(180, 142, 173),     // Nord15
            function: Color::Rgb(136, 192, 208),    // Nord8
            type_: Color::Rgb(235, 203, 139),       // Nord13
            string: Color::Rgb(163, 190, 140),      // Nord14
            number: Color::Rgb(208, 135, 112),      // Nord12
            comment: Color::Rgb(76, 86, 106),       // Nord3
        }
    }

    /// Catppuccin Mocha theme
    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "Catppuccin Mocha".into(),
            accent: Color::Rgb(166, 227, 161),     // Green
            accent_dim: Color::Rgb(116, 199, 236), // Sapphire
            bg: Color::Rgb(30, 30, 46),            // Base
            bg_highlight: Color::Rgb(49, 50, 68),  // Surface0
            bg_panel: Color::Rgb(36, 39, 58),      // Mantle
            fg: Color::Rgb(205, 214, 244),         // Text
            fg_dim: Color::Rgb(205, 214, 244),     // Text (readable)
            fg_muted: Color::Rgb(147, 153, 178),   // Overlay0 brighter
            border: Color::Rgb(69, 71, 90),        // Surface1
            border_focused: Color::Rgb(166, 227, 161),
            tab_active_bg: Color::Rgb(126, 87, 194), // Darker mauve: high contrast for white text
            tab_active_fg: Color::Rgb(255, 255, 255), // White: consistent across themes
            error: Color::Rgb(243, 139, 168),        // Red
            warning: Color::Rgb(249, 226, 175),      // Yellow
            success: Color::Rgb(166, 227, 161),      // Green
            info: Color::Rgb(137, 180, 250),         // Blue
            keyword: Color::Rgb(203, 166, 247),      // Mauve
            function: Color::Rgb(137, 180, 250),     // Blue
            type_: Color::Rgb(249, 226, 175),        // Yellow
            string: Color::Rgb(166, 227, 161),       // Green
            number: Color::Rgb(250, 179, 135),       // Peach
            comment: Color::Rgb(108, 112, 134),      // Overlay0
        }
    }

    /// Dracula theme
    pub fn dracula() -> Self {
        Self {
            name: "Dracula".into(),
            accent: Color::Rgb(80, 250, 123),      // Green
            accent_dim: Color::Rgb(139, 233, 253), // Cyan
            bg: Color::Rgb(40, 42, 54),            // Background
            bg_highlight: Color::Rgb(68, 71, 90),  // Current Line
            bg_panel: Color::Rgb(33, 34, 44),      // Darker bg
            fg: Color::Rgb(248, 248, 242),         // Foreground
            fg_dim: Color::Rgb(230, 230, 235),
            fg_muted: Color::Rgb(139, 153, 195),
            border: Color::Rgb(68, 71, 90), // Current Line
            border_focused: Color::Rgb(80, 250, 123),
            tab_active_bg: Color::Rgb(118, 92, 168), // Darker purple: high contrast for white text
            tab_active_fg: Color::Rgb(255, 255, 255), // White: consistent across themes
            error: Color::Rgb(255, 85, 85),          // Red
            warning: Color::Rgb(255, 184, 108),      // Orange
            success: Color::Rgb(80, 250, 123),       // Green
            info: Color::Rgb(139, 233, 253),         // Cyan
            keyword: Color::Rgb(255, 121, 198),      // Pink
            function: Color::Rgb(80, 250, 123),      // Green
            type_: Color::Rgb(139, 233, 253),        // Cyan
            string: Color::Rgb(241, 250, 140),       // Yellow
            number: Color::Rgb(189, 147, 249),       // Purple
            comment: Color::Rgb(98, 114, 164),       // Comment
        }
    }

    pub fn from_kind(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::DefaultDark => Self::default_dark(),
            ThemeKind::Nord => Self::nord(),
            ThemeKind::CatppuccinMocha => Self::catppuccin_mocha(),
            ThemeKind::Dracula => Self::dracula(),
        }
    }

    pub fn from_name(name: &str) -> Self {
        Self::from_kind(ThemeKind::from_name(name))
    }

    pub fn kind(&self) -> ThemeKind {
        ThemeKind::from_name(&self.name)
    }

    // Style builders
    pub fn style_accent(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn style_accent_bold(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn style_normal(&self) -> Style {
        Style::default().fg(self.fg)
    }

    pub fn style_dim(&self) -> Style {
        Style::default().fg(self.fg_dim)
    }

    pub fn style_muted(&self) -> Style {
        Style::default().fg(self.fg_muted)
    }

    pub fn style_highlight(&self) -> Style {
        Style::default().bg(self.bg_highlight)
    }

    /// Style for selected list rows. Uses explicit fg so text stays readable on the highlight background.
    pub fn style_selected(&self) -> Style {
        Style::default()
            .fg(self.fg)
            .bg(self.bg_highlight)
            .add_modifier(Modifier::BOLD)
    }

    pub fn style_border(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Active tab: button-style highlight (e.g. lavender bg, light text).
    pub fn style_tab_active(&self) -> Style {
        Style::default()
            .fg(self.tab_active_fg)
            .bg(self.tab_active_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Subtle accent-tinted border for the outer frame (soft glow effect).
    pub fn style_border_glow(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::DIM)
    }

    pub fn style_border_focused(&self) -> Style {
        Style::default().fg(self.border_focused)
    }

    pub fn style_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn style_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn style_success(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn style_info(&self) -> Style {
        Style::default().fg(self.info)
    }

    pub fn style_keyword(&self) -> Style {
        Style::default().fg(self.keyword)
    }

    pub fn style_function(&self) -> Style {
        Style::default().fg(self.function)
    }

    pub fn style_type(&self) -> Style {
        Style::default().fg(self.type_)
    }

    pub fn style_string(&self) -> Style {
        Style::default().fg(self.string)
    }

    pub fn style_number(&self) -> Style {
        Style::default().fg(self.number)
    }

    pub fn style_comment(&self) -> Style {
        Style::default().fg(self.comment)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_kind_from_name() {
        assert_eq!(ThemeKind::from_name("default"), ThemeKind::DefaultDark);
        assert_eq!(ThemeKind::from_name("default_dark"), ThemeKind::DefaultDark);
        assert_eq!(ThemeKind::from_name("nord"), ThemeKind::Nord);
        assert_eq!(ThemeKind::from_name("Nord"), ThemeKind::Nord);
        assert_eq!(
            ThemeKind::from_name("catppuccin_mocha"),
            ThemeKind::CatppuccinMocha
        );
        assert_eq!(ThemeKind::from_name("mocha"), ThemeKind::CatppuccinMocha);
        assert_eq!(ThemeKind::from_name("dracula"), ThemeKind::Dracula);
        assert_eq!(ThemeKind::from_name("unknown"), ThemeKind::DefaultDark);
    }

    #[test]
    fn test_theme_kind_next_cycles() {
        let first = ThemeKind::DefaultDark;
        let second = first.next();
        assert_ne!(first, second);
        let mut k = ThemeKind::DefaultDark;
        for _ in 0..ThemeKind::ALL.len() {
            k = k.next();
        }
        assert_eq!(k, ThemeKind::DefaultDark);
    }

    #[test]
    fn test_theme_from_kind() {
        let t = Theme::from_kind(ThemeKind::Nord);
        assert_eq!(t.name, "Nord");
        let t2 = Theme::from_name("dracula");
        assert_eq!(t2.name, "Dracula");
    }
}
