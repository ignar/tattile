//! Fast, offline style checks. They run on each keystroke, so they must stay cheap.
//!
//! All ranges are byte ranges into the original text, and they always fall on char boundaries.

use std::ops::Range;

pub const LONG_SENTENCE_WORDS: usize = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IssueKind {
    LongSentence,
    Passive,
    Wordy,
    Complex,
    Filler,
    Hedge,
    Repeated,
}

impl IssueKind {
    pub const ALL: [IssueKind; 7] = [
        IssueKind::LongSentence,
        IssueKind::Passive,
        IssueKind::Wordy,
        IssueKind::Complex,
        IssueKind::Filler,
        IssueKind::Hedge,
        IssueKind::Repeated,
    ];

    pub fn label(self) -> &'static str {
        match self {
            IssueKind::LongSentence => "Long sentence",
            IssueKind::Passive => "Passive voice",
            IssueKind::Wordy => "Wordy phrase",
            IssueKind::Complex => "Complex word or jargon",
            IssueKind::Filler => "Filler word",
            IssueKind::Hedge => "Hedge",
            IssueKind::Repeated => "Repeated word",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            IssueKind::LongSentence => "Split it. One idea per sentence, 25 words or less.",
            IssueKind::Passive => "Say who does the action: \"The team fixed the bug.\"",
            IssueKind::Wordy => "Use the short form.",
            IssueKind::Complex => "Use the simple word. Plain words sound confident.",
            IssueKind::Filler => "Delete it. The sentence is stronger without it.",
            IssueKind::Hedge => "Commit to the claim, or give the reason for doubt.",
            IssueKind::Repeated => "The same word appears twice in a row.",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub kind: IssueKind,
    pub range: Range<usize>,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub words: usize,
    pub sentences: usize,
    pub paragraphs: usize,
    pub avg_sentence_words: f32,
    /// Flesch reading ease, 0 (hard) to 100 (easy).
    pub reading_ease: f32,
    /// Flesch-Kincaid grade level.
    pub grade: f32,
    pub reading_minutes: f32,
    pub long_sentences: usize,
}

impl Stats {
    pub fn ease_label(&self) -> &'static str {
        match self.reading_ease {
            e if e >= 70.0 => "Easy to read",
            e if e >= 50.0 => "Fairly easy",
            e if e >= 30.0 => "Hard",
            _ => "Very hard",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Analysis {
    pub stats: Stats,
    pub issues: Vec<Issue>,
}

impl Analysis {
    pub fn count(&self, kind: IssueKind) -> usize {
        self.issues.iter().filter(|i| i.kind == kind).count()
    }
}

/// (phrase, shorter form)
const WORDY: &[(&str, &str)] = &[
    ("in order to", "to"),
    ("due to the fact that", "because"),
    ("owing to the fact that", "because"),
    ("at this point in time", "now"),
    ("at the present time", "now"),
    ("in the event that", "if"),
    ("it is important to note that", "(delete)"),
    ("it should be noted that", "(delete)"),
    ("a large number of", "many"),
    ("a lot of", "many / much"),
    ("the majority of", "most"),
    ("in spite of the fact that", "although"),
    ("for the purpose of", "to / for"),
    ("with regard to", "about"),
    ("with respect to", "about"),
    ("in terms of", "(rephrase)"),
    ("at the end of the day", "(delete)"),
    ("has the ability to", "can"),
    ("is able to", "can"),
    ("are able to", "can"),
    ("make a decision", "decide"),
    ("made a decision", "decided"),
    ("come to a conclusion", "conclude"),
    ("take into consideration", "consider"),
    ("give an explanation", "explain"),
    ("on a daily basis", "daily"),
    ("on a regular basis", "regularly"),
    ("in the near future", "soon"),
    ("as a matter of fact", "in fact"),
    ("each and every", "each"),
    ("first and foremost", "first"),
    ("end result", "result"),
    ("past history", "history"),
    ("future plans", "plans"),
    ("whether or not", "whether"),
];

/// (word, plain word). Single words also match -s, -ed, -ing forms.
const COMPLEX: &[(&str, &str)] = &[
    ("utilize", "use"),
    ("utilise", "use"),
    ("utilization", "use"),
    ("leverage", "use"),
    ("facilitate", "help"),
    ("commence", "start"),
    ("terminate", "end"),
    ("endeavor", "try"),
    ("prior to", "before"),
    ("subsequent to", "after"),
    ("in excess of", "more than"),
    ("approximately", "about"),
    ("numerous", "many"),
    ("demonstrate", "show"),
    ("obtain", "get"),
    ("sufficient", "enough"),
    ("assist", "help"),
    ("regarding", "about"),
    ("additional", "more"),
    ("synergy", "cooperation"),
    ("circle back", "return to"),
    ("touch base", "talk"),
    ("move the needle", "make progress"),
    ("low-hanging fruit", "easy wins"),
    ("best-in-class", "(be specific)"),
    ("paradigm", "model"),
    ("going forward", "from now on"),
    ("deep dive", "analysis"),
    ("bandwidth", "time"),
    ("ideate", "think of ideas"),
    ("operationalize", "start to use"),
];

const FILLERS: &[&str] = &[
    "very",
    "really",
    "just",
    "actually",
    "basically",
    "quite",
    "simply",
    "literally",
    "totally",
    "definitely",
    "extremely",
    "honestly",
    "obviously",
    "clearly",
];

const HEDGES: &[&str] = &[
    "i think",
    "i believe",
    "i feel",
    "maybe",
    "perhaps",
    "probably",
    "it seems",
    "seems to",
    "somewhat",
    "fairly",
    "arguably",
    "kind of",
    "sort of",
    "more or less",
    "to some extent",
    "hopefully",
];

const BE_FORMS: &[&str] = &["am", "is", "are", "was", "were", "be", "been", "being"];

const IRREGULAR_PARTICIPLES: &[&str] = &[
    "done",
    "made",
    "written",
    "given",
    "taken",
    "seen",
    "known",
    "shown",
    "built",
    "sent",
    "held",
    "found",
    "kept",
    "left",
    "told",
    "brought",
    "thought",
    "chosen",
    "driven",
    "broken",
    "spoken",
    "begun",
    "set",
    "put",
    "paid",
    "led",
    "met",
    "lost",
    "won",
    "understood",
    "forgotten",
    "hidden",
    "bought",
    "caught",
    "taught",
    "sold",
    "read",
    "run",
    "cut",
    "hit",
    "shut",
    "spent",
    "grown",
    "drawn",
    "thrown",
    "worn",
    "torn",
    "felt",
    "heard",
    "meant",
    "said",
    "got",
    "gotten",
    "frozen",
    "stolen",
    "beaten",
    "bitten",
    "eaten",
    "fallen",
    "forbidden",
    "forgiven",
    "hung",
    "laid",
    "lit",
    "proven",
    "ridden",
    "shaken",
    "sung",
    "struck",
    "sworn",
    "undertaken",
    "withdrawn",
    "overseen",
    "rewritten",
    "rebuilt",
];

const NOT_PARTICIPLES: &[&str] = &[
    "need", "feed", "speed", "seed", "breed", "bleed", "indeed", "hundred", "embed", "proceed", "exceed", "succeed",
    "shed", "red", "bed", "wed", "led",
];

const ABBREVIATIONS: &[&str] = &[
    "e.g", "i.e", "etc", "vs", "mr", "mrs", "ms", "dr", "prof", "inc", "ltd", "jr", "sr", "st", "approx", "fig", "no",
    "cf", "al",
];

/// Word ranges. A word is letters and digits, with inner apostrophes or hyphens ("don't", "on-call").
pub fn words(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut end = 0;
    for (i, c) in text.char_indices() {
        if c.is_alphanumeric() {
            if start.is_none() {
                start = Some(i);
            }
            end = i + c.len_utf8();
        } else if start.is_some() && matches!(c, '\'' | '’' | '-') {
            // Keep going: this may join two parts of one word.
        } else if let Some(s) = start.take() {
            out.push(s..end);
        }
    }
    if let Some(s) = start {
        out.push(s..end);
    }
    out
}

pub fn word_count(text: &str) -> usize {
    words(text).len()
}

/// Sentence ranges, trimmed. A newline also ends a sentence, so list items count as sentences.
pub fn sentences(text: &str) -> Vec<Range<usize>> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut start = 0;
    for (k, &(i, c)) in chars.iter().enumerate() {
        let next = chars.get(k + 1).map(|&(_, n)| n);
        let next_is_gap = next.is_none_or(char::is_whitespace);
        let prev = if k > 0 { Some(chars[k - 1].1) } else { None };
        let end = match c {
            '\n' => Some(i),
            '.' if next_is_gap && !is_abbreviation(text, start, i) => Some(i + 1),
            '!' | '?' if next_is_gap => Some(i + 1),
            '"' | '”' | '’' | ')' if next_is_gap && matches!(prev, Some('.' | '!' | '?')) => Some(i + c.len_utf8()),
            _ => None,
        };
        if let Some(end) = end {
            push_trimmed(text, start..end, &mut out);
            start = if c == '\n' { i + 1 } else { end };
        }
    }
    push_trimmed(text, start..text.len(), &mut out);
    out
}

fn push_trimmed(text: &str, range: Range<usize>, out: &mut Vec<Range<usize>>) {
    if range.start >= range.end {
        return;
    }
    let slice = &text[range.clone()];
    let lead = slice.len() - slice.trim_start().len();
    let trail = slice.len() - slice.trim_end().len();
    if lead + trail >= slice.len() {
        return;
    }
    let trimmed = range.start + lead..range.end - trail;
    if text[trimmed.clone()].chars().any(char::is_alphanumeric) {
        out.push(trimmed);
    }
}

/// True if the period at `dot` ends an abbreviation such as "e.g." or an initial such as "J.".
fn is_abbreviation(text: &str, sentence_start: usize, dot: usize) -> bool {
    let before = &text[sentence_start..dot];
    let token: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '.')
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    if token.is_empty() {
        return false;
    }
    let lower = token.to_lowercase();
    if ABBREVIATIONS.contains(&lower.as_str()) {
        return true;
    }
    let mut chars = token.chars();
    matches!((chars.next(), chars.next()), (Some(c), None) if c.is_uppercase())
}

pub fn syllables(word: &str) -> usize {
    let w: String = word.chars().filter(char::is_ascii_alphabetic).collect::<String>().to_ascii_lowercase();
    if w.len() <= 3 {
        return 1;
    }
    let is_vowel = |c: char| "aeiouy".contains(c);
    let mut count = 0;
    let mut prev_vowel = false;
    for c in w.chars() {
        let v = is_vowel(c);
        if v && !prev_vowel {
            count += 1;
        }
        prev_vowel = v;
    }
    if count > 1 {
        let silent_e = w.ends_with('e') && !w.ends_with("le") && !w.ends_with("ee");
        let silent_ed = w.ends_with("ed") && !w.ends_with("ted") && !w.ends_with("ded");
        let silent_es =
            w.ends_with("es") && !["ses", "xes", "zes", "ces", "ges", "shes", "ches"].iter().any(|s| w.ends_with(s));
        if silent_e || silent_ed || silent_es {
            count -= 1;
        }
    }
    count.max(1)
}

pub fn analyze(text: &str) -> Analysis {
    let word_ranges = words(text);
    let sentence_ranges = sentences(text);
    let lower = text.to_ascii_lowercase();
    let mut issues = Vec::new();

    // Sentence length.
    let mut long_sentences = 0;
    for s in &sentence_ranges {
        let n = word_ranges.iter().filter(|w| w.start >= s.start && w.end <= s.end).count();
        if n > LONG_SENTENCE_WORDS {
            long_sentences += 1;
            issues.push(Issue {
                kind: IssueKind::LongSentence,
                range: s.clone(),
                message: format!("This sentence has {n} words. Try {LONG_SENTENCE_WORDS} or less."),
            });
        }
    }

    // Word lists.
    for &(phrase, short) in WORDY {
        for r in find_phrase(&lower, phrase) {
            issues.push(Issue { kind: IssueKind::Wordy, range: r, message: format!("\"{phrase}\" → {short}") });
        }
    }
    for &(word, plain) in COMPLEX {
        for r in find_word_forms(&lower, word) {
            let found = &text[r.clone()];
            issues.push(Issue { kind: IssueKind::Complex, range: r, message: format!("\"{found}\" → {plain}") });
        }
    }
    for &word in FILLERS {
        for r in find_phrase(&lower, word) {
            issues.push(Issue { kind: IssueKind::Filler, range: r, message: format!("\"{word}\" adds little.") });
        }
    }
    for &phrase in HEDGES {
        for r in find_phrase(&lower, phrase) {
            issues.push(Issue {
                kind: IssueKind::Hedge,
                range: r,
                message: format!("\"{phrase}\" makes the claim weaker."),
            });
        }
    }

    // Passive voice and repeated words look at pairs of neighbor words.
    for pair in word_ranges.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let gap = &text[a.end..b.start];
        if !gap.chars().all(char::is_whitespace) {
            continue;
        }
        let wa = &lower[a.clone()];
        let wb = &lower[b.clone()];
        if wa == wb && wa.chars().all(char::is_alphabetic) {
            issues.push(Issue {
                kind: IssueKind::Repeated,
                range: a.start..b.end,
                message: format!("\"{wa}\" appears twice."),
            });
        }
        if BE_FORMS.contains(&wa) && is_participle(wb) {
            issues.push(Issue {
                kind: IssueKind::Passive,
                range: a.start..b.end,
                message: format!("\"{}\" may be passive. Who does the action?", &text[a.start..b.end]),
            });
        }
    }

