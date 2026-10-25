//! The right panel: offline checks, the Claude and Grok reviews, and the history of the feedback loop.

use std::collections::HashSet;

use egui::{
    Align, CollapsingHeader, Color32, Frame, Layout, Margin, Pos2, Rect, RichText, ScrollArea, Sense, Shape, Stroke,
    Ui, vec2,
};

use super::widgets::{
    TOP_STRIP, bullet, button, card, chip, empty_state, pal, primary_button, score_badge, score_chart, section,
    segmented, soft_card, stat, text_button, window_drag_area,
};
use crate::analysis::{Analysis, IssueKind};
use crate::app::{Action, FeedbackTab, TattileApp};
use crate::model::{Correction, CorrectionStatus, Provider, Session};
use crate::theme::{self, Palette};

impl TattileApp {
    pub fn feedback_panel(&mut self, ui: &mut Ui, index: usize) {
        let analysis = self.highlight.get(&self.data.sessions[index].text).clone();

        let top = ui.max_rect().top();
        let strip =
            Rect::from_min_max(Pos2::new(ui.max_rect().left(), top), Pos2::new(ui.max_rect().right(), top + TOP_STRIP));
        window_drag_area(ui, strip, "feedback_drag");
        ui.add_space(9.0);
        let session = &self.data.sessions[index];
        let mut options = vec![(FeedbackTab::Checks, "Checks".to_owned())];
        for provider in Provider::ALL {
            let label = match session.latest(provider) {
                Some(it) => format!("{} {:.0}", provider.label(), it.review.score),
                None => provider.label().to_owned(),
            };
            options.push((FeedbackTab::Ai(provider), label));
        }
        options.push((FeedbackTab::History, "History".to_owned()));
        let mut tab = self.tab;
        segmented(ui, &mut tab, &options);
        self.tab = tab;
        ui.add_space(14.0);

        let mut actions: Vec<Action> = Vec::new();
        ScrollArea::vertical().id_salt("feedback_scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            match self.tab {
                FeedbackTab::Checks => self.checks_tab(ui, &analysis, &mut actions),
                FeedbackTab::Ai(provider) => self.ai_tab(ui, index, provider, &mut actions),
                FeedbackTab::History => self.history_tab(ui, index, &mut actions),
            }
        });
        let ctx = ui.ctx().clone();
        self.run_actions(index, actions, &ctx);
    }

    fn checks_tab(&self, ui: &mut Ui, analysis: &Analysis, actions: &mut Vec<Action>) {
        let p = pal(ui);
        let st = &analysis.stats;
        card(ui, |ui| {
            ui.columns(3, |cols| {
                stat(&mut cols[0], "Words", &st.words.to_string());
                stat(&mut cols[1], "Sentences", &st.sentences.to_string());
                stat(&mut cols[2], "Words per sentence", &format!("{:.0}", st.avg_sentence_words));
            });
            ui.add_space(8.0);
            ui.columns(3, |cols| {
                stat(&mut cols[0], "Reading ease", &format!("{:.0}", st.reading_ease));
                stat(&mut cols[1], "Grade", &format!("{:.0}", st.grade));
                stat(&mut cols[2], "Paragraphs", &st.paragraphs.to_string());
            });
        });

        section(ui, "What to look at · click to show or hide");
        for kind in IssueKind::ALL {
            let count = analysis.count(kind);
            let on = self.visible_checks.contains(&kind);
            let color = p.issue(kind);
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
            if response.hovered() {
                ui.painter().rect_filled(rect, 8, p.hover);
            }
            let dot = rect.left_center() + vec2(12.0, -7.0);
            if on {
                ui.painter().circle_filled(dot, 4.0, color);
            } else {
                ui.painter().circle_stroke(dot, 4.0, Stroke::new(1.2, color));
            }
            let painter = ui.painter();
            let label_color = if on { p.text } else { p.weak };
            painter.text(
                rect.left_top() + vec2(26.0, 4.0),
                egui::Align2::LEFT_TOP,
                kind.label(),
                egui::FontId::proportional(13.5),
                label_color,
            );
            let hint = super::widgets::truncated(
                ui,
                kind.hint(),
                egui::FontId::proportional(11.5),
                p.weak,
                rect.width() - 60.0,
            );
            ui.painter().galley(rect.left_top() + vec2(26.0, 21.0), hint, p.weak);
            ui.painter().text(
                rect.right_center() - vec2(10.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                count.to_string(),
                egui::FontId::proportional(13.0),
                if count > 0 { p.text2 } else { p.faint },
            );
            if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                actions.push(Action::ToggleCheck(kind));
            }
        }

        if st.words == 0 {
            return;
        }
        if analysis.issues.is_empty() {
            soft_card(ui, |ui| {
                ui.label(RichText::new("No style issues found.").color(p.success));
                ui.label(RichText::new("Ask Claude or Grok for a review of grammar and structure.").color(p.weak));
            });
            return;
        }
        section(ui, "Details · click to select in the draft");
        for issue in analysis.issues.iter().filter(|i| self.visible_checks.contains(&i.kind)).take(60) {
            if clickable_row(ui, &p, p.issue(issue.kind), &issue.message, issue.kind.label()).clicked() {
                actions.push(Action::Select(issue.range.clone()));
            }
        }
    }

    fn ai_tab(&self, ui: &mut Ui, index: usize, provider: Provider, actions: &mut Vec<Action>) {
        let p = pal(ui);
        let session = &self.data.sessions[index];
        let key = (session.id, provider);

        if self.pending.contains(&key) {
            soft_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(format!("{} is reading your draft…", provider.label())).color(p.text));
                });
                ui.label(RichText::new("This takes 20 to 90 seconds. You can keep writing.").color(p.weak));
            });
        }
        if let Some(err) = self.errors.get(&key) {
            soft_card(ui, |ui| {
                ui.label(RichText::new("The last request failed").color(p.danger));
                ui.label(RichText::new(err.as_str()).color(p.text2));
                if button(ui, "Try again").clicked() {
                    actions.push(Action::Ask(provider));
                }
            });
        }

        let Some(it_index) = session.latest_index(provider) else {
            if !self.pending.contains(&key) {
                card(ui, |ui| {
                    empty_state(
                        ui,
                        &format!("No {} review yet", provider.label()),
                        "Finish a first version, then ask for a review. You get a score, exact corrections, \
                         and a polished version to compare with.",
                    );
                    ui.vertical_centered(|ui| {
                        if primary_button(ui, &format!("Ask {}", provider.label())).clicked() {
                            actions.push(Action::Ask(provider));
                        }
                    });
                });
            }
            return;
        };

        let it = &session.iterations[it_index];
        let review = &it.review;
        card(ui, |ui| {
            ui.horizontal_top(|ui| {
                score_badge(ui, review.score, 56.0);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(RichText::new(score_label(review.score)).font(theme::heading(18.0)).color(p.text));
                    ui.label(
                        RichText::new(format!("{} · {} · {} words", it.model, it.at.format("%b %-d, %H:%M"), it.words))
                            .size(12.0)
                            .color(p.weak),
                    );
                    if let Some(prev) = session.previous_score(it_index) {
                        let delta = review.score - prev;
                        let (text, color) = if delta > 0.5 {
                            (format!("+{delta:.0} since the last review"), p.success)
                        } else if delta < -0.5 {
                            (format!("{delta:.0} since the last review"), p.danger)
                        } else {
                            ("Same score as the last review".to_owned(), p.weak)
                        };
                        ui.label(RichText::new(text).size(12.0).color(color));
                    }
                    if review.score >= self.data.goal.target_score as f32 {
                        ui.label(RichText::new("Target score reached").size(12.0).color(p.success));
                    }
                });
            });
            if session.text != it.text {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("You changed the draft after this review.").size(12.5).color(p.weak));
                    if text_button(ui, "Ask again", p.accent).clicked() {
                        actions.push(Action::Ask(provider));
                    }
                });
            }
            if !review.summary.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(review.summary.as_str()).color(p.text2));
            }
        });

        // Corrections.
        let other: HashSet<&str> = session
            .latest(provider.other())
            .map(|o| o.review.corrections.iter().map(|c| c.original.as_str()).collect())
            .unwrap_or_default();
        let open = review.corrections.iter().filter(|c| c.status == CorrectionStatus::Open).count();
        ui.horizontal(|ui| {
            section(ui, &format!("Corrections · {open} open"));
            if open > 0 {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if text_button(ui, "Apply all", p.accent)
                        .on_hover_text("Apply every open correction. You can undo it.")
                        .clicked()
                    {
                        actions.push(Action::ApplyAll(it_index));
                    }
                });
            }
        });
        if review.corrections.is_empty() {
            ui.label(RichText::new("No corrections. Well done.").color(p.success));
        }
        for (ci, c) in review.corrections.iter().enumerate() {
            let found = session.text.find(c.original.as_str());
            correction_card(ui, &p, c, other.contains(c.original.as_str()), found.is_some(), |a| {
                actions.push(match a {
                    CardAction::Apply => Action::Apply(it_index, ci),
                    CardAction::Dismiss => Action::Dismiss(it_index, ci),
                    CardAction::Find => {
                        let start = found.unwrap_or(0);
                        Action::Select(start..start + c.original.len())
                    }
                })
            });
        }

        if !review.strengths.is_empty() {
            section(ui, "What works");
            for s in &review.strengths {
                bullet(ui, "✓", p.success, s);
            }
        }
        if !review.improvements.is_empty() {
            section(ui, "What to improve");
            for s in &review.improvements {
                bullet(ui, "→", p.accent, s);
            }
        }
        if !review.patterns.is_empty() {
            section(ui, "Habits to watch");
            ui.horizontal_wrapped(|ui| {
                for pattern in &review.patterns {
                    chip(ui, pattern.as_str(), p.warning);
                }
            });
        }
        if !review.rewrite.is_empty() {
            ui.add_space(4.0);
            CollapsingHeader::new(RichText::new("Polished version").color(p.text))
                .id_salt(("rewrite", it_index))
                .default_open(false)
                .show(ui, |ui| {
                    Frame::new()
                        .fill(p.surface)
                        .stroke(Stroke::new(1.0, p.border))
                        .corner_radius(10)
                        .inner_margin(14)
                        .show(ui, |ui| {
                            ui.label(RichText::new(review.rewrite.as_str()).font(theme::serif(15.5)).color(p.text));
                        });
                    ui.horizontal(|ui| {
                        if button(ui, "Copy").clicked() {
                            actions.push(Action::Copy(review.rewrite.clone()));
                        }
                        if button(ui, "Use as my draft").on_hover_text("Replace your draft. You can undo it.").clicked()
                        {
                            actions.push(Action::UseRewrite(it_index));
                        }
                    });
                });
        }
        if !review.next_exercise.is_empty() {
            ui.add_space(4.0);
            soft_card(ui, |ui| {
                ui.label(RichText::new("Next exercise").size(12.0).color(p.weak));
                ui.label(RichText::new(review.next_exercise.as_str()).color(p.text2));
                if text_button(ui, "Practice this →", p.accent).clicked() {
                    actions.push(Action::Practice(review.next_exercise.clone()));
                }
            });
        }
    }

    fn history_tab(&self, ui: &mut Ui, index: usize, actions: &mut Vec<Action>) {
        let p = pal(ui);
        let session: &Session = &self.data.sessions[index];
        ui.label(RichText::new("Feedback loop").font(theme::heading(19.0)).color(p.text));
        ui.label(RichText::new("Each review is one round. Revise, ask again, and watch the score move.").color(p.weak));
        if session.iterations.is_empty() {
            card(ui, |ui| empty_state(ui, "No reviews yet", "The reviews of this draft appear here."));
            return;
        }
        card(ui, |ui| {
            let points: Vec<(f32, Color32)> =
                session.iterations.iter().map(|it| (it.review.score, p.provider(it.provider))).collect();
            score_chart(ui, &points, self.data.goal.target_score as f32, 110.0);
        });
        for (i, it) in session.iterations.iter().enumerate().rev() {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Round {}", i + 1)).size(12.5).color(p.weak));
                    ui.label(RichText::new(it.provider.label()).size(12.5).color(p.provider(it.provider)));
                    ui.label(RichText::new(format!("{:.0}", it.review.score)).font(theme::heading(18.0)).color(p.text));
                    if let Some(prev) = session.previous_score(i) {
                        let d = it.review.score - prev;
                        let c = if d > 0.0 {
                            p.success
                        } else if d < 0.0 {
                            p.danger
                        } else {
                            p.weak
                        };
                        ui.label(RichText::new(format!("{d:+.0}")).size(12.5).color(c));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(it.at.format("%b %-d, %H:%M").to_string()).size(12.0).color(p.faint));
                    });
                });
                if !it.review.summary.is_empty() {
                    ui.label(RichText::new(truncate_chars(&it.review.summary, 160)).size(12.5).color(p.weak));
                }
                CollapsingHeader::new(RichText::new(format!("Show this version ({} words)", it.words)).size(12.5))
                    .id_salt(("history", i))
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(RichText::new(it.text.as_str()).font(theme::serif(14.0)).color(p.text2));
                        if session.text != it.text && button(ui, "Restore this version").clicked() {
                            actions.push(Action::Restore(i));
                        }
                    });
            });
        }
    }
}

