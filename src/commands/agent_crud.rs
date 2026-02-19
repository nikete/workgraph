use anyhow::{Context, Result};
use std::path::Path;
use workgraph::identity::{self, Agent, Lineage, RewardHistory};
use workgraph::graph::TrustLevel;

/// Get the identity agents subdirectory (creates identity structure if needed).
fn agents_dir(workgraph_dir: &Path) -> Result<std::path::PathBuf> {
    let identity_dir = workgraph_dir.join("identity");
    identity::init(&identity_dir).context("Failed to initialise identity directory")?;
    Ok(identity_dir.join("agents"))
}

/// Parse a trust level string into a TrustLevel enum.
fn parse_trust_level(s: &str) -> Result<TrustLevel> {
    match s.to_lowercase().as_str() {
        "verified" => Ok(TrustLevel::Verified),
        "provisional" => Ok(TrustLevel::Provisional),
        "unknown" => Ok(TrustLevel::Unknown),
        _ => anyhow::bail!(
            "Invalid trust level '{}'. Expected: verified, provisional, unknown",
            s
        ),
    }
}

/// `wg agent create <name> [--role <hash>] [--objective <hash>] [--capabilities ...] [--rate N] [--capacity N] [--trust-level L] [--contact C] [--executor E]`
#[allow(clippy::too_many_arguments)]
pub fn run_create(
    workgraph_dir: &Path,
    name: &str,
    role_id: Option<&str>,
    objective_id: Option<&str>,
    capabilities: &[String],
    rate: Option<f64>,
    capacity: Option<f64>,
    trust_level: Option<&str>,
    contact: Option<&str>,
    executor: &str,
) -> Result<()> {
    let identity_dir = workgraph_dir.join("identity");
    identity::init(&identity_dir).context("Failed to initialise identity directory")?;

    let roles_dir = identity_dir.join("roles");
    let objectives_dir = identity_dir.join("objectives");

    let is_human = identity::is_human_executor(executor);

    // Resolve role and objective if provided
    let resolved_role = match role_id {
        Some(rid) => Some(
            identity::find_role_by_prefix(&roles_dir, rid)
                .with_context(|| format!("Failed to find role '{}'", rid))?,
        ),
        None => {
            if !is_human {
                anyhow::bail!("--role is required for AI agents (executor={})", executor);
            }
            None
        }
    };

    let resolved_objective = match objective_id {
        Some(mid) => Some(
            identity::find_objective_by_prefix(&objectives_dir, mid)
                .with_context(|| format!("Failed to find objective '{}'", mid))?,
        ),
        None => {
            if !is_human {
                anyhow::bail!(
                    "--objective is required for AI agents (executor={})",
                    executor
                );
            }
            None
        }
    };

    // Compute agent ID based on available identity fields
    let (agent_role_id, agent_objective_id, id) = match (&resolved_role, &resolved_objective) {
        (Some(role), Some(mot)) => {
            let id = identity::content_hash_agent(&role.id, &mot.id);
            (role.id.clone(), mot.id.clone(), id)
        }
        _ => {
            // For human agents without role/objective, hash the name + executor
            use sha2::{Digest, Sha256};
            let input = format!("human-agent:{}:{}", name, executor);
            let digest = Sha256::digest(input.as_bytes());
            let id = format!("{:x}", digest);
            let role_id = resolved_role
                .as_ref()
                .map(|r| r.id.clone())
                .unwrap_or_default();
            let mot_id = resolved_objective
                .as_ref()
                .map(|m| m.id.clone())
                .unwrap_or_default();
            (role_id, mot_id, id)
        }
    };

    let agents_dir = identity_dir.join("agents");
    let agent_path = agents_dir.join(format!("{}.yaml", id));
    if agent_path.exists() {
        anyhow::bail!(
            "Agent with identical identity already exists ({})",
            identity::short_hash(&id)
        );
    }

    let trust = match trust_level {
        Some(s) => parse_trust_level(s)?,
        None => TrustLevel::default(),
    };

    let agent = Agent {
        id,
        role_id: agent_role_id,
        objective_id: agent_objective_id,
        name: name.to_string(),
        performance: RewardHistory {
            task_count: 0,
            mean_reward: None,
            rewards: vec![],
        },
        lineage: Lineage::default(),
        capabilities: capabilities.to_vec(),
        rate,
        capacity,
        trust_level: trust,
        contact: contact.map(std::string::ToString::to_string),
        executor: executor.to_string(),
    };

    let path = identity::save_agent(&agent, &agents_dir).context("Failed to save agent")?;

    println!(
        "Created agent '{}' ({}) at {}",
        name,
        identity::short_hash(&agent.id),
        path.display()
    );

    if let Some(role) = &resolved_role {
        println!(
            "  role:       {} ({})",
            role.name,
            identity::short_hash(&role.id)
        );
    }
    if let Some(mot) = &resolved_objective {
        println!(
            "  objective: {} ({})",
            mot.name,
            identity::short_hash(&mot.id)
        );
    }
    println!("  executor:   {}", executor);
    if !capabilities.is_empty() {
        println!("  capabilities: {}", capabilities.join(", "));
    }
    if let Some(r) = rate {
        println!("  rate:       {}", r);
    }
    if let Some(c) = capacity {
        println!("  capacity:   {}", c);
    }
    if let Some(ct) = contact {
        println!("  contact:    {}", ct);
    }

    Ok(())
}

