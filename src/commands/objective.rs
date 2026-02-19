use anyhow::{Context, Result};
use std::path::Path;
use workgraph::identity::{self};

/// Get the identity base directory (creates it if needed).
fn identity_dir(workgraph_dir: &Path) -> Result<std::path::PathBuf> {
    let dir = workgraph_dir.join("identity");
    identity::init(&dir).context("Failed to initialise identity directory")?;
    Ok(dir)
}

/// Get the objectives subdirectory.
fn objectives_dir(workgraph_dir: &Path) -> Result<std::path::PathBuf> {
    Ok(identity_dir(workgraph_dir)?.join("objectives"))
}

/// `wg objective add <name> --accept ... --reject ... [--description ...]`
pub fn run_add(
    workgraph_dir: &Path,
    name: &str,
    accept: &[String],
    reject: &[String],
    description: Option<&str>,
) -> Result<()> {
    let dir = objectives_dir(workgraph_dir)?;

    let objective = identity::build_objective(
        name,
        description.unwrap_or(""),
        accept.to_vec(),
        reject.to_vec(),
    );

    // Check for duplicates (same content = same hash)
    let mot_path = dir.join(format!("{}.yaml", objective.id));
    if mot_path.exists() {
        anyhow::bail!(
            "Objective with identical content already exists ({})",
            identity::short_hash(&objective.id)
        );
    }

    let path = identity::save_objective(&objective, &dir)?;
    println!(
        "Created objective: {} ({})",
        name,
        identity::short_hash(&objective.id)
    );
    println!("  File: {}", path.display());
    Ok(())
}

/// `wg objective list [--json]`
pub fn run_list(workgraph_dir: &Path, json: bool) -> Result<()> {
    let dir = objectives_dir(workgraph_dir)?;
    let objectives = identity::load_all_objectives(&dir)?;

    if json {
        let output: Vec<serde_json::Value> = objectives
            .iter()
            .map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "name": m.name,
                    "description": m.description,
                    "acceptable_tradeoffs": m.acceptable_tradeoffs.len(),
                    "unacceptable_tradeoffs": m.unacceptable_tradeoffs.len(),
                    "mean_reward": m.performance.mean_reward,
                    "task_count": m.performance.task_count,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if objectives.is_empty() {
        println!("No objectives defined. Use 'wg objective add' to create one.");
    } else {
        println!("Objectives:\n");
        for m in &objectives {
            let value_str = m
                .performance
                .mean_reward
                .map(|s| format!("{:.2}", s))
                .unwrap_or_else(|| "n/a".to_string());
            println!(
                "  {}  {:20} accept:{} reject:{} reward:{} tasks:{}",
                identity::short_hash(&m.id),
                m.name,
                m.acceptable_tradeoffs.len(),
                m.unacceptable_tradeoffs.len(),
                value_str,
                m.performance.task_count,
            );
        }
    }

    Ok(())
}

/// `wg objective show <id> [--json]`
pub fn run_show(workgraph_dir: &Path, id: &str, json: bool) -> Result<()> {
    let dir = objectives_dir(workgraph_dir)?;
    let objective = identity::find_objective_by_prefix(&dir, id)
        .with_context(|| format!("Failed to find objective '{}'", id))?;

    if json {
        let yaml_str = serde_yaml::to_string(&objective)?;
        // Convert YAML to JSON for --json output
        let value: serde_json::Value = serde_yaml::from_str(&yaml_str)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!(
            "Objective: {} ({})",
            objective.name,
            identity::short_hash(&objective.id)
        );
        println!("ID: {}", objective.id);
        if !objective.description.is_empty() {
            println!("Description: {}", objective.description);
        }
        println!();

        if !objective.acceptable_tradeoffs.is_empty() {
            println!("Acceptable tradeoffs:");
            for t in &objective.acceptable_tradeoffs {
                println!("  + {}", t);
            }
        }

        if !objective.unacceptable_tradeoffs.is_empty() {
            println!("Unacceptable tradeoffs:");
            for t in &objective.unacceptable_tradeoffs {
                println!("  - {}", t);
            }
        }

        println!();
        println!(
            "Performance: {} tasks, avg reward: {}",
            objective.performance.task_count,
            objective
                .performance
                .mean_reward
                .map(|s| format!("{:.2}", s))
                .unwrap_or_else(|| "n/a".to_string()),
        );
    }

    Ok(())
}

/// `wg objective lineage <id> [--json]`
pub fn run_lineage(workgraph_dir: &Path, id: &str, json: bool) -> Result<()> {
    let dir = objectives_dir(workgraph_dir)?;

    // Resolve prefix to full ID first
    let objective = identity::find_objective_by_prefix(&dir, id)
        .with_context(|| format!("Failed to find objective '{}'", id))?;

    let ancestry = identity::objective_ancestry(&objective.id, &dir)?;

    if ancestry.is_empty() {
        anyhow::bail!("Objective '{}' not found", id);
    }

    if json {
        let json_nodes: Vec<serde_json::Value> = ancestry
            .iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "name": n.name,
                    "generation": n.generation,
                    "created_by": n.created_by,
                    "created_at": n.created_at.to_rfc3339(),
                    "parent_ids": n.parent_ids,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_nodes)?);
        return Ok(());
    }

    let target = &ancestry[0];
    println!(
        "Lineage for objective: {} ({})",
        identity::short_hash(&target.id),
        target.name
    );
    println!();

    for node in &ancestry {
        let indent = "  ".repeat(node.generation as usize);
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

    if ancestry.len() == 1 && ancestry[0].parent_ids.is_empty() {
        println!();
        println!("This objective has no evolutionary history (manually created).");
    }

    Ok(())
}

