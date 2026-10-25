use chrono::Local;
use egui::{ComboBox, Frame, Margin, RichText, ScrollArea, Slider, TextEdit, Ui};

use super::widgets::{
    bar, card, page_title, pal, pill, primary_button, progress_ring, section, soft_button, tinted_card,
};
use crate::app::TattileApp;
use crate::model::{DocType, Skill};
use crate::prompts;

impl TattileApp {
    pub fn goals_view(&mut self, ui: &mut Ui) {
        let before = self.data.goal.clone();
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            Frame::new().inner_margin(Margin::symmetric(36, 30)).show(ui, |ui| {
                ui.set_max_width(1040.0);
                page_title(ui, "Goals", "Small daily practice beats rare long sessions. Set a goal that you can keep.");
                ui.columns(2, |cols| {
                    self.goal_settings(&mut cols[0]);
                    self.goal_status(&mut cols[1]);
                });
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
            section(ui, "Daily writing");
            ui.add(Slider::new(&mut goal.daily_words, 50..=1500).step_by(50.0).suffix(" words per day"));
            ui.add(Slider::new(&mut goal.weekly_sessions, 1..=14).suffix(" drafts per week"));
            ui.add(Slider::new(&mut goal.target_score, 50..=100).suffix(" target score"));
            ui.add_space(8.0);

            section(ui, "Main document type");
            ComboBox::from_id_salt("goal_doc")
                .selected_text(format!("{} {}", goal.doc_type.icon(), goal.doc_type.label()))
                .width(260.0)
                .show_ui(ui, |ui| {
                    for doc in DocType::ALL {
                        ui.selectable_value(&mut goal.doc_type, doc, format!("{} {}", doc.icon(), doc.label()));
                    }
                });
            ui.add_space(8.0);

            section(ui, "Focus areas");
            ui.label(RichText::new("The AI coach checks these first.").size(12.5).color(p.weak));
            ui.horizontal_wrapped(|ui| {
                for skill in Skill::ALL {
                    let on = goal.focus.contains(&skill);
                    if pill(ui, on, skill.label()).clicked() {
                        if on {
                            goal.focus.retain(|s| *s != skill);
                        } else {
                            goal.focus.push(skill);
                        }
                    }
                }
            });
            ui.add_space(8.0);

            section(ui, "Why I practice");
            ui.add(
                TextEdit::multiline(&mut goal.why)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("Example: I want my strategy docs to sound clear and confident to my VP."),
            );
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
            section(ui, "Today");
            ui.horizontal(|ui| {
                let color = if words >= goal.daily_words { p.success } else { p.accent };
                progress_ring(ui, words as f32 / goal.daily_words.max(1) as f32, 96.0, &words.to_string(), color);
                ui.vertical(|ui| {
                    ui.add_space(14.0);
                    let left = goal.daily_words.saturating_sub(words);
                    let line = if left == 0 {
                        "Daily goal done. Well done!".to_owned()
                    } else {
                        format!("{left} more words to reach your goal.")
                    };
                    ui.label(RichText::new(line).strong());
                    let streak_line =
                        if streak == 0 { "No streak yet.".to_owned() } else { format!("🔥 {streak}-day streak") };
                    ui.label(RichText::new(streak_line).color(p.warning));
                });
            });
            ui.add_space(10.0);

            section(ui, "Last 7 days");
            ui.label(format!("{week} of {} drafts", goal.weekly_sessions));
            bar(ui, week as f32 / goal.weekly_sessions.max(1) as f32, p.accent);
            ui.add_space(8.0);

            section(ui, "Quality (last 5 reviews)");
            match avg {
                Some(a) => {
                    ui.label(format!("Average {a:.0} of a {} target", goal.target_score));
                    bar(ui, a / goal.target_score.max(1) as f32, p.score(a));
                }
                None => {
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
            ui.add_space(12.0);
            tinted_card(ui, p.success, |ui| {
                section(ui, "Suggested by your last review");
                ui.label(exercise.as_str());
                if soft_button(ui, "Practice this →", p.success).clicked() {
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