/// `wg agent list [--json]`
pub fn run_list(workgraph_dir: &Path, json: bool) -> Result<()> {
    let dir = agents_dir(workgraph_dir)?;
    let agents = identity::load_all_agents(&dir).context("Failed to load agents")?;

    if json {
        let output: Vec<serde_json::Value> = agents
            .iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "name": a.name,
                    "role_id": a.role_id,
                    "objective_id": a.objective_id,
                    "executor": a.executor,
                    "capabilities": a.capabilities,
                    "mean_reward": a.performance.mean_reward,
                    "task_count": a.performance.task_count,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if agents.is_empty() {
        println!("No agents defined. Use 'wg agent create' to create one.");
    } else {
        println!("Agents:\n");
        for a in &agents {
            let value_str = a
                .performance
                .mean_reward
                .map(|s| format!("{:.2}", s))
                .unwrap_or_else(|| "n/a".to_string());
            let role_str = if a.role_id.is_empty() {
                "-".to_string()
            } else {
                identity::short_hash(&a.role_id).to_string()
            };
            let mot_str = if a.objective_id.is_empty() {
                "-".to_string()
            } else {
                identity::short_hash(&a.objective_id).to_string()
            };
            println!(
                "  {}  {:20} role:{} mot:{} exec:{} reward:{} tasks:{}",
                identity::short_hash(&a.id),
                a.name,
                role_str,
                mot_str,
                a.executor,
                value_str,
                a.performance.task_count,
            );
        }
    }

    Ok(())
}

/// `wg agent show <hash> [--json]`
pub fn run_show(workgraph_dir: &Path, id: &str, json: bool) -> Result<()> {
    let identity_dir = workgraph_dir.join("identity");
    let dir = identity_dir.join("agents");
    let agent = identity::find_agent_by_prefix(&dir, id)
        .with_context(|| format!("Failed to find agent '{}'", id))?;

    if json {
        // Include resolved role/objective names in JSON output
        let roles_dir = identity_dir.join("roles");
        let objectives_dir = identity_dir.join("objectives");

        let role_name = identity::find_role_by_prefix(&roles_dir, &agent.role_id)
            .map(|r| r.name)
            .unwrap_or_else(|_| "(not found)".to_string());
        let objective_name =
            identity::find_objective_by_prefix(&objectives_dir, &agent.objective_id)
                .map(|m| m.name)
                .unwrap_or_else(|_| "(not found)".to_string());

        let output = serde_json::json!({
            "id": agent.id,
            "name": agent.name,
            "role_id": agent.role_id,
            "role_name": role_name,
            "objective_id": agent.objective_id,
            "objective_name": objective_name,
            "executor": agent.executor,
            "capabilities": agent.capabilities,
            "rate": agent.rate,
            "capacity": agent.capacity,
            "trust_level": agent.trust_level,
            "contact": agent.contact,
            "performance": {
                "task_count": agent.performance.task_count,
                "mean_reward": agent.performance.mean_reward,
                "rewards": agent.performance.rewards.len(),
            },
            "lineage": {
                "generation": agent.lineage.generation,
                "parent_ids": agent.lineage.parent_ids,
                "created_by": agent.lineage.created_by,
                "created_at": agent.lineage.created_at.to_rfc3339(),
            },
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Agent: {} ({})", agent.name, identity::short_hash(&agent.id));
        println!("ID: {}", agent.id);
        println!();

        // Resolve role name
        let roles_dir = identity_dir.join("roles");
        let objectives_dir = identity_dir.join("objectives");

        match identity::find_role_by_prefix(&roles_dir, &agent.role_id) {
            Ok(role) => println!("Role: {} ({})", role.name, identity::short_hash(&role.id)),
            Err(_) => println!("Role: {} (not found)", identity::short_hash(&agent.role_id)),
        }

        match identity::find_objective_by_prefix(&objectives_dir, &agent.objective_id) {
            Ok(objective) => println!(
                "Objective: {} ({})",
                objective.name,
                identity::short_hash(&objective.id)
            ),
            Err(_) => println!(
                "Objective: {} (not found)",
                identity::short_hash(&agent.objective_id)
            ),
        }

        println!();
        println!("Executor: {}", agent.executor);
        if !agent.capabilities.is_empty() {
            println!("Capabilities: {}", agent.capabilities.join(", "));
        }
        if let Some(rate) = agent.rate {
            println!("Rate: {}", rate);
        }
        if let Some(capacity) = agent.capacity {
            println!("Capacity: {}", capacity);
        }
        if agent.trust_level != TrustLevel::Provisional {
            println!("Trust level: {:?}", agent.trust_level);
        }
        if let Some(contact) = &agent.contact {
            println!("Contact: {}", contact);
        }

        println!();
        println!("Performance:");
        println!("  Tasks: {}", agent.performance.task_count);
        let value_str = agent
            .performance
            .mean_reward
            .map(|s| format!("{:.2}", s))
            .unwrap_or_else(|| "n/a".to_string());
        println!("  Avg reward: {}", value_str);
        if !agent.performance.rewards.is_empty() {
            println!("  Rewards: {}", agent.performance.rewards.len());
        }

        println!();
        println!("Lineage:");
        println!("  Generation: {}", agent.lineage.generation);
        println!("  Created by: {}", agent.lineage.created_by);
        if !agent.lineage.parent_ids.is_empty() {
            let short_parents: Vec<&str> = agent
                .lineage
                .parent_ids
                .iter()
                .map(|p| identity::short_hash(p))
                .collect();
            println!("  Parents: {}", short_parents.join(", "));
        }
    }

    Ok(())
}

/// `wg agent rm <hash>`
pub fn run_rm(workgraph_dir: &Path, id: &str) -> Result<()> {
    let dir = agents_dir(workgraph_dir)?;
    let agent = identity::find_agent_by_prefix(&dir, id)
        .with_context(|| format!("Failed to find agent '{}'", id))?;

    let path = dir.join(format!("{}.yaml", agent.id));
    std::fs::remove_file(&path)
        .with_context(|| format!("Failed to remove agent file: {}", path.display()))?;

    println!(
        "Removed agent '{}' ({})",
        agent.name,
        identity::short_hash(&agent.id)
    );
    Ok(())
}

/// `wg agent lineage <hash> [--json]`
///
/// Shows the agent itself plus the ancestry of its constituent role and objective.
pub fn run_lineage(workgraph_dir: &Path, id: &str, json: bool) -> Result<()> {
    let identity_dir = workgraph_dir.join("identity");
    let agents_dir = identity_dir.join("agents");
    let roles_dir = identity_dir.join("roles");
    let objectives_dir = identity_dir.join("objectives");

    let agent = identity::find_agent_by_prefix(&agents_dir, id)
        .with_context(|| format!("Failed to find agent '{}'", id))?;

    let role_ancestry = identity::role_ancestry(&agent.role_id, &roles_dir).unwrap_or_else(|e| {
        eprintln!(
            "Warning: failed to load role ancestry for '{}': {}",
            agent.role_id, e
        );
        Vec::new()
    });
    let objective_ancestry = identity::objective_ancestry(&agent.objective_id, &objectives_dir)
        .unwrap_or_else(|e| {
            eprintln!(
                "Warning: failed to load objective ancestry for '{}': {}",
                agent.objective_id, e
            );
            Vec::new()
        });

    if json {
        let output = serde_json::json!({
            "agent": {
                "id": agent.id,
                "name": agent.name,
                "generation": agent.lineage.generation,
                "created_by": agent.lineage.created_by,
                "created_at": agent.lineage.created_at.to_rfc3339(),
                "parent_ids": agent.lineage.parent_ids,
            },
            "role_ancestry": role_ancestry.iter().map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "name": n.name,
                    "generation": n.generation,
                    "created_by": n.created_by,
                    "created_at": n.created_at.to_rfc3339(),
                    "parent_ids": n.parent_ids,
                })
            }).collect::<Vec<_>>(),
            "objective_ancestry": objective_ancestry.iter().map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "name": n.name,
                    "generation": n.generation,
                    "created_by": n.created_by,
                    "created_at": n.created_at.to_rfc3339(),
                    "parent_ids": n.parent_ids,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!(
        "Lineage for agent: {} ({})",
        agent.name,
        identity::short_hash(&agent.id)
    );
    println!("  Generation: {}", agent.lineage.generation);
    println!("  Created by: {}", agent.lineage.created_by);
    if !agent.lineage.parent_ids.is_empty() {
        let short_parents: Vec<&str> = agent
            .lineage
            .parent_ids
            .iter()
            .map(|p| identity::short_hash(p))
            .collect();
        println!("  Parents: [{}]", short_parents.join(", "));
    }

    println!();
    println!("Role ancestry ({})", identity::short_hash(&agent.role_id));
    if role_ancestry.is_empty() {
        println!("  (role not found)");
    } else {
        for node in &role_ancestry {
            let indent = "  ".repeat(node.generation as usize + 1);
            let gen_label = if node.generation == 0 {
                "gen 0 (root)".to_string()
            } else {
                format!("gen {}", node.generation)
            };
            let parents = if node.parent_ids.is_empty() {
                String::new()
            } else {
                let short_parents: Vec<&str> = node
                    .parent_ids
                    .iter()
                    .map(|p| identity::short_hash(p))
                    .collect();
                format!(" <- [{}]", short_parents.join(", "))
            };
            println!(
                "{}{} ({}) [{}] created by: {}{}",
                indent,
                identity::short_hash(&node.id),
                node.name,
                gen_label,
                node.created_by,
                parents
            );
        }
    }

    println!();
    println!(
        "Objective ancestry ({})",
        identity::short_hash(&agent.objective_id)
    );
    if objective_ancestry.is_empty() {
        println!("  (objective not found)");
    } else {
        for node in &objective_ancestry {
            let indent = "  ".repeat(node.generation as usize + 1);
            let gen_label = if node.generation == 0 {
                "gen 0 (root)".to_string()
            } else {
                format!("gen {}", node.generation)
            };
            let parents = if node.parent_ids.is_empty() {
                String::new()
            } else {
                let short_parents: Vec<&str> = node
                    .parent_ids
                    .iter()
                    .map(|p| identity::short_hash(p))
                    .collect();
                format!(" <- [{}]", short_parents.join(", "))
            };
            println!(
                "{}{} ({}) [{}] created by: {}{}",
                indent,
                identity::short_hash(&node.id),
                node.name,
                gen_label,
                node.created_by,
                parents
            );
        }
    }

    Ok(())
}

/// `wg agent performance <hash> [--json]`
///
/// Shows the reward history for this agent.
pub fn run_performance(workgraph_dir: &Path, id: &str, json: bool) -> Result<()> {
    let identity_dir = workgraph_dir.join("identity");
    let agents_dir = identity_dir.join("agents");

    let agent = identity::find_agent_by_prefix(&agents_dir, id)
        .with_context(|| format!("Failed to find agent '{}'", id))?;

    // Load all rewards and filter to this agent's role+objective pair
    let evals_dir = identity_dir.join("rewards");
    let all_evals = identity::load_all_rewards_or_warn(&evals_dir);

    let agent_evals: Vec<_> = all_evals
        .iter()
        .filter(|e| e.role_id == agent.role_id && e.objective_id == agent.objective_id)
        .collect();

    if json {
        let output = serde_json::json!({
            "agent_id": agent.id,
            "agent_name": agent.name,
            "task_count": agent.performance.task_count,
            "mean_reward": agent.performance.mean_reward,
            "inline_rewards": agent.performance.rewards.iter().map(|e| {
                serde_json::json!({
                    "value": e.value,
                    "task_id": e.task_id,
                    "timestamp": e.timestamp,
                    "context_id": e.context_id,
                })
            }).collect::<Vec<_>>(),
            "full_rewards": agent_evals.iter().map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "task_id": e.task_id,
                    "value": e.value,
                    "dimensions": e.dimensions,
                    "notes": e.notes,
                    "evaluator": e.evaluator,
                    "timestamp": e.timestamp,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!(
        "Performance for agent: {} ({})",
        agent.name,
        identity::short_hash(&agent.id)
    );
    println!("  Tasks: {}", agent.performance.task_count);
    let value_str = agent
        .performance
        .mean_reward
        .map(|s| format!("{:.2}", s))
        .unwrap_or_else(|| "n/a".to_string());
    println!("  Avg reward: {}", value_str);

    // Show inline reward refs from the agent's performance record
    if !agent.performance.rewards.is_empty() {
        println!();
        println!(
            "Reward history ({} entries):",
            agent.performance.rewards.len()
        );
        for eval in &agent.performance.rewards {
            println!(
                "  task:{} reward:{:.2} context:{} at:{}",
                &eval.task_id[..eval.task_id.len().min(12)],
                eval.value,
                identity::short_hash(&eval.context_id),
                eval.timestamp,
            );
        }
    }

    // Show full reward records if any exist
    if !agent_evals.is_empty() {
        println!();
        println!("Full reward records ({}):", agent_evals.len());
        for eval in &agent_evals {
            println!(
                "  {} task:{} reward:{:.2} by:{}",
                identity::short_hash(&eval.id),
                &eval.task_id[..eval.task_id.len().min(12)],
                eval.value,
                eval.evaluator,
            );
            if !eval.dimensions.is_empty() {
                let dims: Vec<String> = eval
                    .dimensions
                    .iter()
                    .map(|(k, v)| format!("{}={:.2}", k, v))
                    .collect();
                println!("    dims: {}", dims.join(", "));
            }
            if !eval.notes.is_empty() {
                let preview: String = eval.notes.chars().take(80).collect();
                if eval.notes.len() > 80 {
                    println!("    notes: {}...", preview);
                } else {
                    println!("    notes: {}", preview);
                }
            }
        }
    }

    if agent.performance.rewards.is_empty() && agent_evals.is_empty() {
        println!();
        println!("No reward history yet.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("identity").join("agents")).unwrap();
        std::fs::create_dir_all(tmp.path().join("identity").join("roles")).unwrap();
        std::fs::create_dir_all(tmp.path().join("identity").join("objectives")).unwrap();
        std::fs::create_dir_all(tmp.path().join("identity").join("rewards")).unwrap();
        tmp
    }

    fn create_role(dir: &Path) -> String {
        let role = identity::build_role("Test Role", "A test role", vec![], "Good output");
        let roles_dir = dir.join("identity").join("roles");
        identity::save_role(&role, &roles_dir).unwrap();
        role.id
    }

    fn create_objective(dir: &Path) -> String {
        let objective = identity::build_objective(
            "Test Objective",
            "A test objective",
            vec!["Slower delivery".to_string()],
            vec!["Skipping tests".to_string()],
        );
        let mots_dir = dir.join("identity").join("objectives");
        identity::save_objective(&objective, &mots_dir).unwrap();
        objective.id
    }

    /// Helper: create an agent with defaults for the new optional fields.
    fn create_agent(dir: &Path, name: &str, role_id: &str, mot_id: &str) -> Result<()> {
        run_create(
            dir,
            name,
            Some(role_id),
            Some(mot_id),
            &[],
            None,
            None,
            None,
            None,
            "claude",
        )
    }

    #[test]
    fn test_create_and_list() {
        let tmp = setup();
        let role_id = create_role(tmp.path());
        let mot_id = create_objective(tmp.path());

        create_agent(tmp.path(), "Test Agent", &role_id, &mot_id).unwrap();

        let agents_dir = tmp.path().join("identity").join("agents");
        let agents = identity::load_all_agents(&agents_dir).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "Test Agent");
        assert_eq!(agents[0].role_id, role_id);
        assert_eq!(agents[0].objective_id, mot_id);
    }

    #[test]
    fn test_create_with_operational_fields() {
        let tmp = setup();
        let role_id = create_role(tmp.path());
        let mot_id = create_objective(tmp.path());

        run_create(
            tmp.path(),
            "Ops Agent",
            Some(&role_id),
            Some(&mot_id),
            &["rust".to_string(), "python".to_string()],
            Some(50.0),
            Some(3.0),
            Some("verified"),
            Some("ops@example.com"),
            "claude",
        )
        .unwrap();

        let agents_dir = tmp.path().join("identity").join("agents");
        let agents = identity::load_all_agents(&agents_dir).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].capabilities, vec!["rust", "python"]);
        assert_eq!(agents[0].rate, Some(50.0));
        assert_eq!(agents[0].capacity, Some(3.0));
        assert_eq!(
            agents[0].trust_level,
            workgraph::graph::TrustLevel::Verified
        );
        assert_eq!(agents[0].contact, Some("ops@example.com".to_string()));
        assert_eq!(agents[0].executor, "claude");
    }

    #[test]
    fn test_create_human_agent_without_role() {
        let tmp = setup();

        run_create(
            tmp.path(),
            "Human Operator",
            None,
            None,
            &["project-management".to_string()],
            None,
            None,
            None,
            Some("@human:matrix.org"),
            "matrix",
        )
        .unwrap();

        let agents_dir = tmp.path().join("identity").join("agents");
        let agents = identity::load_all_agents(&agents_dir).unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].name, "Human Operator");
        assert_eq!(agents[0].executor, "matrix");
        assert_eq!(agents[0].contact, Some("@human:matrix.org".to_string()));
        assert!(agents[0].role_id.is_empty());
        assert!(agents[0].objective_id.is_empty());
    }

    #[test]
    fn test_create_ai_agent_requires_role_and_objective() {
        let tmp = setup();

        // AI agent (executor=claude) without role should fail
        let result = run_create(
            tmp.path(),
            "Bad AI",
            None,
            None,
            &[],
            None,
            None,
            None,
            None,
            "claude",
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("--role is required")
        );
    }

    #[test]
    fn test_create_duplicate_fails() {
        let tmp = setup();
        let role_id = create_role(tmp.path());
        let mot_id = create_objective(tmp.path());

        create_agent(tmp.path(), "Agent 1", &role_id, &mot_id).unwrap();
        let result = create_agent(tmp.path(), "Agent 2", &role_id, &mot_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_create_with_bad_role() {
        let tmp = setup();
        let mot_id = create_objective(tmp.path());
        let result = run_create(
            tmp.path(),
            "Bad Agent",
            Some("nonexistent"),
            Some(&mot_id),
            &[],
            None,
            None,
            None,
            None,
            "claude",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_show_and_rm() {
        let tmp = setup();
        let role_id = create_role(tmp.path());
        let mot_id = create_objective(tmp.path());

        create_agent(tmp.path(), "Show Agent", &role_id, &mot_id).unwrap();

        let agents_dir = tmp.path().join("identity").join("agents");
        let agents = identity::load_all_agents(&agents_dir).unwrap();
        assert_eq!(agents.len(), 1);
        let agent_id = &agents[0].id;
        assert_eq!(agents[0].name, "Show Agent");
        assert_eq!(agents[0].role_id, role_id);
        assert_eq!(agents[0].objective_id, mot_id);

        // Show should work (human-readable + JSON)
        run_show(tmp.path(), agent_id, false).unwrap();
        run_show(tmp.path(), agent_id, true).unwrap();

        // Show by prefix should resolve to the same agent
        let resolved = identity::find_agent_by_prefix(&agents_dir, &agent_id[..8]).unwrap();
        assert_eq!(resolved.id, *agent_id);

        // Remove
        run_rm(tmp.path(), agent_id).unwrap();
        assert_eq!(identity::load_all_agents(&agents_dir).unwrap().len(), 0);
    }

    #[test]
    fn test_rm_not_found() {
        let tmp = setup();
        let result = run_rm(tmp.path(), "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_empty() {
        let tmp = setup();
        // Verify underlying data is empty
        let agents_dir = tmp.path().join("identity").join("agents");
        let agents = identity::load_all_agents(&agents_dir).unwrap();
        assert!(agents.is_empty(), "Expected no agents in fresh setup");
        // Both output modes should succeed
        run_list(tmp.path(), false).unwrap();
        run_list(tmp.path(), true).unwrap();
    }

    #[test]
    fn test_lineage() {
        let tmp = setup();
        let role_id = create_role(tmp.path());
        let mot_id = create_objective(tmp.path());

        create_agent(tmp.path(), "Lineage Agent", &role_id, &mot_id).unwrap();

        let agents_dir = tmp.path().join("identity").join("agents");
        let agents = identity::load_all_agents(&agents_dir).unwrap();
        assert_eq!(agents.len(), 1);
        let agent = &agents[0];
        let agent_id = &agent.id;

        // Verify lineage data is populated
        assert_eq!(agent.lineage.generation, 0);
        assert_eq!(agent.lineage.created_by, "human");
        assert!(agent.lineage.parent_ids.is_empty());

        // Verify role ancestry resolves
        let roles_dir = tmp.path().join("identity").join("roles");
        let role_ancestry = identity::role_ancestry(&agent.role_id, &roles_dir).unwrap_or_default();
        assert!(!role_ancestry.is_empty(), "Role ancestry should resolve");
        assert_eq!(role_ancestry[0].name, "Test Role");

        run_lineage(tmp.path(), agent_id, false).unwrap();
        run_lineage(tmp.path(), agent_id, true).unwrap();
    }

    #[test]
    fn test_performance_empty() {
        let tmp = setup();
        let role_id = create_role(tmp.path());
        let mot_id = create_objective(tmp.path());

        create_agent(tmp.path(), "Perf Agent", &role_id, &mot_id).unwrap();

        let agents_dir = tmp.path().join("identity").join("agents");
        let agents = identity::load_all_agents(&agents_dir).unwrap();
        let agent = &agents[0];
        let agent_id = &agent.id;

        // Verify performance data is initialized correctly
        assert_eq!(agent.performance.task_count, 0);
        assert!(agent.performance.mean_reward.is_none());
        assert!(agent.performance.rewards.is_empty());

        // Verify rewards dir is empty
        let evals_dir = tmp.path().join("identity").join("rewards");
        let all_evals = identity::load_all_rewards(&evals_dir).unwrap_or_default();
        assert!(all_evals.is_empty());

        run_performance(tmp.path(), agent_id, false).unwrap();
        run_performance(tmp.path(), agent_id, true).unwrap();
    }
}
