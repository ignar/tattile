//! Small reusable widgets: buttons, cards, icons, chips, rings, charts, and the window drag strip.

use std::f32::consts::TAU;
use std::sync::Arc;

use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align2, Button, Color32, CursorIcon, FontId, Frame, Galley, Margin, Painter, Pos2, Rect, Response, RichText, Sense,
    Shape, Stroke, Ui, ViewportCommand, vec2,
};

use crate::theme::{self, Palette};

/// Height of the top strip of each column. The window's traffic lights sit in the sidebar's strip.
pub const TOP_STRIP: f32 = 48.0;

pub fn pal(ui: &Ui) -> Palette {
    theme::palette(ui.visuals().dark_mode)
}

/// Lets the user drag the window by this rect (the app has no title bar), and zoom it by double-click.
pub fn window_drag_area(ui: &mut Ui, rect: Rect, salt: &str) {
    let response = ui.interact(rect, ui.scope_id().with(salt), Sense::click_and_drag());
    if response.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if response.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
}

/// A white card with a hairline border.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = pal(ui);
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(12)
        .inner_margin(16)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// A soft block without a border, for secondary content.
pub fn soft_card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = pal(ui);
    Frame::new()
        .fill(p.hover)
        .corner_radius(12)
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// A small colored label. It measures itself before it asks for space, so a wrapping row
/// can move it to the next line. (A `Frame` cannot do that, and it pushed rows too wide.)
pub fn chip(ui: &mut Ui, text: impl Into<String>, color: Color32) -> Response {
    let padding = vec2(8.0, 3.0);
    let wrap_width = (ui.max_rect().width() - 2.0 * padding.x).max(24.0);
    let galley = ui.painter().layout(text.into(), FontId::proportional(11.5), color, wrap_width);
    let (rect, response) = ui.allocate_exact_size(galley.size() + 2.0 * padding, Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 10, color.gamma_multiply(0.12));
        ui.painter().galley(rect.min + padding, galley, color);
    }
    response
}

/// A quiet label above a group of controls.
pub fn section(ui: &mut Ui, text: &str) {
    let p = pal(ui);
    ui.add_space(4.0);
    ui.label(RichText::new(text).size(12.0).color(p.weak));
}

/// A full page: a drag strip at the top, then a centered column that scrolls.
pub fn page(ui: &mut Ui, id: &str, add: impl FnOnce(&mut Ui)) {
    let rect = ui.max_rect().with_max_y(ui.max_rect().top() + TOP_STRIP);
    window_drag_area(ui, rect, id);
    ui.add_space(TOP_STRIP);
    egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, false]).show(ui, |ui| {
        let available = ui.available_width();
        let width = (available - 96.0).clamp(320.0, 880.0);
        ui.horizontal(|ui| {
            ui.add_space(((available - width) / 2.0).max(0.0));
            ui.vertical(|ui| {
                ui.set_width(width);
                add(ui);
                ui.add_space(40.0);
            });
        });
    });
}

pub fn page_title(ui: &mut Ui, title: &str, subtitle: &str) {
    let p = pal(ui);
    ui.label(RichText::new(title).font(theme::heading(30.0)).color(p.text));
    ui.label(RichText::new(subtitle).size(14.0).color(p.weak));
    ui.add_space(18.0);
}

pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let p = pal(ui);
    ui.add(
        Button::new(RichText::new(text).color(p.on_accent))
            .fill(p.accent)
            .stroke(Stroke::NONE)
            .corner_radius(8)
            .min_size(vec2(0.0, 30.0)),
    )
    .on_hover_cursor(CursorIcon::PointingHand)
}

/// A bordered button, like a standard macOS push button.
pub fn button(ui: &mut Ui, text: &str) -> Response {
    ui.add(Button::new(text).corner_radius(8).min_size(vec2(0.0, 30.0)))
}

/// A frameless button with colored text. It shows a soft background on hover.
pub fn text_button(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let p = pal(ui);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(13.0), color);
    let padding = vec2(9.0, 5.0);
    let (rect, response) = ui.allocate_exact_size(galley.size() + 2.0 * padding, Sense::click());
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(rect, 7, p.hover);
        }
        ui.painter().galley(rect.min + padding, galley, color);
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Write,
    Goals,
    Progress,
    Settings,
}

