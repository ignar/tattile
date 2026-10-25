use chrono::Local;
use egui::text::{CCursor, CCursorRange};
use egui::{
    Align, Button, CentralPanel, Frame, Id, Layout, Margin, Panel, RichText, ScrollArea, Stroke, TextBuffer, TextEdit,
    Ui, vec2,
};

use super::editor;
use super::widgets::{chip, empty_state, pal, pill, primary_button, section, tinted_card};
use crate::analysis::word_count;
use crate::app::TattileApp;
use crate::model::{CorrectionStatus, DocType, Provider};
use crate::prompts;
use crate::theme;

const EDITOR_ID: &str = "tattile-editor";

impl TattileApp {
    pub fn write_view(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        let Some(index) = self.current.filter(|&i| i < self.data.sessions.len()) else {
            CentralPanel::default().frame(Frame::new().fill(p.bg).inner_margin(40)).show(ui, |ui| {
                empty_state(
                    ui,
                    "✏",
                    "No draft open",
                    "Start a new draft. Pick the kind of document you want to practice.",
                );
                ui.vertical_centered(|ui| {
                    for doc in DocType::ALL {
                        if ui.button(format!("{}  {}", doc.icon(), doc.label())).clicked() {
                            self.new_session(doc, None);
                        }
                    }
                });
            });
            return;
        };

        Panel::right("feedback")
            .resizable(true)
            .default_size(430.0)
            .min_size(340.0)
            .frame(Frame::new().fill(p.sidebar).inner_margin(Margin::symmetric(16, 18)))
            .show(ui, |ui| self.feedback_panel(ui, index));

        Panel::top("write_header")
            .show_separator_line(false)
            .frame(Frame::new().fill(p.bg).inner_margin(Margin { left: 28, right: 28, top: 20, bottom: 10 }))
            .show(ui, |ui| self.write_header(ui, index));

        Panel::bottom("write_footer")
            .show_separator_line(false)
            .frame(Frame::new().fill(p.bg).inner_margin(Margin { left: 28, right: 28, top: 10, bottom: 14 }))
            .show(ui, |ui| self.write_footer(ui, index));

        CentralPanel::default()
            .frame(Frame::new().fill(p.bg).inner_margin(Margin::symmetric(28, 0)))
            .show(ui, |ui| self.editor(ui, index));
    }

