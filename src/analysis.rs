//! Fast, offline style checks. They run on each keystroke, so they must stay cheap.
//!
//! All ranges are byte ranges into the original text, and they always fall on char boundaries.

use std::ops::Range;

use crate::model::DocType;

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
    // Story checks.
    TellingEmotion,
    FilterWord,
    TenseShift,
    MonotoneRhythm,
    DialogueTag,
    DialoguePunctuation,
}

const BUSINESS_KINDS: [IssueKind; 7] = [
    IssueKind::LongSentence,
    IssueKind::Passive,
    IssueKind::Wordy,
    IssueKind::Complex,
    IssueKind::Filler,
    IssueKind::Hedge,
    IssueKind::Repeated,
];

/// Stories get craft checks and no long-sentence check: a long sentence can serve the rhythm.
const STORY_KINDS: [IssueKind; 12] = [
    IssueKind::TellingEmotion,
    IssueKind::FilterWord,
    IssueKind::TenseShift,
    IssueKind::MonotoneRhythm,
    IssueKind::DialogueTag,
    IssueKind::DialoguePunctuation,
    IssueKind::Passive,
    IssueKind::Wordy,
    IssueKind::Complex,
    IssueKind::Filler,
    IssueKind::Hedge,
    IssueKind::Repeated,
];

impl IssueKind {
    pub const ALL: [IssueKind; 13] = [
        IssueKind::LongSentence,
        IssueKind::Passive,
        IssueKind::Wordy,
        IssueKind::Complex,
        IssueKind::Filler,
        IssueKind::Hedge,
        IssueKind::Repeated,
        IssueKind::TellingEmotion,
        IssueKind::FilterWord,
        IssueKind::TenseShift,
        IssueKind::MonotoneRhythm,
        IssueKind::DialogueTag,
        IssueKind::DialoguePunctuation,
    ];

