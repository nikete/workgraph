use anyhow::{Context, Result};
use std::path::Path;
use workgraph::identity::{self, Agent, Lineage, RewardHistory};
use workgraph::config::Config;
use workgraph::graph::TrustLevel;

/// `wg identity init` — bootstrap identity with starter roles, objectives, a default
/// agent, and enable auto_assign + auto_reward in config.
pub fn run(workgraph_dir: &Path) -> Result<()> {
    let identity_dir = workgraph_dir.join("identity");

    // 1. Seed starter roles and objectives
    let (roles_created, objectives_created) =
        identity::seed_starters(&identity_dir).context("Failed to seed identity starters")?;

    if roles_created > 0 || objectives_created > 0 {
        println!(
            "Seeded {} roles and {} objectives.",
            roles_created, objectives_created
        );
    }

    // 2. Create a default agent: Programmer + Careful
    let agents_dir = identity_dir.join("agents");
    std::fs::create_dir_all(&agents_dir).context("Failed to create agents directory")?;

    let roles = identity::starter_roles();
    let objectives = identity::starter_objectives();

    let programmer = roles
        .iter()
        .find(|r| r.name == "Programmer")
        .ok_or_else(|| {
            anyhow::anyhow!("Programmer starter role missing from identity::starter_roles()")
        })?;
    let careful = objectives
        .iter()
        .find(|m| m.name == "Careful")
        .ok_or_else(|| {
            anyhow::anyhow!("Careful starter objective missing from identity::starter_objectives()")
        })?;

    let agent_id = identity::content_hash_agent(&programmer.id, &careful.id);
    let agent_path = agents_dir.join(format!("{}.yaml", agent_id));

    let agent_created = if agent_path.exists() {
        println!(
            "Default agent already exists ({}).",
            identity::short_hash(&agent_id)
        );
        false
    } else {
        let agent = Agent {
            id: agent_id.clone(),
            role_id: programmer.id.clone(),
            objective_id: careful.id.clone(),
            name: "Careful Programmer".to_string(),
            performance: RewardHistory {
                task_count: 0,
                mean_reward: None,
                rewards: vec![],
            },
            lineage: Lineage::default(),
            capabilities: vec![],
            rate: None,
            capacity: None,
            trust_level: TrustLevel::default(),
            contact: None,
            executor: "claude".to_string(),
        };

        identity::save_agent(&agent, &agents_dir).context("Failed to save default agent")?;
        println!(
            "Created default agent: Careful Programmer ({}).",
            identity::short_hash(&agent_id)
        );
        true
    };

    // 3. Enable auto_assign and auto_reward in config
    let mut config = Config::load(workgraph_dir)?;
    let mut config_changed = false;

    if !config.identity.auto_assign {
        config.identity.auto_assign = true;
        config_changed = true;
    }
    if !config.identity.auto_reward {
        config.identity.auto_reward = true;
        config_changed = true;
    }

    if config_changed {
        config
            .save(workgraph_dir)
            .context("Failed to save config")?;
        println!("Enabled auto_assign and auto_reward in config.");
    }

    // Summary
    if roles_created == 0 && objectives_created == 0 && !agent_created && !config_changed {
        println!("Identity already initialized.");
    } else {
        println!();
        println!("Identity is ready. The service will now auto-assign agents to tasks.");
        println!("  Next: wg service start");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_init_creates_agent_and_config() {
        let tmp = tempfile::tempdir().unwrap();
        let wg_dir = tmp.path().join(".workgraph");
        std::fs::create_dir_all(&wg_dir).unwrap();

        // Run init
        run(&wg_dir).unwrap();

        // Verify roles were created
        let roles_dir = wg_dir.join("identity").join("roles");
        let role_count = std::fs::read_dir(&roles_dir).unwrap().count();
        assert!(
            role_count >= 4,
            "Expected at least 4 roles, got {}",
            role_count
        );

        // Verify objectives were created
        let objectives_dir = wg_dir.join("identity").join("objectives");
        let objective_count = std::fs::read_dir(&objectives_dir).unwrap().count();
        assert!(
            objective_count >= 4,
            "Expected at least 4 objectives, got {}",
            objective_count
        );

        // Verify agent was created
        let agents_dir = wg_dir.join("identity").join("agents");
        let agent_count = std::fs::read_dir(&agents_dir).unwrap().count();
        assert_eq!(agent_count, 1, "Expected 1 default agent");

        // Verify config was updated
        let config = Config::load(&wg_dir).unwrap();
        assert!(config.identity.auto_assign);
        assert!(config.identity.auto_reward);
    }

    #[test]
    fn test_identity_init_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let wg_dir = tmp.path().join(".workgraph");
        std::fs::create_dir_all(&wg_dir).unwrap();

        // Run init twice
        run(&wg_dir).unwrap();
        run(&wg_dir).unwrap();

        // Should still have exactly 1 agent
        let agents_dir = wg_dir.join("identity").join("agents");
        let agent_count = std::fs::read_dir(&agents_dir).unwrap().count();
        assert_eq!(agent_count, 1);
    }
}
