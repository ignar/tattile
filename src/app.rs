//! App state, the AI event loop, and the actions that change the data.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use chrono::Local;
use egui::{Align2, Frame, Id, Margin, RichText, vec2};

use crate::ai::{self, AiEvent, ReviewJob};
use crate::analysis::{IssueKind, word_count};
use crate::model::{AppData, CorrectionStatus, DocType, Iteration, Provider, Session};
use crate::prompts::{self, CoachContext};
use crate::secrets::{self, KeySource};
use crate::storage;
use crate::theme;
use crate::ui::editor::HighlightCache;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Write,
    Goals,
    Progress,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FeedbackTab {
    Checks,
    Ai(Provider),
    History,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub born: Option<f64>,
}

pub struct KeyState {
    /// Empty until the key is needed. See `TattileApp::api_key`.
    pub value: String,
    pub loaded: bool,
    pub source: KeySource,
    pub input: String,
}

/// Things the user can do from the feedback panel. They run after the panel is drawn.
pub enum Action {
    Apply(usize, usize),
    Dismiss(usize, usize),
    ApplyAll(usize),
    UseRewrite(usize),
    Restore(usize),
    Practice(String),
    Ask(Provider),
    Copy(String),
    Select(Range<usize>),
    ToggleCheck(IssueKind),
}

/// The text before the last change that a button made, so the user can undo it.
pub struct Undo {
    pub session_id: i64,
    pub text: String,
    /// Corrections that the change applied. Undo opens them again.
    pub reopen: Vec<(usize, usize)>,
}

pub struct TattileApp {
    pub data: AppData,
    pub view: View,
    pub current: Option<usize>,
    pub tab: FeedbackTab,
    pub highlight: HighlightCache,
    pub visible_checks: HashSet<IssueKind>,
    pub editing_prompt: bool,
    pub confirm_delete: Option<i64>,
    pub keys: HashMap<Provider, KeyState>,
    pub models: HashMap<Provider, Vec<String>>,
    pub loading_models: HashSet<Provider>,
    pub pending: HashSet<(i64, Provider)>,
    pub errors: HashMap<(i64, Provider), String>,
    pub toast: Option<Toast>,
    pub select_request: Option<Range<usize>>,
    pub undo: Option<Undo>,
    tx: Sender<AiEvent>,
    rx: Receiver<AiEvent>,
    dirty: bool,
    last_save: Option<Instant>,
}

