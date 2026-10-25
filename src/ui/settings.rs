use egui::{ComboBox, RichText, Slider, TextEdit, Ui};

use super::widgets::{button, card, chip, page, page_title, pal, primary_button, section};
use crate::app::{TattileApp, ToastKind};
use crate::model::{Provider, ThemeChoice};
use crate::secrets::{self, KeySource};
use crate::storage;
use crate::theme;

impl TattileApp {
    pub fn settings_view(&mut self, ui: &mut Ui) {
        let before = self.data.settings.clone();
        page(ui, "settings_page", |ui| {
            page_title(ui, "Settings", "Connect Claude and Grok, and choose how the app looks.");
            for provider in Provider::ALL {
                self.provider_card(ui, provider);
                ui.add_space(10.0);
            }
            self.appearance_card(ui);
            ui.add_space(10.0);
            self.data_card(ui);
        });
        if self.data.settings.theme != before.theme {
            theme::apply_choice(ui.ctx(), self.data.settings.theme);
        }
        if self.data.settings != before {
            self.mark_dirty();
        }
    }

    fn provider_card(&mut self, ui: &mut Ui, provider: Provider) {
        let p = pal(ui);
        let (mut save, mut remove, mut load) = (false, false, false);
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(provider.label()).font(theme::heading(20.0)).color(p.text));
                match self.keys[&provider].source {
                    KeySource::Keychain => chip(ui, "Key saved in Keychain", p.success),
                    KeySource::Env => chip(ui, format!("Key from ${}", secrets::env_var(provider)), p.info),
                    KeySource::Missing => chip(ui, "No API key", p.warning),
                };
            });
            let (about, url) = match provider {
                Provider::Claude => (
                    "Anthropic Messages API. Default model: claude-opus-5-5, with structured output and refusal fallback.",
                    "https://console.anthropic.com/settings/keys",
                ),
                Provider::Grok => ("xAI API (OpenAI-compatible). Default model: grok-4.5.", "https://console.x.ai"),
            };
            ui.label(RichText::new(about).color(p.weak));
            ui.hyperlink_to("Get an API key ↗", url);

            section(ui, "API key");
            ui.horizontal(|ui| {
                let ks = self.keys.get_mut(&provider).expect("every provider has a key state");
                let hint = if ks.source == KeySource::Missing {
                    "Paste your API key"
                } else {
                    "Paste a new key to replace it"
                };
                ui.add(
                    TextEdit::singleline(&mut ks.input)
                        .password(true)
                        .hint_text(RichText::new(hint).color(p.faint))
                        .desired_width(300.0)
                        .min_size(egui::vec2(0.0, 30.0)),
                );
                if primary_button(ui, "Save to Keychain").clicked() {
                    save = true;
                }
                if ks.source == KeySource::Keychain && button(ui, "Remove").clicked() {
                    remove = true;
                }
            });

            section(ui, "Model");
            ui.horizontal(|ui| {
                let model = match provider {
                    Provider::Claude => &mut self.data.settings.claude_model,
                    Provider::Grok => &mut self.data.settings.grok_model,
                };
                ui.add(TextEdit::singleline(model).desired_width(200.0).min_size(egui::vec2(0.0, 30.0)));
                if let Some(list) = self.models.get(&provider) {
                    ComboBox::from_id_salt(("models", provider)).selected_text("Pick a model").width(180.0).show_ui(
                        ui,
                        |ui| {
                            for id in list {
                                ui.selectable_value(model, id.clone(), id.as_str());
                            }
                        },
                    );
                }
                if self.loading_models.contains(&provider) {
                    ui.spinner();
                } else if button(ui, "Test and load models")
                    .on_hover_text("Checks the key and lists the models you can use")
                    .clicked()
                {
                    load = true;
                }
                if button(ui, "Reset").on_hover_text(provider.default_model()).clicked() {
                    *model = provider.default_model().to_owned();
                }
            });

            if provider == Provider::Claude {
                section(ui, "Review depth");
                ui.horizontal(|ui| {
                    for (value, label) in [("low", "Fast"), ("medium", "Balanced"), ("high", "Deep")] {
                        ui.radio_value(&mut self.data.settings.effort, value.to_owned(), label);
                    }
                });
                ui.label(
                    RichText::new("Deeper reviews find more, but take longer and cost more.").size(12.0).color(p.weak),
                );
            }
        });
        if save {
            self.save_key(provider);
        }
        if remove {
            self.remove_key(provider);
        }
        if load {
            self.load_models(provider, &ui.ctx().clone());
        }
    }

    fn appearance_card(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        card(ui, |ui| {
            ui.label(RichText::new("Appearance").font(theme::heading(20.0)).color(p.text));
            section(ui, "Theme");
            ui.horizontal(|ui| {
                for (value, label) in
                    [(ThemeChoice::System, "Match macOS"), (ThemeChoice::Light, "Light"), (ThemeChoice::Dark, "Dark")]
                {
                    ui.radio_value(&mut self.data.settings.theme, value, label);
                }
            });
            section(ui, "Editor text size");
            ui.add(Slider::new(&mut self.data.settings.editor_font_size, 14.0..=26.0).step_by(1.0).suffix(" pt"));
        });
    }

    fn data_card(&mut self, ui: &mut Ui) {
        let p = pal(ui);
        let dir = storage::data_dir();
        let mut open = false;
        card(ui, |ui| {
            ui.label(RichText::new("Your data").font(theme::heading(20.0)).color(p.text));
            ui.label(
                RichText::new("Drafts, reviews, and goals stay on this Mac. Text goes only to the AI that you ask.")
                    .color(p.weak),
            );
            ui.label(RichText::new(dir.display().to_string()).monospace().size(12.0).color(p.text2));
            if button(ui, "Show in Finder").clicked() {
                open = true;
            }
        });
        if open {
            let result = std::fs::create_dir_all(&dir).map_err(|e| e.to_string()).and_then(|_| {
                std::process::Command::new("open").arg(&dir).spawn().map(|_| ()).map_err(|e| e.to_string())
            });
            if let Err(e) = result {
                self.notify(format!("Could not open the folder: {e}"), ToastKind::Error);
            }
        }
    }
}