/// Thin line icons, drawn with the painter, so they scale and follow the theme.
pub fn paint_icon(painter: &Painter, icon: Icon, center: Pos2, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let at = |x: f32, y: f32| center + vec2(x, y);
    match icon {
        Icon::Write => {
            let (a, b) = (at(-4.0, 4.0), at(4.0, -4.0));
            let n = vec2(1.0, 1.0).normalized() * 1.9;
            let tip = at(-6.2, 6.2);
            painter.add(Shape::closed_line(vec![tip, a + n, b + n, b - n, a - n], stroke));
        }
        Icon::Goals => {
            painter.circle_stroke(center, 6.5, stroke);
            painter.circle_stroke(center, 3.2, stroke);
            painter.circle_filled(center, 1.1, color);
        }
        Icon::Progress => {
            painter.add(Shape::line(vec![at(-6.5, 4.5), at(-2.5, 0.0), at(0.5, 2.5), at(6.5, -4.5)], stroke));
            painter.line_segment([at(-6.5, 7.0), at(6.5, 7.0)], stroke);
        }
        Icon::Settings => {
            for (y, knob) in [(-4.5, 2.5), (0.0, -2.5), (4.5, 1.0)] {
                painter.line_segment([at(-6.5, y), at(6.5, y)], stroke);
                painter.circle_filled(at(knob, y), 2.2, color);
            }
        }
    }
}

/// A row in the sidebar: an icon and a label, with a soft background when selected.
pub fn nav_item(ui: &mut Ui, selected: bool, icon: Icon, label: &str) -> Response {
    let p = pal(ui);
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
    let fill = if selected {
        p.selected
    } else if response.hovered() {
        p.hover
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 8, fill);
    let color = if selected { p.text } else { p.text2 };
    paint_icon(painter, icon, rect.left_center() + vec2(18.0, 0.0), color);
    painter.text(rect.left_center() + vec2(36.0, 0.0), Align2::LEFT_CENTER, label, FontId::proportional(14.0), color);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A one-line text that ends with "…" when it is wider than `width`.
pub fn truncated(ui: &Ui, text: &str, font: FontId, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = TextWrapping::truncate_at_width(width.max(10.0));
    ui.painter().layout_job(job)
}

/// Equal-width segments that always fit the width they get.
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, value: &mut T, options: &[(T, String)]) {
    let p = pal(ui);
    let height = 30.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    ui.painter().rect_filled(rect, 9, p.hover);
    let inner = rect.shrink(3.0);
    let gap = 2.0;
    let n = options.len().max(1) as f32;
    let width = (inner.width() - gap * (n - 1.0)) / n;
    for (i, (option, label)) in options.iter().enumerate() {
        let seg = Rect::from_min_size(inner.min + vec2(i as f32 * (width + gap), 0.0), vec2(width, inner.height()));
        let response = ui.interact(seg, ui.scope_id().with(("segment", i)), Sense::click());
        let selected = *value == *option;
        if selected {
            ui.painter().rect(seg, 7, p.surface, Stroke::new(1.0, p.border), egui::StrokeKind::Inside);
        } else if response.hovered() {
            ui.painter().rect_filled(seg, 7, p.selected.gamma_multiply(0.5));
        }
        let color = if selected { p.text } else { p.weak };
        let galley = truncated(ui, label, FontId::proportional(12.5), color, seg.width() - 8.0);
        ui.painter().galley(seg.center() - galley.size() / 2.0, galley, color);
        if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            *value = *option;
        }
    }
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
    let (rect, response) = ui.allocate_exact_size(vec2(diameter, diameter), Sense::hover());
    let color = p.score(score);
    ring(ui, rect.center(), diameter / 2.0 - 2.5, score / 100.0, 3.0, color, p.border);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("{score:.0}"),
        theme::heading(diameter * 0.38),
        p.text,
    );
    response
}

pub fn progress_ring(ui: &mut Ui, fraction: f32, diameter: f32, center_text: &str, color: Color32) -> Response {
    let p = pal(ui);
    let (rect, response) = ui.allocate_exact_size(vec2(diameter, diameter), Sense::hover());
    ring(ui, rect.center(), diameter / 2.0 - 3.0, fraction, 4.0, color, p.border);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, center_text, theme::heading(diameter * 0.28), p.text);
    response
}

/// A thin horizontal progress bar.
pub fn bar(ui: &mut Ui, fraction: f32, color: Color32) {
    let p = pal(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
    ui.painter().rect_filled(rect, 2, p.border);
    let mut filled = rect;
    filled.set_width(rect.width() * fraction.clamp(0.0, 1.0));
    if filled.width() > 0.0 {
        ui.painter().rect_filled(filled, 2, color);
    }
}

/// A number with a label under it. Used in grids inside a card.
pub fn stat(ui: &mut Ui, label: &str, value: &str) {
    let p = pal(ui);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(RichText::new(value).font(theme::heading(22.0)).color(p.text));
        ui.label(RichText::new(label).size(12.0).color(p.weak));
    });
}

pub fn empty_state(ui: &mut Ui, title: &str, body: &str) {
    let p = pal(ui);
    ui.vertical_centered(|ui| {
        ui.add_space(18.0);
        ui.label(RichText::new(title).font(theme::heading(18.0)).color(p.text));
        ui.add_space(2.0);
        ui.label(RichText::new(body).color(p.weak));
        ui.add_space(10.0);
    });
}

pub fn bullet(ui: &mut Ui, mark: &str, color: Color32, text: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(RichText::new(mark).color(color));
        ui.label(text);
    });
}