enum CardAction {
    Apply,
    Dismiss,
    Find,
}

fn correction_card(ui: &mut Ui, p: &Palette, c: &Correction, agree: bool, found: bool, mut on: impl FnMut(CardAction)) {
    if c.status != CorrectionStatus::Open {
        let label = if c.status == CorrectionStatus::Applied { "Applied" } else { "Dismissed" };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(RichText::new(format!("{label}:")).size(12.0).color(p.faint));
            ui.label(RichText::new(truncate_chars(&c.original, 40)).size(12.0).strikethrough().color(p.faint));
            ui.label(RichText::new(format!("→ {}", truncate_chars(&c.suggestion, 40))).size(12.0).color(p.faint));
        });
        return;
    }
    let kind_color = if c.is_error() { p.danger } else { p.purple };
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(12)
        .inner_margin(Margin { left: 14, right: 10, top: 10, bottom: 6 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let (dot, _) = ui.allocate_exact_size(vec2(8.0, 14.0), Sense::hover());
                ui.painter().circle_filled(dot.center(), 3.0, kind_color);
                ui.label(RichText::new(pretty_category(&c.category)).size(12.0).color(p.weak));
                if agree {
                    ui.label(RichText::new("· Claude and Grok agree").size(12.0).color(p.success));
                }
                if !found {
                    ui.label(RichText::new("· not in the draft now").size(12.0).color(p.faint));
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    RichText::new(c.original.as_str())
                        .font(theme::serif(15.0))
                        .strikethrough()
                        .color(p.danger.gamma_multiply(0.85)),
                );
                ui.label(RichText::new("→").color(p.faint));
                ui.label(RichText::new(c.suggestion.as_str()).font(theme::serif(15.0)).color(p.success));
            });
            if !c.explanation.is_empty() {
                ui.label(RichText::new(c.explanation.as_str()).size(12.5).color(p.weak));
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if found && text_button(ui, "Apply", p.accent).clicked() {
                    on(CardAction::Apply);
                }
                if found && text_button(ui, "Find", p.text2).on_hover_text("Select it in the draft").clicked() {
                    on(CardAction::Find);
                }
                if text_button(ui, "Dismiss", p.weak).clicked() {
                    on(CardAction::Dismiss);
                }
            });
        });
}

