//! Turns the draft text into a highlighted `LayoutJob` for the editor.

use std::collections::HashSet;
use std::ops::Range;

use egui::text::{LayoutJob, TextFormat};
use egui::{FontId, Stroke};

use crate::analysis::{Analysis, Issue, IssueKind, analyze};
use crate::model::DocType;
use crate::theme::Palette;

/// Caches the analysis, so it runs only when the text or the document type changes.
pub struct HighlightCache {
    text: String,
    doc: DocType,
    analysis: Analysis,
}

impl Default for HighlightCache {
    fn default() -> Self {
        Self { text: String::new(), doc: DocType::Free, analysis: Analysis::default() }
    }
}

impl HighlightCache {
    pub fn get(&mut self, text: &str, doc: DocType) -> &Analysis {
        if self.text != text || self.doc != doc {
            self.analysis = analyze(text, doc);
            self.text = text.to_owned();
            self.doc = doc;
        }
        &self.analysis
    }
}

/// An open AI correction, found in the text.
pub struct Mark {
    pub range: Range<usize>,
    pub error: bool,
}

/// Finds the first place of each `(original, is_error)` in the text.
pub fn ai_marks(text: &str, corrections: &[(String, bool)]) -> Vec<Mark> {
    corrections
        .iter()
        .filter_map(|(original, error)| {
            text.find(original.as_str()).map(|start| Mark { range: start..start + original.len(), error: *error })
        })
        .collect()
}

/// When marks overlap, the higher number wins.
fn priority(kind: IssueKind) -> u8 {
    match kind {
        IssueKind::DialoguePunctuation | IssueKind::SlowStart => 10,
        IssueKind::InlineList => 3,
        IssueKind::LongParagraph => 1,
        IssueKind::TellingEmotion => 9,
        IssueKind::FilterWord | IssueKind::DialogueTag => 8,
        IssueKind::Repeated => 6,
        IssueKind::Passive => 5,
        IssueKind::Complex => 4,
        IssueKind::Wordy => 3,
        IssueKind::Filler => 2,
        IssueKind::Hedge => 1,
        IssueKind::TenseShift => 2,
        IssueKind::MonotoneRhythm => 1,
        IssueKind::LongSentence => 0,
    }
}

pub fn build_job(
    text: &str,
    analysis: &Analysis,
    marks: &[Mark],
    visible: &HashSet<IssueKind>,
    font: FontId,
    p: &Palette,
) -> LayoutJob {
    let issues: Vec<_> = analysis.issues.iter().filter(|i| visible.contains(&i.kind)).collect();
    let mut cuts: Vec<usize> = vec![0, text.len()];
    for i in &issues {
        cuts.extend([i.range.start, i.range.end]);
    }
    for m in marks {
        cuts.extend([m.range.start, m.range.end]);
    }
    cuts.sort_unstable();
    cuts.dedup();

    let line_height = Some(font.size * 1.6);
    let mut job = LayoutJob::default();
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a >= b {
            continue;
        }
        let covers = |r: &Range<usize>| r.start <= a && r.end >= b;
        let mut format = TextFormat { font_id: font.clone(), color: p.text, line_height, ..Default::default() };

        // Sentence-level checks paint a soft background. Word-level checks draw an underline.
        let (sentence_marks, word_marks): (Vec<&Issue>, Vec<&Issue>) =
            issues.iter().copied().filter(|i| covers(&i.range)).partition(|i| i.kind.marks_sentences());
        if let Some(issue) = sentence_marks.iter().max_by_key(|i| priority(i.kind)) {
            format.background = p.issue(issue.kind).gamma_multiply(0.10);
        }
        if let Some(issue) = word_marks.iter().max_by_key(|i| priority(i.kind)) {
            format.underline = Stroke::new(2.0, p.issue(issue.kind));
        }
        if let Some(mark) = marks.iter().find(|m| covers(&m.range)) {
            let color = if mark.error { p.danger } else { p.purple };
            format.background = color.gamma_multiply(0.16);
            format.underline = Stroke::new(2.0, color);
        }
        job.append(&text[a..b], 0.0, format);
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::palette;

    #[test]
    fn job_covers_the_whole_text_once() {
        let text = "We utilize a very long process in order to ship. 🚀 Done.";
        let analysis = analyze(text, DocType::TechMemo);
        let marks = ai_marks(text, &[("long process".to_owned(), true), ("not in text".to_owned(), false)]);
        assert_eq!(marks.len(), 1);
        let visible: HashSet<IssueKind> = IssueKind::ALL.into_iter().collect();
        let job = build_job(text, &analysis, &marks, &visible, FontId::proportional(16.0), &palette(false));
        assert_eq!(job.text, text);
        assert!(job.sections.len() > 3);
    }

    #[test]
    fn cache_runs_again_when_the_document_type_changes() {
        let long = format!("{}.", vec!["word"; 30].join(" "));
        let mut cache = HighlightCache::default();
        assert_eq!(cache.get(&long, DocType::TechMemo).count(IssueKind::LongSentence), 1);
        assert_eq!(cache.get(&long, DocType::Story).count(IssueKind::LongSentence), 0);
    }
}