/// A line chart of scores (0-100) with a dashed target line.
pub fn score_chart(ui: &mut Ui, points: &[(f32, Color32)], target: f32, height: f32) {
    let p = pal(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    let plot = Rect::from_min_max(rect.min + vec2(28.0, 8.0), rect.max - vec2(8.0, 8.0));
    let y = |s: f32| plot.bottom() - (s / 100.0) * plot.height();
    for g in [0.0, 50.0, 100.0] {
        painter.line_segment([Pos2::new(plot.left(), y(g)), Pos2::new(plot.right(), y(g))], Stroke::new(1.0, p.border));
        painter.text(
            Pos2::new(rect.left(), y(g)),
            Align2::LEFT_CENTER,
            format!("{g:.0}"),
            FontId::proportional(10.5),
            p.faint,
        );
    }
    let target_y = y(target);
    painter.add(Shape::dashed_line(
        &[Pos2::new(plot.left(), target_y), Pos2::new(plot.right(), target_y)],
        Stroke::new(1.0, p.success.gamma_multiply(0.8)),
        5.0,
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
        painter.add(Shape::line(line.clone(), Stroke::new(1.5, p.faint)));
    }
    for (pos, (_, color)) in line.iter().zip(points) {
        painter.circle_filled(*pos, 3.5, *color);
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
        let left = plot.left() + slot * i as f32 + slot * 0.2;
        let bar = Rect::from_min_max(Pos2::new(left, plot.bottom() - h), Pos2::new(left + slot * 0.6, plot.bottom()));
        let c = if *v >= goal { color } else { color.gamma_multiply(0.35) };
        painter.rect_filled(bar, 3, c);
        painter.text(
            Pos2::new(left + slot * 0.3, rect.bottom() - 2.0),
            Align2::CENTER_BOTTOM,
            label,
            FontId::proportional(10.0),
            p.faint,
        );
    }
    let gy = plot.bottom() - plot.height() * goal / max;
    painter.add(Shape::dashed_line(
        &[Pos2::new(plot.left(), gy), Pos2::new(plot.right(), gy)],
        Stroke::new(1.0, p.success.gamma_multiply(0.8)),
        5.0,
        4.0,
    ));
}

/// Horizontal bars, one per item, for counts.
pub fn count_bars(ui: &mut Ui, items: &[(String, usize)], color: Color32) {
    let p = pal(ui);
    let max = items.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1) as f32;
    let label_width = 120.0;
    for (label, n) in items {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
        let galley = truncated(ui, label, FontId::proportional(13.0), p.text2, label_width - 8.0);
        ui.painter().galley(Pos2::new(rect.left(), rect.center().y - galley.size().y / 2.0), galley, p.text2);
        let track = Rect::from_min_max(
            Pos2::new(rect.left() + label_width, rect.center().y - 3.0),
            Pos2::new(rect.right() - 28.0, rect.center().y + 3.0),
        );
        ui.painter().rect_filled(track, 3, p.border);
        let mut filled = track;
        filled.set_width(track.width() * *n as f32 / max);
        ui.painter().rect_filled(filled, 3, color);
        ui.painter().text(rect.right_center(), Align2::RIGHT_CENTER, n.to_string(), FontId::proportional(12.5), p.weak);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chips inside `horizontal_wrapped` must move to a new row instead of making the row wider than
    /// the space it got. A row that was too wide made the feedback panel grow and refuse to shrink.
    #[test]
    fn chips_wrap_inside_the_available_width() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))),
            ..Default::default()
        };
        let (mut available, mut row_width) = (0.0, 0.0);
        // The first pass loads the fonts. Measure on the second pass.
        for _ in 0..2 {
            let output = ctx.run_ui(input.clone(), |ui| {
                ui.set_max_width(240.0);
                available = ui.available_width();
                let row = ui.horizontal_wrapped(|ui| {
                    for text in [
                        "Missing articles before singular nouns",
                        "Present perfect for changes over time",
                        "Wrong preposition with time expressions",
                    ] {
                        chip(ui, text, Color32::RED);
                    }
                });
                row_width = row.response.rect.width();
            });
            output.drop_without_applying_deltas();
        }
        assert!(row_width <= available + 0.5, "the chip row is {row_width} wide, but only {available} is available");
    }

    /// The segmented control must never ask for more width than it has, at any width.
    #[test]
    fn segmented_control_fits_narrow_widths() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))),
            ..Default::default()
        };
        let options: Vec<(u8, String)> = (0..4).map(|i| (i, format!("Long label number {i}"))).collect();
        let mut value = 0u8;
        let (mut available, mut used) = (0.0, 0.0);
        for _ in 0..2 {
            let output = ctx.run_ui(input.clone(), |ui| {
                ui.set_max_width(180.0);
                available = ui.available_width();
                used = ui.scope(|ui| segmented(ui, &mut value, &options)).response.rect.width();
            });
            output.drop_without_applying_deltas();
        }
        assert!(used <= available + 0.5, "the segmented control is {used} wide, but only {available} is available");
    }
}
