use chrono::Local;
use egui::text::{CCursor, CCursorRange};
use egui::{
    Align, Button, CentralPanel, ComboBox, Frame, Id, Layout, Margin, Panel, RichText, ScrollArea, Shadow, Stroke,
    TextBuffer, TextEdit, Ui, vec2,
};

use super::editor;
use super::widgets::{TOP_STRIP, button, empty_state, pal, primary_button, soft_card, text_button, window_drag_area};
use crate::analysis::word_count;
use crate::app::TattileApp;
use crate::model::{CorrectionStatus, DocType, Provider};
use crate::prompts;
use crate::theme;

const EDITOR_ID: &str = "tattile-editor";
/// The width of the text column, as in a comfortable book page.
const COLUMN: f32 = 700.0;

/// The text column and the space on each side of it, for the width `available`.
fn column(available: f32) -> (f32, f32) {
    let width = (available - 64.0).clamp(240.0, COLUMN);
    (width, ((available - width) / 2.0).max(0.0))
}

impl TattileApp {
    pub fn write_view(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        let Some(index) = self.current.filter(|&i| i < self.data.sessions.len()) else {
            CentralPanel::default().frame(Frame::new().fill(p.bg)).show(ui, |ui| {
                let rect = ui.max_rect().with_max_y(ui.max_rect().top() + TOP_STRIP);
                window_drag_area(ui, rect, "empty_drag");
                ui.add_space(120.0);
                empty_state(ui, "No draft open", "Pick the kind of document that you want to practice.");
                ui.vertical_centered(|ui| {
                    for doc in DocType::ALL {
                        if button(ui, doc.label()).clicked() {
                            self.new_session(doc, None);
                        }
                    }
                });
            });
            return;
        };

        Panel::right("feedback")
            .resizable(true)
            .default_size(400.0)
            .min_size(340.0)
            .max_size(620.0)
            .frame(Frame::new().fill(p.panel).inner_margin(Margin { left: 16, right: 16, top: 0, bottom: 16 }))
            .show(ui, |ui| self.feedback_panel(ui, index));

        Panel::top("write_top")
            .exact_size(TOP_STRIP)
            .show_separator_line(false)
            .frame(Frame::new().fill(p.bg).inner_margin(Margin::symmetric(20, 0)))
            .show(ui, |ui| self.write_top(ui, index));

        Panel::bottom("composer")
            .show_separator_line(false)
            .frame(Frame::new().fill(p.bg).inner_margin(Margin { left: 0, right: 0, top: 8, bottom: 18 }))
            .show(ui, |ui| self.composer(ui, index));

        CentralPanel::default().frame(Frame::new().fill(p.bg)).show(ui, |ui| self.document(ui, index));
    }

