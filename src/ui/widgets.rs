//! Small reusable widgets: cards, chips, buttons, score badges, rings, and charts.

use std::f32::consts::TAU;

use egui::{
    Align2, Button, Color32, CursorIcon, FontId, Frame, Margin, Pos2, Rect, Response, RichText, Sense, Shape, Stroke,
    Ui, vec2,
};

use crate::theme::{self, Palette};

pub fn pal(ui: &Ui) -> Palette {
    theme::palette(ui.visuals().dark_mode)
}

pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = pal(ui);
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(12)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub fn tinted_card<R>(ui: &mut Ui, color: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(color.gamma_multiply(0.08))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.30)))
        .corner_radius(12)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

pub fn chip(ui: &mut Ui, text: impl Into<String>, color: Color32) -> Response {
    Frame::new()
        .fill(color.gamma_multiply(0.14))
        .corner_radius(10)
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| ui.label(RichText::new(text.into()).size(11.5).color(color)))
        .response
}

pub fn section(ui: &mut Ui, text: &str) {
    let p = pal(ui);
    ui.label(RichText::new(text.to_uppercase()).size(11.0).strong().color(p.weak));
}

pub fn page_title(ui: &mut Ui, title: &str, subtitle: &str) {
    let p = pal(ui);
    ui.label(RichText::new(title).font(theme::heading(30.0)).color(p.text));
    ui.label(RichText::new(subtitle).size(14.5).color(p.weak));
    ui.add_space(14.0);
}

pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let p = pal(ui);
    ui.add(
        Button::new(RichText::new(text).color(p.on_accent).strong())
            .fill(p.accent)
            .stroke(Stroke::NONE)
            .corner_radius(8)
            .min_size(vec2(0.0, 32.0)),
    )
    .on_hover_cursor(CursorIcon::PointingHand)
}

pub fn soft_button(ui: &mut Ui, text: &str, color: Color32) -> Response {
    ui.add(
        Button::new(RichText::new(text).color(color))
            .fill(color.gamma_multiply(0.10))
            .stroke(Stroke::new(1.0, color.gamma_multiply(0.30)))
            .corner_radius(8),
    )
    .on_hover_cursor(CursorIcon::PointingHand)
}

pub fn link_button(ui: &mut Ui, text: &str, color: Color32) -> Response {
    ui.add(Button::new(RichText::new(text).color(color)).frame(false)).on_hover_cursor(CursorIcon::PointingHand)
}

/// A pill that looks selected or not. Used for tabs and toggles.
pub fn pill(ui: &mut Ui, selected: bool, text: &str) -> Response {
    let p = pal(ui);
    let (fill, stroke, color) = if selected {
        (p.accent.gamma_multiply(0.14), p.accent.gamma_multiply(0.55), p.accent)
    } else {
        (p.control, p.border, p.text)
    };
    ui.add(Button::new(RichText::new(text).color(color)).fill(fill).stroke(Stroke::new(1.0, stroke)).corner_radius(14))
        .on_hover_cursor(CursorIcon::PointingHand)
}

/// A row in the sidebar: an icon and a label, with a soft highlight when selected.
pub fn nav_item(ui: &mut Ui, selected: bool, icon: &str, label: &str) -> Response {
    let p = pal(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
    let fill = if selected {
        p.accent.gamma_multiply(0.14)
    } else if resp.hovered() {
        p.text.gamma_multiply(0.06)
    } else {
        Color32::TRANSPARENT
    };
    let color = if selected { p.accent } else { p.text };
    let painter = ui.painter();
    painter.rect_filled(rect, 8, fill);
    painter.text(rect.left_center() + vec2(12.0, 0.0), Align2::LEFT_CENTER, icon, FontId::proportional(15.0), color);
    painter.text(rect.left_center() + vec2(38.0, 0.0), Align2::LEFT_CENTER, label, FontId::proportional(14.5), color);
    resp.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn ring(ui: &Ui, center: Pos2, radius: f32, fraction: f32, width: f32, color: Color32, track: Color32) {
    let painter = ui.painter();
    painter.circle_stroke(center, radius, Stroke::new(width, track));
    let fraction = fraction.clamp(0.0, 1.0);
    if fraction <= 0.0 {
        return;
    }
    let steps = (64.0 * fraction).ceil().max(2.0) as usize;
    let points: Vec<Pos2> = (0..=steps)
        .map(|i| {
            let angle = -TAU / 4.0 + TAU * fraction * i as f32 / steps as f32;
            center + vec2(angle.cos(), angle.sin()) * radius
        })
        .collect();
    painter.add(Shape::line(points, Stroke::new(width, color)));
}

pub fn score_badge(ui: &mut Ui, score: f32, diameter: f32) -> Response {
    let p = pal(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(diameter, diameter), Sense::hover());
    let color = p.score(score);
    let center = rect.center();
    ui.painter().circle_filled(center, diameter / 2.0 - 2.0, color.gamma_multiply(0.10));
    ring(ui, center, diameter / 2.0 - 3.0, score / 100.0, 4.0, color, p.border);
    ui.painter().text(center, Align2::CENTER_CENTER, format!("{score:.0}"), theme::heading(diameter * 0.36), color);
    resp
}

pub fn progress_ring(ui: &mut Ui, fraction: f32, diameter: f32, center_text: &str, color: Color32) -> Response {
    let p = pal(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(diameter, diameter), Sense::hover());
    ring(ui, rect.center(), diameter / 2.0 - 4.0, fraction, 6.0, color, p.border);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, center_text, theme::heading(diameter * 0.26), p.text);
    resp
}

/// A thin horizontal progress bar.
pub fn bar(ui: &mut Ui, fraction: f32, color: Color32) {
    let p = pal(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
    ui.painter().rect_filled(rect, 3, p.border);
    let mut filled = rect;
    filled.set_width(rect.width() * fraction.clamp(0.0, 1.0));
    if filled.width() > 0.0 {
        ui.painter().rect_filled(filled, 3, color);
    }
}

pub fn stat_tile(ui: &mut Ui, label: &str, value: &str, sub: &str) {
    let p = pal(ui);
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(12)
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).size(12.0).color(p.weak));
            ui.label(RichText::new(value).font(theme::heading(24.0)).color(p.text));
            if !sub.is_empty() {
                ui.label(RichText::new(sub).size(11.5).color(p.weak));
            }
        });
}

