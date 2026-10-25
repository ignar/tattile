//! Practice assignments and the instructions for the AI writing coach.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{DocType, Iteration, Skill};

pub fn assignments(doc: DocType) -> &'static [&'static str] {
    match doc {
        DocType::Strategy => &[
            "Your team runs 40 microservices, and the on-call load doubled this year. Write the \"Problem\" and \"Proposal\" sections of a one-page strategy for the next two quarters.",
            "Leadership asks if your team should build or buy a feature-flag system. Write a strategy note with the options, one recommendation, and the main risks.",
            "Write a strategy to reduce the time from merge to production from 3 days to 1 hour. Say where you are now, where you want to be, and the first three steps.",
            "Your company wants to enter a new market in 6 months. Write the engineering section: what you will build, what you will not build, and why.",
        ],
        DocType::TechMemo => &[
            "Write a technical memo that explains why your team will move from REST polling to webhooks. Start with the decision, then the context, then the risks.",
            "An outage stopped the checkout service for 47 minutes. Write a short incident memo: impact, root cause, what you fixed, and what you will do next.",
            "Explain to a non-technical director why you need two sprints to pay down technical debt in the billing system.",
            "Write a memo that compares PostgreSQL and DynamoDB for a new event-history feature, and recommend one.",
        ],
        DocType::Promo => &[
            "Write the \"Impact\" section of a promotion document for a senior engineer who led a database migration with zero downtime.",
            "Write a self-review paragraph about a project where you grew a new team from 3 to 8 engineers. Use concrete numbers.",
            "Describe how an engineer on your team showed leadership without formal authority. Use one specific story and its result.",
            "Write a promo case for an engineer who made the CI pipeline 60% faster. Explain the scope, the difficulty, and the business impact.",
        ],
        DocType::Story => &[
            "Write a short story (200–400 words) about the night before a big product launch. Show the feeling; do not name it.",
            "Describe your first week as a manager as a story. Use one scene, one conversation, and one lesson.",
            "Write a story about a bug that nobody could reproduce. Give it a beginning, a turning point, and an end.",
            "Tell the story of a decision you regret. Use the past tense and vivid but simple details.",
        ],
        DocType::Update => &[
            "Write a weekly status update for your director: what shipped, what is at risk, and what you need from them.",
            "Write a Slack message that announces a two-week delay for a project. Be honest, calm, and clear about the new date.",
            "Write an email that asks another team for help with a dependency. Keep it under 150 words.",
            "Summarize a one-hour architecture meeting in five bullet points, with owners and dates.",
        ],
        DocType::Message => &[
            "Your VP asks in Slack: \"Why did the payments migration slip two weeks, and will it slip again?\"",
            "A product manager asks: \"Can we ship the export feature on Friday if we skip the load tests?\"",
            "A new engineer asks: \"How do we decide if a bug is P1 or P2?\" Answer so that they can decide alone next time.",
            "Your director asks by email: \"Should we hire two senior engineers or three mid-level engineers?\"",
            "A customer success lead asks: \"Why does the dashboard show different numbers than the CSV export?\"",
            "A peer manager asks: \"My team keeps missing sprint goals. What did you change on your team?\"",
        ],
        DocType::Free => &[
            "Write about anything on your mind today. Aim for clear sentences, not perfect ones.",
            "Explain a technical concept that you know well to a 12-year-old.",
            "Describe your ideal engineering culture in three paragraphs.",
            "Write about a book, talk, or article that changed how you work.",
        ],
    }
}

/// A random assignment for this document type that is different from `current`.
pub fn pick_assignment(doc: DocType, current: &str) -> String {
    let list = assignments(doc);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0) as usize;
    let mut index = nanos % list.len();
    if list[index] == current {
        index = (index + 1) % list.len();
    }
    list[index].to_owned()
}

pub fn system_prompt() -> String {
    "You are Tattile, a writing coach and copy editor. Your student is an engineering manager \
who writes in English as a second language. They practice strategy documents, technical memos, \
promotion documents, status updates, and stories. Their goal is to write elegantly, precisely, and cleanly.

How to review:
- Be direct and specific. Do not flatter. Praise only what is really good.
- First find the errors that a native reader notices: articles (a/an/the), prepositions, verb tense \
and agreement, word order, countable and uncountable nouns, unnatural collocations, false friends, spelling, punctuation.
- Then find style problems for this document type: wordiness, vague claims, weak structure, passive voice \
that hides ownership, buzzwords, and wrong tone.
- Give at most 25 corrections, the most important first.

Rules for each correction:
- \"original\" must be copied exactly, character for character, from the student's text. Use the smallest \
span that contains the problem, usually 1 to 12 words, so the app can find and replace it.
- \"suggestion\" replaces exactly that span and fits the sentence around it.
- \"explanation\" is one short sentence in simple English that names the rule, for example: \
'Use \"the\" before a noun that the reader already knows.'
- \"severity\" is \"error\" for a mistake in English, and \"suggestion\" for a style improvement.

Scoring (0-100): 90+ means a strong native writer could publish it. 75-89 means clear, with small issues. \
60-74 means understandable, but it needs edits. Below 60 means hard to follow or many errors. \
Score the text, not the effort.

\"patterns\": up to 3 recurring habits, as short labels that the student can track over time, for example \
'Missing articles before singular nouns'.
\"rewrite\": a polished version. Keep the author's meaning, voice, and about the same length. Do not add facts.
\"next_exercise\": one short, concrete exercise (2-3 sentences) for the weakest area.
\"criteria\": a score from 0 to 10 and a one-sentence comment for each craft criterion that the message lists. \
Use the exact names. 5 means average for a careful amateur; 8 or more means skilled. If the message lists \
no criteria, return an empty list.

Write every explanation and comment in simple, clear English."
        .to_owned()
}