    issues.sort_by_key(|i| (i.range.start, i.range.end));

    let words_n = word_ranges.len();
    let sentences_n = sentence_ranges.len().max(usize::from(words_n > 0));
    let syllables_n: usize = word_ranges.iter().map(|r| syllables(&text[r.clone()])).sum();
    let (avg, ease, grade) = if words_n == 0 {
        (0.0, 0.0, 0.0)
    } else {
        let wps = words_n as f32 / sentences_n as f32;
        let spw = syllables_n as f32 / words_n as f32;
        (wps, (206.835 - 1.015 * wps - 84.6 * spw).clamp(0.0, 100.0), (0.39 * wps + 11.8 * spw - 15.59).max(0.0))
    };
    let paragraphs = text.split("\n\n").filter(|p| p.chars().any(char::is_alphanumeric)).count();

    Analysis {
        stats: Stats {
            words: words_n,
            sentences: sentence_ranges.len(),
            paragraphs,
            avg_sentence_words: avg,
            reading_ease: ease,
            grade,
            reading_minutes: words_n as f32 / 230.0,
            long_sentences,
        },
        issues,
    }
}

fn is_participle(word: &str) -> bool {
    if NOT_PARTICIPLES.contains(&word) {
        return false;
    }
    (word.len() > 3 && word.ends_with("ed")) || IRREGULAR_PARTICIPLES.contains(&word)
}

