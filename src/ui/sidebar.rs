use chrono::Local;
use egui::{Align, Align2, Color32, CursorIcon, FontId, Layout, RichText, ScrollArea, Sense, Ui, vec2};

use super::widgets::{card, nav_item, pal, progress_ring, section};
use crate::analysis::word_count;
use crate::app::{TattileApp, View};
use crate::theme;

impl TattileApp {
    pub fn sidebar(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        ui.label(RichText::new("Tattile").font(theme::heading(26.0)).color(p.text));
        ui.label(RichText::new("Write clearly. Every day.").size(12.5).color(p.weak));
        ui.add_space(18.0);

        for (view, icon, label) in [
            (View::Write, "✏", "Write"),
            (View::Goals, "🎯", "Goals"),
            (View::Progress, "📈", "Progress"),
            (View::Settings, "⚙", "Settings"),
        ] {
            if nav_item(ui, self.view == view, icon, label).clicked() {
                self.view = view;
            }
        }

        ui.add_space(14.0);
        self.today_card(ui);
        ui.add_space(14.0);

        ui.horizontal(|ui| {
            section(ui, "Drafts");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.small_button("+ New").on_hover_text("Start a new draft").clicked() {
                    let doc = self.data.goal.doc_type;
                    self.new_session(doc, None);
                }
            });
        });

        let mut order: Vec<usize> = (0..self.data.sessions.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(self.data.sessions[i].updated_at));
        let mut open: Option<usize> = None;
        let mut delete: Option<i64> = None;
        ScrollArea::vertical().id_salt("drafts").auto_shrink([false, false]).show(ui, |ui| {
            for i in order {
                let s = &self.data.sessions[i];
                let selected = self.current == Some(i) && self.view == View::Write;
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::click());
                let fill = if selected {
                    p.card
                } else if resp.hovered() {
                    p.text.gamma_multiply(0.05)
                } else {
                    Color32::TRANSPARENT
                };
                let painter = ui.painter();
                painter.rect_filled(rect, 8, fill);
                if selected {
                    painter.rect_stroke(rect, 8, egui::Stroke::new(1.0, p.border), egui::StrokeKind::Inside);
                }
                let title = truncate(&s.display_title(), 24);
                painter.text(
                    rect.left_top() + vec2(10.0, 9.0),
                    Align2::LEFT_TOP,
                    title,
                    FontId::proportional(13.5),
                    p.text,
                );
                let meta = format!("{} {} · {} words", s.doc_type.icon(), s.doc_type.label(), word_count(&s.text));
                painter.text(
                    rect.left_top() + vec2(10.0, 29.0),
                    Align2::LEFT_TOP,
                    meta,
                    FontId::proportional(11.0),
                    p.weak,
                );
                if let Some(score) = s.last_score() {
                    painter.text(
                        rect.right_center() - vec2(10.0, 0.0),
                        Align2::RIGHT_CENTER,
                        format!("{score:.0}"),
                        theme::heading(15.0),
                        p.score(score),
                    );
                }
                let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                if resp.clicked() {
                    open = Some(i);
                }
                let id = s.id;
                resp.context_menu(|ui| {
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

    fn today_card(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        let today = Local::now().date_naive();
        let words = self.data.words_on(today);
        let goal = self.data.goal.daily_words.max(1);
        let streak = self.data.streak(today);
        card(ui, |ui| {
            ui.horizontal(|ui| {
                let color = if words >= goal { p.success } else { p.accent };
                progress_ring(
                    ui,
                    words as f32 / goal as f32,
                    58.0,
                    &format!("{}%", (100 * words / goal).min(999)),
                    color,
                );
                ui.vertical(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Today").strong().color(p.text));
                    ui.label(RichText::new(format!("{words} / {goal} words")).size(12.5).color(p.weak));
                    let streak_text = match streak {
                        0 => "Start a streak today".to_owned(),
                        1 => "🔥 1-day streak".to_owned(),
                        n => format!("🔥 {n}-day streak"),
                    };
                    ui.label(RichText::new(streak_text).size(12.5).color(if streak > 0 { p.warning } else { p.weak }));
                });
            });
        });
    }
}

pub fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_owned()
    } else {
        let cut: String = text.chars().take(max_chars - 1).collect();
        format!("{}…", cut.trim_end())
    }
}