pub fn empty_state(ui: &mut Ui, icon: &str, title: &str, body: &str) {
    let p = pal(ui);
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.label(RichText::new(icon).size(34.0));
        ui.add_space(4.0);
        ui.label(RichText::new(title).font(theme::heading(18.0)).color(p.text));
        ui.label(RichText::new(body).color(p.weak));
        ui.add_space(8.0);
    });
}

pub fn bullet(ui: &mut Ui, mark: &str, color: Color32, text: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(RichText::new(mark).color(color).strong());
        ui.label(text);
    });
}

/// A line chart of scores (0-100) with a dashed target line.
pub fn score_chart(ui: &mut Ui, points: &[(f32, Color32)], target: f32, height: f32) {
    let p = pal(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    let plot = Rect::from_min_max(rect.min + vec2(30.0, 8.0), rect.max - vec2(8.0, 8.0));
    let y = |s: f32| plot.bottom() - (s / 100.0) * plot.height();
    for g in [0.0, 25.0, 50.0, 75.0, 100.0] {
        painter.line_segment([Pos2::new(plot.left(), y(g)), Pos2::new(plot.right(), y(g))], Stroke::new(1.0, p.border));
        painter.text(
            Pos2::new(rect.left() + 2.0, y(g)),
            Align2::LEFT_CENTER,
            format!("{g:.0}"),
            FontId::proportional(10.5),
            p.weak,
        );
    }
    let target_y = y(target);
    painter.add(Shape::dashed_line(
        &[Pos2::new(plot.left(), target_y), Pos2::new(plot.right(), target_y)],
        Stroke::new(1.5, p.success.gamma_multiply(0.7)),
        6.0,
        4.0,
    ));
    painter.text(
        Pos2::new(plot.right(), target_y - 8.0),
        Align2::RIGHT_CENTER,
        "target",
        FontId::proportional(10.5),
        p.success,
    );
    if points.is_empty() {
        return;
    }
    let x = |i: usize| {
        if points.len() == 1 {
            plot.center().x
        } else {
            plot.left() + plot.width() * i as f32 / (points.len() - 1) as f32
        }
    };
    let line: Vec<Pos2> = points.iter().enumerate().map(|(i, (s, _))| Pos2::new(x(i), y(*s))).collect();
    if line.len() > 1 {
        painter.add(Shape::line(line.clone(), Stroke::new(2.0, p.accent.gamma_multiply(0.6))));
    }
    for (pos, (_, color)) in line.iter().zip(points) {
        painter.circle_filled(*pos, 4.0, *color);
        painter.circle_stroke(*pos, 4.0, Stroke::new(1.5, p.card));
    }
}

/// Vertical bars with labels under them and a dashed goal line.
pub fn bar_chart(ui: &mut Ui, values: &[(String, f32)], goal: f32, color: Color32, height: f32) {
    let p = pal(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    let plot = Rect::from_min_max(rect.min + vec2(0.0, 8.0), rect.max - vec2(0.0, 18.0));
    let max = values.iter().map(|(_, v)| *v).fold(goal, f32::max).max(1.0);
    let slot = plot.width() / values.len().max(1) as f32;
    for (i, (label, v)) in values.iter().enumerate() {
        let h = plot.height() * v / max;
        let left = plot.left() + slot * i as f32 + slot * 0.18;
        let bar = Rect::from_min_max(Pos2::new(left, plot.bottom() - h), Pos2::new(left + slot * 0.64, plot.bottom()));
        let c = if *v >= goal { color } else { color.gamma_multiply(0.45) };
        painter.rect_filled(bar, 3, c);
        painter.text(
            Pos2::new(left + slot * 0.32, rect.bottom() - 2.0),
            Align2::CENTER_BOTTOM,
            label,
            FontId::proportional(10.0),
            p.weak,
        );
    }
    let gy = plot.bottom() - plot.height() * goal / max;
    painter.add(Shape::dashed_line(
        &[Pos2::new(plot.left(), gy), Pos2::new(plot.right(), gy)],
        Stroke::new(1.5, p.success.gamma_multiply(0.7)),
        6.0,
        4.0,
    ));
}

/// Horizontal bars, one per item, for counts.
pub fn count_bars(ui: &mut Ui, items: &[(String, usize)], color: Color32) {
    let p = pal(ui);
    let max = items.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1) as f32;
    for (label, n) in items {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(vec2(130.0, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_width(130.0);
                ui.add(egui::Label::new(RichText::new(label).size(13.0)).truncate());
            });
            let width = (ui.available_width() - 36.0).max(10.0);
            let (rect, _) = ui.allocate_exact_size(vec2(width, 10.0), Sense::hover());
            ui.painter().rect_filled(rect, 5, p.border);
            let mut filled = rect;
            filled.set_width(rect.width() * *n as f32 / max);
            ui.painter().rect_filled(filled, 5, color);
            ui.label(RichText::new(n.to_string()).size(12.5).color(p.weak));
        });
    }
}