    fn write_header(&mut self, ui: &mut Ui, index: usize) {
        let p = pal(ui);
        ui.horizontal_wrapped(|ui| {
            for doc in DocType::ALL {
                let selected = self.data.sessions[index].doc_type == doc;
                if pill(ui, selected, &format!("{} {}", doc.icon(), doc.label())).clicked() && !selected {
                    let s = &mut self.data.sessions[index];
                    s.doc_type = doc;
                    if s.text.trim().is_empty() {
                        s.prompt = prompts::pick_assignment(doc, &s.prompt);
                    }
                    self.mark_dirty();
                }
            }
        });
        ui.add_space(8.0);

        let title = TextEdit::singleline(&mut self.data.sessions[index].title)
            .hint_text(RichText::new("Untitled draft").font(theme::heading(28.0)).color(p.weak.gamma_multiply(0.7)))
            .font(theme::heading(28.0))
            .frame(Frame::NONE)
            .desired_width(f32::INFINITY);
        if ui.add(title).changed() {
            self.mark_dirty();
        }
        ui.add_space(6.0);

        let doc = self.data.sessions[index].doc_type;
        tinted_card(ui, p.accent, |ui| {
            ui.horizontal(|ui| {
                section(ui, "Assignment");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let edit_label = if self.editing_prompt { "Done" } else { "✏ Edit" };
                    if ui.small_button(edit_label).on_hover_text("Write your own topic").clicked() {
                        self.editing_prompt = !self.editing_prompt;
                    }
                    if ui.small_button("↻ New prompt").on_hover_text("Get another assignment").clicked() {
                        let s = &mut self.data.sessions[index];
                        s.prompt = prompts::pick_assignment(doc, &s.prompt);
                        self.mark_dirty();
                    }
                });
            });
            let s = &mut self.data.sessions[index];
            if self.editing_prompt {
                let edit = TextEdit::multiline(&mut s.prompt)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text("Describe what you want to write.");
                if ui.add(edit).changed() {
                    self.mark_dirty();
                }
            } else if s.prompt.trim().is_empty() {
                ui.label(RichText::new("No assignment. Write about anything.").color(p.weak));
            } else {
                ui.label(RichText::new(s.prompt.as_str()).size(14.5).color(p.text));
            }
            ui.label(RichText::new(format!("What good looks like: {}", doc.rubric())).size(12.0).color(p.weak));
        });
    }

    fn editor(&mut self, ui: &mut Ui, index: usize) {
        let p = pal(ui);
        let editor_id = Id::unique(EDITOR_ID);
        let font = theme::serif(self.data.settings.editor_font_size);

        if let Some(range) = self.select_request.take() {
            let text = &self.data.sessions[index].text;
            if range.end <= text.len() && text.is_char_boundary(range.start) && text.is_char_boundary(range.end) {
                let a = text[..range.start].chars().count();
                let b = a + text[range].chars().count();
                let ctx = ui.ctx().clone();
                let mut state = egui::text_edit::TextEditState::load(&ctx, editor_id).unwrap_or_default();
                state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
                state.store(&ctx, editor_id);
                ctx.memory_mut(|m| m.request_focus(editor_id));
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

        let changed = {
            let cache = &mut self.highlight;
            let text = &mut self.data.sessions[index].text;
            let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
                let s = buf.as_str();
                let marks = editor::ai_marks(s, &marks_src);
                let mut job = editor::build_job(s, cache.get(s), &marks, &visible, font.clone(), &p);
                job.wrap.max_width = wrap_width;
                ui.fonts_mut(|f| f.layout_job(job))
            };
            Frame::new()
                .fill(p.paper)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(14)
                .show(ui, |ui| {
                    ScrollArea::vertical()
                        .id_salt("editor_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add_space(28.0);
                            ui.vertical_centered(|ui| {
                                let width = (ui.available_width() - 64.0).clamp(200.0, 720.0);
                                let min_height = (ui.available_height() - 40.0).max(200.0);
                                let edit = TextEdit::multiline(text)
                                    .id(editor_id)
                                    .font(font.clone())
                                    .frame(Frame::NONE)
                                    .desired_width(width)
                                    .min_size(vec2(width, min_height))
                                    .align(egui::Align2::LEFT_TOP)
                                    .lock_focus(true)
                                    .hint_text("Start writing here. Do not stop to fix mistakes. Write first, then ask for feedback.")
                                    .layouter(&mut layouter);
                                ui.add(edit).changed()
                            })
                            .inner
                        })
                        .inner
                })
                .inner
        };

        if changed {
            self.data.record_edit(index, words_before, Local::now().date_naive());
            self.mark_dirty();
        }
    }

    fn write_footer(&mut self, ui: &mut Ui, index: usize) {
        let p = pal(ui);
        let stats = self.highlight.get(&self.data.sessions[index].text).stats.clone();
        let id = self.data.sessions[index].id;
        let mut ask: Vec<Provider> = Vec::new();
        let mut undo = false;
        // The buttons go first, from the right. The stats then fill the space that is left,
        // and drop details when the column is narrow, so nothing overlaps.
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let busy = |prov| self.pending.contains(&(id, prov));
            if primary_button(ui, "✨ Ask both")
                .on_hover_text("Get two opinions. Corrections that both agree on are marked.")
                .clicked()
            {
                ask.extend(Provider::ALL);
            }
            for provider in [Provider::Grok, Provider::Claude] {
                let label = format!("Ask {}", provider.label());
                let button = Button::new(RichText::new(label).color(p.provider(provider))).min_size(vec2(0.0, 32.0));
                if ui.add_enabled(!busy(provider), button).clicked() {
                    ask.push(provider);
                }
            }
            if busy(Provider::Claude) || busy(Provider::Grok) {
                ui.spinner();
            }

            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let room = ui.available_width();
                ui.label(RichText::new(format!("{} words", stats.words)).strong().color(p.text));
                if self.undo.as_ref().is_some_and(|u| u.session_id == id) && ui.small_button("↩ Undo").clicked() {
                    undo = true;
                }
                if stats.words > 0 && room > 220.0 {
                    let ease_color = if stats.reading_ease >= 50.0 { p.success } else { p.warning };
                    let text = if room > 430.0 {
                        format!("Grade {:.0} · {}", stats.grade, stats.ease_label())
                    } else {
                        format!("Grade {:.0}", stats.grade)
                    };
                    chip(ui, text, ease_color)
                        .on_hover_text("Flesch-Kincaid grade and reading ease. Clear business writing is grade 8-10.");
                }
                if room > 560.0 {
                    let detail =
                        format!("{} sentences · {:.0} min read", stats.sentences, stats.reading_minutes.ceil());
                    ui.label(RichText::new(detail).color(p.weak));
                }
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
