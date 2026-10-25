use chrono::{Days, Local};
use egui::{Color32, RichText, Ui};

use super::feedback::pretty_category;
use super::widgets::{
    bar_chart, card, count_bars, empty_state, level_bars, page, page_title, pal, score_chart, section, stat,
};
use crate::analysis::word_count;
use crate::app::TattileApp;
use crate::model::DocType;

impl TattileApp {
    pub fn progress_view(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        let today = Local::now().date_naive();
        let reviews = self.data.reviews();
        let total_words: usize = self.data.sessions.iter().map(|s| word_count(&s.text)).sum();
        let best =
            reviews.iter().map(|it| it.review.score).fold(None, |m: Option<f32>, s| Some(m.map_or(s, |m| m.max(s))));
        let avg = self.data.recent_average_score(10);
        let streak = self.data.streak(today);

        page(ui, "progress_page", |ui| {
            page_title(ui, "Progress", "See how your writing changes over time.");

            card(ui, |ui| {
                ui.columns(6, |cols| {
                    stat(&mut cols[0], "Drafts", &self.data.sessions.len().to_string());
                    stat(&mut cols[1], "Reviews", &reviews.len().to_string());
                    stat(&mut cols[2], "Words", &total_words.to_string());
                    stat(&mut cols[3], "Average", &avg.map_or("–".into(), |a| format!("{a:.0}")));
                    stat(&mut cols[4], "Best", &best.map_or("–".into(), |b| format!("{b:.0}")));
                    stat(&mut cols[5], "Streak", &format!("{streak} days"));
                });
            });
            ui.add_space(10.0);

            card(ui, |ui| {
                section(ui, "Score over time");
                if reviews.is_empty() {
                    empty_state(ui, "No reviews yet", "Ask Claude or Grok to review a draft. Each score appears here.");
                } else {
                    let points: Vec<(f32, Color32)> =
                        reviews.iter().map(|it| (it.review.score, p.provider(it.provider))).collect();
                    score_chart(ui, &points, self.data.goal.target_score as f32, 180.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("●").size(10.0).color(p.claude));
                        ui.label(RichText::new("Claude").size(12.0).color(p.weak));
                        ui.add_space(8.0);
                        ui.label(RichText::new("●").size(10.0).color(p.grok));
                        ui.label(RichText::new("Grok").size(12.0).color(p.weak));
                    });
                }
            });
            ui.add_space(10.0);

            let craft = self.data.criteria_averages(DocType::Story, 5);
            if !craft.is_empty() {
                card(ui, |ui| {
                    section(ui, "Story craft · average of the last 5 story reviews, out of 10");
                    level_bars(ui, &craft, 10.0);
                });
                ui.add_space(10.0);
            }

            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    section(ui, "Words per day · last 14 days");
                    let values: Vec<(String, f32)> = (0..14u64)
                        .rev()
                        .map(|back| {
                            let day = today - Days::new(back);
                            (day.format("%d").to_string(), self.data.words_on(day) as f32)
                        })
                        .collect();
                    bar_chart(ui, &values, self.data.goal.daily_words as f32, p.accent, 170.0);
                });
                card(&mut cols[1], |ui| {
                    section(ui, "Mistake journal");
                    let categories = self.data.category_counts();
                    if categories.is_empty() {
                        ui.label(
                            RichText::new("Your most common mistake types appear here after a review.").color(p.weak),
                        );
                    } else {
                        let items: Vec<(String, usize)> =
                            categories.into_iter().take(7).map(|(c, n)| (pretty_category(&c), n)).collect();
                        count_bars(ui, &items, p.danger.gamma_multiply(0.75));
                    }
                    let patterns = self.data.top_patterns(5);
                    if !patterns.is_empty() {
                        section(ui, "Recurring habits");
                        for (pattern, n) in patterns {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(format!("{n}×")).color(p.warning));
                                ui.label(RichText::new(pattern).color(p.text2));
                            });
                        }
                    }
                });
            });
        });
    }
}
