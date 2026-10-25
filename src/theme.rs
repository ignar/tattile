//! Colors, fonts, and widget styles.
//!
//! The look follows the Claude desktop app: warm neutral surfaces, one clay accent,
//! a serif for reading text, and the macOS system font for controls.

use std::sync::Arc;

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Shadow, Stroke, TextStyle, Theme,
    ThemePreference, vec2,
};

use crate::analysis::IssueKind;
use crate::model::{Provider, ThemeChoice};

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// The main canvas.
    pub bg: Color32,
    pub sidebar: Color32,
    /// The feedback panel on the right.
    pub panel: Color32,
    /// Cards, inputs, and the composer bar.
    pub surface: Color32,
    /// Hovered rows and soft blocks.
    pub hover: Color32,
    /// Selected rows.
    pub selected: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text2: Color32,
    pub weak: Color32,
    pub faint: Color32,
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
    pub shadow: Color32,
    /// The toast: a dark pill in light mode, a light pill in dark mode.
    pub toast_bg: Color32,
    pub toast_text: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: rgb(0x262624),
            sidebar: rgb(0x1F1E1D),
            panel: rgb(0x232322),
            surface: rgb(0x30302E),
            hover: rgb(0x2E2E2B),
            selected: rgb(0x3A3A36),
            border: rgb(0x3A3935),
            border_strong: rgb(0x4D4C46),
            text: rgb(0xF2F0E8),
            text2: rgb(0xD4D2C9),
            weak: rgb(0xA19F96),
            faint: rgb(0x706E67),
            accent: rgb(0xD97757),
            on_accent: rgb(0xFFFFFF),
            success: rgb(0x74B486),
            warning: rgb(0xDDA14E),
            danger: rgb(0xE57A6D),
            info: rgb(0x82A9E3),
            purple: rgb(0xB49CF2),
            teal: rgb(0x6CC3BD),
            claude: rgb(0xD97757),
            grok: rgb(0xD4D2C9),
            shadow: Color32::from_black_alpha(90),
            toast_bg: rgb(0xF2F0E8),
            toast_text: rgb(0x1F1E1D),
        }
    } else {
        Palette {
            bg: rgb(0xFAF9F5),
            sidebar: rgb(0xF3F1EA),
            panel: rgb(0xF6F5F0),
            surface: rgb(0xFFFFFF),
            hover: rgb(0xEDEBE3),
            selected: rgb(0xE6E3D8),
            border: rgb(0xE6E3D9),
            border_strong: rgb(0xD3D0C5),
            text: rgb(0x1F1E1D),
            text2: rgb(0x3D3C38),
            weak: rgb(0x76746D),
            faint: rgb(0xA8A59C),
            accent: rgb(0xC96442),
            on_accent: rgb(0xFFFFFF),
            success: rgb(0x3F7F55),
            warning: rgb(0xB36B12),
            danger: rgb(0xC2483B),
            info: rgb(0x3D6FB0),
            purple: rgb(0x7B5BC7),
            teal: rgb(0x2D8580),
            claude: rgb(0xC96442),
            grok: rgb(0x3D3C38),
            shadow: Color32::from_black_alpha(22),
            toast_bg: rgb(0x1F1E1D),
            toast_text: rgb(0xF2F0E8),
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
            IssueKind::LongSentence | IssueKind::Filler | IssueKind::MonotoneRhythm | IssueKind::DialogueTag => {
                self.warning
            }
            IssueKind::Passive | IssueKind::FilterWord => self.info,
            IssueKind::Wordy | IssueKind::Complex | IssueKind::TellingEmotion => self.purple,
            IssueKind::Hedge => self.teal,
            IssueKind::Repeated | IssueKind::TenseShift | IssueKind::DialoguePunctuation | IssueKind::SlowStart => {
                self.danger
            }
            IssueKind::LongParagraph => self.warning,
            IssueKind::InlineList => self.info,
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

pub fn serif(size: f32) -> FontId {
    FontId::new(size, serif_family())
}

/// Headings use the same serif in its regular weight, as in the Claude desktop app.
pub fn heading(size: f32) -> FontId {
    serif(size)
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
    if load_font(&mut fonts, "new-york", "/System/Library/Fonts/NewYork.ttf") {
        serif.push("new-york".to_owned());
    } else if load_font(&mut fonts, "georgia", "/System/Library/Fonts/Supplemental/Georgia.ttf") {
        serif.push("georgia".to_owned());
    }
    serif.extend(fallback);
    fonts.families.insert(serif_family(), serif);
    ctx.set_fonts(fonts);
}

fn style(s: &mut egui::Style, dark: bool) {
    let p = palette(dark);
    s.text_styles = [
        (TextStyle::Heading, FontId::new(26.0, serif_family())),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, FontFamily::Monospace)),
    ]
    .into();
    s.spacing.item_spacing = vec2(8.0, 6.0);
    s.spacing.button_padding = vec2(12.0, 6.0);
    s.spacing.interact_size.y = 30.0;
    s.spacing.slider_width = 220.0;
    s.spacing.menu_margin = 6.into();
    // Development aid (debug builds): marks widgets that are wider than the space they got.
    #[cfg(debug_assertions)]
    {
        s.debug.show_expand_width = std::env::var_os("TATTILE_DEBUG_LAYOUT").is_some();
    }

    let v = &mut s.visuals;
    v.panel_fill = p.bg;
    v.window_fill = p.surface;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(12);
    v.window_shadow = Shadow { offset: [0, 8], blur: 28, spread: 0, color: p.shadow };
    v.popup_shadow = Shadow { offset: [0, 4], blur: 16, spread: 0, color: p.shadow };
    v.menu_corner_radius = CornerRadius::same(10);
    v.extreme_bg_color = p.surface;
    v.faint_bg_color = p.hover;
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.text_cursor.stroke = Stroke::new(2.0, p.accent);
    v.slider_trailing_fill = true;

    let w = &mut v.widgets;
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.corner_radius = CornerRadius::same(8);
    for (state, fill, stroke) in [
        (&mut w.inactive, p.surface, p.border),
        (&mut w.hovered, p.hover, p.border_strong),
        (&mut w.active, p.selected, p.border_strong),
        (&mut w.open, p.hover, p.border_strong),
    ] {
        // Buttons paint `weak_bg_fill`. Slider rails and check boxes paint `bg_fill`,
        // so it must stand out from the surface color.
        state.weak_bg_fill = fill;
        state.bg_fill = if fill == p.surface { p.border_strong } else { fill };
        state.bg_stroke = Stroke::new(1.0, stroke);
        state.fg_stroke = Stroke::new(1.5, p.text);
        state.corner_radius = CornerRadius::same(8);
        state.expansion = 0.0;
    }
}
