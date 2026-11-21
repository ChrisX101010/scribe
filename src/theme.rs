//! colors, and the slow argument about color support.
//!
//! views ask for roles like `accent` or `muted`; this module decides what that
//! means for the active theme. every theme here has its own background on
//! purpose, so switching themes changes the whole screen, not just the text.

use ratatui::style::Color;
use std::env;

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color,
    pub mantle: Color,
    pub surface: Color,
    pub surface1: Color,
    pub border: Color,
    pub text: Color,
    pub subtext0: Color,
    pub muted: Color,
    pub accent: Color,
    pub secondary_accent: Color,
    pub success: Color,
    pub warn: Color,
    pub error: Color,
}

/// build a theme from hex triples. keeps the table below readable.
macro_rules! rgb {
    ($hex:expr) => {{
        const H: &str = $hex;
        Color::Rgb(
            u8::from_str_radix(&H[0..2], 16).unwrap(),
            u8::from_str_radix(&H[2..4], 16).unwrap(),
            u8::from_str_radix(&H[4..6], 16).unwrap(),
        )
    }};
}

macro_rules! palette {
    ($bg:expr,$mantle:expr,$surface:expr,$surface1:expr,$border:expr,$text:expr,$sub:expr,$muted:expr,$accent:expr,$accent2:expr,$ok:expr,$warn:expr,$err:expr) => {
        Palette {
            bg: rgb!($bg),
            mantle: rgb!($mantle),
            surface: rgb!($surface),
            surface1: rgb!($surface1),
            border: rgb!($border),
            text: rgb!($text),
            subtext0: rgb!($sub),
            muted: rgb!($muted),
            accent: rgb!($accent),
            secondary_accent: rgb!($accent2),
            success: rgb!($ok),
            warn: rgb!($warn),
            error: rgb!($err),
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    CatppuccinMocha,
    CatppuccinMacchiato,
    CatppuccinFrappe,
    CatppuccinLatte,
    TokyoNight,
    TokyoNightStorm,
    Nord,
    Dracula,
    Gruvbox,
    OneDark,
    SolarizedDark,
    SolarizedLight,
    GitHubDark,
    GitHubLight,
    Everforest,
    Kanagawa,
    RosePine,
    RosePineDawn,
    Material,
    Palenight,
    Cyberpunk,
    Contrast,
}

impl ThemeKind {
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().replace(['_', ' '], "-").as_str() {
            "dark" | "mocha" | "catppuccin" | "catppuccin-mocha" => Self::CatppuccinMocha,
            "macchiato" | "catppuccin-macchiato" => Self::CatppuccinMacchiato,
            "frappe" | "catppuccin-frappe" => Self::CatppuccinFrappe,
            "light" | "latte" | "catppuccin-latte" => Self::CatppuccinLatte,
            "tokyo" | "tokyonight" | "tokyo-night" => Self::TokyoNight,
            "tokyo-storm" | "tokyo-night-storm" => Self::TokyoNightStorm,
            "nord" | "nordic" => Self::Nord,
            "dracula" => Self::Dracula,
            "gruvbox" | "gruvbox-dark" => Self::Gruvbox,
            "one-dark" | "onedark" | "atom" => Self::OneDark,
            "solarized" | "solarized-dark" => Self::SolarizedDark,
            "solarized-light" => Self::SolarizedLight,
            "github" | "github-dark" => Self::GitHubDark,
            "github-light" => Self::GitHubLight,
            "everforest" | "everforest-dark" => Self::Everforest,
            "kanagawa" => Self::Kanagawa,
            "rose-pine" | "rosepine" => Self::RosePine,
            "rose-pine-dawn" | "rosepine-dawn" => Self::RosePineDawn,
            "material" | "material-dark" => Self::Material,
            "palenight" | "palenight-dark" => Self::Palenight,
            "cyberpunk" | "cyberpunk-neon" => Self::Cyberpunk,
            "contrast" | "high-contrast" | "oled" => Self::Contrast,
            _ => Self::CatppuccinMocha,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "catppuccin mocha",
            Self::CatppuccinMacchiato => "catppuccin macchiato",
            Self::CatppuccinFrappe => "catppuccin frappe",
            Self::CatppuccinLatte => "catppuccin latte",
            Self::TokyoNight => "tokyo night",
            Self::TokyoNightStorm => "tokyo night storm",
            Self::Nord => "nord",
            Self::Dracula => "dracula",
            Self::Gruvbox => "gruvbox dark",
            Self::OneDark => "one dark",
            Self::SolarizedDark => "solarized dark",
            Self::SolarizedLight => "solarized light",
            Self::GitHubDark => "github dark",
            Self::GitHubLight => "github light",
            Self::Everforest => "everforest",
            Self::Kanagawa => "kanagawa",
            Self::RosePine => "rose pine",
            Self::RosePineDawn => "rose pine dawn",
            Self::Material => "material dark",
            Self::Palenight => "palenight",
            Self::Cyberpunk => "cyberpunk",
            Self::Contrast => "high contrast",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "warm dark  ·  mauve accent  ·  easy on the eyes",
            Self::CatppuccinMacchiato => "warm dark  ·  blue mauve  ·  a touch brighter",
            Self::CatppuccinFrappe => "warm dark  ·  soft purple  ·  gentler contrast",
            Self::CatppuccinLatte => "warm light  ·  purple accent  ·  paper-like clarity",
            Self::TokyoNight => "cool dark  ·  blue accent  ·  neon city vibes",
            Self::TokyoNightStorm => "cool dark  ·  softer blue  ·  stormy night",
            Self::Nord => "arctic dark  ·  cyan accent  ·  calm and icy",
            Self::Dracula => "dark  ·  pink accent  ·  the classic vampire palette",
            Self::Gruvbox => "retro dark  ·  warm yellow  ·  vintage terminal",
            Self::OneDark => "dark  ·  blue accent  ·  the atom editor look",
            Self::SolarizedDark => "dark  ·  blue accent  ·  precision colors",
            Self::SolarizedLight => "light  ·  blue accent  ·  sunny terminal",
            Self::GitHubDark => "dark  ·  blue accent  ·  familiar code review",
            Self::GitHubLight => "light  ·  blue accent  ·  clean and bright",
            Self::Everforest => "green dark  ·  forest tones  ·  easy on the eyes",
            Self::Kanagawa => "ink dark  ·  wave blue  ·  ukiyo-e inspired",
            Self::RosePine => "mauve dark  ·  pine accent  ·  soft and elegant",
            Self::RosePineDawn => "mauve light  ·  pine accent  ·  morning light",
            Self::Material => "dark  ·  indigo accent  ·  material design",
            Self::Palenight => "dark  ·  violet accent  ·  deep purple glow",
            Self::Cyberpunk => "black  ·  neon cyan  ·  night city lights",
            Self::Contrast => "true black  ·  yellow accent  ·  maximum contrast",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "dark",
            Self::CatppuccinMacchiato => "macchiato",
            Self::CatppuccinFrappe => "frappe",
            Self::CatppuccinLatte => "light",
            Self::TokyoNight => "tokyo-night",
            Self::TokyoNightStorm => "tokyo-storm",
            Self::Nord => "nord",
            Self::Dracula => "dracula",
            Self::Gruvbox => "gruvbox",
            Self::OneDark => "one-dark",
            Self::SolarizedDark => "solarized",
            Self::SolarizedLight => "solarized-light",
            Self::GitHubDark => "github-dark",
            Self::GitHubLight => "github-light",
            Self::Everforest => "everforest",
            Self::Kanagawa => "kanagawa",
            Self::RosePine => "rose-pine",
            Self::RosePineDawn => "rose-pine-dawn",
            Self::Material => "material",
            Self::Palenight => "palenight",
            Self::Cyberpunk => "cyberpunk",
            Self::Contrast => "contrast",
        }
    }

    pub const ALL: [ThemeKind; 22] = [
        Self::CatppuccinMocha,
        Self::CatppuccinMacchiato,
        Self::CatppuccinFrappe,
        Self::CatppuccinLatte,
        Self::TokyoNight,
        Self::TokyoNightStorm,
        Self::Nord,
        Self::Dracula,
        Self::Gruvbox,
        Self::OneDark,
        Self::SolarizedDark,
        Self::SolarizedLight,
        Self::GitHubDark,
        Self::GitHubLight,
        Self::Everforest,
        Self::Kanagawa,
        Self::RosePine,
        Self::RosePineDawn,
        Self::Material,
        Self::Palenight,
        Self::Cyberpunk,
        Self::Contrast,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorDepth {
    Monochrome,
    Ansi16,
    Ansi256,
    Truecolor,
}

/// sniff the environment. `NO_COLOR` wins, always. someone setting it is
/// telling you something, and ignoring that is how you lose friends.
pub fn detect_depth() -> ColorDepth {
    if env::var_os("NO_COLOR").is_some() {
        return ColorDepth::Monochrome;
    }
    let colorterm = env::var("COLORTERM").unwrap_or_default();
    if colorterm.contains("truecolor") || colorterm.contains("24bit") {
        return ColorDepth::Truecolor;
    }
    let term = env::var("TERM").unwrap_or_default();
    if term.contains("256color") || term.contains("direct") {
        return ColorDepth::Ansi256;
    }
    if term == "dumb" || term.is_empty() {
        return ColorDepth::Monochrome;
    }
    ColorDepth::Ansi16
}

fn quantize(c: Color, depth: ColorDepth) -> Color {
    match c {
        Color::Rgb(r, g, b) => match depth {
            ColorDepth::Truecolor => c,
            ColorDepth::Monochrome => {
                let luma = 0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32;
                if luma > 128.0 {
                    Color::White
                } else {
                    Color::Black
                }
            }
            _ => Color::Indexed(rgb_to_256(r, g, b)),
        },
        _ => c,
    }
}

/// map rgb onto the xterm cube. the math is boring on purpose; clever color
/// quantization is a great way to make everything look slightly wrong.
pub fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    fn level(v: u8) -> u8 {
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            16 + ((v as u16 - 35) / 40).min(5) as u8
        }
    }
    let (lr, lg, lb) = (level(r), level(g), level(b));
    if lr == lg && lg == lb && r == g && g == b {
        if r < 8 {
            16
        } else if r > 248 {
            231
        } else {
            (232 + ((r as u16 - 8) / 10).min(23)) as u8
        }
    } else {
        16 + 36 * lr + 6 * lg + lb
    }
}

pub fn palette(kind: ThemeKind, depth: ColorDepth) -> Palette {
    let p = match kind {
        ThemeKind::CatppuccinMocha => palette!(
            "1e1e2e", "181825", "313244", "45475a", "45475a", "cdd6f4", "a6adc8", "7f849c",
            "cba6f7", "b4befe", "a6e3a1", "f9e2af", "f38ba8"
        ),
        ThemeKind::CatppuccinMacchiato => palette!(
            "24273a", "1e2030", "363a4f", "494d64", "494d64", "cad3f5", "a5adcb", "8087a2",
            "b7bdf8", "8aadf4", "a6da95", "eed49f", "ed8796"
        ),
        ThemeKind::CatppuccinFrappe => palette!(
            "303446", "292c3c", "414559", "51576d", "51576d", "c6d0f5", "a5adce", "838ba7",
            "ca9ee6", "8caaee", "a6d189", "e5c890", "ea999c"
        ),
        ThemeKind::CatppuccinLatte => palette!(
            "eff1f5", "e6e9ef", "ccd0da", "bcc0cc", "bcc0cc", "4c4f69", "6c6f85", "9ca0b0",
            "8839ef", "7287fd", "40a02b", "df8e1d", "d20f39"
        ),
        ThemeKind::TokyoNight => palette!(
            "1a1b26", "16161e", "24283b", "414868", "414868", "c0caf5", "a9b1d6", "565f89",
            "7aa2f7", "bb9af7", "9ece6a", "e0af68", "f7768e"
        ),
        ThemeKind::TokyoNightStorm => palette!(
            "24283b", "1f2335", "292e42", "3b4261", "3b4261", "c0caf5", "a9b1d6", "565f89",
            "7aa2f7", "bb9af7", "9ece6a", "e0af68", "f7768e"
        ),
        ThemeKind::Nord => palette!(
            "2e3440", "292e39", "3b4252", "434c5e", "434c5e", "eceff4", "d8dee9", "81a1c1",
            "88c0d0", "8fbcbb", "a3be8c", "ebcb8b", "bf616a"
        ),
        ThemeKind::Dracula => palette!(
            "282a36", "21222c", "44475a", "6272a4", "6272a4", "f8f8f2", "bd93f9", "6272a4",
            "ff79c6", "8be9fd", "50fa7b", "f1fa8c", "ff5555"
        ),
        ThemeKind::Gruvbox => palette!(
            "282828", "1d2021", "3c3836", "504945", "504945", "ebdbb2", "d5c4a1", "a89984",
            "b8bb26", "83a598", "b8bb26", "fabd2f", "fb4934"
        ),
        ThemeKind::OneDark => palette!(
            "282c34", "21252b", "3f4451", "4f5666", "4f5666", "abb2bf", "98a4b6", "7f848e",
            "61afef", "c678dd", "98c379", "e5c07b", "e06c75"
        ),
        ThemeKind::SolarizedDark => palette!(
            "002b36", "073642", "073642", "586e75", "586e75", "eee8d5", "93a1a1", "657b83",
            "268bd2", "6c71c4", "859900", "b58900", "dc322f"
        ),
        ThemeKind::SolarizedLight => palette!(
            "fdf6e3", "eee8d5", "eee8d5", "93a1a1", "93a1a1", "073642", "586e75", "657b83",
            "268bd2", "6c71c4", "859900", "b58900", "dc322f"
        ),
        ThemeKind::GitHubDark => palette!(
            "0d1117", "161b22", "21262d", "30363d", "30363d", "e6edf3", "c9d1d9", "8b949e",
            "58a6ff", "bc8cff", "3fb950", "d29922", "f85149"
        ),
        ThemeKind::GitHubLight => palette!(
            "ffffff", "f6f8fa", "eaeef2", "d0d7de", "d0d7de", "1f2328", "424a53", "656d76",
            "0969da", "8250df", "1a7f37", "9a6700", "cf222e"
        ),
        ThemeKind::Everforest => palette!(
            "2d353b", "272e33", "343f44", "485658", "485658", "d3c6aa", "c7b98f", "8da101",
            "a7c080", "83c092", "a7c080", "e5c07b", "e67e80"
        ),
        ThemeKind::Kanagawa => palette!(
            "1f1f28", "16161d", "2a2a37", "363646", "363646", "dcd7ba", "c8c093", "727169",
            "7e9cd8", "957fb8", "98bb6c", "e0c68c", "e46876"
        ),
        ThemeKind::RosePine => palette!(
            "191724", "1f1d2e", "26233a", "524f67", "524f67", "e0def4", "c4c0da", "6e6a86",
            "c4a7e7", "ebbcba", "31748f", "f6c177", "eb6f92"
        ),
        ThemeKind::RosePineDawn => palette!(
            "faf4ed", "fffaf3", "f2e9e1", "d8cdd0", "d8cdd0", "575279", "6e6a86", "797593",
            "907aa9", "d7827e", "286983", "ea9a97", "b4637a"
        ),
        ThemeKind::Material => palette!(
            "263238", "212c31", "37474f", "546e7a", "546e7a", "eceff1", "cfd8dc", "90a4ae",
            "82aaff", "c792ea", "c3e88d", "ffcb6b", "f07178"
        ),
        ThemeKind::Palenight => palette!(
            "292d3e", "242837", "343950", "4a5171", "4a5171", "a6accd", "9195b8", "676e95",
            "82aaff", "c792ea", "c3e88d", "ffcb6b", "f07178"
        ),
        ThemeKind::Cyberpunk => palette!(
            "0a0a12", "0d0d18", "161625", "232340", "232340", "e0e0ff", "a0a0d0", "7070a0",
            "00e5ff", "ff2a6d", "39ff14", "fcee0a", "ff003c"
        ),
        ThemeKind::Contrast => palette!(
            "000000", "080808", "141414", "505050", "b4b4b4", "ffffff", "dcdcdc", "a0a0a0",
            "ffe650", "78dcff", "5aff8c", "ffbe3c", "ff5a5a"
        ),
    };
    if depth == ColorDepth::Truecolor {
        return p;
    }
    Palette {
        bg: quantize(p.bg, depth),
        mantle: quantize(p.mantle, depth),
        surface: quantize(p.surface, depth),
        surface1: quantize(p.surface1, depth),
        border: quantize(p.border, depth),
        text: quantize(p.text, depth),
        subtext0: quantize(p.subtext0, depth),
        muted: quantize(p.muted, depth),
        accent: quantize(p.accent, depth),
        secondary_accent: quantize(p.secondary_accent, depth),
        success: quantize(p.success, depth),
        warn: quantize(p.warn, depth),
        error: quantize(p.error, depth),
    }
}

/// blend two colors for the state fade. linear rgb interpolation, because
/// doing it in gamma space is a rabbit hole nobody asked us to climb.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let (ar, ag, ab) = rgb_of(a);
    let (br, bg, bb) = rgb_of(b);
    let t = t.clamp(0.0, 1.0);
    Color::Rgb(
        crate::anim::lerp(ar as f32, br as f32, t) as u8,
        crate::anim::lerp(ag as f32, bg as f32, t) as u8,
        crate::anim::lerp(ab as f32, bb as f32, t) as u8,
    )
}

