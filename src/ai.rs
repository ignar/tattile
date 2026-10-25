//! HTTP clients for Claude (Anthropic Messages API) and Grok (xAI, OpenAI-compatible API).
//!
//! Rust has no official Anthropic SDK, so this module uses raw HTTP. Each request runs on its own
//! thread and sends the result back to the UI through a channel.

use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::model::{Provider, Review};

const CLAUDE_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const CLAUDE_MODELS_URL: &str = "https://api.anthropic.com/v1/models?limit=100";
const ANTHROPIC_VERSION: &str = "2026-10-21";
/// Lets the API rerun a declined request on a suitable fallback model.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

const GROK_CHAT_URL: &str = "https://api.x.ai/v1/chat/completions";
const GROK_MODELS_URL: &str = "https://api.x.ai/v1/models";

pub struct ReviewJob {
    pub session_id: i64,
    pub provider: Provider,
    pub model: String,
    pub api_key: String,
    pub effort: String,
    pub system: String,
    pub user: String,
    /// The draft as it was sent. The review is stored with it.
    pub text: String,
}

pub enum AiEvent {
    Review { session_id: i64, provider: Provider, model: String, text: String, result: Result<Box<Review>, String> },
    Models { provider: Provider, result: Result<Vec<String>, String> },
}

pub fn spawn_review(job: ReviewJob, tx: Sender<AiEvent>, ctx: egui::Context) {
    thread::spawn(move || {
        let raw = match job.provider {
            Provider::Claude => call_claude(&job),
            Provider::Grok => call_grok(&job),
        };
        let result = raw.and_then(|text| parse_review(&text)).map(Box::new);
        let _ = tx.send(AiEvent::Review {
            session_id: job.session_id,
            provider: job.provider,
            model: job.model,
            text: job.text,
            result,
        });
        ctx.request_repaint();
    });
}

pub fn spawn_list_models(provider: Provider, api_key: String, tx: Sender<AiEvent>, ctx: egui::Context) {
    thread::spawn(move || {
        let result = list_models(provider, &api_key);
        let _ = tx.send(AiEvent::Models { provider, result });
        ctx.request_repaint();
    });
}

/// The JSON shape that both providers must return.
pub fn review_schema() -> Value {
    let strings = json!({ "type": "array", "items": { "type": "string" } });
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["score", "summary", "strengths", "improvements", "corrections", "patterns", "rewrite", "next_exercise", "criteria"],
        "properties": {
            "score": { "type": "integer", "description": "Overall quality from 0 to 100." },
            "summary": { "type": "string", "description": "Two to four sentences about the draft." },
            "strengths": strings,
            "improvements": strings,
            "corrections": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["original", "suggestion", "category", "explanation", "severity"],
                    "properties": {
                        "original": { "type": "string", "description": "Exact span copied from the text." },
                        "suggestion": { "type": "string" },
                        "category": {
                            "type": "string",
                            "enum": ["grammar", "articles", "prepositions", "verb_tense", "word_choice", "spelling",
                                     "punctuation", "concision", "clarity", "tone", "structure"]
                        },
                        "explanation": { "type": "string" },
                        "severity": { "type": "string", "enum": ["error", "suggestion"] }
                    }
                }
            },
            "patterns": strings,
            "rewrite": { "type": "string" },
            "next_exercise": { "type": "string" },
            "criteria": {
                "type": "array",
                "description": "Craft scores for the criteria that the message lists. Empty if it lists none.",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "score", "comment"],
                    "properties": {
                        "name": { "type": "string" },
                        "score": { "type": "integer", "description": "From 0 to 10." },
                        "comment": { "type": "string" }
                    }
                }
            }
        }
    })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        // Read the error body ourselves, so the user sees the real reason.
        .http_status_as_error(false)
        .build()
        .into()
}

