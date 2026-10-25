//! The right panel: offline checks, the Claude and Grok reviews, and the history of the feedback loop.

use std::collections::HashSet;

use egui::{Align, CollapsingHeader, Color32, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui, vec2};

use super::sidebar::truncate;
use super::widgets::{
    bullet, card, chip, empty_state, link_button, pal, primary_button, score_badge, score_chart, section, soft_button,
    tinted_card,
};
use crate::analysis::{Analysis, IssueKind};
use crate::app::{Action, FeedbackTab, TattileApp};
use crate::model::{Correction, CorrectionStatus, Provider, Session};
use crate::theme::{self, Palette};

impl TattileApp {
    pub fn feedback_panel(&mut self, ui: &mut Ui, index: usize) {
        let p = pal(ui);
        let analysis = self.highlight.get(&self.data.sessions[index].text).clone();
        let session = &self.data.sessions[index];

        // Tabs.
        let mut tab = self.tab;
        Frame::new().fill(p.border.gamma_multiply(0.5)).corner_radius(10).inner_margin(3).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let issues = analysis.issues.len();
                let checks = if issues > 0 { format!("Checks · {issues}") } else { "Checks".to_owned() };
                tab_button(ui, &p, &mut tab, FeedbackTab::Checks, &checks);
                for provider in Provider::ALL {
                    let label = match session.latest(provider) {
                        Some(it) => format!("{} · {:.0}", provider.label(), it.review.score),
                        None => provider.label().to_owned(),
                    };
                    tab_button(ui, &p, &mut tab, FeedbackTab::Ai(provider), &label);
                }
                tab_button(ui, &p, &mut tab, FeedbackTab::History, "History");
            });
        });
        self.tab = tab;
        ui.add_space(12.0);

        let mut actions: Vec<Action> = Vec::new();
        ScrollArea::vertical().id_salt("feedback_scroll").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
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
        ui.label(RichText::new("Writing checks").font(theme::heading(19.0)));
        ui.label(RichText::new("Instant and offline. The highlights change as you type.").color(p.weak));

        ui.columns(3, |cols| {
            mini_stat(&mut cols[0], "Words", &st.words.to_string());
            mini_stat(&mut cols[1], "Sentences", &st.sentences.to_string());
            mini_stat(&mut cols[2], "Words / sentence", &format!("{:.1}", st.avg_sentence_words));
        });
        ui.columns(3, |cols| {
            mini_stat(&mut cols[0], "Reading ease", &format!("{:.0}", st.reading_ease));
            mini_stat(&mut cols[1], "Grade", &format!("{:.1}", st.grade));
            mini_stat(&mut cols[2], "Paragraphs", &st.paragraphs.to_string());
        });

        section(ui, "What to look at");
        for kind in IssueKind::ALL {
            let count = analysis.count(kind);
            let on = self.visible_checks.contains(&kind);
            let color = p.issue(kind);
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 8, p.text.gamma_multiply(0.04));
            }
            let dot = rect.left_center() + vec2(10.0, -6.0);
            if on {
                ui.painter().circle_filled(dot, 5.0, color);
            } else {
                ui.painter().circle_stroke(dot, 5.0, Stroke::new(1.5, color));
            }
            let text_color = if on { p.text } else { p.weak };
            ui.painter().text(
                rect.left_top() + vec2(24.0, 4.0),
                egui::Align2::LEFT_TOP,
                kind.label(),
                egui::FontId::proportional(14.0),
                text_color,
            );
            ui.painter().text(
                rect.left_top() + vec2(24.0, 22.0),
                egui::Align2::LEFT_TOP,
                kind.hint(),
                egui::FontId::proportional(11.5),
                p.weak,
            );
            ui.painter().text(
                rect.right_center() - vec2(8.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                count.to_string(),
                theme::heading(15.0),
                if count > 0 { color } else { p.weak },
            );
            if resp.on_hover_text("Click to show or hide these highlights").clicked() {
                actions.push(Action::ToggleCheck(kind));
            }
        }

        if st.words == 0 {
            return;
        }
        if analysis.issues.is_empty() {
            tinted_card(ui, p.success, |ui| {
                ui.label(RichText::new("✓ No style issues found.").strong().color(p.success));
                ui.label("Ask Claude or Grok for a deeper review of grammar and structure.");
            });
            return;
        }
        section(ui, "Details · click to select in the draft");
        for issue in analysis.issues.iter().filter(|i| self.visible_checks.contains(&i.kind)).take(60) {
            let color = p.issue(issue.kind);
            let resp = Frame::new()
                .fill(p.card)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(8)
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("●").color(color));
                        ui.label(RichText::new(issue.kind.label()).size(12.0).color(color));
                    });
                    ui.label(RichText::new(&issue.message).size(13.0));
                })
                .response
                .interact(Sense::click());
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                actions.push(Action::Select(issue.range.clone()));
            }
        }
    }

    fn ai_tab(&self, ui: &mut Ui, index: usize, provider: Provider, actions: &mut Vec<Action>) {
        let p = pal(ui);
        let session = &self.data.sessions[index];
        let key = (session.id, provider);
        let color = p.provider(provider);

        if self.pending.contains(&key) {
            tinted_card(ui, color, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(format!("{} is reading your draft…", provider.label())).strong());
                });
                ui.label(RichText::new("This takes 20 to 90 seconds. You can keep writing.").color(p.weak));
            });
        }
        if let Some(err) = self.errors.get(&key) {
            tinted_card(ui, p.danger, |ui| {
                ui.label(RichText::new("The last request failed").strong().color(p.danger));
                ui.label(err.as_str());
                if ui.button("Try again").clicked() {
                    actions.push(Action::Ask(provider));
                }
            });
        }

        let Some(it_index) = session.latest_index(provider) else {
            if !self.pending.contains(&key) {
                card(ui, |ui| {
                    empty_state(
                        ui,
                        "💬",
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
            ui.horizontal(|ui| {
                score_badge(ui, review.score, 68.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(score_label(review.score)).font(theme::heading(17.0)));
                    ui.label(
                        RichText::new(format!("{} · {} · {} words", it.model, it.at.format("%b %-d, %H:%M"), it.words))
                            .size(11.5)
                            .color(p.weak),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if let Some(prev) = session.previous_score(it_index) {
                            let delta = review.score - prev;
                            let (text, c) = if delta > 0.5 {
                                (format!("▲ +{delta:.0} since the last review"), p.success)
                            } else if delta < -0.5 {
                                (format!("▼ {delta:.0} since the last review"), p.danger)
                            } else {
                                ("No change since the last review".to_owned(), p.weak)
                            };
                            chip(ui, text, c);
                        }
                        if review.score >= self.data.goal.target_score as f32 {
                            chip(ui, "🎯 Goal reached", p.success);
                        }
                    });
                });
            });
            if session.text != it.text {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("You changed the draft after this review.").size(12.5).color(p.weak));
                    if link_button(ui, "Ask again →", p.accent).clicked() {
                        actions.push(Action::Ask(provider));
                    }
                });
            }
            if !review.summary.is_empty() {
                ui.add_space(4.0);
                ui.label(review.summary.as_str());
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
                    if ui
                        .small_button("Apply all")
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
            CollapsingHeader::new(RichText::new("Polished version").strong())
                .id_salt(("rewrite", it_index))
                .default_open(false)
                .show(ui, |ui| {
                    Frame::new().fill(p.paper).corner_radius(8).inner_margin(12).show(ui, |ui| {
                        ui.label(RichText::new(review.rewrite.as_str()).font(theme::serif(15.5)));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Copy").clicked() {
                            actions.push(Action::Copy(review.rewrite.clone()));
                        }
                        if ui.button("Use as my draft").on_hover_text("Replace your draft. You can undo it.").clicked()
                        {
                            actions.push(Action::UseRewrite(it_index));
                        }
                    });
                });
        }
        if !review.next_exercise.is_empty() {
            tinted_card(ui, p.success, |ui| {
                section(ui, "Next exercise");
                ui.label(review.next_exercise.as_str());
                if soft_button(ui, "Practice this →", p.success).clicked() {
                    actions.push(Action::Practice(review.next_exercise.clone()));
                }
            });
        }
    }

    fn history_tab(&self, ui: &mut Ui, index: usize, actions: &mut Vec<Action>) {
        let p = pal(ui);
        let session: &Session = &self.data.sessions[index];
        ui.label(RichText::new("Feedback loop").font(theme::heading(19.0)));
        ui.label(RichText::new("Each review is one round. Revise, ask again, and watch the score move.").color(p.weak));
        if session.iterations.is_empty() {
            card(ui, |ui| empty_state(ui, "↻", "No reviews yet", "Your reviews of this draft appear here."));
            return;
        }
        card(ui, |ui| {
            let points: Vec<(f32, Color32)> =
                session.iterations.iter().map(|it| (it.review.score, p.provider(it.provider))).collect();
            score_chart(ui, &points, self.data.goal.target_score as f32, 120.0);
        });
        for (i, it) in session.iterations.iter().enumerate().rev() {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    chip(ui, format!("Round {}", i + 1), p.weak);
                    chip(ui, it.provider.label(), p.provider(it.provider));
                    ui.label(
                        RichText::new(format!("{:.0}", it.review.score))
                            .font(theme::heading(18.0))
                            .color(p.score(it.review.score)),
                    );
                    if let Some(prev) = session.previous_score(i) {
                        let d = it.review.score - prev;
                        let c = if d > 0.0 {
                            p.success
                        } else if d < 0.0 {
                            p.danger
                        } else {
                            p.weak
                        };
                        ui.label(RichText::new(format!("{d:+.0}")).color(c));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(it.at.format("%b %-d, %H:%M").to_string()).size(11.5).color(p.weak));
                    });
                });
                ui.label(RichText::new(truncate(&it.review.summary, 160)).size(12.5).color(p.weak));
                CollapsingHeader::new(format!("Show this version ({} words)", it.words))
                    .id_salt(("history", i))
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(RichText::new(it.text.as_str()).font(theme::serif(14.0)));
                        if session.text != it.text && ui.button("Restore this version").clicked() {
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
        let (mark, label) =
            if c.status == CorrectionStatus::Applied { ("✓", "Applied") } else { ("–", "Dismissed") };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{mark} {label}:")).size(12.0).color(p.weak));
            ui.label(RichText::new(truncate(&c.original, 40)).size(12.0).strikethrough().color(p.weak));
            ui.label(RichText::new(format!("→ {}", truncate(&c.suggestion, 40))).size(12.0).color(p.weak));
        });
        return;
    }
    let color = if c.is_error() { p.danger } else { p.purple };
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(10)
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                chip(ui, pretty_category(&c.category), color);
                if agree {
                    chip(ui, "Claude + Grok agree", p.success);
                }
                if !found {
                    chip(ui, "not in the draft now", p.weak);
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new(c.original.as_str()).font(theme::serif(15.0)).strikethrough().color(p.danger));
                ui.label(RichText::new("→").color(p.weak));
                ui.label(RichText::new(c.suggestion.as_str()).font(theme::serif(15.0)).strong().color(p.success));
            });
            if !c.explanation.is_empty() {
                ui.label(RichText::new(c.explanation.as_str()).size(12.5).color(p.weak));
            }
            ui.horizontal(|ui| {
                if found && soft_button(ui, "Apply", p.accent).clicked() {
                    on(CardAction::Apply);
                }
                if found && ui.button("Find").on_hover_text("Select it in the draft").clicked() {
                    on(CardAction::Find);
                }
                if ui.button("Dismiss").clicked() {
                    on(CardAction::Dismiss);
                }
            });
        });
}

fn tab_button(ui: &mut Ui, p: &Palette, tab: &mut FeedbackTab, value: FeedbackTab, label: &str) {
    let selected = *tab == value;
    let text = RichText::new(label).size(13.0).color(if selected { p.text } else { p.weak });
    let button = egui::Button::new(text)
        .fill(if selected { p.card } else { Color32::TRANSPARENT })
        .stroke(if selected { Stroke::new(1.0, p.border) } else { Stroke::NONE })
        .corner_radius(8);
    if ui.add(button).clicked() {
        *tab = value;
    }
}

fn mini_stat(ui: &mut Ui, label: &str, value: &str) {
    let p = pal(ui);
    Frame::new().fill(p.card).stroke(Stroke::new(1.0, p.border)).corner_radius(10).inner_margin(10).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(value).font(theme::heading(18.0)));
        ui.label(RichText::new(label).size(11.0).color(p.weak));
    });
}

pub fn score_label(score: f32) -> &'static str {
    match score {
        s if s >= 90.0 => "Ready to publish",
        s if s >= 75.0 => "Clear, with small issues",
        s if s >= 60.0 => "Understandable, needs edits",
        _ => "Hard to follow — revise it",
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
