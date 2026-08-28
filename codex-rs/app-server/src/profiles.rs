//! Config-defined profile discovery for the model picker.
//!
//! Profiles (`[profiles.<name>]` in `config.toml` or `<name>.config.toml`
//! files in CODEX_HOME) describe a model + provider route. We surface them as
//! `model/list` entries and expand the profile name back into its real
//! model/provider pair when a thread starts with one selected.

use std::collections::HashMap;

use codex_core::config::Config;
use codex_protocol::openai_models::ReasoningEffort;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectableProfile {
    pub name: String,
    pub model: String,
    pub model_provider: Option<String>,
    pub reasoning_effort: Option<ReasoningEffort>,
}

pub(crate) fn selectable_profiles(config: &Config) -> Vec<SelectableProfile> {
    let mut profiles = legacy_config_profiles(config);
    for profile in file_config_profiles(config) {
        if !profiles.iter().any(|existing| existing.name == profile.name) {
            profiles.push(profile);
        }
    }
    profiles.retain(|profile| !profile.name.is_empty() && !profile.model.is_empty());
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles
}

pub(crate) fn find_profile<'a>(
    profiles: &'a [SelectableProfile],
    name: &str,
) -> Option<&'a SelectableProfile> {
    profiles.iter().find(|profile| profile.name == name)
}

/// Resolve a picker selection at thread start: a `model` value matching a
/// profile name expands into the profile's real model, provider, and effort.
pub(crate) fn expand_profile_for_new_thread(
    config: &Config,
    model: Option<String>,
    model_provider: Option<String>,
    mut overrides: Option<HashMap<String, serde_json::Value>>,
) -> (
    Option<String>,
    Option<String>,
    Option<HashMap<String, serde_json::Value>>,
) {
    let Some(requested) = model.as_deref() else {
        return (model, model_provider, overrides);
    };
    let profiles = selectable_profiles(config);
    let Some(profile) = find_profile(&profiles, requested) else {
        return (model, model_provider, overrides);
    };

    let model_provider = model_provider.or_else(|| profile.model_provider.clone());
    if let Some(effort) = &profile.reasoning_effort {
        overrides
            .get_or_insert_with(HashMap::new)
            .entry("model_reasoning_effort".to_string())
            .or_insert_with(|| serde_json::Value::String(effort.as_str().to_string()));
    }
    (Some(profile.model.clone()), model_provider, overrides)
}

/// Resolve a picker selection on a live thread. Providers cannot change
/// mid-thread, so a profile routing elsewhere is rejected with guidance.
pub(crate) fn expand_profile_for_active_thread(
    config: &Config,
    active_provider: &str,
    model: Option<String>,
    effort: Option<ReasoningEffort>,
) -> Result<(Option<String>, Option<ReasoningEffort>), String> {
    let Some(requested) = model.as_deref() else {
        return Ok((model, effort));
    };
    let profiles = selectable_profiles(config);
    let Some(profile) = find_profile(&profiles, requested) else {
        return Ok((model, effort));
    };

    if let Some(provider) = &profile.model_provider
        && provider != active_provider
    {
        return Err(format!(
            "profile `{}` routes to provider `{provider}` but this thread uses `{active_provider}`; start a new thread to switch providers",
            profile.name
        ));
    }
    Ok((
        Some(profile.model.clone()),
        effort.or(profile.reasoning_effort.clone()),
    ))
}

fn legacy_config_profiles(config: &Config) -> Vec<SelectableProfile> {
    let effective = config.config_layer_stack.effective_config();
    let Some(profiles) = effective.get("profiles").and_then(|value| value.as_table()) else {
        return Vec::new();
    };
    profiles
        .iter()
        .filter_map(|(name, value)| profile_from_toml_table(name.clone(), value.as_table()?))
        .collect()
}

fn file_config_profiles(config: &Config) -> Vec<SelectableProfile> {
    let Ok(entries) = std::fs::read_dir(config.codex_home.as_path()) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let file_name = path.file_name()?.to_str()?;
            let name = file_name.strip_suffix(".config.toml")?;
            let contents = std::fs::read_to_string(&path).ok()?;
            let parsed = contents.parse::<codex_config::TomlValue>().ok()?;
            profile_from_toml_table(name.to_string(), parsed.as_table()?)
        })
        .collect()
}

fn profile_from_toml_table(
    name: String,
    table: &toml::map::Map<String, codex_config::TomlValue>,
) -> Option<SelectableProfile> {
    let model = table.get("model")?.as_str()?.to_string();
    let model_provider = table
        .get("model_provider")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let reasoning_effort = table
        .get("model_reasoning_effort")
        .and_then(|value| value.clone().try_into::<ReasoningEffort>().ok());
    Some(SelectableProfile {
        name,
        model,
        model_provider,
        reasoning_effort,
    })
}