/// A row with a colored dot, a message, and a small label. The whole row is clickable.
fn clickable_row(ui: &mut Ui, p: &Palette, color: Color32, message: &str, label: &str) -> egui::Response {
    let background = ui.painter().add(Shape::Noop);
    let inner = Frame::new().inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_top(|ui| {
            let (dot, _) = ui.allocate_exact_size(vec2(10.0, 18.0), Sense::hover());
            ui.painter().circle_filled(dot.center(), 3.5, color);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(RichText::new(message).size(13.0).color(p.text2));
                ui.label(RichText::new(label).size(11.5).color(p.weak));
            });
        });
    });
    let response = inner.response.interact(Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.hovered() {
        ui.painter().set(background, Shape::rect_filled(response.rect, 8, p.hover));
    }
    response
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_owned()
    } else {
        let cut: String = text.chars().take(max_chars - 1).collect();
        format!("{}…", cut.trim_end())
    }
}

pub fn score_label(score: f32) -> &'static str {
    match score {
        s if s >= 90.0 => "Ready to publish",
        s if s >= 75.0 => "Clear, with small issues",
        s if s >= 60.0 => "Understandable, needs edits",
        _ => "Hard to follow: revise it",
    }
}

pub fn pretty_category(category: &str) -> String {
    let text = category.replace('_', " ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Other".to_owned(),
    }
}