fn post_json(url: &str, headers: &[(&str, &str)], body: &Value) -> Result<(u16, String), String> {
    let mut req = agent().post(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let mut resp = req.send_json(body).map_err(|e| format!("Network error: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().map_err(|e| format!("Could not read the response: {e}"))?;
    Ok((status, text))
}

fn get(url: &str, headers: &[(&str, &str)]) -> Result<(u16, String), String> {
    let mut req = agent().get(url);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let mut resp = req.call().map_err(|e| format!("Network error: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().map_err(|e| format!("Could not read the response: {e}"))?;
    Ok((status, text))
}

fn api_error(provider: &str, status: u16, body: &str) -> String {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let detail = parsed
        .as_ref()
        .and_then(|v| {
            v.pointer("/error/message")
                .and_then(Value::as_str)
                .or_else(|| v.get("error").and_then(Value::as_str))
                .or_else(|| v.get("message").and_then(Value::as_str))
        })
        .map(str::to_owned)
        .unwrap_or_else(|| body.chars().take(300).collect());
    match status {
        401 | 403 => format!("{provider} did not accept the API key ({status}). Check the key in Settings. {detail}"),
        404 => format!("{provider} does not know this model ({status}). Pick a model in Settings. {detail}"),
        429 => format!("{provider} rate limit ({status}). Wait a minute, then try again. {detail}"),
        s if s >= 500 => format!("{provider} has a server problem ({status}). Try again later. {detail}"),
        _ => format!("{provider} error {status}: {detail}"),
    }
}

fn call_claude(job: &ReviewJob) -> Result<String, String> {
    let mut body = json!({
        "model": job.model,
        "max_tokens": 16000,
        "fallbacks": "default",
        "system": job.system,
        "output_config": {
            "effort": job.effort,
            "format": { "type": "json_schema", "schema": review_schema() }
        },
        "messages": [{ "role": "user", "content": job.user }]
    });
    let base = [
        ("x-api-key", job.api_key.as_str()),
        ("anthropic-version", ANTHROPIC_VERSION),
        ("content-type", "application/json"),
    ];
    let with_beta = [base[0], base[1], base[2], ("anthropic-beta", FALLBACK_BETA)];

    let (mut status, mut text) = post_json(CLAUDE_MESSAGES_URL, &with_beta, &body)?;
    if status == 400 && text.contains("fallback") {
        // Some models do not support server-side fallback. Try once without it.
        if let Some(obj) = body.as_object_mut() {
            obj.remove("fallbacks");
        }
        (status, text) = post_json(CLAUDE_MESSAGES_URL, &base, &body)?;
    }
    if status != 200 {
        return Err(api_error("Claude", status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Claude sent an answer that is not JSON: {e}"))?;
    match v["stop_reason"].as_str() {
        Some("refusal") => return Err("Claude declined to review this text.".to_owned()),
        Some("max_tokens") => {
            return Err("The review was too long and was cut off. Try a shorter text.".to_owned());
        }
        _ => {}
    }
    let out: String = v["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|block| block["type"] == "text")
        .filter_map(|block| block["text"].as_str())
        .collect();
    if out.trim().is_empty() { Err("Claude returned an empty answer.".to_owned()) } else { Ok(out) }
}

fn call_grok(job: &ReviewJob) -> Result<String, String> {
    let mut body = json!({
        "model": job.model,
        "messages": [
            { "role": "system", "content": job.system },
            { "role": "user", "content": job.user }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": { "name": "writing_review", "strict": true, "schema": review_schema() }
        }
    });
    let auth = format!("Bearer {}", job.api_key);
    let headers = [("authorization", auth.as_str()), ("content-type", "application/json")];

    let (mut status, mut text) = post_json(GROK_CHAT_URL, &headers, &body)?;
    if status == 400 && (text.contains("response_format") || text.contains("json_schema")) {
        // Older models may not support structured output. The prompt still asks for JSON.
        if let Some(obj) = body.as_object_mut() {
            obj.remove("response_format");
        }
        (status, text) = post_json(GROK_CHAT_URL, &headers, &body)?;
    }
    if status != 200 {
        return Err(api_error("Grok", status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Grok sent an answer that is not JSON: {e}"))?;
    let choice = &v["choices"][0];
    if choice["finish_reason"] == "length" {
        return Err("The review was too long and was cut off. Try a shorter text.".to_owned());
    }
    match choice["message"]["content"].as_str() {
        Some(s) if !s.trim().is_empty() => Ok(s.to_owned()),
        _ => Err("Grok returned an empty answer.".to_owned()),
    }
}

fn list_models(provider: Provider, api_key: &str) -> Result<Vec<String>, String> {
    let (status, text) = match provider {
        Provider::Claude => {
            get(CLAUDE_MODELS_URL, &[("x-api-key", api_key), ("anthropic-version", ANTHROPIC_VERSION)])?
        }
        Provider::Grok => {
            let auth = format!("Bearer {api_key}");
            get(GROK_MODELS_URL, &[("authorization", auth.as_str())])?
        }
    };
    if status != 200 {
        return Err(api_error(provider.label(), status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Could not read the model list: {e}"))?;
    let mut ids: Vec<String> =
        v["data"].as_array().into_iter().flatten().filter_map(|m| m["id"].as_str().map(str::to_owned)).collect();
    ids.sort();
    Ok(ids)
}

/// Reads the review JSON. Tolerates code fences or text around the object.
pub fn parse_review(raw: &str) -> Result<Review, String> {
    let start = raw.find('{').ok_or("The answer has no JSON object.")?;
    let end = raw.rfind('}').ok_or("The answer has no JSON object.")?;
    if end < start {
        return Err("The answer has no JSON object.".to_owned());
    }
    let mut review: Review =
        serde_json::from_str(&raw[start..=end]).map_err(|e| format!("Could not read the review JSON: {e}"))?;
    review.score = review.score.clamp(0.0, 100.0);
    review.corrections.retain(|c| !c.original.trim().is_empty() && c.original != c.suggestion);
    review.criteria.retain(|c| !c.name.trim().is_empty());
    for c in &mut review.criteria {
        c.score = c.score.clamp(0.0, 10.0);
    }
    Ok(review)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CorrectionStatus;

    #[test]
    fn parses_plain_and_fenced_json() {
        let json = r#"{"score": 72, "summary": "Clear.", "strengths": ["Good start"], "improvements": [],
            "corrections": [{"original": "She lead", "suggestion": "She led", "category": "verb_tense",
            "explanation": "Use the past tense.", "severity": "error"}],
            "patterns": ["Verb tense"], "rewrite": "She led it.", "next_exercise": "Write 5 sentences."}"#;
        let review = parse_review(json).unwrap();
        assert_eq!(review.score, 72.0);
        assert_eq!(review.corrections.len(), 1);
        assert!(review.corrections[0].is_error());
        assert_eq!(review.corrections[0].status, CorrectionStatus::Open);

        let fenced = format!("Here is the review:\n```json\n{json}\n```");
        assert_eq!(parse_review(&fenced).unwrap().summary, "Clear.");
    }

    #[test]
    fn fills_missing_fields_clamps_score_and_drops_empty_corrections() {
        let review = parse_review(
            r#"{"score": 140, "corrections": [{"original": "", "suggestion": "x"}, {"original": "a", "suggestion": "a"}]}"#,
        )
        .unwrap();
        assert_eq!(review.score, 100.0);
        assert!(review.corrections.is_empty());
        assert!(review.rewrite.is_empty());
    }

    #[test]
    fn rejects_text_without_json() {
        assert!(parse_review("Sorry, I cannot help.").is_err());
        assert!(parse_review("} oops {").is_err());
    }

    #[test]
    fn schema_requires_every_property() {
        let schema = review_schema();
        let props = schema["properties"].as_object().unwrap();
        let required: Vec<&str> = schema["required"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
        for key in props.keys() {
            assert!(required.contains(&key.as_str()), "{key} is not required");
        }
    }

    #[test]
    fn api_error_explains_common_statuses() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let msg = api_error("Claude", 401, body);
        assert!(msg.contains("API key") && msg.contains("invalid x-api-key"));
        assert!(api_error("Grok", 500, "boom").contains("server problem"));
    }

    #[test]
    fn parses_criteria_and_clamps_them_to_ten() {
        let review = parse_review(
            r#"{"score": 70, "criteria": [{"name": "Arc", "score": 7, "comment": "Clear."},
                {"name": "Rhythm", "score": 14, "comment": "Too high."}, {"name": " ", "score": 5, "comment": ""}]}"#,
        )
        .unwrap();
        assert_eq!(review.criteria.len(), 2);
        assert_eq!(review.criteria[0].score, 7.0);
        assert_eq!(review.criteria[1].score, 10.0);
        assert!(review_schema()["properties"]["criteria"].is_object());
    }
}