/// ratatui and crossterm agree on colors in principle, not in types. this is
/// the one bridge the cli needs so `--help` can wear the configured palette.
pub fn to_crossterm(c: Color) -> crossterm::style::Color {
    use crossterm::style::Color as Ct;
    match c {
        Color::Rgb(r, g, b) => Ct::Rgb { r, g, b },
        Color::Indexed(i) => Ct::AnsiValue(i),
        other => {
            let (r, g, b) = rgb_of(other);
            Ct::Rgb { r, g, b }
        }
    }
}

pub fn rgb_of(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        Color::Gray => (128, 128, 128),
        Color::DarkGray => (64, 64, 64),
        Color::Red => (200, 60, 60),
        Color::Green => (60, 200, 60),
        Color::Yellow => (220, 220, 60),
        Color::Blue => (60, 100, 220),
        Color::Magenta => (200, 60, 200),
        Color::Cyan => (60, 200, 200),
        Color::LightRed => (255, 120, 120),
        Color::LightGreen => (120, 255, 120),
        Color::LightYellow => (255, 255, 120),
        Color::LightBlue => (120, 160, 255),
        Color::LightMagenta => (255, 120, 255),
        Color::LightCyan => (120, 255, 255),
        Color::Indexed(i) => {
            let i = i as u16;
            if i < 16 {
                let base = [
                    (0, 0, 0),
                    (128, 0, 0),
                    (0, 128, 0),
                    (128, 128, 0),
                    (0, 0, 128),
                    (128, 0, 128),
                    (0, 128, 128),
                    (192, 192, 192),
                    (128, 128, 128),
                    (255, 0, 0),
                    (0, 255, 0),
                    (255, 255, 0),
                    (0, 0, 255),
                    (255, 0, 255),
                    (0, 255, 255),
                    (255, 255, 255),
                ];
                base.get(i as usize).copied().unwrap_or((128, 128, 128))
            } else if (16..232).contains(&i) {
                let v = i - 16;
                let cube = |n: u16| -> u8 { if n == 0 { 0 } else { (55 + n * 40) as u8 } };
                (cube(v / 36), cube((v / 6) % 6), cube(v % 6))
            } else {
                let g = 8 + (i - 232) * 10;
                (g as u8, g as u8, g as u8)
            }
        }
        Color::Reset => (128, 128, 128),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_cube_corners() {
        assert_eq!(rgb_to_256(0, 0, 0), 16);
        assert_eq!(rgb_to_256(255, 255, 255), 231);
    }

    #[test]
    fn mix_endpoints() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(100, 200, 50);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }

    #[test]
    fn palettes_distinct() {
        assert_ne!(
            rgb_of(palette(ThemeKind::CatppuccinMocha, ColorDepth::Truecolor).text),
            rgb_of(palette(ThemeKind::CatppuccinLatte, ColorDepth::Truecolor).text)
        );
    }

    #[test]
    fn all_themes_have_distinct_backgrounds() {
        let mut bgs = std::collections::HashSet::new();
        for kind in ThemeKind::ALL {
            let bg = rgb_of(palette(kind, ColorDepth::Truecolor).bg);
            assert!(bgs.insert(bg), "duplicate background for {}", kind.name());
        }
        assert_eq!(bgs.len(), ThemeKind::ALL.len());
    }

    #[test]
    fn all_keys_parse() {
        for kind in ThemeKind::ALL {
            assert_eq!(ThemeKind::parse(kind.key()), kind);
        }
    }
}
