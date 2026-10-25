use chrono::Local;
use egui::{
    Align2, Color32, CursorIcon, FontId, Frame, Panel, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, Ui, vec2,
};

use super::widgets::{Icon, TOP_STRIP, bar, nav_item, pal, truncated, window_drag_area};
use crate::app::{TattileApp, View};
use crate::theme;

impl TattileApp {
    pub fn sidebar(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        // The traffic lights sit in this strip, so it stays empty and drags the window.
        let top = ui.max_rect().top();
        let strip =
            Rect::from_min_max(Pos2::new(ui.max_rect().left(), top), Pos2::new(ui.max_rect().right(), top + TOP_STRIP));
        window_drag_area(ui, strip, "sidebar_drag");
        ui.add_space(TOP_STRIP);

        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("Tattile").font(theme::heading(21.0)).color(p.text));
        });
        ui.add_space(10.0);

        if new_draft_row(ui).clicked() {
            let doc = self.data.goal.doc_type;
            self.new_session(doc, None);
        }
        for (view, icon, label) in [
            (View::Write, Icon::Write, "Write"),
            (View::Goals, Icon::Goals, "Goals"),
            (View::Progress, Icon::Progress, "Progress"),
            (View::Settings, Icon::Settings, "Settings"),
        ] {
            if nav_item(ui, self.view == view, icon, label).clicked() {
                self.view = view;
            }
        }

        Panel::bottom("sidebar_today")
            .show_separator_line(false)
            .frame(Frame::new().inner_margin(egui::Margin { left: 8, right: 8, top: 10, bottom: 4 }))
            .show(ui, |ui| self.today_block(ui));

        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("Drafts").size(12.0).color(p.weak));
        });
        ui.add_space(2.0);

        let mut order: Vec<usize> = (0..self.data.sessions.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(self.data.sessions[i].updated_at));
        let mut open: Option<usize> = None;
        let mut delete: Option<i64> = None;
        ScrollArea::vertical().id_salt("drafts").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            for i in order {
                let s = &self.data.sessions[i];
                let selected = self.current == Some(i) && self.view == View::Write;
                let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                let fill = if selected {
                    p.selected
                } else if response.hovered() {
                    p.hover
                } else {
                    Color32::TRANSPARENT
                };
                ui.painter().rect_filled(rect, 8, fill);
                let score_width = if s.last_score().is_some() { 28.0 } else { 0.0 };
                let color = if selected { p.text } else { p.text2 };
                let title = truncated(
                    ui,
                    &s.display_title(),
                    FontId::proportional(13.5),
                    color,
                    rect.width() - 20.0 - score_width,
                );
                let pos = Pos2::new(rect.left() + 10.0, rect.center().y - title.size().y / 2.0);
                ui.painter().galley(pos, title, color);
                if let Some(score) = s.last_score() {
                    ui.painter().text(
                        rect.right_center() - vec2(10.0, 0.0),
                        Align2::RIGHT_CENTER,
                        format!("{score:.0}"),
                        FontId::proportional(12.0),
                        p.weak,
                    );
                }
                let response = response.on_hover_cursor(CursorIcon::PointingHand);
                if response.clicked() {
                    open = Some(i);
                }
                let id = s.id;
                response.context_menu(|ui| {
                    if ui.button("Delete draft…").clicked() {
                        delete = Some(id);
                        ui.close();
                    }
                });
            }
        });
        if let Some(i) = open {
            self.current = Some(i);
            self.view = View::Write;
            self.editing_prompt = false;
        }
        if delete.is_some() {
            self.confirm_delete = delete;
        }
    }

    fn today_block(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        let today = Local::now().date_naive();
        let words = self.data.words_on(today);
        let goal = self.data.goal.daily_words.max(1);
        let streak = self.data.streak(today);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Today").size(13.0).color(p.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("{words} / {goal} words")).size(12.0).color(p.weak));
            });
        });
        bar(ui, words as f32 / goal as f32, if words >= goal { p.success } else { p.accent });
        let streak_text = match streak {
            0 => "Write today to start a streak".to_owned(),
            1 => "1-day streak".to_owned(),
            n => format!("{n}-day streak"),
        };
        ui.label(RichText::new(streak_text).size(12.0).color(p.weak));
    }
}

/// "New draft", with a clay plus icon, like "New chat" in the Claude desktop app.
fn new_draft_row(ui: &mut Ui) -> egui::Response {
    let p = pal(ui);
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 8, p.hover);
    }
    let center = rect.left_center() + vec2(18.0, 0.0);
    ui.painter().circle_filled(center, 10.0, p.accent);
    let stroke = Stroke::new(1.6, p.on_accent);
    ui.painter().line_segment([center - vec2(4.5, 0.0), center + vec2(4.5, 0.0)], stroke);
    ui.painter().line_segment([center - vec2(0.0, 4.5), center + vec2(0.0, 4.5)], stroke);
    ui.painter().text(
        rect.left_center() + vec2(36.0, 0.0),
        Align2::LEFT_CENTER,
        "New draft",
        FontId::proportional(14.0),
        p.text,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}