impl TattileApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut data = storage::load();
        theme::install(&cc.egui_ctx, data.settings.reading_font);
        theme::apply_choice(&cc.egui_ctx, data.settings.theme);
        if data.sessions.is_empty() {
            let doc = data.goal.doc_type;
            data.sessions.push(Session::new(doc, prompts::pick_assignment(doc, "")));
        }
        let current = (0..data.sessions.len()).max_by_key(|&i| data.sessions[i].updated_at);
        let keys = Provider::ALL
            .into_iter()
            .map(|p| {
                // Do not read the secret yet: that can show a Keychain dialog before the window opens.
                (p, KeyState { value: String::new(), loaded: false, source: secrets::source(p), input: String::new() })
            })
            .collect();
        let (tx, rx) = mpsc::channel();
        Self {
            data,
            view: View::Write,
            current,
            tab: FeedbackTab::Checks,
            highlight: HighlightCache::default(),
            visible_checks: IssueKind::ALL.into_iter().collect(),
            editing_prompt: false,
            confirm_delete: None,
            keys,
            models: HashMap::new(),
            loading_models: HashSet::new(),
            pending: HashSet::new(),
            errors: HashMap::new(),
            toast: None,
            select_request: None,
            undo: None,
            tx,
            rx,
            dirty: false,
            last_save: None,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn notify(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast { text: text.into(), kind, born: None });
    }

    pub fn new_session(&mut self, doc: DocType, prompt: Option<String>) {
        let prompt = prompt.unwrap_or_else(|| prompts::pick_assignment(doc, ""));
        self.data.sessions.push(Session::new(doc, prompt));
        self.current = Some(self.data.sessions.len() - 1);
        self.view = View::Write;
        self.tab = FeedbackTab::Checks;
        self.editing_prompt = false;
        self.dirty = true;
    }

    pub fn delete_session(&mut self, id: i64) {
        let Some(index) = self.data.sessions.iter().position(|s| s.id == id) else { return };
        self.data.sessions.remove(index);
        self.current = (0..self.data.sessions.len()).max_by_key(|&i| self.data.sessions[i].updated_at);
        self.dirty = true;
        self.notify("The draft is deleted.", ToastKind::Info);
    }

    /// Replaces the draft text from a button (not from typing), and keeps the old text for undo.
    pub fn set_text(&mut self, index: usize, text: String) {
        let session = &mut self.data.sessions[index];
        let words_before = word_count(&session.text);
        let old = std::mem::replace(&mut session.text, text);
        self.undo = Some(Undo { session_id: session.id, text: old, reopen: Vec::new() });
        self.data.record_edit(index, words_before, Local::now().date_naive());
        self.dirty = true;
    }

    pub fn undo_last_change(&mut self, index: usize) {
        let id = self.data.sessions[index].id;
        if self.undo.as_ref().is_some_and(|u| u.session_id == id)
            && let Some(undo) = self.undo.take()
        {
            self.set_text(index, undo.text);
            for (it, ci) in undo.reopen {
                if let Some(c) =
                    self.data.sessions[index].iterations.get_mut(it).and_then(|i| i.review.corrections.get_mut(ci))
                {
                    c.status = CorrectionStatus::Open;
                }
            }
            // `set_text` saved the text we just left. Undo is one step, so drop it.
            self.undo = None;
        }
    }

    pub fn request_review(&mut self, provider: Provider, ctx: &egui::Context) {
        let Some(index) = self.current else { return };
        let Some(key) = self.api_key(provider) else {
            self.notify(format!("Add your {} API key in Settings first.", provider.label()), ToastKind::Error);
            self.view = View::Settings;
            return;
        };
        let session = &self.data.sessions[index];
        if word_count(&session.text) < 15 {
            self.notify("Write at least 15 words. Then ask for feedback.", ToastKind::Info);
            return;
        }
        if self.pending.contains(&(session.id, provider)) {
            return;
        }
        let patterns: Vec<String> = self.data.top_patterns(5).into_iter().map(|(p, _)| p).collect();
        let previous = session.latest(provider).or(session.iterations.last());
        let revision = session.iterations.iter().filter(|it| it.provider == provider).count() + 1;
        let user = prompts::user_message(&CoachContext {
            doc_type: session.doc_type,
            assignment: &session.prompt,
            focus: &self.data.goal.focus,
            patterns: &patterns,
            previous,
            revision,
            text: &session.text,
        });
        let model = match provider {
            Provider::Claude => self.data.settings.claude_model.clone(),
            Provider::Grok => self.data.settings.grok_model.clone(),
        };
        let job = ReviewJob {
            session_id: session.id,
            provider,
            model,
            api_key: key,
            effort: self.data.settings.effort.clone(),
            system: prompts::system_prompt(),
            user,
            text: session.text.clone(),
        };
        let id = session.id;
        self.errors.remove(&(id, provider));
        self.pending.insert((id, provider));
        self.tab = FeedbackTab::Ai(provider);
        ai::spawn_review(job, self.tx.clone(), ctx.clone());
    }

    /// The API key, read from the Keychain (or the environment) the first time it is needed.
    pub fn api_key(&mut self, provider: Provider) -> Option<String> {
        let ks = self.keys.get_mut(&provider).expect("every provider has a key state");
        if !ks.loaded && ks.source != KeySource::Missing {
            let (value, source) = secrets::load(provider);
            ks.value = value;
            ks.source = source;
            ks.loaded = true;
        }
        Some(ks.value.clone()).filter(|k| !k.is_empty())
    }

    pub fn load_models(&mut self, provider: Provider, ctx: &egui::Context) {
        let Some(key) = self.api_key(provider) else {
            self.notify(format!("Save a {} API key first.", provider.label()), ToastKind::Error);
            return;
        };
        self.loading_models.insert(provider);
        ai::spawn_list_models(provider, key, self.tx.clone(), ctx.clone());
    }

    pub fn save_key(&mut self, provider: Provider) {
        let input = self.keys[&provider].input.trim().to_owned();
        if input.is_empty() {
            return;
        }
        match secrets::save(provider, &input) {
            Ok(()) => {
                let ks = self.keys.get_mut(&provider).expect("every provider has a key state");
                ks.value = input;
                ks.loaded = true;
                ks.source = KeySource::Keychain;
                ks.input.clear();
                self.notify(format!("The {} key is saved in your Keychain.", provider.label()), ToastKind::Success);
            }
            Err(e) => self.notify(e, ToastKind::Error),
        }
    }

    pub fn remove_key(&mut self, provider: Provider) {
        match secrets::delete(provider) {
            Ok(()) => {
                let ks = self.keys.get_mut(&provider).expect("every provider has a key state");
                ks.value.clear();
                ks.loaded = false;
                ks.source = secrets::source(provider);
                self.notify(format!("The {} key is removed from your Keychain.", provider.label()), ToastKind::Info);
            }
            Err(e) => self.notify(e, ToastKind::Error),
        }
    }

    pub fn run_actions(&mut self, index: usize, actions: Vec<Action>, ctx: &egui::Context) {
        for action in actions {
            match action {
                Action::Apply(it, ci) => {
                    if !self.apply_correction(index, it, ci) {
                        self.notify(
                            "This text is not in the draft now. You may have changed it already.",
                            ToastKind::Info,
                        );
                    }
                }
                Action::Dismiss(it, ci) => {
                    self.data.sessions[index].iterations[it].review.corrections[ci].status =
                        CorrectionStatus::Dismissed;
                    self.dirty = true;
                }
                Action::ApplyAll(it) => {
                    let before = self.data.sessions[index].text.clone();
                    let count = self.data.sessions[index].iterations[it].review.corrections.len();
                    let mut applied = Vec::new();
                    let mut skipped = 0;
                    for ci in 0..count {
                        if self.data.sessions[index].iterations[it].review.corrections[ci].status
                            != CorrectionStatus::Open
                        {
                            continue;
                        }
                        if self.apply_correction(index, it, ci) { applied.push((it, ci)) } else { skipped += 1 }
                    }
                    let applied_count = applied.len();
                    // One undo step for the whole batch.
                    self.undo = Some(Undo { session_id: self.data.sessions[index].id, text: before, reopen: applied });
                    let msg = if skipped == 0 {
                        format!("Applied {applied_count} corrections.")
                    } else {
                        format!("Applied {applied_count} corrections. {skipped} did not match the text.")
                    };
                    self.notify(msg, ToastKind::Success);
                }
                Action::UseRewrite(it) => {
                    let rewrite = self.data.sessions[index].iterations[it].review.rewrite.clone();
                    if !rewrite.trim().is_empty() {
                        self.set_text(index, rewrite);
                        self.notify("The polished version is now your draft. Use Undo to go back.", ToastKind::Success);
                    }
                }
                Action::Restore(it) => {
                    let old = self.data.sessions[index].iterations[it].text.clone();
                    self.set_text(index, old);
                    self.notify("This version is now your draft. Use Undo to go back.", ToastKind::Success);
                }
                Action::Practice(prompt) => {
                    let doc = self.data.sessions[index].doc_type;
                    self.new_session(doc, Some(prompt));
                }
                Action::Ask(provider) => self.request_review(provider, ctx),
                Action::Copy(text) => {
                    ctx.copy_text(text);
                    self.notify("Copied to the clipboard.", ToastKind::Info);
                }
                Action::Select(range) => self.select_request = Some(range),
                Action::ToggleCheck(kind) => {
                    if !self.visible_checks.remove(&kind) {
                        self.visible_checks.insert(kind);
                    }
                }
            }
        }
    }

    fn apply_correction(&mut self, index: usize, it: usize, ci: usize) -> bool {
        let session = &self.data.sessions[index];
        let c = &session.iterations[it].review.corrections[ci];
        if !session.text.contains(&c.original) {
            return false;
        }
        let new_text = session.text.replacen(&c.original, &c.suggestion, 1);
        self.set_text(index, new_text);
        self.data.sessions[index].iterations[it].review.corrections[ci].status = CorrectionStatus::Applied;
        if let Some(undo) = &mut self.undo {
            undo.reopen.push((it, ci));
        }
        true
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AiEvent::Review { session_id, provider, model, text, result } => {
                    self.pending.remove(&(session_id, provider));
                    match result {
                        Ok(review) => {
                            let score = review.score;
                            if let Some(s) = self.data.sessions.iter_mut().find(|s| s.id == session_id) {
                                let words = word_count(&text);
                                s.iterations.push(Iteration {
                                    provider,
                                    model,
                                    at: Local::now(),
                                    words,
                                    text,
                                    review: *review,
                                });
                            }
                            self.notify(
                                format!("{} scored this draft {score:.0}/100.", provider.label()),
                                ToastKind::Success,
                            );
                            self.dirty = true;
                        }
                        Err(e) => {
                            self.errors.insert((session_id, provider), e.clone());
                            self.notify(e, ToastKind::Error);
                        }
                    }
                }
                AiEvent::Models { provider, result } => {
                    self.loading_models.remove(&provider);
                    match result {
                        Ok(list) => {
                            self.notify(
                                format!("{} is connected. It has {} models.", provider.label(), list.len()),
                                ToastKind::Success,
                            );
                            self.models.insert(provider, list);
                        }
                        Err(e) => self.notify(e, ToastKind::Error),
                    }
                }
            }
        }
    }

    fn draw_toast(&mut self, ctx: &egui::Context, now: f64, p: theme::Palette) {
        let Some(toast) = &mut self.toast else { return };
        let born = *toast.born.get_or_insert(now);
        let life = if toast.kind == ToastKind::Error { 9.0 } else { 4.0 };
        if now - born > life {
            self.toast = None;
            return;
        }
        let color = match toast.kind {
            ToastKind::Info => p.info,
            ToastKind::Success => p.success,
            ToastKind::Error => p.danger,
        };
        let text = toast.text.clone();
        let mut close = false;
        // A quiet pill near the bottom, above the composer bar.
        egui::Area::new(Id::unique("toast")).anchor(Align2::CENTER_BOTTOM, vec2(0.0, -92.0)).show(ctx, |ui| {
            Frame::new()
                .fill(p.toast_bg)
                .corner_radius(12)
                .inner_margin(Margin { left: 14, right: 6, top: 6, bottom: 6 })
                .shadow(egui::Shadow { offset: [0, 6], blur: 20, spread: 0, color: p.shadow })
                .show(ui, |ui| {
                    ui.set_max_width(520.0);
                    ui.horizontal(|ui| {
                        let (dot, _) = ui.allocate_exact_size(vec2(8.0, 16.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot.center(), 3.5, color);
                        ui.label(RichText::new(text).color(p.toast_text));
                        if crate::ui::widgets::text_button(ui, "×", p.toast_text).clicked() {
                            close = true;
                        }
                    });
                });
        });
        if close {
            self.toast = None;
        } else {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    fn delete_dialog(&mut self, ctx: &egui::Context) {
        let Some(id) = self.confirm_delete else { return };
        let title = self.data.sessions.iter().find(|s| s.id == id).map(|s| s.display_title()).unwrap_or_default();
        let mut decision: Option<bool> = None;
        let modal = egui::Modal::new(Id::unique("confirm_delete")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(RichText::new("Delete this draft?").font(theme::heading(18.0)));
            ui.label(format!("\"{title}\" and all its reviews will be deleted. You cannot undo this."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if crate::ui::widgets::button(ui, "Cancel").clicked() {
                    decision = Some(false);
                }
                let p = theme::palette(ui.visuals().dark_mode);
                let delete =
                    egui::Button::new(RichText::new("Delete").color(p.on_accent)).fill(p.danger).corner_radius(8);
                if ui.add(delete.min_size(vec2(0.0, 30.0))).clicked() {
                    decision = Some(true);
                }
            });
        });
        if modal.should_close() && decision.is_none() {
            decision = Some(false);
        }
        match decision {
            Some(true) => {
                self.confirm_delete = None;
                self.delete_session(id);
            }
            Some(false) => self.confirm_delete = None,
            None => {}
        }
    }
}

impl TattileApp {
    /// Saves at most every 1.5 seconds while there are changes, or at once when `force` is set.
    fn autosave(&mut self, ctx: &egui::Context, force: bool) {
        if !self.dirty {
            return;
        }
        let due = self.last_save.is_none_or(|t| t.elapsed() > Duration::from_millis(1500));
        if force || due {
            if let Err(e) = storage::save(&self.data) {
                self.notify(e, ToastKind::Error);
            }
            self.dirty = false;
            self.last_save = Some(Instant::now());
        } else {
            ctx.request_repaint_after(Duration::from_secs(2));
        }
    }
}

impl eframe::App for TattileApp {
    /// eframe calls this even while the window is hidden, so reviews that arrive
    /// while you work in another app are stored at once.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events();
        self.autosave(ctx, false);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let p = theme::palette(ui.visuals().dark_mode);

        egui::Panel::left("sidebar")
            .resizable(false)
            .exact_size(248.0)
            .show_separator_line(true)
            .frame(Frame::new().fill(p.sidebar).inner_margin(Margin { left: 10, right: 10, top: 0, bottom: 10 }))
            .show(ui, |ui| self.sidebar(ui));

        egui::CentralPanel::default().frame(Frame::new().fill(p.bg)).show(ui, |ui| match self.view {
            View::Write => self.write_view(ui),
            View::Goals => self.goals_view(ui),
            View::Progress => self.progress_view(ui),
            View::Settings => self.settings_view(ui),
        });

        let ctx = ui.ctx().clone();
        let now = ui.input(|i| i.time);
        self.delete_dialog(&ctx);
        self.draw_toast(&ctx, now, p);

        let closing = ui.input(|i| i.viewport().close_requested());
        self.autosave(&ctx, closing);
    }
}
