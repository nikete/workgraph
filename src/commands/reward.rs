use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

use workgraph::identity::{
    self, Reward, EvaluatorInput, load_objective, load_role, record_reward,
    render_evaluator_prompt,
};
use workgraph::config::Config;
use workgraph::graph::{LogEntry, Status};
use workgraph::parser::load_graph;

/// Extract the model from a task's spawn log entry.
///
/// Spawn log entries have the format:
///   "Spawned by coordinator --executor claude --model anthropic/claude-opus-4-6"
/// Returns the model string if found.
fn extract_spawn_model(log: &[LogEntry]) -> Option<String> {
    for entry in log {
        if let Some(rest) = entry.message.strip_prefix("Spawned by ")
            && let Some(idx) = rest.find("--model ")
        {
            let model_start = idx + "--model ".len();
            let model = rest[model_start..].trim();
            if !model.is_empty() {
                return Some(model.to_string());
            }
        }
    }
    None
}

/// Run `wg reward <task-id>` — trigger reward of a completed task.
pub fn run(
    dir: &Path,
    task_id: &str,
    evaluator_model: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    let path = super::graph_path(dir);
    if !path.exists() {
        bail!("Workgraph not initialized. Run `wg init` first.");
    }

    let graph = load_graph(&path)?;
    let task = graph.get_task_or_err(task_id)?;

    // Step 1: Verify task is done or failed
    // Failed tasks are also rewarded — there is useful signal in what kinds
    // of tasks cause which agents to fail (see §4.3 of identity design).
    match task.status {
        Status::Done | Status::Failed => {}
        ref other => {
            bail!(
                "Task '{}' has status {:?} — must be done or failed to reward",
                task_id,
                other
            );
        }
    }

    // Step 2: Load the task's agent and resolve its role + objective
    let identity_dir = dir.join("identity");
    let roles_dir = identity_dir.join("roles");
    let objectives_dir = identity_dir.join("objectives");
    let agents_dir = identity_dir.join("agents");

    let (resolved_agent, role, objective, agent_role_id, agent_objective_id) = if let Some(
        ref agent_hash,
    ) = task.agent
    {
        match identity::find_agent_by_prefix(&agents_dir, agent_hash) {
            Ok(agent) => {
                let role_path = roles_dir.join(format!("{}.yaml", agent.role_id));
                let objective_path = objectives_dir.join(format!("{}.yaml", agent.objective_id));

                let role = if role_path.exists() {
                    Some(load_role(&role_path).context("Failed to load role")?)
                } else {
                    eprintln!(
                        "Warning: role '{}' not found, evaluating without role context",
                        agent.role_id
                    );
                    None
                };

                let objective = if objective_path.exists() {
                    Some(load_objective(&objective_path).context("Failed to load objective")?)
                } else {
                    eprintln!(
                        "Warning: objective '{}' not found, evaluating without objective context",
                        agent.objective_id
                    );
                    None
                };

                let role_id = agent.role_id.clone();
                let objective_id = agent.objective_id.clone();
                (Some(agent), role, objective, role_id, objective_id)
            }
            Err(e) => {
                eprintln!(
                    "Warning: agent '{}' not found ({}), evaluating without agent context",
                    agent_hash, e
                );
                (
                    None,
                    None,
                    None,
                    "unknown".to_string(),
                    "unknown".to_string(),
                )
            }
        }
    } else {
        eprintln!("Note: task has no assigned agent — evaluating without role/objective context");
        (
            None,
            None,
            None,
            "unknown".to_string(),
            "unknown".to_string(),
        )
    };

    // Step 3: Collect task artifacts and log entries
    let artifacts = &task.artifacts;
    let log_entries = &task.log;

    // Step 4: Build evaluator prompt
    let evaluator_input = EvaluatorInput {
        task_title: &task.title,
        task_description: task.description.as_deref(),
        task_skills: &task.skills,
        verify: task.verify.as_deref(),
        agent: resolved_agent.as_ref(),
        role: role.as_ref(),
        objective: objective.as_ref(),
        artifacts,
        log_entries,
        started_at: task.started_at.as_deref(),
        completed_at: task.completed_at.as_deref(),
    };

    let prompt = render_evaluator_prompt(&evaluator_input);

    // Determine the model to use
    let config = Config::load_or_default(dir);
    let model = evaluator_model
        .map(std::string::ToString::to_string)
        .or(config.identity.evaluator_model.clone())
        .or(task.model.clone())
        .unwrap_or_else(|| config.agent.model.clone());

    // Resolve the task execution model early so dry-run can show it
    let task_model_preview = extract_spawn_model(&task.log).or_else(|| task.model.clone());

    // Step 5: --dry-run shows what would be rewarded
    if dry_run {
        println!("=== Dry Run: wg reward {} ===\n", task_id);
        println!("Task: {} ({})", task.title, task_id);
        println!("Status: {:?}", task.status);
        if let Some(ref agent_hash) = task.agent {
            println!("Agent: {}", agent_hash);
            println!("Role: {}", agent_role_id);
            println!("Objective: {}", agent_objective_id);
        } else {
            println!("Agent: (none)");
        }
        println!(
            "Task model:     {}",
            task_model_preview.as_deref().unwrap_or("(unknown)")
        );
        println!("Artifacts: {}", artifacts.len());
        println!("Log entries: {}", log_entries.len());
        println!("Evaluator model: {}", model);
        println!("\n--- Evaluator Prompt ---\n");
        println!("{}", prompt);
        return Ok(());
    }

    // Step 6: Spawn a Claude agent with the evaluator prompt (--print for non-interactive)
    println!("Evaluating task '{}' with model '{}'...", task_id, model);

    let output = Command::new("claude")
        .env_remove("CLAUDE_CODE_ENTRYPOINT")
        .env_remove("CLAUDECODE")
        .arg("--model")
        .arg(&model)
        .arg("--print")
        .arg("--dangerously-skip-permissions")
        .arg(&prompt)
        .output()
        .context("Failed to run claude CLI — is it installed and in PATH?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Claude evaluator failed (exit code {:?}):\n{}",
            output.status.code(),
            stderr
        );
    }

    let raw_output = String::from_utf8_lossy(&output.stdout);

    // Step 7: Parse the JSON output from the evaluator
    let eval_json =
        extract_json(&raw_output).context("Failed to extract valid JSON from evaluator output")?;

    let parsed: EvalOutput = serde_json::from_str(&eval_json)
        .with_context(|| format!("Failed to parse evaluator JSON:\n{}", eval_json))?;

    // Build the Reward record using the agent/role/objective resolved above
    let agent_id = resolved_agent
        .as_ref()
        .map(|a| a.id.clone())
        .unwrap_or_default();
    let role_id = agent_role_id;
    let objective_id = agent_objective_id;

    // Resolve the model that was used to execute this task.
    // Best source: the spawn log entry which records the effective model.
    // Fallback: task.model field.
    let task_model = extract_spawn_model(&task.log).or_else(|| task.model.clone());

    let timestamp = chrono::Utc::now().to_rfc3339();
    let eval_id = format!("eval-{}-{}", task_id, timestamp.replace(':', "-"));

    let reward = Reward {
        id: eval_id,
        task_id: task_id.to_string(),
        agent_id,
        role_id: role_id.clone(),
        objective_id: objective_id.clone(),
        value: parsed.value,
        dimensions: parsed.dimensions,
        notes: parsed.notes,
        evaluator: format!("claude:{}", model),
        timestamp,
        model: task_model.clone(),
        source: "llm".to_string(),
    };

    // Step 8: Save reward and update performance records
    if role_id != "unknown" && objective_id != "unknown" {
        let eval_path =
            record_reward(&reward, &identity_dir).context("Failed to record reward")?;

        if json {
            let out = serde_json::json!({
                "task_id": task_id,
                "reward_id": reward.id,
                "value": reward.value,
                "dimensions": reward.dimensions,
                "notes": reward.notes,
                "evaluator": reward.evaluator,
                "model": reward.model,
                "path": eval_path.display().to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("\n=== Reward Complete ===");
            println!("Task:       {} ({})", task.title, task_id);
            if let Some(ref m) = reward.model {
                println!("Model:      {}", m);
            }
            println!("Reward:     {:.2}", reward.value);
            if let Some(c) = reward.dimensions.get("correctness") {
                println!("  correctness:      {:.2}", c);
            }
            if let Some(c) = reward.dimensions.get("completeness") {
                println!("  completeness:     {:.2}", c);
            }
            if let Some(e) = reward.dimensions.get("efficiency") {
                println!("  efficiency:       {:.2}", e);
            }
            if let Some(s) = reward.dimensions.get("style_adherence") {
                println!("  style_adherence:  {:.2}", s);
            }
            println!("Notes:      {}", reward.notes);
            println!("Evaluator:  {}", reward.evaluator);
            println!("Saved to:   {}", eval_path.display());
        }
    } else {
        // No identity — save reward directly without updating performance records
        identity::init(&identity_dir)?;
        let eval_path = identity::save_reward(&reward, &identity_dir.join("rewards"))
            .context("Failed to save reward")?;

        if json {
            let out = serde_json::json!({
                "task_id": task_id,
                "reward_id": reward.id,
                "value": reward.value,
                "dimensions": reward.dimensions,
                "notes": reward.notes,
                "evaluator": reward.evaluator,
                "model": reward.model,
                "path": eval_path.display().to_string(),
                "warning": "No identity assigned — performance records not updated",
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("\n=== Reward Complete ===");
            println!("Task:       {} ({})", task.title, task_id);
            if let Some(ref m) = reward.model {
                println!("Model:      {}", m);
            }
            println!("Reward:     {:.2}", reward.value);
            println!("Notes:      {}", reward.notes);
            println!("Evaluator:  {}", reward.evaluator);
            println!("Saved to:   {}", eval_path.display());
            println!(
                "Warning: no identity assigned — role/objective performance records not updated"
            );
        }
    }

    Ok(())
}

/// Output shape we expect from the evaluator LLM.
#[derive(serde::Deserialize)]
struct EvalOutput {
    value: f64,
    #[serde(default)]
    dimensions: std::collections::HashMap<String, f64>,
    #[serde(default)]
    notes: String,
}

/// Extract a JSON object from potentially noisy LLM output.
///
/// The evaluator is instructed to return only JSON, but it may wrap it in
/// markdown fences or include leading/trailing commentary. This function
/// finds the first `{...}` that parses as valid JSON.
fn extract_json(raw: &str) -> Option<String> {
    // Try the whole string first (ideal case)
    let trimmed = raw.trim();
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return Some(trimmed.to_string());
    }

    // Strip markdown code fences if present
    let stripped = if trimmed.starts_with("```") {
        let inner = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        if serde_json::from_str::<serde_json::Value>(inner).is_ok() {
            return Some(inner.to_string());
        }
        inner
    } else {
        trimmed
    };

    // Find the first { and last } and try to parse
    if let Some(start) = stripped.find('{')
        && let Some(end) = stripped.rfind('}')
    {
        let candidate = &stripped[start..=end];
        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
            return Some(candidate.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_plain() {
        let input = r#"{"value": 0.85, "dimensions": {}, "notes": "Good work"}"#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("0.85"));
    }

    #[test]
    fn extract_json_with_fences() {
        let input = "```json\n{\"value\": 0.7, \"dimensions\": {}, \"notes\": \"ok\"}\n```";
        let result = extract_json(input).unwrap();
        assert!(result.contains("0.7"));
    }

    #[test]
    fn extract_json_with_surrounding_text() {
        let input = "Here is my reward:\n{\"value\": 0.9, \"notes\": \"great\"}\nEnd.";
        let result = extract_json(input).unwrap();
        assert!(result.contains("0.9"));
    }

    #[test]
    fn extract_json_returns_none_for_garbage() {
        assert!(extract_json("no json here at all").is_none());
    }

    #[test]
    fn parse_eval_output_minimal() {
        let json = r#"{"value": 0.75}"#;
        let parsed: EvalOutput = serde_json::from_str(json).unwrap();
        assert!((parsed.value - 0.75).abs() < f64::EPSILON);
        assert!(parsed.dimensions.is_empty());
        assert!(parsed.notes.is_empty());
    }

    #[test]
    fn parse_eval_output_full() {
        let json = r#"{
            "value": 0.82,
            "dimensions": {
                "correctness": 0.9,
                "completeness": 0.8,
                "efficiency": 0.75,
                "style_adherence": 0.8
            },
            "notes": "Well implemented but could be more efficient"
        }"#;
        let parsed: EvalOutput = serde_json::from_str(json).unwrap();
        assert!((parsed.value - 0.82).abs() < f64::EPSILON);
        assert_eq!(parsed.dimensions.len(), 4);
        assert_eq!(parsed.notes, "Well implemented but could be more efficient");
    }
}
