use egui::{Color32, Context, Id};

pub(crate) const THEME_DATA_ID: &str = "pgs2srt-theme";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeKind {
    #[default]
    Light,
    Dark,
}

/// Saved user theme preference. `System` follows the OS dark/light mode with `Light` fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub fn from_code(code: &str) -> Self {
        match code {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::System => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }

    pub fn resolve(self, system_dark: Option<bool>, fallback: ThemeKind) -> ThemeKind {
        match self {
            Self::Light => ThemeKind::Light,
            Self::Dark => ThemeKind::Dark,
            Self::System => match system_dark {
                Some(true) => ThemeKind::Dark,
                Some(false) => ThemeKind::Light,
                None => fallback,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub kind: ThemeKind,
    /// Top tab/title strip.
    pub titlebar: Color32,
    /// Command toolbar & structural bars.
    pub chrome: Color32,
    /// Side inspector panel.
    pub panel: Color32,
    /// Central workspace canvas behind cards.
    pub pasteboard: Color32,
    /// Elevated cards, rows, popups.
    pub card: Color32,
    pub border: Color32,
    pub divider: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_faint: Color32,
    pub icon: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub selected: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub accent_soft: Color32,
    pub field: Color32,
    pub success: Color32,
    pub success_soft: Color32,
    pub warning: Color32,
    pub warning_soft: Color32,
    pub error: Color32,
    pub error_soft: Color32,
    pub cinema_bg: Color32,
    pub cinema_text: Color32,
    pub page_shadow: Color32,
    pub radius: u8,
}

impl Tokens {
    pub fn for_kind(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Light => Self {
                kind,
                titlebar: Color32::from_rgb(0xE9, 0xE9, 0xEB),
                chrome: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                pasteboard: Color32::from_rgb(0xF1, 0xF1, 0xF3),
                card: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                border: Color32::from_rgb(0xDA, 0xDA, 0xDE),
                divider: Color32::from_rgb(0xE8, 0xE8, 0xEB),
                text: Color32::from_rgb(0x22, 0x22, 0x26),
                text_muted: Color32::from_rgb(0x5E, 0x5E, 0x66),
                text_faint: Color32::from_rgb(0x6B, 0x6B, 0x73),
                icon: Color32::from_rgb(0x44, 0x44, 0x4B),
                hover: Color32::from_rgb(0xF0, 0xF0, 0xF3),
                pressed: Color32::from_rgb(0xE4, 0xE4, 0xE9),
                selected: Color32::from_rgb(0xE6, 0xEE, 0xFD),
                accent: Color32::from_rgb(0x1B, 0x63, 0xE0),
                accent_text: Color32::from_rgb(0x17, 0x55, 0xC4),
                accent_soft: Color32::from_rgb(0xE3, 0xEC, 0xFD),
                field: Color32::from_rgb(0xFF, 0xFF, 0xFF),
                success: Color32::from_rgb(0x15, 0x7F, 0x3C),
                success_soft: Color32::from_rgb(0xE3, 0xF6, 0xEA),
                warning: Color32::from_rgb(0xB4, 0x53, 0x09),
                warning_soft: Color32::from_rgb(0xFE, 0xF3, 0xC7),
                error: Color32::from_rgb(0xC9, 0x2A, 0x2A),
                error_soft: Color32::from_rgb(0xFD, 0xEC, 0xEC),
                cinema_bg: Color32::from_rgb(0x14, 0x15, 0x19),
                cinema_text: Color32::from_rgb(0xF8, 0xFA, 0xFC),
                page_shadow: Color32::from_black_alpha(34),
                radius: 6,
            },
            ThemeKind::Dark => Self {
                kind,
                titlebar: Color32::from_rgb(0x1B, 0x1B, 0x1E),
                chrome: Color32::from_rgb(0x26, 0x26, 0x2A),
                panel: Color32::from_rgb(0x26, 0x26, 0x2A),
                pasteboard: Color32::from_rgb(0x19, 0x19, 0x1C),
                card: Color32::from_rgb(0x2E, 0x2E, 0x33),
                border: Color32::from_rgb(0x3C, 0x3C, 0x43),
                divider: Color32::from_rgb(0x33, 0x33, 0x39),
                text: Color32::from_rgb(0xEC, 0xEC, 0xEF),
                text_muted: Color32::from_rgb(0xAE, 0xAE, 0xB6),
                text_faint: Color32::from_rgb(0x97, 0x97, 0x9E),
                icon: Color32::from_rgb(0xD4, 0xD4, 0xDA),
                hover: Color32::from_rgb(0x34, 0x34, 0x3A),
                pressed: Color32::from_rgb(0x3E, 0x3E, 0x45),
                selected: Color32::from_rgb(0x23, 0x3A, 0x63),
                accent: Color32::from_rgb(0x4B, 0x8B, 0xF5),
                accent_text: Color32::from_rgb(0x8C, 0xB6, 0xFA),
                accent_soft: Color32::from_rgb(0x24, 0x36, 0x57),
                field: Color32::from_rgb(0x1E, 0x1E, 0x22),
                success: Color32::from_rgb(0x4A, 0xDE, 0x80),
                success_soft: Color32::from_rgb(0x1B, 0x3B, 0x28),
                warning: Color32::from_rgb(0xFB, 0xBF, 0x24),
                warning_soft: Color32::from_rgb(0x3F, 0x2E, 0x14),
                error: Color32::from_rgb(0xF8, 0x71, 0x71),
                error_soft: Color32::from_rgb(0x42, 0x1E, 0x20),
                cinema_bg: Color32::from_rgb(0x10, 0x11, 0x14),
                cinema_text: Color32::from_rgb(0xF8, 0xFA, 0xFC),
                page_shadow: Color32::from_black_alpha(120),
                radius: 6,
            },
        }
    }

    pub fn get(ctx: &Context) -> Self {
        ctx.data(|d| d.get_temp::<Tokens>(Id::unique(THEME_DATA_ID))).unwrap_or_else(|| Self::for_kind(ThemeKind::Light))
    }

    pub fn dark(&self) -> bool {
        self.kind == ThemeKind::Dark
    }
}
