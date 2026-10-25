//! Colors, fonts, and widget styles for the light and dark themes.

use std::sync::Arc;

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle, Theme, ThemePreference,
    vec2,
};

use crate::analysis::IssueKind;
use crate::model::{Provider, ThemeChoice};

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color32,
    pub sidebar: Color32,
    pub paper: Color32,
    pub card: Color32,
    pub border: Color32,
    pub control: Color32,
    pub control_hover: Color32,
    pub control_active: Color32,
    pub text: Color32,
    pub weak: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub info: Color32,
    pub purple: Color32,
    pub teal: Color32,
    pub claude: Color32,
    pub grok: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: rgb(0x17181C),
            sidebar: rgb(0x121316),
            paper: rgb(0x1E2025),
            card: rgb(0x22242A),
            border: rgb(0x34373F),
            control: rgb(0x272A31),
            control_hover: rgb(0x2F3140),
            control_active: rgb(0x3A3C55),
            text: rgb(0xE9E7E3),
            weak: rgb(0x9B9DA6),
            accent: rgb(0x9D97FF),
            on_accent: rgb(0x16142E),
            success: rgb(0x4CC38A),
            warning: rgb(0xE5A13A),
            danger: rgb(0xF0716D),
            info: rgb(0x6AA5F8),
            purple: rgb(0xC08CFF),
            teal: rgb(0x4FC1C0),
            claude: rgb(0xE08D6D),
            grok: rgb(0xA9B4C6),
        }
    } else {
        Palette {
            bg: rgb(0xF7F5F1),
            sidebar: rgb(0xEFECE6),
            paper: rgb(0xFFFFFF),
            card: rgb(0xFFFFFF),
            border: rgb(0xE3DFD7),
            control: rgb(0xFFFFFF),
            control_hover: rgb(0xF2F0FF),
            control_active: rgb(0xE5E1FF),
            text: rgb(0x1E2126),
            weak: rgb(0x6B6F78),
            accent: rgb(0x5B4FE0),
            on_accent: rgb(0xFFFFFF),
            success: rgb(0x1B8F55),
            warning: rgb(0xB86E00),
            danger: rgb(0xD1453F),
            info: rgb(0x2F6FDB),
            purple: rgb(0x8A4FE6),
            teal: rgb(0x0F8A8A),
            claude: rgb(0xC15F3C),
            grok: rgb(0x3D4A5C),
        }
    }
}

impl Palette {
    pub fn provider(&self, provider: Provider) -> Color32 {
        match provider {
            Provider::Claude => self.claude,
            Provider::Grok => self.grok,
        }
    }

    pub fn issue(&self, kind: IssueKind) -> Color32 {
        match kind {
            IssueKind::LongSentence => self.warning,
            IssueKind::Passive => self.info,
            IssueKind::Wordy | IssueKind::Complex => self.purple,
            IssueKind::Filler => self.warning,
            IssueKind::Hedge => self.teal,
            IssueKind::Repeated => self.danger,
        }
    }

    pub fn score(&self, score: f32) -> Color32 {
        match score {
            s if s >= 85.0 => self.success,
            s if s >= 70.0 => self.info,
            s if s >= 55.0 => self.warning,
            _ => self.danger,
        }
    }
}

pub fn serif_family() -> FontFamily {
    FontFamily::Name("serif".into())
}

pub fn heading_family() -> FontFamily {
    FontFamily::Name("heading".into())
}

pub fn serif(size: f32) -> FontId {
    FontId::new(size, serif_family())
}

pub fn heading(size: f32) -> FontId {
    FontId::new(size, heading_family())
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.style_mut_of(Theme::Light, |s| style(s, false));
    ctx.style_mut_of(Theme::Dark, |s| style(s, true));
}

pub fn apply_choice(ctx: &egui::Context, choice: ThemeChoice) {
    ctx.set_theme(match choice {
        ThemeChoice::System => ThemePreference::System,
        ThemeChoice::Light => ThemePreference::Light,
        ThemeChoice::Dark => ThemePreference::Dark,
    });
}

fn load_font(fonts: &mut FontDefinitions, name: &str, path: &str) -> bool {
    match std::fs::read(path) {
        Ok(bytes) => {
            fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_owned(bytes)));
            true
        }
        Err(_) => false,
    }
}

/// Uses the macOS system fonts when they exist, and falls back to the egui fonts.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let fallback = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();

    if load_font(&mut fonts, "sf", "/System/Library/Fonts/SFNS.ttf") {
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "sf".to_owned());
    }

    let mut serif = Vec::new();
    if load_font(&mut fonts, "georgia", "/System/Library/Fonts/Supplemental/Georgia.ttf") {
        serif.push("georgia".to_owned());
    }
    serif.extend(fallback.iter().cloned());

    let mut heading = Vec::new();
    if load_font(&mut fonts, "georgia-bold", "/System/Library/Fonts/Supplemental/Georgia Bold.ttf") {
        heading.push("georgia-bold".to_owned());
    }
    heading.extend(fallback);

    fonts.families.insert(serif_family(), serif);
    fonts.families.insert(heading_family(), heading);
    ctx.set_fonts(fonts);
}

fn style(s: &mut egui::Style, dark: bool) {
    let p = palette(dark);
    s.text_styles = [
        (TextStyle::Heading, FontId::new(28.0, heading_family())),
        (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into();
    s.spacing.item_spacing = vec2(8.0, 8.0);
    s.spacing.button_padding = vec2(12.0, 6.0);
    s.spacing.interact_size.y = 28.0;
    s.spacing.slider_width = 220.0;

    let v = &mut s.visuals;
    v.slider_trailing_fill = true;
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(10);
    v.extreme_bg_color = p.paper;
    v.faint_bg_color = p.control_hover;
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent.gamma_multiply(0.28);
    v.selection.stroke = Stroke::new(1.0, p.accent);

    let w = &mut v.widgets;
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.corner_radius = CornerRadius::same(8);
    for (state, fill, stroke) in [
        (&mut w.inactive, p.control, p.border),
        (&mut w.hovered, p.control_hover, p.accent.gamma_multiply(0.55)),
        (&mut w.active, p.control_active, p.accent),
        (&mut w.open, p.control_hover, p.accent.gamma_multiply(0.55)),
    ] {
        // Buttons paint `weak_bg_fill`. Slider rails and check boxes paint `bg_fill`,
        // so it must stand out from the white cards.
        state.weak_bg_fill = fill;
        state.bg_fill = if fill == p.control { p.border } else { fill };
        state.bg_stroke = Stroke::new(1.0, stroke);
        state.fg_stroke = Stroke::new(1.5, p.text);
        state.corner_radius = CornerRadius::same(8);
        state.expansion = 0.0;
    }
}
