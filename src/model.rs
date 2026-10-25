//! Data that Tattile keeps on disk: goals, drafts, AI reviews, and settings.

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::analysis::word_count;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocType {
    Strategy,
    TechMemo,
    Promo,
    Story,
    Update,
    /// A reply to a question, in chat or email.
    Message,
    Free,
}

impl DocType {
    pub const ALL: [DocType; 7] = [
        DocType::Strategy,
        DocType::TechMemo,
        DocType::Promo,
        DocType::Story,
        DocType::Update,
        DocType::Message,
        DocType::Free,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DocType::Strategy => "Strategy doc",
            DocType::TechMemo => "Technical memo",
            DocType::Promo => "Promo packet",
            DocType::Story => "Story",
            DocType::Update => "Status update",
            DocType::Message => "Message",
            DocType::Free => "Free writing",
        }
    }

    /// For a message, the assignment is the question that the writer answers.
    pub fn answers_a_question(self) -> bool {
        self == DocType::Message
    }

    /// Craft criteria that the AI coach scores from 0 to 10: (name, what it means).
    pub fn criteria(self) -> &'static [(&'static str, &'static str)] {
        match self {
            DocType::Story => &[
                ("Arc", "A clear setup, rising tension, a turning point, and a resolution."),
                ("Show, don't tell", "Feelings come through actions, speech, and details, not named emotions."),
                ("Specific detail", "Concrete, sensory details instead of general words."),
                ("Rhythm", "Sentence lengths vary on purpose, and the pace fits the moment."),
                ("Consistency", "One tense and one point of view in the narration."),
                (
                    "Dialogue",
                    "Speech sounds natural and is punctuated correctly. Leave this out if there is no dialogue.",
                ),
            ],
            DocType::Message => &[
                ("Answer first", "The first one or two sentences answer the question directly."),
                ("Easy to scan", "Short paragraphs, a list for several items, and the key point easy to find."),
                ("Plain language", "Simple words and short sentences. A reader outside the team understands it."),
                ("Complete", "It gives what the reader needs to act: the reason, the risk, or the next step."),
                ("Concise", "No extra words or repeated points. Every sentence earns its place."),
                ("Tone", "Calm, direct, and respectful for this reader."),
            ],
            _ => &[],
        }
    }

    /// What good writing looks like for this type. The AI coach gets this text.
    pub fn rubric(self) -> &'static str {
        match self {
            DocType::Strategy => {
                "A clear diagnosis of the problem; explicit choices and trade-offs; one recommendation; \
                 measurable outcomes; no buzzwords. The reader is senior leadership with little time."
            }
            DocType::TechMemo => {
                "Bottom line up front; context that a smart outsider can follow; precise technical terms; \
                 risks and alternatives; short paragraphs."
            }
            DocType::Promo => {
                "Concrete impact with numbers; scope and difficulty; behaviors that match the next level; \
                 evidence instead of adjectives; active voice with clear ownership."
            }
            DocType::Story => {
                "A narrative arc (setup, tension, resolution); vivid, specific details; show, do not tell; \
                 varied sentence rhythm; consistent tense."
            }
            DocType::Update => {
                "Status first; what changed, the risks, and the asks; easy to scan; calm and direct tone; no filler."
            }
            DocType::Message => {
                "The answer comes first, in one or two sentences. Then the reason and the details, in short \
                 paragraphs or a list. Plain words. A clear next step if one is needed. The reader understands \
                 it in 30 seconds, on a phone, even when the question was complex."
            }
            DocType::Free => "Clear, correct, and natural English. Focus on grammar and natural word choice.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Skill {
    Grammar,
    Articles,
    Prepositions,
    WordChoice,
    Concision,
    Clarity,
    Structure,
    Tone,
}