/// Finds a lowercase ASCII phrase in the ASCII-lowercased text, at word boundaries.
fn find_phrase(lower: &str, phrase: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(pos) = lower[from..].find(phrase) {
        let start = from + pos;
        let end = start + phrase.len();
        let before_ok = lower[..start].chars().next_back().is_none_or(|c| !c.is_alphanumeric());
        let after_ok = lower[end..].chars().next().is_none_or(|c| !c.is_alphanumeric());
        if before_ok && after_ok {
            out.push(start..end);
        }
        // `start` is the first byte of an ASCII char, so `start + 1` is a char boundary.
        from = start + 1;
    }
    out
}

fn find_word_forms(lower: &str, word: &str) -> Vec<Range<usize>> {
    if word.contains(' ') {
        return find_phrase(lower, word);
    }
    let stem = word.strip_suffix('e').unwrap_or(word);
    let mut forms = vec![
        word.to_owned(),
        format!("{word}s"),
        format!("{word}d"),
        format!("{word}ed"),
        format!("{stem}ing"),
        format!("{stem}ed"),
    ];
    forms.sort();
    forms.dedup();
    let mut out: Vec<Range<usize>> = forms.iter().flat_map(|f| find_phrase(lower, f)).collect();
    out.sort_by_key(|r| r.start);
    out.dedup_by_key(|r| r.start);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slices<'a>(text: &'a str, ranges: &[Range<usize>]) -> Vec<&'a str> {
        ranges.iter().map(|r| &text[r.clone()]).collect()
    }

    #[test]
    fn words_keep_inner_apostrophes_and_hyphens() {
        let t = "We don't run on-call — yet. Users' data!";
        assert_eq!(slices(t, &words(t)), vec!["We", "don't", "run", "on-call", "yet", "Users", "data"]);
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("Привет, мир"), 2);
    }

    #[test]
    fn sentences_split_on_terminators_and_newlines() {
        let t = "We shipped v2.1 today. It works!  Next: tests?\n- item one\nDone";
        assert_eq!(
            slices(t, &sentences(t)),
            vec!["We shipped v2.1 today.", "It works!", "Next: tests?", "- item one", "Done"]
        );
    }

    #[test]
    fn sentences_ignore_abbreviations_and_initials() {
        let t = "Use a cache, e.g. Redis, for this. J. Smith agreed.";
        assert_eq!(slices(t, &sentences(t)), vec!["Use a cache, e.g. Redis, for this.", "J. Smith agreed."]);
    }

    #[test]
    fn sentences_include_closing_quotes() {
        let t = "She said \"ship it.\" We did.";
        assert_eq!(slices(t, &sentences(t)), vec!["She said \"ship it.\"", "We did."]);
    }

    #[test]
    fn syllable_heuristic_is_close_enough() {
        for (w, n) in [
            ("cat", 1),
            ("table", 2),
            ("make", 1),
            ("readability", 5),
            ("beautiful", 3),
            ("used", 1),
            ("wanted", 2),
            ("boxes", 2),
            ("makes", 1),
        ] {
            assert_eq!(syllables(w), n, "{w}");
        }
    }

    #[test]
    fn finds_passive_voice() {
        let t = "The bug was fixed by the team. The report is written. The team needs help.";
        let a = analyze(t);
        let passive: Vec<&str> =
            a.issues.iter().filter(|i| i.kind == IssueKind::Passive).map(|i| &t[i.range.clone()]).collect();
        assert_eq!(passive, vec!["was fixed", "is written"]);
    }

    #[test]
    fn finds_wordy_complex_filler_and_hedges_at_word_boundaries() {
        let t = "In order to ship, we utilized a very simple plan. I think it works. Justice is not filler.";
        let a = analyze(t);
        let found =
            |k: IssueKind| a.issues.iter().filter(|i| i.kind == k).map(|i| &t[i.range.clone()]).collect::<Vec<&str>>();
        assert_eq!(found(IssueKind::Wordy), vec!["In order to"]);
        assert_eq!(found(IssueKind::Complex), vec!["utilized"]);
        assert_eq!(found(IssueKind::Filler), vec!["very"]);
        assert_eq!(found(IssueKind::Hedge), vec!["I think"]);
    }

    #[test]
    fn flags_long_sentences_and_repeated_words() {
        let long = format!("{}.", vec!["word"; 30].join(" x "));
        let t = format!("This is is short. {long}");
        let a = analyze(&t);
        assert_eq!(a.count(IssueKind::Repeated), 1);
        assert_eq!(a.count(IssueKind::LongSentence), 1);
        assert_eq!(a.stats.long_sentences, 1);
    }

    #[test]
    fn ranges_stay_on_char_boundaries_with_unicode() {
        let t = "Ünïcödé text — we utilize “smart quotes” and émojis 🚀 in order to test.";
        let a = analyze(t);
        assert!(!a.issues.is_empty());
        for i in &a.issues {
            assert!(t.is_char_boundary(i.range.start) && t.is_char_boundary(i.range.end));
        }
    }

    #[test]
    fn stats_for_simple_text() {
        let a = analyze("The cat sat. The dog ran.");
        assert_eq!(a.stats.words, 6);
        assert_eq!(a.stats.sentences, 2);
        assert!(a.stats.reading_ease > 90.0);
        assert_eq!(analyze("").stats, Stats::default());
    }
}
