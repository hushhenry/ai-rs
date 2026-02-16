use futures::StreamExt;
use zeroai::auth::config::ConfigManager;
use zeroai::mapper::ModelMapper;

pub async fn run_doctor(model_filter: Option<&str>) -> anyhow::Result<()> {
    let config = ConfigManager::default_path();
    let enabled_models = config.get_enabled_models()?;

    if enabled_models.is_empty() {
        println!("No models configured. Run `zeroai-proxy config` first.");
        return Ok(());
    }

    let mapper = ModelMapper::new();

    let models_to_check: Vec<String> = if let Some(filter) = model_filter {
        if enabled_models.contains(&filter.to_string()) {
            vec![filter.to_string()]
        } else {
            println!("Model not found in enabled list: {}", filter);
            return Ok(());
        }
    } else {
        use rand::seq::IndexedRandom;
        let mut provider_models: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for full_id in &enabled_models {
            if let Some((provider, _)) = mapper.split_id(full_id) {
                provider_models
                    .entry(provider.to_string())
                    .or_default()
                    .push(full_id.clone());
            }
        }
        let mut rng = rand::rng();
        provider_models
            .values()
            .filter_map(|models| models.choose(&mut rng).cloned())
            .collect()
    };

    if models_to_check.is_empty() {
        println!("No models to check.");
        return Ok(());
    }

    for full_id in &models_to_check {
        let (provider, _) = match mapper.split_id(full_id) {
            Some(p) => p,
            None => continue,
        };

        let api_key = config.resolve_api_key(provider).await?;
        if api_key.is_none() {
            println!("  {} - No credentials", full_id);
            continue;
        }

        println!("\nChecking {}...", full_id);

        match check_model(full_id, &api_key.unwrap()).await {
            Ok(report) => {
                if report.success {
                    println!("  Result: OK ({} chars)", report.response_len);
                } else {
                    println!("  Result: FAILED - {}", report.error.unwrap_or_default());
                }
            }
            Err(e) => {
                println!("  Result: ERROR - {}", e);
            }
        }
    }

    println!("\nDoctor check complete.");
    Ok(())
}

struct CheckReport {
    success: bool,
    response_len: usize,
    error: Option<String>,
}

async fn check_model(full_id: &str, api_key: &str) -> anyhow::Result<CheckReport> {
    let mapper = ModelMapper::new();
    let (provider, short_model) = mapper.split_id(full_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid model ID: {}", full_id))?;

    let adapter_kind = zeroai::adapter::AdapterKind::from_lower_str(provider)
        .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", provider))?;

    let model_iden = zeroai::ModelIden::new(adapter_kind, short_model);
    let target = zeroai::ServiceTarget {
        endpoint: zeroai::Client::default_endpoint(adapter_kind),
        auth: zeroai::AuthData::from_single(api_key),
        model: model_iden,
    };

    let client = zeroai::Client::default();
    let chat_req = zeroai::chat::ChatRequest::from_user("Say hello in one word.");

    match client.exec_chat(target, chat_req, None).await {
        Ok(res) => {
            let text = res.first_text().unwrap_or("").to_string();
            Ok(CheckReport {
                success: true,
                response_len: text.len(),
                error: None,
            })
        }
        Err(e) => Ok(CheckReport {
            success: false,
            response_len: 0,
            error: Some(e.to_string()),
        }),
    }
}