impl Skill {
    pub const ALL: [Skill; 8] = [
        Skill::Grammar,
        Skill::Articles,
        Skill::Prepositions,
        Skill::WordChoice,
        Skill::Concision,
        Skill::Clarity,
        Skill::Structure,
        Skill::Tone,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Skill::Grammar => "Grammar",
            Skill::Articles => "Articles (a / the)",
            Skill::Prepositions => "Prepositions",
            Skill::WordChoice => "Natural word choice",
            Skill::Concision => "Concision",
            Skill::Clarity => "Clarity",
            Skill::Structure => "Structure",
            Skill::Tone => "Tone",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Goal {
    pub daily_words: u32,
    pub weekly_sessions: u32,
    pub target_score: u32,
    pub doc_type: DocType,
    pub focus: Vec<Skill>,
    pub why: String,
}

impl Default for Goal {
    fn default() -> Self {
        Self {
            daily_words: 300,
            weekly_sessions: 4,
            target_score: 80,
            doc_type: DocType::TechMemo,
            focus: vec![Skill::Articles, Skill::Concision],
            why: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    Claude,
    Grok,
}

impl Provider {
    pub const ALL: [Provider; 2] = [Provider::Claude, Provider::Grok];

    pub fn label(self) -> &'static str {
        match self {
            Provider::Claude => "Claude",
            Provider::Grok => "Grok",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Claude => "claude-opus-5-5",
            Provider::Grok => "grok-4.5",
        }
    }

    pub fn other(self) -> Provider {
        match self {
            Provider::Claude => Provider::Grok,
            Provider::Grok => Provider::Claude,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrectionStatus {
    #[default]
    Open,
    Applied,
    Dismissed,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Correction {
    pub original: String,
    pub suggestion: String,
    pub category: String,
    pub explanation: String,
    pub severity: String,
    pub status: CorrectionStatus,
}

impl Correction {
    pub fn is_error(&self) -> bool {
        self.severity.eq_ignore_ascii_case("error")
    }
}

/// A craft score from 0 to 10, for one criterion of the document type (for example "Rhythm").
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CriterionScore {
    pub name: String,
    pub score: f32,
    pub comment: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Review {
    pub score: f32,
    pub summary: String,
    pub strengths: Vec<String>,
    pub improvements: Vec<String>,
    pub corrections: Vec<Correction>,
    pub patterns: Vec<String>,
    pub rewrite: String,
    pub next_exercise: String,
    pub criteria: Vec<CriterionScore>,
}

/// One round of the feedback loop: the draft as it was sent, and the review that came back.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Iteration {
    pub provider: Provider,
    pub model: String,
    pub at: DateTime<Local>,
    pub words: usize,
    pub text: String,
    pub review: Review,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub doc_type: DocType,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub text: String,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
    #[serde(default)]
    pub iterations: Vec<Iteration>,
    /// The word count at the first edit of the day. Used for the daily word goal.
    #[serde(default)]
    pub baseline: Option<(NaiveDate, usize)>,
}

impl Session {
    pub fn new(doc_type: DocType, prompt: String) -> Self {
        let now = Local::now();
        Self {
            id: now.timestamp_millis(),
            doc_type,
            title: String::new(),
            prompt,
            text: String::new(),
            created_at: now,
            updated_at: now,
            iterations: Vec::new(),
            baseline: None,
        }
    }

    pub fn display_title(&self) -> String {
        if !self.title.trim().is_empty() {
            return self.title.trim().to_owned();
        }
        let first: Vec<&str> = self.text.split_whitespace().take(6).collect();
        if first.is_empty() { "Untitled draft".to_owned() } else { format!("{}…", first.join(" ")) }
    }

    pub fn latest_index(&self, provider: Provider) -> Option<usize> {
        self.iterations.iter().rposition(|it| it.provider == provider)
    }

    pub fn latest(&self, provider: Provider) -> Option<&Iteration> {
        self.latest_index(provider).map(|i| &self.iterations[i])
    }

    /// The score of the review before `index` from the same provider.
    pub fn previous_score(&self, index: usize) -> Option<f32> {
        let provider = self.iterations.get(index)?.provider;
        self.iterations[..index].iter().rev().find(|it| it.provider == provider).map(|it| it.review.score)
    }

    pub fn last_score(&self) -> Option<f32> {
        self.iterations.last().map(|it| it.review.score)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub claude_model: String,
    pub grok_model: String,
    /// Claude effort: "low", "medium", or "high".
    pub effort: String,
    pub theme: ThemeChoice,
    pub editor_font_size: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            claude_model: Provider::Claude.default_model().to_owned(),
            grok_model: Provider::Grok.default_model().to_owned(),
            effort: "medium".to_owned(),
            theme: ThemeChoice::System,
            editor_font_size: 18.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppData {
    pub goal: Goal,
    pub sessions: Vec<Session>,
    pub settings: Settings,
    /// Net new words written per day, over all drafts.
    pub daily_words: BTreeMap<NaiveDate, u32>,
}

impl AppData {
    /// Call after the text of a draft changed. `words_before` is the word count before the change.
    pub fn record_edit(&mut self, index: usize, words_before: usize, today: NaiveDate) {
        let session = &mut self.sessions[index];
        if session.baseline.map(|(day, _)| day) != Some(today) {
            session.baseline = Some((today, words_before));
        }
        session.updated_at = Local::now();
        let total: usize = self
            .sessions
            .iter()
            .filter_map(|s| match s.baseline {
                Some((day, base)) if day == today => Some(word_count(&s.text).saturating_sub(base)),
                _ => None,
            })
            .sum();
        self.daily_words.insert(today, total as u32);
    }

    pub fn words_on(&self, day: NaiveDate) -> u32 {
        self.daily_words.get(&day).copied().unwrap_or(0)
    }

    /// Days in a row that met the daily word goal. Today counts only when it is done.
    pub fn streak(&self, today: NaiveDate) -> u32 {
        let goal = self.goal.daily_words.max(1);
        let mut day = today;
        if self.words_on(today) < goal {
            match today.pred_opt() {
                Some(d) => day = d,
                None => return 0,
            }
        }
        let mut count = 0;
        while self.words_on(day) >= goal {
            count += 1;
            match day.pred_opt() {
                Some(d) => day = d,
                None => break,
            }
        }
        count
    }

    pub fn sessions_last_7_days(&self, today: NaiveDate) -> usize {
        let since = today - chrono::Days::new(6);
        self.sessions.iter().filter(|s| s.updated_at.date_naive() >= since && word_count(&s.text) > 0).count()
    }

    /// All reviews, oldest first.
    pub fn reviews(&self) -> Vec<&Iteration> {
        let mut all: Vec<&Iteration> = self.sessions.iter().flat_map(|s| s.iterations.iter()).collect();
        all.sort_by_key(|it| it.at);
        all
    }

    pub fn recent_average_score(&self, n: usize) -> Option<f32> {
        let all = self.reviews();
        let recent: Vec<f32> = all.iter().rev().take(n).map(|it| it.review.score).collect();
        if recent.is_empty() { None } else { Some(recent.iter().sum::<f32>() / recent.len() as f32) }
    }

    /// The recurring habits the AI saw most often, most frequent first.
    pub fn top_patterns(&self, n: usize) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, (String, usize)> = HashMap::new();
        for it in self.reviews() {
            for p in &it.review.patterns {
                let key = p.trim().trim_end_matches('.').to_lowercase();
                if key.is_empty() {
                    continue;
                }
                let entry = counts.entry(key).or_insert_with(|| (p.trim().trim_end_matches('.').to_owned(), 0));
                entry.1 += 1;
            }
        }
        let mut list: Vec<(String, usize)> = counts.into_values().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list.truncate(n);
        list
    }

    /// The average craft score per criterion, over the last `n` reviews of this document type
    /// that have craft scores. The order follows `DocType::criteria`.
    pub fn criteria_averages(&self, doc: DocType, n: usize) -> Vec<(String, f32)> {
        let mut rounds: Vec<&Iteration> = self
            .sessions
            .iter()
            .filter(|s| s.doc_type == doc)
            .flat_map(|s| s.iterations.iter())
            .filter(|it| !it.review.criteria.is_empty())
            .collect();
        rounds.sort_by_key(|it| it.at);
        let recent = &rounds[rounds.len().saturating_sub(n)..];

        // (name, sum, count). Names from the model can differ in case: use the official name.
        let mut sums: Vec<(String, f32, usize)> = Vec::new();
        for c in recent.iter().flat_map(|it| &it.review.criteria) {
            let name = doc
                .criteria()
                .iter()
                .find(|(official, _)| official.eq_ignore_ascii_case(c.name.trim()))
                .map_or_else(|| c.name.trim().to_owned(), |(official, _)| (*official).to_owned());
            match sums.iter_mut().find(|(known, ..)| known.eq_ignore_ascii_case(&name)) {
                Some(entry) => {
                    entry.1 += c.score;
                    entry.2 += 1;
                }
                None => sums.push((name, c.score, 1)),
            }
        }
        let order =
            |name: &str| doc.criteria().iter().position(|(official, _)| *official == name).unwrap_or(usize::MAX);
        sums.sort_by_key(|(name, ..)| order(name));
        sums.into_iter().map(|(name, sum, count)| (name, sum / count as f32)).collect()
    }

    /// How often each correction category came up, most frequent first.
    pub fn category_counts(&self) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for it in self.reviews() {
            for c in &it.review.corrections {
                let key = if c.category.is_empty() { "other".to_owned() } else { c.category.to_lowercase() };
                *counts.entry(key).or_default() += 1;
            }
        }
        let mut list: Vec<(String, usize)> = counts.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn streak_counts_consecutive_days_that_met_the_goal() {
        let mut data = AppData::default();
        data.goal.daily_words = 100;
        data.daily_words.insert(day("2026-10-21"), 150);
        data.daily_words.insert(day("2026-10-22"), 120);
        data.daily_words.insert(day("2026-10-23"), 100);
        // Today is not done yet, so the streak still counts the days before.
        data.daily_words.insert(day("2026-10-24"), 40);
        assert_eq!(data.streak(day("2026-10-24")), 3);
        data.daily_words.insert(day("2026-10-24"), 100);
        assert_eq!(data.streak(day("2026-10-24")), 4);
    }

    #[test]
    fn streak_breaks_on_a_missed_day() {
        let mut data = AppData::default();
        data.goal.daily_words = 100;
        data.daily_words.insert(day("2026-10-21"), 150);
        data.daily_words.insert(day("2026-10-23"), 150);
        assert_eq!(data.streak(day("2026-10-24")), 1);
    }

    #[test]
    fn record_edit_counts_only_new_words_today() {
        let mut data = AppData::default();
        let mut s = Session::new(DocType::Free, String::new());
        s.text = "one two three".into();
        data.sessions.push(s);
        let today = day("2026-10-24");
        data.sessions[0].text = "one two three four five".into();
        data.record_edit(0, 3, today);
        assert_eq!(data.words_on(today), 2);
        data.sessions[0].text = "one two three four five six".into();
        data.record_edit(0, 5, today);
        assert_eq!(data.words_on(today), 3);
    }

    #[test]
    fn demo_data_loads_and_its_corrections_match_their_drafts() {
        let data: AppData = serde_json::from_str(include_str!("../demo/data.json")).unwrap();
        assert_eq!(data.sessions.len(), 4);
        assert!(!data.criteria_averages(DocType::Story, 5).is_empty(), "the demo needs a story with craft scores");
        assert!(!data.criteria_averages(DocType::Message, 5).is_empty(), "the demo needs a message with craft scores");
        for it in data.sessions.iter().flat_map(|s| &s.iterations) {
            for c in &it.review.corrections {
                assert!(it.text.contains(&c.original), "not in the draft: {}", c.original);
            }
        }
    }

    #[test]
    fn top_patterns_merges_case_and_trailing_period() {
        let mut data = AppData::default();
        let mut s = Session::new(DocType::Free, String::new());
        for p in ["Missing articles.", "missing articles", "Long sentences"] {
            s.iterations.push(Iteration {
                provider: Provider::Claude,
                model: "m".into(),
                at: Local::now(),
                words: 1,
                text: String::new(),
                review: Review { patterns: vec![p.into()], ..Default::default() },
            });
        }
        data.sessions.push(s);
        let top = data.top_patterns(5);
        assert_eq!(top[0].1, 2);
        assert_eq!(top[0].0.to_lowercase(), "missing articles");
    }

    fn review_with(criteria: &[(&str, f32)]) -> Review {
        let criteria = criteria
            .iter()
            .map(|(name, score)| CriterionScore { name: (*name).into(), score: *score, comment: String::new() })
            .collect();
        Review { criteria, ..Default::default() }
    }

    fn round(review: Review) -> Iteration {
        Iteration {
            provider: Provider::Claude,
            model: "m".into(),
            at: Local::now(),
            words: 1,
            text: String::new(),
            review,
        }
    }

    #[test]
    fn criteria_averages_use_recent_reviews_of_one_type_in_rubric_order() {
        let mut data = AppData::default();
        let mut story = Session::new(DocType::Story, String::new());
        story.iterations.push(round(review_with(&[("Rhythm", 6.0), ("arc", 4.0)])));
        story.iterations.push(round(review_with(&[("Rhythm", 8.0), ("Arc", 8.0)])));
        let mut memo = Session::new(DocType::TechMemo, String::new());
        memo.iterations.push(round(review_with(&[("Arc", 0.0)])));
        data.sessions.push(story);
        data.sessions.push(memo);
        assert_eq!(
            data.criteria_averages(DocType::Story, 5),
            vec![("Arc".to_owned(), 6.0), ("Rhythm".to_owned(), 7.0)]
        );
        assert_eq!(
            data.criteria_averages(DocType::Story, 1),
            vec![("Arc".to_owned(), 8.0), ("Rhythm".to_owned(), 8.0)]
        );
    }
}
