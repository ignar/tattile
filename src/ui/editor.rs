//! Turns the draft text into a highlighted `LayoutJob` for the editor.

use std::collections::HashSet;
use std::ops::Range;

use egui::text::{LayoutJob, TextFormat};
use egui::{FontId, Stroke};

use crate::analysis::{Analysis, IssueKind, analyze};
use crate::theme::Palette;

/// Caches the analysis, so it runs only when the text changes.
#[derive(Default)]
pub struct HighlightCache {
    text: String,
    analysis: Analysis,
}

impl HighlightCache {
    pub fn get(&mut self, text: &str) -> &Analysis {
        if self.text != text {
            self.analysis = analyze(text);
            self.text = text.to_owned();
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

fn priority(kind: IssueKind) -> u8 {
    match kind {
        IssueKind::Repeated => 6,
        IssueKind::Passive => 5,
        IssueKind::Complex => 4,
        IssueKind::Wordy => 3,
        IssueKind::Filler => 2,
        IssueKind::Hedge => 1,
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

        if issues.iter().any(|i| i.kind == IssueKind::LongSentence && covers(&i.range)) {
            format.background = p.warning.gamma_multiply(0.10);
        }
        if let Some(issue) = issues
            .iter()
            .filter(|i| i.kind != IssueKind::LongSentence && covers(&i.range))
            .max_by_key(|i| priority(i.kind))
        {
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
        let analysis = analyze(text);
        let marks = ai_marks(text, &[("long process".to_owned(), true), ("not in text".to_owned(), false)]);
        assert_eq!(marks.len(), 1);
        let visible: HashSet<IssueKind> = IssueKind::ALL.into_iter().collect();
        let job = build_job(text, &analysis, &marks, &visible, FontId::proportional(16.0), &palette(false));
        assert_eq!(job.text, text);
        assert!(job.sections.len() > 3);
    }
}