    fn write_top(&mut self, ui: &mut Ui, index: usize) {
        let p = pal(ui);
        let rect = ui.max_rect();
        window_drag_area(ui, rect, "write_drag");
        let current = self.data.sessions[index].doc_type;
        let mut doc = current;
        ui.horizontal_centered(|ui| {
            ComboBox::from_id_salt("doc_type").selected_text(doc.label()).width(160.0).show_ui(ui, |ui| {
                for d in DocType::ALL {
                    ui.selectable_value(&mut doc, d, d.label());
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let edited = self.data.sessions[index].updated_at.format("Edited %b %-d, %H:%M").to_string();
                ui.label(RichText::new(edited).size(12.0).color(p.faint));
            });
        });
        if doc != current {
            let s = &mut self.data.sessions[index];
            s.doc_type = doc;
            if s.text.trim().is_empty() {
                s.prompt = prompts::pick_assignment(doc, &s.prompt);
            }
            self.mark_dirty();
        }
    }

    fn document(&mut self, ui: &mut Ui, index: usize) {
        let p = pal(ui);
        let editor_id = Id::unique(EDITOR_ID);
        let font = theme::serif(self.data.settings.editor_font_size);

        // "Find" selects text in the editor. The click on the Find button also tells egui to take
        // the focus away from the editor, in the same frame. So wait for a frame without a click,
        // then select the text, give the editor focus, and scroll to the selection.
        let mut scroll_to: Option<(usize, usize)> = None;
        if self.select_request.is_some() {
            if ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released()) {
                ui.ctx().request_repaint();
            } else if let Some(range) = self.select_request.take() {
                let text = &self.data.sessions[index].text;
                if range.end <= text.len() && text.is_char_boundary(range.start) && text.is_char_boundary(range.end) {
                    let a = text[..range.start].chars().count();
                    let b = a + text[range].chars().count();
                    let ctx = ui.ctx().clone();
                    let mut state = egui::text_edit::TextEditState::load(&ctx, editor_id).unwrap_or_default();
                    state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
                    state.store(&ctx, editor_id);
                    ctx.memory_mut(|m| m.request_focus(editor_id));
                    scroll_to = Some((a, b));
                }
            }
        }

        // Open AI corrections from the latest review of each provider.
        let session = &self.data.sessions[index];
        let mut marks_src: Vec<(String, bool)> = Vec::new();
        for provider in Provider::ALL {
            if let Some(it) = session.latest(provider) {
                for c in it.review.corrections.iter().filter(|c| c.status == CorrectionStatus::Open) {
                    if !marks_src.iter().any(|(o, _)| *o == c.original) {
                        marks_src.push((c.original.clone(), c.is_error()));
                    }
                }
            }
        }
        let visible = self.visible_checks.clone();
        let words_before = word_count(&session.text);
        let doc = session.doc_type;
        let rubric = doc.rubric();
        let editing_prompt = self.editing_prompt;
        let view_height = ui.available_height();

        let (mut text_changed, mut meta_changed, mut toggle_edit, mut new_prompt) = (false, false, false, false);
        {
            let cache = &mut self.highlight;
            let session = &mut self.data.sessions[index];
            let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
                let s = buf.as_str();
                let marks = editor::ai_marks(s, &marks_src);
                let mut job = editor::build_job(s, cache.get(s, doc), &marks, &visible, font.clone(), &p);
                job.wrap.max_width = wrap_width;
                ui.fonts_mut(|f| f.layout_job(job))
            };
            ScrollArea::vertical().id_salt("document").auto_shrink([false, false]).show(ui, |ui| {
                let (width, side) = column(ui.available_width());
                ui.horizontal(|ui| {
                    ui.add_space(side);
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        ui.add_space(8.0);
                        let title = TextEdit::singleline(&mut session.title)
                            .hint_text(RichText::new("Untitled draft").font(theme::heading(30.0)).color(p.faint))
                            .font(theme::heading(30.0))
                            .frame(Frame::NONE)
                            .margin(Margin::ZERO)
                            .desired_width(width);
                        meta_changed |= ui.add(title).changed();
                        ui.add_space(12.0);

                        soft_card(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Assignment").size(12.0).color(p.weak));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.spacing_mut().item_spacing.x = 2.0;
                                    if text_button(ui, if editing_prompt { "Done" } else { "Edit" }, p.weak).clicked() {
                                        toggle_edit = true;
                                    }
                                    if text_button(ui, "New prompt", p.weak).clicked() {
                                        new_prompt = true;
                                    }
                                });
                            });
                            if editing_prompt {
                                let edit = TextEdit::multiline(&mut session.prompt)
                                    .desired_rows(2)
                                    .desired_width(f32::INFINITY)
                                    .hint_text("Describe what you want to write.");
                                meta_changed |= ui.add(edit).changed();
                            } else if session.prompt.trim().is_empty() {
                                ui.label(RichText::new("No assignment. Write about anything.").color(p.weak));
                            } else {
                                ui.label(RichText::new(session.prompt.as_str()).size(14.5).color(p.text2));
                            }
                            ui.add_space(2.0);
                            ui.label(RichText::new(rubric).size(12.0).color(p.weak));
                        });
                        ui.add_space(24.0);