/// `wg objective edit <id>` - opens in $EDITOR
///
/// After editing, the objective is re-hashed. If the content changed, the file is
/// renamed to the new hash and the old file is removed.
pub fn run_edit(workgraph_dir: &Path, id: &str) -> Result<()> {
    let dir = objectives_dir(workgraph_dir)?;
    let objective = identity::find_objective_by_prefix(&dir, id)
        .with_context(|| format!("Failed to find objective '{}'", id))?;

    let mot_path = dir.join(format!("{}.yaml", objective.id));

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    let status = std::process::Command::new(&editor)
        .arg(&mot_path)
        .status()
        .with_context(|| format!("Failed to launch editor '{}'", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with non-zero status");
    }

    // Validate and re-hash
    let mut edited = identity::load_objective(&mot_path)
        .context("Edited file is not valid objective YAML - changes may be malformed")?;

    let new_id = identity::content_hash_objective(
        &edited.acceptable_tradeoffs,
        &edited.unacceptable_tradeoffs,
        &edited.description,
    );
    if new_id != edited.id {
        // Content changed — rename to new hash
        let old_path = mot_path;
        edited.id = new_id;
        identity::save_objective(&edited, &dir)?;
        std::fs::remove_file(&old_path).ok();
        println!(
            "Objective content changed, new ID: {}",
            identity::short_hash(&edited.id)
        );
    } else {
        // Mutable fields (name, etc.) may have changed; re-save in place
        identity::save_objective(&edited, &dir)?;
        println!("Objective '{}' updated", identity::short_hash(&edited.id));
    }

    Ok(())
}

/// `wg objective rm <id>`
pub fn run_rm(workgraph_dir: &Path, id: &str) -> Result<()> {
    let dir = objectives_dir(workgraph_dir)?;
    let objective = identity::find_objective_by_prefix(&dir, id)
        .with_context(|| format!("Failed to find objective '{}'", id))?;

    let path = dir.join(format!("{}.yaml", objective.id));
    std::fs::remove_file(&path).context("Failed to remove objective file")?;
    println!(
        "Removed objective: {} ({})",
        objective.name,
        identity::short_hash(&objective.id)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        let tmp = TempDir::new().unwrap();
        // Create the workgraph dir structure
        std::fs::create_dir_all(tmp.path().join("identity").join("objectives")).unwrap();
        tmp
    }

    #[test]
    fn test_content_hash_deterministic() {
        let h1 = identity::content_hash_objective(&["Slow".into()], &["Broken".into()], "desc");
        let h2 = identity::content_hash_objective(&["Slow".into()], &["Broken".into()], "desc");
        assert_eq!(h1, h2);
        // Different content produces different hash
        let h3 = identity::content_hash_objective(&["Fast".into()], &["Broken".into()], "desc");
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_add_and_list() {
        let tmp = setup();
        run_add(
            tmp.path(),
            "Quality First",
            &["Slower delivery".to_string()],
            &["Skipping tests".to_string()],
            Some("Prioritise correctness"),
        )
        .unwrap();

        let dir = objectives_dir(tmp.path()).unwrap();
        let all = identity::load_all_objectives(&dir).unwrap();
        assert_eq!(all.len(), 1);
        // ID is now a content hash, not a slug
        assert_eq!(all[0].id.len(), 64); // SHA-256 hex = 64 chars
        assert_eq!(all[0].name, "Quality First");
        assert_eq!(all[0].acceptable_tradeoffs, vec!["Slower delivery"]);
        assert_eq!(all[0].unacceptable_tradeoffs, vec!["Skipping tests"]);
    }

    #[test]
    fn test_add_duplicate_fails() {
        let tmp = setup();
        run_add(tmp.path(), "Quality First", &[], &[], None).unwrap();
        let result = run_add(tmp.path(), "Quality First", &[], &[], None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_show_not_found() {
        let tmp = setup();
        let result = run_show(tmp.path(), "nonexistent", false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found")
                || err.contains("Failed to find")
                || err.contains("No objective matching"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_show_existing_by_prefix() {
        let tmp = setup();
        run_add(
            tmp.path(),
            "Speed Demon",
            &["Lower quality".to_string()],
            &["Data loss".to_string()],
            Some("Ship fast"),
        )
        .unwrap();

        // Look up by full hash
        let dir = objectives_dir(tmp.path()).unwrap();
        let all = identity::load_all_objectives(&dir).unwrap();
        let full_id = &all[0].id;
        let result = run_show(tmp.path(), full_id, false);
        assert!(result.is_ok());

        // Look up by short prefix
        let prefix = &full_id[..8];
        let result = run_show(tmp.path(), prefix, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rm() {
        let tmp = setup();
        run_add(tmp.path(), "Temp Objective", &[], &[], None).unwrap();

        let dir = objectives_dir(tmp.path()).unwrap();
        let all = identity::load_all_objectives(&dir).unwrap();
        assert_eq!(all.len(), 1);
        let full_id = all[0].id.clone();

        run_rm(tmp.path(), &full_id).unwrap();
        assert_eq!(identity::load_all_objectives(&dir).unwrap().len(), 0);
    }

    #[test]
    fn test_rm_not_found() {
        let tmp = setup();
        let result = run_rm(tmp.path(), "nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found")
                || err.contains("Failed to find")
                || err.contains("No objective matching"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_list_empty() {
        let tmp = setup();
        let result = run_list(tmp.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_json() {
        let tmp = setup();
        run_add(
            tmp.path(),
            "Test Mot",
            &["a".to_string()],
            &["b".to_string()],
            None,
        )
        .unwrap();
        let result = run_list(tmp.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_json() {
        let tmp = setup();
        run_add(tmp.path(), "Test Mot", &[], &[], Some("desc")).unwrap();
        let dir = objectives_dir(tmp.path()).unwrap();
        let all = identity::load_all_objectives(&dir).unwrap();
        let result = run_show(tmp.path(), &all[0].id, true);
        assert!(result.is_ok());
    }
}
