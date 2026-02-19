use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use workgraph::identity::{self, Reward, Objective, Role};
use workgraph::parser::load_graph;

/// A (role_id, objective_id) pair used as a key in the synergy matrix.
type Pair = (String, String);

/// Per-entity aggregated stats.
struct EntityStats {
    id: String,
    name: String,
    task_count: u32,
    mean_reward: Option<f64>,
    /// Recent values for trend computation (oldest first).
    recent_values: Vec<f64>,
}

/// Synergy cell: stats for a specific (role, objective) pair.
struct SynergyCell {
    role_id: String,
    objective_id: String,
    count: u32,
    mean_reward: f64,
}

/// Tag breakdown cell: stats for (entity_id, tag).
struct TagCell {
    entity_id: String,
    tag: String,
    count: u32,
    mean_reward: f64,
}

/// Per-model aggregated stats.
struct ModelStats {
    model: String,
    count: u32,
    mean_reward: f64,
    values: Vec<f64>,
}

/// Compute a simple trend indicator from recent values.
/// Returns "up", "down", "flat", or "-" if insufficient data.
fn trend(values: &[f64]) -> &'static str {
    if values.len() < 2 {
        return "-";
    }
    let mid = values.len() / 2;
    let first_half: f64 = values[..mid].iter().sum::<f64>() / mid as f64;
    let second_half: f64 = values[mid..].iter().sum::<f64>() / (values.len() - mid) as f64;
    let diff = second_half - first_half;
    if diff > 0.03 {
        "up"
    } else if diff < -0.03 {
        "down"
    } else {
        "flat"
    }
}

