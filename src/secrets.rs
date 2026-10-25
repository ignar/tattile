//! API keys live in the macOS Keychain. Environment variables work as a fallback.

use crate::model::Provider;

const SERVICE: &str = "com.tattile.app";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySource {
    Keychain,
    Env,
    Missing,
}

fn account(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "anthropic-api-key",
        Provider::Grok => "xai-api-key",
    }
}

pub fn env_var(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "ANTHROPIC_API_KEY",
        Provider::Grok => "XAI_API_KEY",
    }
}

/// Where the key is, without reading it. A search for attributes only does not show
/// the macOS Keychain permission dialog, so the app can start without waiting for it.
pub fn source(provider: Provider) -> KeySource {
    if keychain::exists(account(provider)) {
        return KeySource::Keychain;
    }
    match std::env::var(env_var(provider)) {
        Ok(key) if !key.trim().is_empty() => KeySource::Env,
        _ => KeySource::Missing,
    }
}

/// Reads the key. After a rebuild, macOS can show its Keychain permission dialog here.
pub fn load(provider: Provider) -> (String, KeySource) {
    if let Some(key) = keychain::get(account(provider)) {
        return (key, KeySource::Keychain);
    }
    match std::env::var(env_var(provider)) {
        Ok(key) if !key.trim().is_empty() => (key.trim().to_owned(), KeySource::Env),
        _ => (String::new(), KeySource::Missing),
    }
}

pub fn save(provider: Provider, key: &str) -> Result<(), String> {
    keychain::set(account(provider), key.trim())
}

pub fn delete(provider: Provider) -> Result<(), String> {
    keychain::delete(account(provider))
}

#[cfg(target_os = "macos")]
mod keychain {
    use security_framework::item::{ItemClass, ItemSearchOptions};
    use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};

    use super::SERVICE;

    pub fn exists(account: &str) -> bool {
        ItemSearchOptions::new()
            .class(ItemClass::generic_password())
            .service(SERVICE)
            .account(account)
            .load_attributes(true)
            .limit(1)
            .search()
            .is_ok_and(|found| !found.is_empty())
    }

    pub fn get(account: &str) -> Option<String> {
        let bytes = get_generic_password(SERVICE, account).ok()?;
        String::from_utf8(bytes).ok().filter(|s| !s.is_empty())
    }

    pub fn set(account: &str, value: &str) -> Result<(), String> {
        set_generic_password(SERVICE, account, value.as_bytes()).map_err(|e| format!("Keychain error: {e}"))
    }

    pub fn delete(account: &str) -> Result<(), String> {
        delete_generic_password(SERVICE, account).map_err(|e| format!("Keychain error: {e}"))
    }
}

#[cfg(not(target_os = "macos"))]
mod keychain {
    pub fn exists(_account: &str) -> bool {
        false
    }

    pub fn get(_account: &str) -> Option<String> {
        None
    }

    pub fn set(_account: &str, _value: &str) -> Result<(), String> {
        Err("The Keychain is only available on macOS. Use an environment variable.".to_owned())
    }

    pub fn delete(_account: &str) -> Result<(), String> {
        Ok(())
    }
}
