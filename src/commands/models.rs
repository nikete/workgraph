use anyhow::Result;
use std::path::Path;
use workgraph::models::{ModelEntry, ModelRegistry, ModelTier};

/// List all models in the registry
pub fn run_list(workgraph_dir: &Path, tier: Option<&str>, json: bool) -> Result<()> {
    let registry = ModelRegistry::load(workgraph_dir)?;

    let tier_filter = tier.map(|t| t.parse::<ModelTier>()).transpose()?;
    let models = registry.list(tier_filter.as_ref());

    if json {
        let json_val: Vec<_> = models
            .iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "provider": m.provider,
                    "cost_per_1m_input": m.cost_per_1m_input,
                    "cost_per_1m_output": m.cost_per_1m_output,
                    "context_window": m.context_window,
                    "capabilities": m.capabilities,
                    "tier": m.tier,
                    "is_default": registry.default_model.as_deref() == Some(&*m.id),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_val)?);
        return Ok(());
    }

    if models.is_empty() {
        println!("No models found.");
        return Ok(());
    }

    // Table header
    println!(
        "{:<35} {:<8} {:>10} {:>11} {:>10} {}",
        "MODEL", "TIER", "IN/1M", "OUT/1M", "CTX", "CAPABILITIES"
    );
    println!("{}", "-".repeat(100));

    for model in &models {
        let is_default = registry.default_model.as_deref() == Some(&*model.id);
        let marker = if is_default { " *" } else { "" };
        let ctx = format_context_window(model.context_window);
        let caps = model.capabilities.join(", ");

        println!(
            "{:<35} {:<8} {:>9.2} {:>10.2} {:>10} {}",
            format!("{}{}", model.id, marker),
            model.tier,
            model.cost_per_1m_input,
            model.cost_per_1m_output,
            ctx,
            caps,
        );
    }

    if let Some(default) = &registry.default_model {
        println!("\n  * = default model ({})", default);
    }

    Ok(())
}

/// Add a custom model to the registry
pub fn run_add(
    workgraph_dir: &Path,
    id: &str,
    provider: Option<&str>,
    cost_in: f64,
    cost_out: f64,
    context_window: Option<u64>,
    capabilities: &[String],
    tier: &str,
) -> Result<()> {
    let mut registry = ModelRegistry::load(workgraph_dir)?;

    let tier = tier.parse::<ModelTier>()?;

    let entry = ModelEntry {
        id: id.to_string(),
        provider: provider.unwrap_or("openrouter").to_string(),
        cost_per_1m_input: cost_in,
        cost_per_1m_output: cost_out,
        context_window: context_window.unwrap_or(128_000),
        capabilities: capabilities.to_vec(),
        tier,
    };

    let existed = registry.get(id).is_some();
    registry.add(entry);
    registry.save(workgraph_dir)?;

    if existed {
        println!("Updated model: {}", id);
    } else {
        println!("Added model: {}", id);
    }

    Ok(())
}

/// Set the default model for the coordinator
pub fn run_set_default(workgraph_dir: &Path, id: &str) -> Result<()> {
    let mut registry = ModelRegistry::load(workgraph_dir)?;
    registry.set_default(id)?;
    registry.save(workgraph_dir)?;
    println!("Default model set to: {}", id);
    Ok(())
}

/// Initialize the models.yaml file with defaults if it doesn't exist
pub fn run_init(workgraph_dir: &Path) -> Result<()> {
    let path = workgraph_dir.join("models.yaml");
    if path.exists() {
        println!("models.yaml already exists. Use 'wg models list' to view.");
        return Ok(());
    }

    let registry = ModelRegistry::with_defaults();
    registry.save(workgraph_dir)?;
    println!(
        "Created models.yaml with {} default models.",
        registry.models.len()
    );
    Ok(())
}

fn format_context_window(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{}M", tokens / 1_000_000)
    } else {
        format!("{}k", tokens / 1_000)
    }
}
