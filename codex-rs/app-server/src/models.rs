use std::sync::Arc;

use codex_app_server_protocol::Model;
use codex_app_server_protocol::ModelServiceTier;
use codex_app_server_protocol::ModelUpgradeInfo;
use codex_app_server_protocol::ReasoningEffortOption;
use codex_core::ThreadManager;
use codex_core::config::Config;
use codex_http_client::HttpClientFactory;
use codex_models_manager::manager::RefreshStrategy;
use codex_protocol::openai_models::ModelPreset;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::openai_models::default_input_modalities;

use crate::profiles::SelectableProfile;
use crate::profiles::selectable_profiles;

pub async fn supported_models(
    thread_manager: Arc<ThreadManager>,
    include_hidden: bool,
    http_client_factory: HttpClientFactory,
    config: &Config,
) -> Vec<Model> {
    let mut models: Vec<Model> = thread_manager
        .list_models(RefreshStrategy::OnlineIfUncached, http_client_factory)
        .await
        .into_iter()
        .filter(|preset| include_hidden || preset.show_in_picker)
        .map(model_from_preset)
        .collect();

    // Config-defined profiles join the picker so clients can switch routes
    // (model + provider) without hand-writing a model catalog file.
    for profile in selectable_profiles(config) {
        if models.iter().any(|model| model.model == profile.name) {
            continue;
        }
        models.push(model_from_preset(preset_from_profile(&profile)));
    }
    models
}

fn preset_from_profile(profile: &SelectableProfile) -> ModelPreset {
    let effort = profile
        .reasoning_effort
        .clone()
        .unwrap_or(ReasoningEffort::Medium);
    let description = match &profile.model_provider {
        Some(provider) => format!("{} · {provider}", profile.model),
        None => profile.model.clone(),
    };
    ModelPreset {
        id: profile.name.clone(),
        model: profile.name.clone(),
        display_name: profile.name.clone(),
        description,
        model_specialty: None,
        default_reasoning_effort: effort.clone(),
        supported_reasoning_efforts: vec![ReasoningEffortPreset {
            effort,
            description: String::new(),
        }],
        supports_personality: false,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        default_service_tier: None,
        is_default: false,
        upgrade: None,
        show_in_picker: true,
        multi_agent_version: None,
        availability_nux: None,
        supported_in_api: true,
        input_modalities: default_input_modalities(),
    }
}

fn model_from_preset(preset: ModelPreset) -> Model {
    Model {
        id: preset.id.to_string(),
        model: preset.model.to_string(),
        upgrade: preset.upgrade.as_ref().map(|upgrade| upgrade.id.clone()),
        upgrade_info: preset.upgrade.as_ref().map(|upgrade| ModelUpgradeInfo {
            model: upgrade.id.clone(),
            upgrade_copy: upgrade.upgrade_copy.clone(),
            model_link: upgrade.model_link.clone(),
            migration_markdown: upgrade.migration_markdown.clone(),
            retirement_at: upgrade
                .retirement_at
                .as_ref()
                .map(chrono::DateTime::timestamp),
        }),
        availability_nux: preset.availability_nux.map(Into::into),
        display_name: preset.display_name.to_string(),
        description: preset.description.to_string(),
        model_specialty: preset.model_specialty,
        hidden: !preset.show_in_picker,
        supported_reasoning_efforts: reasoning_efforts_from_preset(
            preset.supported_reasoning_efforts,
        ),
        default_reasoning_effort: preset.default_reasoning_effort,
        input_modalities: preset.input_modalities,
        supports_personality: preset.supports_personality,
        multi_agent_version: preset.multi_agent_version.map(Into::into),
        additional_speed_tiers: preset.additional_speed_tiers,
        service_tiers: preset
            .service_tiers
            .into_iter()
            .map(|service_tier| ModelServiceTier {
                id: service_tier.id,
                name: service_tier.name,
                description: service_tier.description,
            })
            .collect(),
        default_service_tier: preset.default_service_tier,
        is_default: preset.is_default,
    }
}

fn reasoning_efforts_from_preset(
    efforts: Vec<ReasoningEffortPreset>,
) -> Vec<ReasoningEffortOption> {
    efforts
        .into_iter()
        .map(|preset| ReasoningEffortOption {
            reasoning_effort: preset.effort,
            description: preset.description,
        })
        .collect()
}