                        let output = TextEdit::multiline(&mut session.text)
                            .id(editor_id)
                            .font(font.clone())
                            .frame(Frame::NONE)
                            .margin(Margin::ZERO)
                            .desired_width(width)
                            .min_size(vec2(width, (view_height - 300.0).max(240.0)))
                            .lock_focus(true)
                            .hint_text(
                                RichText::new(
                                    "Start writing. Do not stop to fix mistakes: write first, then ask for feedback.",
                                )
                                .font(font.clone())
                                .color(p.faint),
                            )
                            .layouter(&mut layouter)
                            .show(ui);
                        if let Some((a, b)) = scroll_to {
                            let galley = &output.galley;
                            let offset = output.galley_pos.to_vec2() - vec2(galley.rect.left(), 0.0);
                            let rect = galley
                                .pos_from_cursor(CCursor::new(a))
                                .union(galley.pos_from_cursor(CCursor::new(b)))
                                .translate(offset);
                            ui.scroll_to_rect(rect.expand(24.0), Some(Align::Center));
                        }
                        text_changed = output.response.changed();
                        ui.add_space(48.0);
                    });
                });
            });
        }

        if text_changed {
            self.data.record_edit(index, words_before, Local::now().date_naive());
            self.mark_dirty();
        }
        if meta_changed {
            self.mark_dirty();
        }
        if toggle_edit {
            self.editing_prompt = !self.editing_prompt;
        }
        if new_prompt {
            let s = &mut self.data.sessions[index];
            s.prompt = prompts::pick_assignment(s.doc_type, &s.prompt);
            self.mark_dirty();
        }
    }

    /// The bar at the bottom, like the message box in the Claude desktop app.
    fn composer(&mut self, ui: &mut Ui, index: usize) {
        let p = pal(ui);
        let session = &self.data.sessions[index];
        let stats = self.highlight.get(&session.text, session.doc_type).stats.clone();
        let id = self.data.sessions[index].id;
        let mut ask: Vec<Provider> = Vec::new();
        let mut undo = false;
        let (width, side) = column(ui.available_width());
        let inner = width - 32.0;
        ui.horizontal(|ui| {
            ui.add_space(side);
            Frame::new()
                .fill(p.surface)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(16)
                .inner_margin(Margin::symmetric(16, 10))
                .shadow(Shadow { offset: [0, 2], blur: 12, spread: 0, color: p.shadow })
                .show(ui, |ui| {
                    ui.set_width(inner);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let busy = |prov| self.pending.contains(&(id, prov));
                        if primary_button(ui, "Ask both")
                            .on_hover_text("Get two opinions. Corrections that both agree on are marked.")
                            .clicked()
                        {
                            ask.extend(Provider::ALL);
                        }
                        for provider in [Provider::Grok, Provider::Claude] {
                            let label = format!("Ask {}", provider.label());
                            let b = Button::new(label).corner_radius(8).min_size(vec2(0.0, 30.0));
                            if ui.add_enabled(!busy(provider), b).clicked() {
                                ask.push(provider);
                            }
                        }
                        if busy(Provider::Claude) || busy(Provider::Grok) {
                            ui.spinner();
                        }
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            let room = ui.available_width();
                            ui.label(RichText::new(format!("{} words", stats.words)).color(p.text2));
                            if self.undo.as_ref().is_some_and(|u| u.session_id == id)
                                && text_button(ui, "Undo", p.accent).clicked()
                            {
                                undo = true;
                            }
                            if stats.words > 0 && room > 200.0 {
                                ui.label(RichText::new(format!("Grade {:.0}", stats.grade)).color(p.weak))
                                    .on_hover_text("Flesch-Kincaid grade. Clear business writing is grade 8 to 10.");
                            }
                            if stats.words > 0 && room > 330.0 {
                                ui.label(RichText::new(stats.ease_label()).color(p.weak));
                            }
                        });
                    });
                });
        });
        if undo {
            self.undo_last_change(index);
        }
        let ctx = ui.ctx().clone();
        for provider in ask {
            self.request_review(provider, &ctx);
        }
    }
}