    /// The checks that make sense for this kind of document.
    pub fn for_doc(doc: DocType) -> &'static [IssueKind] {
        if doc == DocType::Story { &STORY_KINDS } else { &BUSINESS_KINDS }
    }

    /// Checks that mark a whole sentence, or a group of sentences, with a background color.
    pub fn marks_sentences(self) -> bool {
        matches!(self, IssueKind::LongSentence | IssueKind::TenseShift | IssueKind::MonotoneRhythm)
    }

    pub fn label(self) -> &'static str {
        match self {
            IssueKind::LongSentence => "Long sentence",
            IssueKind::Passive => "Passive voice",
            IssueKind::Wordy => "Wordy phrase",
            IssueKind::Complex => "Complex word or jargon",
            IssueKind::Filler => "Filler word",
            IssueKind::Hedge => "Hedge",
            IssueKind::Repeated => "Repeated word",
            IssueKind::TellingEmotion => "Named emotion",
            IssueKind::FilterWord => "Filter word",
            IssueKind::TenseShift => "Tense shift",
            IssueKind::MonotoneRhythm => "Same sentence length",
            IssueKind::DialogueTag => "Adverb in a dialogue tag",
            IssueKind::DialoguePunctuation => "Dialogue punctuation",
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
            IssueKind::TellingEmotion => "Show the feeling through an action, speech, or a detail.",
            IssueKind::FilterWord => "Show what the character sees or hears, directly.",
            IssueKind::TenseShift => "Keep one tense in the narration.",
            IssueKind::MonotoneRhythm => "Mix short and long sentences.",
            IssueKind::DialogueTag => "Let the words, or an action, show how it was said.",
            IssueKind::DialoguePunctuation => "Use a comma before a tag: \"Go,\" she said.",
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
    /// The standard deviation of the sentence lengths, in words. Low values sound monotone.
    pub length_variety: f32,
    /// The part of the words that is inside quotation marks (0 to 1).
    pub dialogue_share: f32,
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

// Word lists for the story checks.

const EMOTIONS: &[&str] = &[
    "angry",
    "sad",
    "happy",
    "nervous",
    "afraid",
    "scared",
    "excited",
    "anxious",
    "upset",
    "worried",
    "tired",
    "lonely",
    "frustrated",
    "jealous",
    "proud",
    "ashamed",
    "embarrassed",
    "furious",
    "terrified",
    "depressed",
    "bored",
    "surprised",
    "shocked",
    "relieved",
    "confused",
    "hopeful",
    "disappointed",
    "glad",
    "calm",
    "annoyed",
    "guilty",
    "desperate",
    "devastated",
    "delighted",
    "thrilled",
    "exhausted",
    "miserable",
    "grateful",
    "overwhelmed",
    "stressed",
    "heartbroken",
    "joyful",
    "irritated",
    "panicked",
    "uneasy",
];

/// Verbs that name a state: "was angry", "felt nervous".
const TELL_VERBS: &[&str] = &[
    "was", "were", "is", "are", "am", "be", "been", "felt", "feel", "feels", "feeling", "seemed", "seems", "looked",
    "looks", "became", "becomes", "got", "gets", "grew",
];

/// Words that can sit between a state verb and an emotion: "was very angry", "felt a little nervous".
const INTENSIFIERS: &[&str] = &[
    "very",
    "so",
    "really",
    "extremely",
    "quite",
    "too",
    "a",
    "bit",
    "little",
    "rather",
    "deeply",
    "incredibly",
    "terribly",
    "slightly",
    "more",
];

const SUBJECTS: &[&str] = &["i", "he", "she", "they", "we", "you"];

/// Verbs that filter the scene through a character: "she saw", "I could hear".
const FILTER_VERBS: &[&str] = &[
    "see", "saw", "hear", "heard", "feel", "felt", "notice", "noticed", "realize", "realized", "realise", "realised",
    "wonder", "wondered", "watch", "watched", "smell", "smelled", "know", "knew", "think", "thought", "decide",
    "decided", "sense", "sensed",
];

const SPEECH_VERBS: &[&str] = &[
    "said",
    "says",
    "asked",
    "asks",
    "replied",
    "whispered",
    "shouted",
    "answered",
    "muttered",
    "yelled",
    "cried",
    "added",
    "snapped",
    "called",
    "murmured",
];

/// Words that end in "-ly" but are not adverbs of manner.
const NOT_ADVERBS: &[&str] = &[
    "only", "early", "family", "reply", "supply", "apply", "july", "italy", "holy", "ugly", "daily", "weekly",
    "monthly", "yearly", "friendly", "lonely", "lovely", "silly", "belly", "jelly", "rally", "rely", "bully", "likely",
    "elderly", "curly", "costly", "deadly", "lively", "orderly",
];

const PAST_MARKERS: &[&str] = &[
    "was", "were", "had", "did", "wasn't", "weren't", "hadn't", "didn't", "went", "came", "said", "saw", "took", "ran",
    "told", "thought", "felt", "knew", "got", "made", "left", "found", "gave", "became", "began", "sat", "stood",
    "heard", "kept", "brought",
];

const PRESENT_MARKERS: &[&str] = &["is", "are", "am", "isn't", "aren't", "has", "does", "doesn't"];

/// Words after "he/she/it" that end in "s" but are not present-tense verbs.
const NOT_VERBS_IN_S: &[&str] = &[
    "always",
    "perhaps",
    "sometimes",
    "nevertheless",
    "besides",
    "towards",
    "afterwards",
    "thus",
    "unless",
    "less",
    "was",
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

pub fn analyze(text: &str, doc: DocType) -> Analysis {
    let story = doc == DocType::Story;
    let word_ranges = words(text);
    let sentence_ranges = sentences(text);
    let lower = text.to_ascii_lowercase();
    let mut issues = Vec::new();
    let lengths: Vec<usize> = sentence_ranges
        .iter()
        .map(|s| word_ranges.iter().filter(|w| w.start >= s.start && w.end <= s.end).count())
        .collect();

    // Sentence length. Stories skip this check.
    let mut long_sentences = 0;
    for (s, &n) in sentence_ranges.iter().zip(&lengths) {
        if n > LONG_SENTENCE_WORDS {
            long_sentences += 1;
            if !story {
                issues.push(Issue {
                    kind: IssueKind::LongSentence,
                    range: s.clone(),
                    message: format!("This sentence has {n} words. Try {LONG_SENTENCE_WORDS} or less."),
                });
            }
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

    let dialogue = dialogue_ranges(text);
    if story {
        // Normalized lowercase words: the curly apostrophe becomes a straight one ("didn’t" → "didn't").
        let lw: Vec<String> = word_ranges.iter().map(|r| lower[r.clone()].replace('’', "'")).collect();
        let ctx = StoryText { text, words: &word_ranges, lw: &lw, dialogue: &dialogue };
        ctx.named_emotions_and_filters(&mut issues);
        ctx.dialogue_tags(&mut issues);
        ctx.dialogue_punctuation(&mut issues);
        ctx.tense_shifts(&sentence_ranges, &mut issues);
        monotone_runs(&sentence_ranges, &lengths, &mut issues);
        // People talk with fillers and hedges. Do not flag style inside dialogue.
        issues.retain(|i| {
            !(matches!(
                i.kind,
                IssueKind::Passive
                    | IssueKind::Wordy
                    | IssueKind::Complex
                    | IssueKind::Filler
                    | IssueKind::Hedge
                    | IssueKind::Repeated
            ) && inside(&dialogue, &i.range))
        });
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
    let length_variety = if lengths.is_empty() {
        0.0
    } else {
        let mean = lengths.iter().sum::<usize>() as f32 / lengths.len() as f32;
        let variance = lengths.iter().map(|&n| (n as f32 - mean).powi(2)).sum::<f32>() / lengths.len() as f32;
        variance.sqrt()
    };
    let dialogue_words = word_ranges.iter().filter(|w| inside(&dialogue, w)).count();
    let dialogue_share = if words_n == 0 { 0.0 } else { dialogue_words as f32 / words_n as f32 };

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
            length_variety,
            dialogue_share,
        },
        issues,
    }
}

/// Text inside quotation marks, with the marks. An unclosed quote ends at the end of its line.
pub fn dialogue_ranges(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut open: Option<usize> = None;
    for (i, c) in text.char_indices() {
        match (c, open) {
            ('"' | '“', None) => open = Some(i),
            ('"' | '”', Some(start)) => {
                out.push(start..i + c.len_utf8());
                open = None;
            }
            ('\n', Some(start)) => {
                out.push(start..i);
                open = None;
            }
            _ => {}
        }
    }
    if let Some(start) = open {
        out.push(start..text.len());
    }
    out
}

fn inside(ranges: &[Range<usize>], r: &Range<usize>) -> bool {
    ranges.iter().any(|d| d.start <= r.start && r.end <= d.end)
}

/// The words of a story, prepared for the craft checks.
struct StoryText<'a> {
    text: &'a str,
    words: &'a [Range<usize>],
    lw: &'a [String],
    dialogue: &'a [Range<usize>],
}

impl StoryText<'_> {
    fn is(&self, k: usize, list: &[&str]) -> bool {
        self.lw.get(k).is_some_and(|w| list.contains(&w.as_str()))
    }

    /// True if only spaces are between word `a` and word `b`.
    fn adjacent(&self, a: usize, b: usize) -> bool {
        self.text[self.words[a].end..self.words[b].start].chars().all(char::is_whitespace)
    }

    fn narration(&self, r: &Range<usize>) -> bool {
        !inside(self.dialogue, r)
    }

    /// "was very angry" names a feeling. "she saw" filters a scene. A filter that is part of
    /// a named emotion ("he felt sad") gets only the emotion flag.
    fn named_emotions_and_filters(&self, issues: &mut Vec<Issue>) {
        let n = self.words.len();
        let mut telling: Vec<Range<usize>> = Vec::new();
        for i in 0..n {
            if !self.is(i, TELL_VERBS) {
                continue;
            }
            let mut j = i + 1;
            while j < n && j <= i + 3 && self.is(j, INTENSIFIERS) && self.adjacent(j - 1, j) {
                j += 1;
            }
            if j < n && self.adjacent(j - 1, j) && self.is(j, EMOTIONS) {
                let range = self.words[i].start..self.words[j].end;
                if self.narration(&range) {
                    issues.push(Issue {
                        kind: IssueKind::TellingEmotion,
                        message: format!(
                            "\"{}\" names the feeling. Show it: what does the person do, say, or notice?",
                            &self.text[range.clone()]
                        ),
                        range: range.clone(),
                    });
                    telling.push(range);
                }
            }
        }
        for i in 0..n {
            if !self.is(i, SUBJECTS) {
                continue;
            }
            let mut j = i + 1;
            if j < n && self.is(j, &["could", "can", "would"]) && self.adjacent(i, j) {
                j += 1;
            }
            if j < n && self.adjacent(j - 1, j) && self.is(j, FILTER_VERBS) {
                let range = self.words[i].start..self.words[j].end;
                let part_of_emotion = telling.iter().any(|t| t.start < range.end && range.start < t.end);
                if self.narration(&range) && !part_of_emotion {
                    issues.push(Issue {
                        kind: IssueKind::FilterWord,
                        message: format!(
                            "\"{}\" filters the scene through a character. Show what is there, directly.",
                            &self.text[range.clone()]
                        ),
                        range,
                    });
                }
            }
        }
    }

    /// "said quietly": the adverb tells how something was said.
    fn dialogue_tags(&self, issues: &mut Vec<Issue>) {
        for i in 0..self.words.len().saturating_sub(1) {
            let next = &self.lw[i + 1];
            let adverb = next.len() > 4 && next.ends_with("ly") && !NOT_ADVERBS.contains(&next.as_str());
            if self.is(i, SPEECH_VERBS) && adverb && self.adjacent(i, i + 1) {
                let range = self.words[i].start..self.words[i + 1].end;
                if self.narration(&range) {
                    issues.push(Issue {
                        kind: IssueKind::DialogueTag,
                        message: format!(
                            "\"{}\": let the words, or an action, show how it was said.",
                            &self.text[range.clone()]
                        ),
                        range,
                    });
                }
            }
        }
    }

    /// `"We are late." she said` needs a comma: `"We are late," she said`.
    fn dialogue_punctuation(&self, issues: &mut Vec<Issue>) {
        for d in self.dialogue {
            let quote = &self.text[d.clone()];
            let Some(close) = quote.chars().next_back().filter(|c| matches!(c, '"' | '”')) else { continue };
            let before_close = &quote[..quote.len() - close.len_utf8()];
            if quote.chars().count() < 2 || !before_close.ends_with('.') {
                continue;
            }
            let after: Vec<usize> = (0..self.words.len()).filter(|&k| self.words[k].start >= d.end).take(2).collect();
            let Some(&first) = after.first() else { continue };
            // Only a tag that continues the sentence. A capital letter starts a new sentence.
            let same_sentence = self.text[d.end..self.words[first].start].chars().all(char::is_whitespace)
                && self.text[self.words[first].clone()].starts_with(char::is_lowercase);
            let is_tag = self.is(first, SPEECH_VERBS)
                || (self.is(first, SUBJECTS) && after.get(1).is_some_and(|&k| self.is(k, SPEECH_VERBS)));
            if same_sentence && is_tag {
                let period = d.start + before_close.len() - 1;
                issues.push(Issue {
                    kind: IssueKind::DialoguePunctuation,
                    range: period..d.end,
                    message: "Use a comma, not a period, before a dialogue tag: \"We are late,\" she said.".to_owned(),
                });
            }
        }
    }

    /// Finds the main tense of the narration, and flags sentences in the other tense.
    /// Dialogue does not count: people speak in any tense.
    fn tense_shifts(&self, sentences: &[Range<usize>], issues: &mut Vec<Issue>) {
        let mut tenses: Vec<(Range<usize>, bool)> = Vec::new(); // (sentence, is past)
        for s in sentences {
            let idx: Vec<usize> = (0..self.words.len())
                .filter(|&k| {
                    self.words[k].start >= s.start && self.words[k].end <= s.end && self.narration(&self.words[k])
                })
                .collect();
            let (mut past, mut present) = (0, 0);
            for (n, &k) in idx.iter().enumerate() {
                let w = self.lw[k].as_str();
                past += usize::from(PAST_MARKERS.contains(&w));
                present += usize::from(PRESENT_MARKERS.contains(&w));
                let Some(&prev) = n.checked_sub(1).and_then(|p| idx.get(p)) else { continue };
                let prev = self.lw[prev].as_str();
                if SUBJECTS.contains(&prev) || prev == "it" {
                    if w.len() > 3 && w.ends_with("ed") && !NOT_PARTICIPLES.contains(&w) {
                        past += 1;
                    }
                    let third_person = matches!(prev, "he" | "she" | "it");
                    if third_person
                        && w.len() > 2
                        && w.ends_with('s')
                        && !w.ends_with("ss")
                        && !PRESENT_MARKERS.contains(&w)
                        && !NOT_VERBS_IN_S.contains(&w)
                    {
                        present += 1;
                    }
                }
            }
            if past != present {
                tenses.push((s.clone(), past > present));
            }
        }
        if tenses.len() < 4 {
            return;
        }
        let past = tenses.iter().filter(|(_, is_past)| *is_past).count();
        let (main_is_past, main) = if past * 2 >= tenses.len() { (true, past) } else { (false, tenses.len() - past) };
        if (main as f32) < 0.7 * tenses.len() as f32 {
            return;
        }
        let (main_name, other_name) = if main_is_past { ("past", "present") } else { ("present", "past") };
        for (range, is_past) in tenses {
            if is_past != main_is_past {
                issues.push(Issue {
                    kind: IssueKind::TenseShift,
                    range,
                    message: format!(
                        "Tense shift: most of the story is in the {main_name} tense, but this sentence is in the {other_name}."
                    ),
                });
            }
        }
    }
}

/// Four or more sentences in a row with almost the same length sound monotone.
fn monotone_runs(sentences: &[Range<usize>], lengths: &[usize], issues: &mut Vec<Issue>) {
    let mut i = 0;
    while i < sentences.len() {
        let (mut lo, mut hi) = (lengths[i], lengths[i]);
        let mut j = i;
        while lengths[i] >= 4 && j + 1 < sentences.len() && lengths[j + 1] >= 4 {
            let (next_lo, next_hi) = (lo.min(lengths[j + 1]), hi.max(lengths[j + 1]));
            if next_hi - next_lo > 2 {
                break;
            }
            (lo, hi, j) = (next_lo, next_hi, j + 1);
        }
        let run = j - i + 1;
        if run >= 4 {
            issues.push(Issue {
                kind: IssueKind::MonotoneRhythm,
                range: sentences[i].start..sentences[j].end,
                message: format!(
                    "{run} sentences in a row have almost the same length (about {hi} words). Add a short one or a long one."
                ),
            });
            i = j + 1;
        } else {
            i += 1;
        }
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
    use crate::model::DocType;

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
        let a = analyze(t, DocType::TechMemo);
        let passive: Vec<&str> =
            a.issues.iter().filter(|i| i.kind == IssueKind::Passive).map(|i| &t[i.range.clone()]).collect();
        assert_eq!(passive, vec!["was fixed", "is written"]);
    }

    #[test]
    fn finds_wordy_complex_filler_and_hedges_at_word_boundaries() {
        let t = "In order to ship, we utilized a very simple plan. I think it works. Justice is not filler.";
        let a = analyze(t, DocType::TechMemo);
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
        let a = analyze(&t, DocType::TechMemo);
        assert_eq!(a.count(IssueKind::Repeated), 1);
        assert_eq!(a.count(IssueKind::LongSentence), 1);
        assert_eq!(a.stats.long_sentences, 1);
    }

    #[test]
    fn ranges_stay_on_char_boundaries_with_unicode() {
        let t = "Ünïcödé text — we utilize “smart quotes” and émojis 🚀 in order to test.";
        let a = analyze(t, DocType::TechMemo);
        assert!(!a.issues.is_empty());
        for i in &a.issues {
            assert!(t.is_char_boundary(i.range.start) && t.is_char_boundary(i.range.end));
        }
    }

    #[test]
    fn stats_for_simple_text() {
        let a = analyze("The cat sat. The dog ran.", DocType::TechMemo);
        assert_eq!(a.stats.words, 6);
        assert_eq!(a.stats.sentences, 2);
        assert!(a.stats.reading_ease > 90.0);
        assert_eq!(analyze("", DocType::TechMemo).stats, Stats::default());
    }

    fn found<'a>(a: &Analysis, t: &'a str, kind: IssueKind) -> Vec<&'a str> {
        a.issues.iter().filter(|i| i.kind == kind).map(|i| &t[i.range.clone()]).collect()
    }

    #[test]
    fn stories_do_not_flag_long_sentences() {
        let long = format!("{}.", vec!["word"; 30].join(" x "));
        assert_eq!(analyze(&long, DocType::Story).count(IssueKind::LongSentence), 0);
        assert_eq!(analyze(&long, DocType::TechMemo).count(IssueKind::LongSentence), 1);
        assert!(!IssueKind::for_doc(DocType::Story).contains(&IssueKind::LongSentence));
        assert!(IssueKind::for_doc(DocType::Strategy).contains(&IssueKind::LongSentence));
    }

    #[test]
    fn story_checks_run_only_for_stories() {
        let t = "She was very angry. I could hear the rain.";
        let memo = analyze(t, DocType::TechMemo);
        assert_eq!(memo.count(IssueKind::TellingEmotion) + memo.count(IssueKind::FilterWord), 0);
    }

    #[test]
    fn stories_flag_named_emotions() {
        let t = "She was very angry. He felt a little nervous. The angry sea rose. They were tired of meetings.";
        let a = analyze(t, DocType::Story);
        assert_eq!(
            found(&a, t, IssueKind::TellingEmotion),
            vec!["was very angry", "felt a little nervous", "were tired"]
        );
    }

    #[test]
    fn stories_flag_filter_words_but_not_when_an_emotion_is_named() {
        let t = "I could hear the rain. She saw the door open. He felt sad. The saw cut wood.";
        let a = analyze(t, DocType::Story);
        assert_eq!(found(&a, t, IssueKind::FilterWord), vec!["I could hear", "She saw"]);
        assert_eq!(found(&a, t, IssueKind::TellingEmotion), vec!["felt sad"]);
    }

    #[test]
    fn stories_flag_dialogue_tags_with_adverbs_and_wrong_punctuation() {
        let t = "\"We are not ready.\" she said quietly. \"Go,\" he said. \"Stop!\" She left.";
        let a = analyze(t, DocType::Story);
        assert_eq!(found(&a, t, IssueKind::DialogueTag), vec!["said quietly"]);
        assert_eq!(found(&a, t, IssueKind::DialoguePunctuation), vec![".\""]);
    }

    #[test]
    fn stories_ignore_style_checks_inside_dialogue() {
        let t = "\"I think it is very late,\" he said. I think the night was very long.";
        let a = analyze(t, DocType::Story);
        let narration_start = t.find("I think the night").unwrap();
        for kind in [IssueKind::Hedge, IssueKind::Filler] {
            let hits: Vec<_> = a.issues.iter().filter(|i| i.kind == kind).collect();
            assert_eq!(hits.len(), 1, "{kind:?}");
            assert!(hits[0].range.start >= narration_start, "{kind:?} was flagged inside dialogue");
        }
    }

    #[test]
    fn stories_flag_tense_shifts_in_narration() {
        let t = "I stayed late. The floor was quiet. She walked in. We checked the logs. \"It is fine,\" she said. The build is green.";
        let a = analyze(t, DocType::Story);
        assert_eq!(found(&a, t, IssueKind::TenseShift), vec!["The build is green."]);
        let calm = "I stayed late. The floor was quiet. She walked in. We checked the logs.";
        assert_eq!(analyze(calm, DocType::Story).count(IssueKind::TenseShift), 0);
    }

    #[test]
    fn stories_flag_runs_of_sentences_with_the_same_length() {
        let t = "We read the notes again. We checked the graphs again. We ran the tests again. We fixed the bug again. \
                 Then, at two in the morning, after six hours of work, the build turned green.";
        let a = analyze(t, DocType::Story);
        assert_eq!(
            found(&a, t, IssueKind::MonotoneRhythm),
            vec![
                "We read the notes again. We checked the graphs again. We ran the tests again. We fixed the bug again."
            ]
        );
        let varied = "We read the notes. Then we checked every graph on the wall, one by one. Nothing. We ran the tests again and waited.";
        assert_eq!(analyze(varied, DocType::Story).count(IssueKind::MonotoneRhythm), 0);
    }

    #[test]
    fn story_stats_measure_length_variety_and_dialogue() {
        let a = analyze("\"Go,\" she said. We ran.", DocType::Story);
        assert!((a.stats.dialogue_share - 0.2).abs() < 0.01, "{}", a.stats.dialogue_share);
        assert!((a.stats.length_variety - 0.5).abs() < 0.01, "{}", a.stats.length_variety);
    }
}