pub struct CoachContext<'a> {
    pub doc_type: DocType,
    pub assignment: &'a str,
    pub focus: &'a [Skill],
    pub patterns: &'a [String],
    pub previous: Option<&'a Iteration>,
    pub revision: usize,
    pub text: &'a str,
}

pub fn user_message(ctx: &CoachContext<'_>) -> String {
    let focus = if ctx.focus.is_empty() {
        "none chosen".to_owned()
    } else {
        ctx.focus.iter().map(|s| s.label()).collect::<Vec<_>>().join(", ")
    };
    let patterns = if ctx.patterns.is_empty() { "none recorded yet".to_owned() } else { ctx.patterns.join("; ") };
    let assignment = if ctx.assignment.trim().is_empty() {
        if ctx.doc_type.answers_a_question() {
            "Not given. Judge the reply on its own."
        } else {
            "Free choice of topic."
        }
    } else {
        ctx.assignment.trim()
    };
    let assignment_label = if ctx.doc_type.answers_a_question() { "The question being answered" } else { "Assignment" };

    let mut msg = format!(
        "Document type: {}\nWhat good looks like for this type: {}\n{}: {}\n\
         The student's focus areas: {}\nThe student's known recurring mistakes (check for these): {}\n",
        ctx.doc_type.label(),
        ctx.doc_type.rubric(),
        assignment_label,
        assignment,
        focus,
        patterns,
    );
    let criteria = ctx.doc_type.criteria();
    if criteria.is_empty() {
        msg.push_str("Return an empty \"criteria\" list.\n");
    } else {
        msg.push_str(
            "\nScore these craft criteria from 0 to 10, each with one short comment. Use these exact names:\n",
        );
        for (name, meaning) in criteria {
            msg.push_str(&format!("- {name}: {meaning}\n"));
        }
    }
    if ctx.doc_type == DocType::Message {
        msg.push_str(
            "This is a reply to a question. Judge it the way the reader reads it: quickly, on a phone. \
             The question can be complex, but the reply must be easy to read. Check that the first sentence \
             answers the question. Suggest a short list where it helps. The rewrite must answer in its first sentence.\n",
        );
    }
    if ctx.doc_type == DocType::Story {
        msg.push_str(
            "This is creative writing. Do not ask for shorter sentences when a long sentence serves the rhythm. \
             Inside dialogue, natural speech is fine: correct only real errors there.\n",
        );
    }
    if let Some(prev) = ctx.previous {
        let issues: Vec<String> =
            prev.review.corrections.iter().take(6).map(|c| format!("\"{}\" ({})", c.original, c.category)).collect();
        msg.push_str(&format!(
            "\nThis is revision {} of this draft. The previous review gave {:.0}/100. \
             Its main issues were: {}. In the summary, say what improved and what did not.\n",
            ctx.revision,
            prev.review.score,
            if issues.is_empty() { "none listed".to_owned() } else { issues.join(", ") },
        ));
    }
    msg.push_str("\n--- STUDENT TEXT START ---\n");
    msg.push_str(ctx.text);
    msg.push_str("\n--- STUDENT TEXT END ---");
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_doc_type_has_assignments_and_pick_changes_it() {
        for doc in DocType::ALL {
            let list = assignments(doc);
            assert!(list.len() >= 2);
            assert_ne!(pick_assignment(doc, list[0]), "");
            for _ in 0..20 {
                let current = list[1];
                assert_ne!(pick_assignment(doc, current), current);
            }
        }
    }

    #[test]
    fn user_message_contains_context_and_text() {
        let patterns = vec!["Missing articles".to_owned()];
        let msg = user_message(&CoachContext {
            doc_type: DocType::Promo,
            assignment: "",
            focus: &[Skill::Articles],
            patterns: &patterns,
            previous: None,
            revision: 1,
            text: "She lead the migration.",
        });
        assert!(msg.contains("Promo packet"));
        assert!(msg.contains("Free choice of topic."));
        assert!(msg.contains("Missing articles"));
        assert!(msg.contains("She lead the migration."));
        assert!(!msg.contains("revision"));
    }

    fn message_for(doc_type: DocType) -> String {
        user_message(&CoachContext {
            doc_type,
            assignment: "",
            focus: &[],
            patterns: &[],
            previous: None,
            revision: 1,
            text: "Text.",
        })
    }

    #[test]
    fn story_messages_ask_for_craft_scores_and_other_types_ask_for_none() {
        let story = message_for(DocType::Story);
        for (name, _) in DocType::Story.criteria() {
            assert!(story.contains(name), "the story message does not name {name}");
        }
        assert!(story.contains("0 to 10"));
        assert!(story.contains("dialogue"));
        let memo = message_for(DocType::TechMemo);
        assert!(memo.contains("Return an empty \"criteria\" list."));
        assert!(system_prompt().contains("\"criteria\""));
    }

    #[test]
    fn message_prompts_treat_the_assignment_as_the_question() {
        let msg = user_message(&CoachContext {
            doc_type: DocType::Message,
            assignment: "Can we ship on Friday?",
            focus: &[],
            patterns: &[],
            previous: None,
            revision: 1,
            text: "Yes.",
        });
        assert!(msg.contains("The question being answered: Can we ship on Friday?"));
        assert!(msg.contains("on a phone"));
        for (name, _) in DocType::Message.criteria() {
            assert!(msg.contains(name), "the message prompt does not name {name}");
        }
    }
}
