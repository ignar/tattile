use chrono::Local;
use egui::{Button, ComboBox, RichText, Slider, TextEdit, Ui};

use super::widgets::{
    bar, card, page, page_title, pal, primary_button, progress_ring, section, soft_card, text_button,
};
use crate::app::TattileApp;
use crate::model::{DocType, Skill};
use crate::prompts;

impl TattileApp {
    pub fn goals_view(&mut self, ui: &mut Ui) {
        let before = self.data.goal.clone();
        page(ui, "goals_page", |ui| {
            page_title(ui, "Goals", "Small daily practice beats rare long sessions. Set a goal that you can keep.");
            ui.columns(2, |cols| {
                self.goal_settings(&mut cols[0]);
                self.goal_status(&mut cols[1]);
            });
        });
        if self.data.goal != before {
            self.mark_dirty();
        }
    }

    fn goal_settings(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        card(ui, |ui| {
            let goal = &mut self.data.goal;
            section(ui, "Words per day");
            ui.add(Slider::new(&mut goal.daily_words, 50..=1500).step_by(50.0));
            section(ui, "Drafts per week");
            ui.add(Slider::new(&mut goal.weekly_sessions, 1..=14));
            section(ui, "Target review score");
            ui.add(Slider::new(&mut goal.target_score, 50..=100));

            section(ui, "Main document type");
            ComboBox::from_id_salt("goal_doc").selected_text(goal.doc_type.label()).width(220.0).show_ui(ui, |ui| {
                for doc in DocType::ALL {
                    ui.selectable_value(&mut goal.doc_type, doc, doc.label());
                }
            });

            section(ui, "Focus areas · the AI coach checks these first");
            ui.horizontal_wrapped(|ui| {
                for skill in Skill::ALL {
                    let on = goal.focus.contains(&skill);
                    let toggle =
                        Button::new(skill.label()).selected(on).corner_radius(14).min_size(egui::vec2(0.0, 28.0));
                    if ui.add(toggle).clicked() {
                        if on {
                            goal.focus.retain(|s| *s != skill);
                        } else {
                            goal.focus.push(skill);
                        }
                    }
                }
            });

            section(ui, "Why I practice");
            ui.add(TextEdit::multiline(&mut goal.why).desired_rows(3).desired_width(f32::INFINITY).hint_text(
                RichText::new("Example: I want my strategy docs to sound clear and confident.").color(p.faint),
            ));
        });
    }

    fn goal_status(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        let today = Local::now().date_naive();
        let goal = self.data.goal.clone();
        let words = self.data.words_on(today);
        let streak = self.data.streak(today);
        let week = self.data.sessions_last_7_days(today);
        let avg = self.data.recent_average_score(5);
        let mut start = false;

        card(ui, |ui| {
            ui.horizontal(|ui| {
                let color = if words >= goal.daily_words { p.success } else { p.accent };
                progress_ring(ui, words as f32 / goal.daily_words.max(1) as f32, 84.0, &words.to_string(), color);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.add_space(16.0);
                    let left = goal.daily_words.saturating_sub(words);
                    let line = if left == 0 {
                        "You reached today's goal.".to_owned()
                    } else {
                        format!("{left} more words today")
                    };
                    ui.label(RichText::new(line).color(p.text));
                    let streak_line =
                        if streak == 0 { "No streak yet".to_owned() } else { format!("{streak}-day streak") };
                    ui.label(RichText::new(streak_line).size(12.5).color(p.weak));
                });
            });
            ui.add_space(6.0);
            section(ui, &format!("Last 7 days · {week} of {} drafts", goal.weekly_sessions));
            bar(ui, week as f32 / goal.weekly_sessions.max(1) as f32, p.accent);
            match avg {
                Some(a) => {
                    section(ui, &format!("Last 5 reviews · average {a:.0} of a {} target", goal.target_score));
                    bar(ui, a / goal.target_score.max(1) as f32, p.score(a));
                }
                None => {
                    section(ui, "Reviews");
                    ui.label(RichText::new("No reviews yet. Ask Claude or Grok on the Write page.").color(p.weak));
                }
            }
            ui.add_space(12.0);
            if primary_button(ui, &format!("Start today's practice: {}", goal.doc_type.label())).clicked() {
                start = true;
            }
        });

        let latest_exercise =
            self.data.reviews().last().map(|it| it.review.next_exercise.clone()).filter(|e| !e.trim().is_empty());
        let mut practice: Option<String> = None;
        if let Some(exercise) = latest_exercise {
            ui.add_space(10.0);
            soft_card(ui, |ui| {
                ui.label(RichText::new("Suggested by your last review").size(12.0).color(p.weak));
                ui.label(RichText::new(exercise.as_str()).color(p.text2));
                if text_button(ui, "Practice this →", p.accent).clicked() {
                    practice = Some(exercise.clone());
                }
            });
        }

        if start {
            self.new_session(goal.doc_type, Some(prompts::pick_assignment(goal.doc_type, "")));
        }
        if let Some(exercise) = practice {
            self.new_session(goal.doc_type, Some(exercise));
        }
    }
}