/// Run `wg identity stats [--json] [--min-evals N] [--by-model]`.
pub fn run(dir: &Path, json: bool, min_evals: u32, by_model: bool) -> Result<()> {
    let identity_dir = dir.join("identity");
    let roles_dir = identity_dir.join("roles");
    let objectives_dir = identity_dir.join("objectives");
    let evals_dir = identity_dir.join("rewards");

    let roles = identity::load_all_roles(&roles_dir).context("Failed to load roles")?;
    let objectives =
        identity::load_all_objectives(&objectives_dir).context("Failed to load objectives")?;
    let rewards =
        identity::load_all_rewards(&evals_dir).context("Failed to load rewards")?;

    // Try to load graph for tag-based breakdown (non-fatal if missing)
    let graph_path = super::graph_path(dir);
    let task_tags: HashMap<String, Vec<String>> = if graph_path.exists() {
        match load_graph(&graph_path) {
            Ok(graph) => graph
                .tasks()
                .map(|t| (t.id.clone(), t.tags.clone()))
                .collect(),
            Err(_) => HashMap::new(),
        }
    } else {
        HashMap::new()
    };

    if json {
        output_json(
            &roles,
            &objectives,
            &rewards,
            &task_tags,
            min_evals,
            by_model,
        )
    } else {
        output_text(
            &roles,
            &objectives,
            &rewards,
            &task_tags,
            min_evals,
            by_model,
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Computation helpers
// ---------------------------------------------------------------------------

fn build_role_stats(roles: &[Role]) -> Vec<EntityStats> {
    roles
        .iter()
        .map(|r| {
            let mut values: Vec<f64> = r.performance.rewards.iter().map(|e| e.value).collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            EntityStats {
                id: r.id.clone(),
                name: r.name.clone(),
                task_count: r.performance.task_count,
                mean_reward: r.performance.mean_reward,
                recent_values: r.performance.rewards.iter().map(|e| e.value).collect(),
            }
        })
        .collect()
}

fn build_objective_stats(objectives: &[Objective]) -> Vec<EntityStats> {
    objectives
        .iter()
        .map(|m| EntityStats {
            id: m.id.clone(),
            name: m.name.clone(),
            task_count: m.performance.task_count,
            mean_reward: m.performance.mean_reward,
            recent_values: m.performance.rewards.iter().map(|e| e.value).collect(),
        })
        .collect()
}

fn build_synergy_matrix(rewards: &[Reward]) -> Vec<SynergyCell> {
    let mut map: HashMap<Pair, Vec<f64>> = HashMap::new();
    for eval in rewards {
        map.entry((eval.role_id.clone(), eval.objective_id.clone()))
            .or_default()
            .push(eval.value);
    }
    let mut cells: Vec<SynergyCell> = map
        .into_iter()
        .map(|((role_id, objective_id), values)| {
            let avg = values.iter().sum::<f64>() / values.len() as f64;
            SynergyCell {
                role_id,
                objective_id,
                count: values.len() as u32,
                mean_reward: avg,
            }
        })
        .collect();
    cells.sort_by(|a, b| {
        b.mean_reward
            .partial_cmp(&a.mean_reward)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    cells
}

fn build_tag_breakdown(
    rewards: &[Reward],
    task_tags: &HashMap<String, Vec<String>>,
    by_role: bool,
) -> Vec<TagCell> {
    // Group rewards by (entity_id, tag)
    let mut map: HashMap<(String, String), Vec<f64>> = HashMap::new();
    for eval in rewards {
        let entity_id = if by_role {
            &eval.role_id
        } else {
            &eval.objective_id
        };
        if let Some(tags) = task_tags.get(&eval.task_id) {
            for tag in tags {
                map.entry((entity_id.clone(), tag.clone()))
                    .or_default()
                    .push(eval.value);
            }
        }
    }
    let mut cells: Vec<TagCell> = map
        .into_iter()
        .map(|((entity_id, tag), values)| {
            let avg = values.iter().sum::<f64>() / values.len() as f64;
            TagCell {
                entity_id,
                tag,
                count: values.len() as u32,
                mean_reward: avg,
            }
        })
        .collect();
    cells.sort_by(|a, b| a.entity_id.cmp(&b.entity_id).then(a.tag.cmp(&b.tag)));
    cells
}

fn build_model_stats(rewards: &[Reward]) -> Vec<ModelStats> {
    let mut map: HashMap<String, Vec<f64>> = HashMap::new();
    for eval in rewards {
        let model_key = eval.model.as_deref().unwrap_or("(unknown)").to_string();
        map.entry(model_key).or_default().push(eval.value);
    }
    let mut stats: Vec<ModelStats> = map
        .into_iter()
        .map(|(model, values)| {
            let avg = values.iter().sum::<f64>() / values.len() as f64;
            ModelStats {
                model,
                count: values.len() as u32,
                mean_reward: avg,
                values,
            }
        })
        .collect();
    stats.sort_by(|a, b| {
        b.mean_reward
            .partial_cmp(&a.mean_reward)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    stats
}

fn find_underexplored(
    roles: &[Role],
    objectives: &[Objective],
    rewards: &[Reward],
    min_evals: u32,
) -> Vec<(String, String, u32)> {
    // Count rewards per (role, objective) pair
    let mut counts: HashMap<Pair, u32> = HashMap::new();
    for eval in rewards {
        *counts
            .entry((eval.role_id.clone(), eval.objective_id.clone()))
            .or_insert(0) += 1;
    }

    let mut under: Vec<(String, String, u32)> = Vec::new();
    for role in roles {
        for objective in objectives {
            let count = counts
                .get(&(role.id.clone(), objective.id.clone()))
                .copied()
                .unwrap_or(0);
            if count < min_evals {
                under.push((role.id.clone(), objective.id.clone(), count));
            }
        }
    }
    under.sort_by(|a, b| a.2.cmp(&b.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    under
}

// ---------------------------------------------------------------------------
// Text output
// ---------------------------------------------------------------------------

fn output_text(
    roles: &[Role],
    objectives: &[Objective],
    rewards: &[Reward],
    task_tags: &HashMap<String, Vec<String>>,
    min_evals: u32,
    by_model: bool,
) {
    // 1. Overall stats
    let total_roles = roles.len();
    let total_objectives = objectives.len();
    let total_rewards = rewards.len();
    let overall_avg = if rewards.is_empty() {
        None
    } else {
        Some(rewards.iter().map(|e| e.value).sum::<f64>() / rewards.len() as f64)
    };

    println!("=== Identity Performance Stats ===\n");
    println!("  Roles:        {}", total_roles);
    println!("  Objectives:  {}", total_objectives);
    println!("  Rewards:  {}", total_rewards);
    println!(
        "  Avg reward:    {}",
        overall_avg
            .map(|s| format!("{:.2}", s))
            .unwrap_or_else(|| "-".to_string())
    );

    if rewards.is_empty() {
        println!("\nNo rewards recorded yet. Run 'wg reward <task-id>' to generate data.");
        return;
    }

    // 2. Role leaderboard
    let mut role_stats = build_role_stats(roles);
    role_stats.sort_by(|a, b| {
        b.mean_reward
            .partial_cmp(&a.mean_reward)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("\n--- Role Leaderboard ---\n");
    println!(
        "  {:<20} {:>8} {:>6} {:>6}",
        "Role", "Avg", "Tasks", "Trend"
    );
    println!("  {}", "-".repeat(44));
    for s in &role_stats {
        let avg_str = s
            .mean_reward
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "  {:<20} {:>8} {:>6} {:>6}",
            identity::short_hash(&s.id),
            avg_str,
            s.task_count,
            trend(&s.recent_values),
        );
    }

    // 3. Objective leaderboard
    let mut mot_stats = build_objective_stats(objectives);
    mot_stats.sort_by(|a, b| {
        b.mean_reward
            .partial_cmp(&a.mean_reward)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    println!("\n--- Objective Leaderboard ---\n");
    println!(
        "  {:<20} {:>8} {:>6} {:>6}",
        "Objective", "Avg", "Tasks", "Trend"
    );
    println!("  {}", "-".repeat(44));
    for s in &mot_stats {
        let avg_str = s
            .mean_reward
            .map(|v| format!("{:.2}", v))
            .unwrap_or_else(|| "-".to_string());
        println!(
            "  {:<20} {:>8} {:>6} {:>6}",
            identity::short_hash(&s.id),
            avg_str,
            s.task_count,
            trend(&s.recent_values),
        );
    }

    // 4. Synergy matrix
    let synergy = build_synergy_matrix(rewards);
    if !synergy.is_empty() {
        println!("\n--- Synergy Matrix (Role x Objective) ---\n");
        println!(
            "  {:<20} {:<20} {:>8} {:>6} {:>8}",
            "Role", "Objective", "Avg", "Count", "Rating"
        );
        println!("  {}", "-".repeat(66));
        for cell in &synergy {
            let rating = if cell.mean_reward >= 0.8 {
                "HIGH"
            } else if cell.mean_reward <= 0.4 {
                "LOW"
            } else {
                ""
            };
            println!(
                "  {:<20} {:<20} {:>8.2} {:>6} {:>8}",
                identity::short_hash(&cell.role_id),
                identity::short_hash(&cell.objective_id),
                cell.mean_reward,
                cell.count,
                rating,
            );
        }
    }

    // 5. Tag breakdown (only if we have tags)
    let role_tags = build_tag_breakdown(rewards, task_tags, true);
    if !role_tags.is_empty() {
        println!("\n--- Reward by Role x Tag ---\n");
        println!("  {:<20} {:<20} {:>8} {:>6}", "Role", "Tag", "Avg", "Count");
        println!("  {}", "-".repeat(58));
        for cell in &role_tags {
            println!(
                "  {:<20} {:<20} {:>8.2} {:>6}",
                identity::short_hash(&cell.entity_id),
                cell.tag,
                cell.mean_reward,
                cell.count,
            );
        }
    }

    let mot_tags = build_tag_breakdown(rewards, task_tags, false);
    if !mot_tags.is_empty() {
        println!("\n--- Reward by Objective x Tag ---\n");
        println!(
            "  {:<20} {:<20} {:>8} {:>6}",
            "Objective", "Tag", "Avg", "Count"
        );
        println!("  {}", "-".repeat(58));
        for cell in &mot_tags {
            println!(
                "  {:<20} {:<20} {:>8.2} {:>6}",
                identity::short_hash(&cell.entity_id),
                cell.tag,
                cell.mean_reward,
                cell.count,
            );
        }
    }

    // 6. Under-explored combinations
    let under = find_underexplored(roles, objectives, rewards, min_evals);
    if !under.is_empty() {
        println!(
            "\n--- Under-explored Combinations (< {} evals) ---\n",
            min_evals
        );
        println!("  {:<20} {:<20} {:>6}", "Role", "Objective", "Evals");
        println!("  {}", "-".repeat(50));
        for (role_id, mot_id, count) in &under {
            println!(
                "  {:<20} {:<20} {:>6}",
                identity::short_hash(role_id),
                identity::short_hash(mot_id),
                count
            );
        }
    }

    // 7. Model leaderboard (if --by-model)
    if by_model {
        let model_stats = build_model_stats(rewards);
        println!("\n--- Model Leaderboard ---\n");
        println!(
            "  {:<40} {:>8} {:>6} {:>6}",
            "Model", "Avg", "Evals", "Trend"
        );
        println!("  {}", "-".repeat(64));
        for s in &model_stats {
            println!(
                "  {:<40} {:>8.2} {:>6} {:>6}",
                s.model,
                s.mean_reward,
                s.count,
                trend(&s.values),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// JSON output
// ---------------------------------------------------------------------------

fn output_json(
    roles: &[Role],
    objectives: &[Objective],
    rewards: &[Reward],
    task_tags: &HashMap<String, Vec<String>>,
    min_evals: u32,
    by_model: bool,
) -> Result<()> {
    let total_rewards = rewards.len();
    let overall_avg = if rewards.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::json!(
            rewards.iter().map(|e| e.value).sum::<f64>() / total_rewards as f64
        )
    };

    // Role leaderboard
    let mut role_stats = build_role_stats(roles);
    role_stats.sort_by(|a, b| {
        b.mean_reward
            .partial_cmp(&a.mean_reward)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let role_board: Vec<serde_json::Value> = role_stats
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "mean_reward": s.mean_reward,
                "task_count": s.task_count,
                "trend": trend(&s.recent_values),
            })
        })
        .collect();

    // Objective leaderboard
    let mut mot_stats = build_objective_stats(objectives);
    mot_stats.sort_by(|a, b| {
        b.mean_reward
            .partial_cmp(&a.mean_reward)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mot_board: Vec<serde_json::Value> = mot_stats
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "mean_reward": s.mean_reward,
                "task_count": s.task_count,
                "trend": trend(&s.recent_values),
            })
        })
        .collect();

    // Synergy matrix
    let synergy = build_synergy_matrix(rewards);
    let synergy_json: Vec<serde_json::Value> = synergy
        .iter()
        .map(|c| {
            let rating = if c.mean_reward >= 0.8 {
                "high"
            } else if c.mean_reward <= 0.4 {
                "low"
            } else {
                "medium"
            };
            serde_json::json!({
                "role_id": c.role_id,
                "objective_id": c.objective_id,
                "mean_reward": c.mean_reward,
                "count": c.count,
                "rating": rating,
            })
        })
        .collect();

    // Tag breakdowns
    let role_tags = build_tag_breakdown(rewards, task_tags, true);
    let role_tags_json: Vec<serde_json::Value> = role_tags
        .iter()
        .map(|c| {
            serde_json::json!({
                "role_id": c.entity_id,
                "tag": c.tag,
                "mean_reward": c.mean_reward,
                "count": c.count,
            })
        })
        .collect();

    let mot_tags = build_tag_breakdown(rewards, task_tags, false);
    let mot_tags_json: Vec<serde_json::Value> = mot_tags
        .iter()
        .map(|c| {
            serde_json::json!({
                "objective_id": c.entity_id,
                "tag": c.tag,
                "mean_reward": c.mean_reward,
                "count": c.count,
            })
        })
        .collect();

    // Under-explored
    let under = find_underexplored(roles, objectives, rewards, min_evals);
    let under_json: Vec<serde_json::Value> = under
        .iter()
        .map(|(r, m, c)| {
            serde_json::json!({
                "role_id": r,
                "objective_id": m,
                "eval_count": c,
            })
        })
        .collect();

    let mut output = serde_json::json!({
        "overview": {
            "total_roles": roles.len(),
            "total_objectives": objectives.len(),
            "total_rewards": total_rewards,
            "mean_reward": overall_avg,
        },
        "role_leaderboard": role_board,
        "objective_leaderboard": mot_board,
        "synergy_matrix": synergy_json,
        "tag_breakdown": {
            "by_role": role_tags_json,
            "by_objective": mot_tags_json,
        },
        "underexplored": under_json,
    });

    if by_model {
        let model_stats = build_model_stats(rewards);
        let model_board: Vec<serde_json::Value> = model_stats
            .iter()
            .map(|s| {
                serde_json::json!({
                    "model": s.model,
                    "mean_reward": s.mean_reward,
                    "eval_count": s.count,
                    "trend": trend(&s.values),
                })
            })
            .collect();
        output["model_leaderboard"] = serde_json::json!(model_board);
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_insufficient_data() {
        assert_eq!(trend(&[]), "-");
        assert_eq!(trend(&[0.5]), "-");
    }

    #[test]
    fn test_trend_up() {
        assert_eq!(trend(&[0.3, 0.4, 0.7, 0.8]), "up");
    }

    #[test]
    fn test_trend_down() {
        assert_eq!(trend(&[0.8, 0.7, 0.3, 0.2]), "down");
    }

    #[test]
    fn test_trend_flat() {
        assert_eq!(trend(&[0.5, 0.5, 0.5, 0.5]), "flat");
    }

    #[test]
    fn test_build_synergy_matrix() {
        let evals = vec![
            Reward {
                id: "e1".into(),
                task_id: "t1".into(),
                agent_id: String::new(),
                role_id: "r1".into(),
                objective_id: "m1".into(),
                value: 0.8,
                dimensions: HashMap::new(),
                notes: String::new(),
                evaluator: "test".into(),
                timestamp: "2025-01-01T00:00:00Z".into(),
                model: None, source: "llm".to_string(),
            },
            Reward {
                id: "e2".into(),
                task_id: "t2".into(),
                agent_id: String::new(),
                role_id: "r1".into(),
                objective_id: "m1".into(),
                value: 0.6,
                dimensions: HashMap::new(),
                notes: String::new(),
                evaluator: "test".into(),
                timestamp: "2025-01-02T00:00:00Z".into(),
                model: None, source: "llm".to_string(),
            },
        ];

        let cells = build_synergy_matrix(&evals);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].role_id, "r1");
        assert_eq!(cells[0].objective_id, "m1");
        assert_eq!(cells[0].count, 2);
        assert!((cells[0].mean_reward - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_find_underexplored() {
        use workgraph::identity::{Lineage, RewardHistory};

        let roles = vec![Role {
            id: "r1".into(),
            name: "Role 1".into(),
            description: String::new(),
            skills: vec![],
            desired_outcome: String::new(),
            performance: RewardHistory {
                task_count: 0,
                mean_reward: None,
                rewards: vec![],
            },
            lineage: Lineage::default(),
        }];
        let objectives = vec![Objective {
            id: "m1".into(),
            name: "Mot 1".into(),
            description: String::new(),
            acceptable_tradeoffs: vec![],
            unacceptable_tradeoffs: vec![],
            performance: RewardHistory {
                task_count: 0,
                mean_reward: None,
                rewards: vec![],
            },
            lineage: Lineage::default(),
        }];

        let under = find_underexplored(&roles, &objectives, &[], 3);
        assert_eq!(under.len(), 1);
        assert_eq!(under[0], ("r1".to_string(), "m1".to_string(), 0));
    }

    #[test]
    fn test_build_tag_breakdown() {
        let evals = vec![Reward {
            id: "e1".into(),
            task_id: "t1".into(),
            agent_id: String::new(),
            role_id: "r1".into(),
            objective_id: "m1".into(),
            value: 0.9,
            dimensions: HashMap::new(),
            notes: String::new(),
            evaluator: "test".into(),
            timestamp: "2025-01-01T00:00:00Z".into(),
            model: None, source: "llm".to_string(),
        }];
        let mut tags = HashMap::new();
        tags.insert("t1".to_string(), vec!["cli".to_string()]);

        let cells = build_tag_breakdown(&evals, &tags, true);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].entity_id, "r1");
        assert_eq!(cells[0].tag, "cli");
        assert!((cells[0].mean_reward - 0.9).abs() < f64::EPSILON);
    }
}
