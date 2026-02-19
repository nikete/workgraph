//! Edge-case tests for the identity system.
//!
//! Covers: nonexistent entity references, deletion of referenced entities,
//! extreme performance values, content hash collision resistance, prefix lookup
//! edge cases, and corrupted YAML handling.

use std::collections::HashMap;
use tempfile::TempDir;

use workgraph::identity::{
    self, Agent, Reward, RewardRef, Lineage, RewardHistory, SkillRef,
};

// ---------------------------------------------------------------------------
// 1. Creating an agent with nonexistent role_id or objective_id
// ---------------------------------------------------------------------------

/// An agent can be created (saved) referencing role/objective IDs that don't
/// exist on disk. The agent is just data — it doesn't validate references at
/// save time.
#[test]
fn test_create_agent_with_nonexistent_role_and_objective() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let agents_dir = identity_dir.join("agents");
    let fake_role_id = "aaaa".repeat(16); // 64 hex chars
    let fake_mot_id = "bbbb".repeat(16);

    let agent_id = identity::content_hash_agent(&fake_role_id, &fake_mot_id);
    let agent = Agent {
        id: agent_id.clone(),
        role_id: fake_role_id.clone(),
        objective_id: fake_mot_id.clone(),
        name: "orphan-agent".to_string(),
        performance: RewardHistory {
            task_count: 0,
            mean_reward: None,
            rewards: vec![],
        },
        lineage: Lineage::default(),
        capabilities: Vec::new(),
        rate: None,
        capacity: None,
        trust_level: Default::default(),
        contact: None,
        executor: "claude".to_string(),
    };

    // Save succeeds even though the role/objective don't exist
    identity::save_agent(&agent, &agents_dir).unwrap();

    // Agent can be loaded back
    let loaded = identity::find_agent_by_prefix(&agents_dir, &agent_id).unwrap();
    assert_eq!(loaded.role_id, fake_role_id);
    assert_eq!(loaded.objective_id, fake_mot_id);
}

// ---------------------------------------------------------------------------
// 2. Recording a reward for a nonexistent agent
// ---------------------------------------------------------------------------

/// record_reward gracefully handles a nonexistent agent_id — it still saves
/// the reward JSON and skips the agent performance update without error.
#[test]
fn test_record_reward_nonexistent_agent() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    // Create a role and objective so those parts succeed
    let roles_dir = identity_dir.join("roles");
    let objectives_dir = identity_dir.join("objectives");

    let role = identity::build_role("R", "desc", vec![], "outcome");
    identity::save_role(&role, &roles_dir).unwrap();

    let mot = identity::build_objective("M", "desc", vec![], vec![]);
    identity::save_objective(&mot, &objectives_dir).unwrap();

    let eval = Reward {
        id: "eval-ghost-1".to_string(),
        task_id: "ghost-task".to_string(),
        agent_id: "nonexistent_agent_id_1234567890abcdef".to_string(),
        role_id: role.id.clone(),
        objective_id: mot.id.clone(),
        value: 0.75,
        dimensions: HashMap::new(),
        notes: "Agent doesn't exist".to_string(),
        evaluator: "test".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        model: None, source: "llm".to_string(),
    };

    // Should succeed — reward saved, role/objective updated, agent skipped
    let eval_path = identity::record_reward(&eval, &identity_dir).unwrap();
    assert!(eval_path.exists());

    // Role and objective should still have their performance updated
    let updated_role = identity::load_role(&roles_dir.join(format!("{}.yaml", role.id))).unwrap();
    assert_eq!(updated_role.performance.task_count, 1);

    let updated_mot =
        identity::load_objective(&objectives_dir.join(format!("{}.yaml", mot.id))).unwrap();
    assert_eq!(updated_mot.performance.task_count, 1);
}

/// record_reward with empty agent_id skips agent update entirely.
#[test]
fn test_record_reward_empty_agent_id() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");
    let objectives_dir = identity_dir.join("objectives");

    let role = identity::build_role("R", "desc", vec![], "outcome");
    identity::save_role(&role, &roles_dir).unwrap();

    let mot = identity::build_objective("M", "desc", vec![], vec![]);
    identity::save_objective(&mot, &objectives_dir).unwrap();

    let eval = Reward {
        id: "eval-no-agent-1".to_string(),
        task_id: "task-1".to_string(),
        agent_id: String::new(),
        role_id: role.id.clone(),
        objective_id: mot.id.clone(),
        value: 0.80,
        dimensions: HashMap::new(),
        notes: "No agent".to_string(),
        evaluator: "test".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        model: None, source: "llm".to_string(),
    };

    let eval_path = identity::record_reward(&eval, &identity_dir).unwrap();
    assert!(eval_path.exists());

    // No agents should exist
    let agents = identity::load_all_agents(&identity_dir.join("agents")).unwrap();
    assert!(agents.is_empty());
}

/// record_reward with nonexistent role_id and objective_id still saves the
/// reward JSON — it just skips the role/objective performance updates.
#[test]
fn test_record_reward_nonexistent_role_and_objective() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let eval = Reward {
        id: "eval-orphan-1".to_string(),
        task_id: "orphan-task".to_string(),
        agent_id: String::new(),
        role_id: "nonexistent_role_id".to_string(),
        objective_id: "nonexistent_objective_id".to_string(),
        value: 0.50,
        dimensions: HashMap::new(),
        notes: "Orphan eval".to_string(),
        evaluator: "test".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        model: None, source: "llm".to_string(),
    };

    // Should succeed — the eval JSON is saved even if role/objective not found
    let eval_path = identity::record_reward(&eval, &identity_dir).unwrap();
    assert!(eval_path.exists());

    let loaded = identity::load_reward(&eval_path).unwrap();
    assert_eq!(loaded.value, 0.50);
}

// ---------------------------------------------------------------------------
// 3. Deleting a role referenced by an existing agent
// ---------------------------------------------------------------------------

/// Deleting a role YAML file that is referenced by an agent succeeds (it's just
/// a file delete). The agent still loads fine — it just has a dangling role_id.
#[test]
fn test_delete_role_referenced_by_agent() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");
    let objectives_dir = identity_dir.join("objectives");
    let agents_dir = identity_dir.join("agents");

    let role = identity::build_role("Doomed Role", "Will be deleted", vec![], "Outcome");
    identity::save_role(&role, &roles_dir).unwrap();

    let mot = identity::build_objective("M", "desc", vec![], vec![]);
    identity::save_objective(&mot, &objectives_dir).unwrap();

    let agent_id = identity::content_hash_agent(&role.id, &mot.id);
    let agent = Agent {
        id: agent_id.clone(),
        role_id: role.id.clone(),
        objective_id: mot.id.clone(),
        name: "agent-with-doomed-role".to_string(),
        performance: RewardHistory {
            task_count: 0,
            mean_reward: None,
            rewards: vec![],
        },
        lineage: Lineage::default(),
        capabilities: Vec::new(),
        rate: None,
        capacity: None,
        trust_level: Default::default(),
        contact: None,
        executor: "claude".to_string(),
    };
    identity::save_agent(&agent, &agents_dir).unwrap();

    // Delete the role file
    let role_path = roles_dir.join(format!("{}.yaml", role.id));
    std::fs::remove_file(&role_path).unwrap();

    // Agent is still loadable with dangling role_id
    let loaded_agent = identity::find_agent_by_prefix(&agents_dir, &agent_id).unwrap();
    assert_eq!(loaded_agent.role_id, role.id);

    // Looking up the role fails with NotFound
    let result = identity::find_role_by_prefix(&roles_dir, &role.id);
    assert!(result.is_err());

    // Recording a reward still succeeds (role update is skipped gracefully)
    let eval = Reward {
        id: "eval-after-delete-1".to_string(),
        task_id: "task-1".to_string(),
        agent_id: agent_id.clone(),
        role_id: role.id.clone(),
        objective_id: mot.id.clone(),
        value: 0.85,
        dimensions: HashMap::new(),
        notes: "Role was deleted".to_string(),
        evaluator: "test".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        model: None, source: "llm".to_string(),
    };
    let eval_path = identity::record_reward(&eval, &identity_dir).unwrap();
    assert!(eval_path.exists());

    // Agent performance was still updated (agent exists)
    let updated_agent = identity::find_agent_by_prefix(&agents_dir, &agent_id).unwrap();
    assert_eq!(updated_agent.performance.task_count, 1);

    // Objective performance was updated (objective exists)
    let updated_mot =
        identity::load_objective(&objectives_dir.join(format!("{}.yaml", mot.id))).unwrap();
    assert_eq!(updated_mot.performance.task_count, 1);
}

// ---------------------------------------------------------------------------
// 4. Deleting an objective referenced by an existing agent
// ---------------------------------------------------------------------------

/// Deleting an objective YAML that is referenced by an agent. The agent still
/// loads, and rewards recording gracefully skips the missing objective.
#[test]
fn test_delete_objective_referenced_by_agent() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");
    let objectives_dir = identity_dir.join("objectives");
    let agents_dir = identity_dir.join("agents");

    let role = identity::build_role("R", "desc", vec![], "Outcome");
    identity::save_role(&role, &roles_dir).unwrap();

    let mot = identity::build_objective("Doomed Mot", "Will be deleted", vec![], vec![]);
    identity::save_objective(&mot, &objectives_dir).unwrap();

    let agent_id = identity::content_hash_agent(&role.id, &mot.id);
    let agent = Agent {
        id: agent_id.clone(),
        role_id: role.id.clone(),
        objective_id: mot.id.clone(),
        name: "agent-with-doomed-mot".to_string(),
        performance: RewardHistory {
            task_count: 0,
            mean_reward: None,
            rewards: vec![],
        },
        lineage: Lineage::default(),
        capabilities: Vec::new(),
        rate: None,
        capacity: None,
        trust_level: Default::default(),
        contact: None,
        executor: "claude".to_string(),
    };
    identity::save_agent(&agent, &agents_dir).unwrap();

    // Delete the objective file
    let mot_path = objectives_dir.join(format!("{}.yaml", mot.id));
    std::fs::remove_file(&mot_path).unwrap();

    // Agent is still loadable
    let loaded_agent = identity::find_agent_by_prefix(&agents_dir, &agent_id).unwrap();
    assert_eq!(loaded_agent.objective_id, mot.id);

    // Objective lookup fails
    let result = identity::find_objective_by_prefix(&objectives_dir, &mot.id);
    assert!(result.is_err());

    // Reward recording still succeeds (objective update skipped)
    let eval = Reward {
        id: "eval-mot-deleted-1".to_string(),
        task_id: "task-1".to_string(),
        agent_id: agent_id.clone(),
        role_id: role.id.clone(),
        objective_id: mot.id.clone(),
        value: 0.90,
        dimensions: HashMap::new(),
        notes: "Objective was deleted".to_string(),
        evaluator: "test".to_string(),
        timestamp: "2025-01-01T00:00:00Z".to_string(),
        model: None, source: "llm".to_string(),
    };
    let eval_path = identity::record_reward(&eval, &identity_dir).unwrap();
    assert!(eval_path.exists());

    // Agent got updated
    let updated_agent = identity::find_agent_by_prefix(&agents_dir, &agent_id).unwrap();
    assert_eq!(updated_agent.performance.task_count, 1);

    // Role got updated (still exists)
    let updated_role = identity::load_role(&roles_dir.join(format!("{}.yaml", role.id))).unwrap();
    assert_eq!(updated_role.performance.task_count, 1);
}

// ---------------------------------------------------------------------------
// 5. Performance record updates with extreme values
// ---------------------------------------------------------------------------

/// Value of exactly 0.0 is valid and correctly computed.
#[test]
fn test_performance_value_zero() {
    let mut record = RewardHistory {
        task_count: 0,
        mean_reward: None,
        rewards: vec![],
    };

    identity::update_performance(
        &mut record,
        RewardRef {
            value: 0.0,
            task_id: "t1".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            context_id: "ctx".to_string(),
        },
    );

    assert_eq!(record.task_count, 1);
    assert!((record.mean_reward.unwrap() - 0.0).abs() < 1e-10);
}

/// Value of exactly 1.0 is valid and correctly computed.
#[test]
fn test_performance_value_one() {
    let mut record = RewardHistory {
        task_count: 0,
        mean_reward: None,
        rewards: vec![],
    };

    identity::update_performance(
        &mut record,
        RewardRef {
            value: 1.0,
            task_id: "t1".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            context_id: "ctx".to_string(),
        },
    );

    assert_eq!(record.task_count, 1);
    assert!((record.mean_reward.unwrap() - 1.0).abs() < 1e-10);
}

/// Negative values are not rejected by the data model (no validation boundary).
/// The system stores them as-is and computes averages correctly.
#[test]
fn test_performance_value_negative() {
    let mut record = RewardHistory {
        task_count: 0,
        mean_reward: None,
        rewards: vec![],
    };

    identity::update_performance(
        &mut record,
        RewardRef {
            value: -0.5,
            task_id: "t1".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            context_id: "ctx".to_string(),
        },
    );

    assert_eq!(record.task_count, 1);
    assert!((record.mean_reward.unwrap() - (-0.5)).abs() < 1e-10);
}

/// Mixed extreme values average correctly: (0.0 + 1.0) / 2 = 0.5
#[test]
fn test_performance_mixed_extreme_values() {
    let mut record = RewardHistory {
        task_count: 0,
        mean_reward: None,
        rewards: vec![],
    };

    identity::update_performance(
        &mut record,
        RewardRef {
            value: 0.0,
            task_id: "t1".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            context_id: "ctx".to_string(),
        },
    );
    identity::update_performance(
        &mut record,
        RewardRef {
            value: 1.0,
            task_id: "t2".to_string(),
            timestamp: "2025-01-01T00:00:01Z".to_string(),
            context_id: "ctx".to_string(),
        },
    );

    assert_eq!(record.task_count, 2);
    assert!((record.mean_reward.unwrap() - 0.5).abs() < 1e-10);
}

/// recalculate_mean_reward returns None for an empty list.
#[test]
fn test_recalculate_mean_reward_empty() {
    assert!(identity::recalculate_mean_reward(&[]).is_none());
}

/// Extreme values round-trip through YAML serialization on a role.
#[test]
fn test_extreme_values_yaml_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();
    let roles_dir = identity_dir.join("roles");

    let mut role = identity::build_role("R", "desc", vec![], "outcome");
    identity::update_performance(
        &mut role.performance,
        RewardRef {
            value: 0.0,
            task_id: "zero".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            context_id: "ctx".to_string(),
        },
    );
    identity::update_performance(
        &mut role.performance,
        RewardRef {
            value: 1.0,
            task_id: "one".to_string(),
            timestamp: "2025-01-01T00:00:01Z".to_string(),
            context_id: "ctx".to_string(),
        },
    );

    identity::save_role(&role, &roles_dir).unwrap();
    let loaded = identity::load_role(&roles_dir.join(format!("{}.yaml", role.id))).unwrap();
    assert_eq!(loaded.performance.task_count, 2);
    assert!((loaded.performance.mean_reward.unwrap() - 0.5).abs() < 1e-10);
    assert!((loaded.performance.rewards[0].value - 0.0).abs() < 1e-10);
    assert!((loaded.performance.rewards[1].value - 1.0).abs() < 1e-10);
}

// ---------------------------------------------------------------------------
// 6. Content hash collision resistance
// ---------------------------------------------------------------------------

/// Slightly different descriptions produce different role hashes.
#[test]
fn test_content_hash_role_different_descriptions() {
    let h1 = identity::content_hash_role(&[], "outcome", "Description A");
    let h2 = identity::content_hash_role(&[], "outcome", "Description B");
    assert_ne!(
        h1, h2,
        "Different descriptions must produce different hashes"
    );
}

/// Slightly different desired_outcomes produce different role hashes.
#[test]
fn test_content_hash_role_different_outcomes() {
    let h1 = identity::content_hash_role(&[], "outcome A", "desc");
    let h2 = identity::content_hash_role(&[], "outcome B", "desc");
    assert_ne!(h1, h2, "Different outcomes must produce different hashes");
}

/// Slightly different skills produce different role hashes.
#[test]
fn test_content_hash_role_different_skills() {
    let h1 = identity::content_hash_role(&[SkillRef::Name("rust".to_string())], "outcome", "desc");
    let h2 = identity::content_hash_role(&[SkillRef::Name("python".to_string())], "outcome", "desc");
    assert_ne!(h1, h2, "Different skills must produce different hashes");
}

/// Skill order matters for content hashing (different order = different hash).
#[test]
fn test_content_hash_role_skill_order_matters() {
    let h1 = identity::content_hash_role(
        &[
            SkillRef::Name("a".to_string()),
            SkillRef::Name("b".to_string()),
        ],
        "outcome",
        "desc",
    );
    let h2 = identity::content_hash_role(
        &[
            SkillRef::Name("b".to_string()),
            SkillRef::Name("a".to_string()),
        ],
        "outcome",
        "desc",
    );
    assert_ne!(
        h1, h2,
        "Different skill order must produce different hashes"
    );
}

/// Different objective descriptions produce different hashes.
#[test]
fn test_content_hash_objective_different_descriptions() {
    let h1 = identity::content_hash_objective(&[], &[], "Description A");
    let h2 = identity::content_hash_objective(&[], &[], "Description B");
    assert_ne!(
        h1, h2,
        "Different descriptions must produce different hashes"
    );
}

/// Different tradeoffs produce different hashes.
#[test]
fn test_content_hash_objective_different_tradeoffs() {
    let h1 = identity::content_hash_objective(&["speed".to_string()], &[], "desc");
    let h2 = identity::content_hash_objective(&["quality".to_string()], &[], "desc");
    assert_ne!(h1, h2);
}

/// Swapping acceptable and unacceptable tradeoffs produces different hashes.
#[test]
fn test_content_hash_objective_swapped_tradeoff_categories() {
    let h1 = identity::content_hash_objective(&["X".to_string()], &["Y".to_string()], "desc");
    let h2 = identity::content_hash_objective(&["Y".to_string()], &["X".to_string()], "desc");
    assert_ne!(
        h1, h2,
        "Swapping tradeoff categories must produce different hashes"
    );
}

/// Different agent pairings produce different hashes.
#[test]
fn test_content_hash_agent_different_pairings() {
    let h1 = identity::content_hash_agent("role_a", "mot_a");
    let h2 = identity::content_hash_agent("role_a", "mot_b");
    let h3 = identity::content_hash_agent("role_b", "mot_a");
    assert_ne!(h1, h2);
    assert_ne!(h1, h3);
    assert_ne!(h2, h3);
}

/// Swapping role_id and objective_id produces a different agent hash.
#[test]
fn test_content_hash_agent_swapped_ids() {
    let h1 = identity::content_hash_agent("aaa", "bbb");
    let h2 = identity::content_hash_agent("bbb", "aaa");
    assert_ne!(
        h1, h2,
        "Swapping role/objective IDs must produce different hashes"
    );
}

/// Content hashing is deterministic — same inputs always yield same hash.
#[test]
fn test_content_hash_determinism() {
    let h1 = identity::content_hash_role(&[SkillRef::Name("x".to_string())], "out", "desc");
    let h2 = identity::content_hash_role(&[SkillRef::Name("x".to_string())], "out", "desc");
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64, "SHA-256 hash should be 64 hex chars");
}

/// Role name does NOT affect the content hash (name is mutable metadata).
#[test]
fn test_content_hash_role_name_independent() {
    let r1 = identity::build_role("Name A", "desc", vec![], "outcome");
    let r2 = identity::build_role("Name B", "desc", vec![], "outcome");
    assert_eq!(r1.id, r2.id, "Name should not affect content hash");
}

/// Objective name does NOT affect the content hash.
#[test]
fn test_content_hash_objective_name_independent() {
    let m1 = identity::build_objective("Name A", "desc", vec![], vec![]);
    let m2 = identity::build_objective("Name B", "desc", vec![], vec![]);
    assert_eq!(m1.id, m2.id, "Name should not affect content hash");
}

// ---------------------------------------------------------------------------
// 7. Prefix lookup edge cases
// ---------------------------------------------------------------------------

/// Prefix lookup with zero matches returns NotFound.
#[test]
fn test_prefix_lookup_zero_matches() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");
    let objectives_dir = identity_dir.join("objectives");
    let agents_dir = identity_dir.join("agents");

    // Save one entity of each type so the directories aren't empty
    let role = identity::build_role("R", "desc", vec![], "outcome");
    identity::save_role(&role, &roles_dir).unwrap();

    let mot = identity::build_objective("M", "desc", vec![], vec![]);
    identity::save_objective(&mot, &objectives_dir).unwrap();

    let agent_id = identity::content_hash_agent(&role.id, &mot.id);
    let agent = Agent {
        id: agent_id,
        role_id: role.id.clone(),
        objective_id: mot.id.clone(),
        name: "a".to_string(),
        performance: RewardHistory {
            task_count: 0,
            mean_reward: None,
            rewards: vec![],
        },
        lineage: Lineage::default(),
        capabilities: Vec::new(),
        rate: None,
        capacity: None,
        trust_level: Default::default(),
        contact: None,
        executor: "claude".to_string(),
    };
    identity::save_agent(&agent, &agents_dir).unwrap();

    // Prefixes that don't match anything
    let r = identity::find_role_by_prefix(&roles_dir, "zzzzzzz");
    assert!(r.is_err());
    assert!(r.unwrap_err().to_string().contains("No role matching"));

    let m = identity::find_objective_by_prefix(&objectives_dir, "zzzzzzz");
    assert!(m.is_err());
    assert!(
        m.unwrap_err()
            .to_string()
            .contains("No objective matching")
    );

    let a = identity::find_agent_by_prefix(&agents_dir, "zzzzzzz");
    assert!(a.is_err());
    assert!(a.unwrap_err().to_string().contains("No agent matching"));
}

/// Prefix lookup with exactly one match succeeds.
#[test]
fn test_prefix_lookup_exact_one_match() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");
    let role = identity::build_role("Only Role", "desc", vec![], "outcome");
    identity::save_role(&role, &roles_dir).unwrap();

    // Full ID match
    let found = identity::find_role_by_prefix(&roles_dir, &role.id).unwrap();
    assert_eq!(found.id, role.id);

    // Short prefix match (first 4 chars)
    let found = identity::find_role_by_prefix(&roles_dir, &role.id[..4]).unwrap();
    assert_eq!(found.id, role.id);
}

/// Prefix lookup with ambiguous prefix (multiple matches) returns Ambiguous error.
#[test]
fn test_prefix_lookup_ambiguous() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();
    let roles_dir = identity_dir.join("roles");

    // Create two roles that start with the same prefix by using slug-based IDs
    let yaml_a = "id: abc123\nname: A\ndescription: d\nskills: []\ndesired_outcome: o\nperformance:\n  task_count: 0\n  mean_reward: null\n";
    let yaml_b = "id: abc456\nname: B\ndescription: d\nskills: []\ndesired_outcome: o\nperformance:\n  task_count: 0\n  mean_reward: null\n";

    std::fs::write(roles_dir.join("abc123.yaml"), yaml_a).unwrap();
    std::fs::write(roles_dir.join("abc456.yaml"), yaml_b).unwrap();

    // "abc" matches both
    let result = identity::find_role_by_prefix(&roles_dir, "abc");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("matches 2 roles"), "Got: {}", err_msg);
    assert!(err_msg.contains("abc123"));
    assert!(err_msg.contains("abc456"));

    // "abc1" matches only one
    let found = identity::find_role_by_prefix(&roles_dir, "abc1").unwrap();
    assert_eq!(found.id, "abc123");
}

/// Prefix lookup on empty directory returns NotFound.
#[test]
fn test_prefix_lookup_empty_directory() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let result = identity::find_role_by_prefix(&identity_dir.join("roles"), "anything");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No role matching"));
}

/// Prefix lookup on nonexistent directory returns NotFound (load_all returns empty vec).
#[test]
fn test_prefix_lookup_nonexistent_directory() {
    let tmp = TempDir::new().unwrap();
    let nowhere = tmp.path().join("does-not-exist");

    let result = identity::find_role_by_prefix(&nowhere, "anything");
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// 8. Loading corrupted YAML files gracefully
// ---------------------------------------------------------------------------

/// A corrupted role YAML file causes load_role to return an error, not panic.
#[test]
fn test_load_corrupted_role_yaml() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");
    std::fs::write(roles_dir.join("corrupt.yaml"), "{{{{not valid yaml!!!!").unwrap();

    let result = identity::load_role(&roles_dir.join("corrupt.yaml"));
    assert!(result.is_err());
}

/// A corrupted objective YAML file returns an error.
#[test]
fn test_load_corrupted_objective_yaml() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let objectives_dir = identity_dir.join("objectives");
    std::fs::write(objectives_dir.join("corrupt.yaml"), "not: [valid: {yaml").unwrap();

    let result = identity::load_objective(&objectives_dir.join("corrupt.yaml"));
    assert!(result.is_err());
}

/// A corrupted agent YAML file returns an error.
#[test]
fn test_load_corrupted_agent_yaml() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let agents_dir = identity_dir.join("agents");
    std::fs::write(agents_dir.join("corrupt.yaml"), ":::broken:::").unwrap();

    let result = identity::load_agent(&agents_dir.join("corrupt.yaml"));
    assert!(result.is_err());
}

/// A corrupted reward JSON file returns an error.
#[test]
fn test_load_corrupted_reward_json() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let evals_dir = identity_dir.join("rewards");
    std::fs::write(evals_dir.join("corrupt.json"), "{broken json}").unwrap();

    let result = identity::load_reward(&evals_dir.join("corrupt.json"));
    assert!(result.is_err());
}

/// load_all_roles fails (returns Err) if one YAML file in the dir is corrupted.
#[test]
fn test_load_all_roles_with_one_corrupted() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");

    // Save a valid role
    let role = identity::build_role("Good Role", "desc", vec![], "outcome");
    identity::save_role(&role, &roles_dir).unwrap();

    // Write a corrupted role
    std::fs::write(roles_dir.join("bad.yaml"), "not valid yaml {{{{").unwrap();

    // load_all_roles should return an error because it can't deserialize the corrupted file
    let result = identity::load_all_roles(&roles_dir);
    assert!(
        result.is_err(),
        "load_all_roles should fail with corrupted file"
    );
}

/// An empty YAML file produces a deserialization error (not a panic).
#[test]
fn test_load_empty_yaml_file() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");
    std::fs::write(roles_dir.join("empty.yaml"), "").unwrap();

    let result = identity::load_role(&roles_dir.join("empty.yaml"));
    assert!(result.is_err());
}

/// A YAML file with valid YAML but wrong schema returns an error.
#[test]
fn test_load_wrong_schema_yaml() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");
    // Valid YAML but missing required Role fields
    std::fs::write(roles_dir.join("wrong-schema.yaml"), "foo: bar\nbaz: 42\n").unwrap();

    let result = identity::load_role(&roles_dir.join("wrong-schema.yaml"));
    assert!(result.is_err());
}

/// A YAML file with partial role fields (missing required fields) returns an error.
#[test]
fn test_load_partial_role_yaml() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();

    let roles_dir = identity_dir.join("roles");
    // Has id and name but missing performance, desired_outcome, etc.
    let partial = "id: partial\nname: Partial Role\n";
    std::fs::write(roles_dir.join("partial.yaml"), partial).unwrap();

    let result = identity::load_role(&roles_dir.join("partial.yaml"));
    assert!(result.is_err());
}

/// Loading a nonexistent file returns an IO error.
#[test]
fn test_load_nonexistent_file() {
    let tmp = TempDir::new().unwrap();
    let result = identity::load_role(&tmp.path().join("nonexistent.yaml"));
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Additional edge cases
// ---------------------------------------------------------------------------

/// Content hash with empty fields is stable and produces valid 64-char hex.
#[test]
fn test_content_hash_empty_fields() {
    let h = identity::content_hash_role(&[], "", "");
    assert_eq!(h.len(), 64);
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()));

    let h2 = identity::content_hash_objective(&[], &[], "");
    assert_eq!(h2.len(), 64);
    assert!(h2.chars().all(|c| c.is_ascii_hexdigit()));

    let h3 = identity::content_hash_agent("", "");
    assert_eq!(h3.len(), 64);
    assert!(h3.chars().all(|c| c.is_ascii_hexdigit()));

    // All different from each other
    assert_ne!(h, h2);
    assert_ne!(h, h3);
    assert_ne!(h2, h3);
}

/// short_hash returns first 8 chars of a hash.
#[test]
fn test_short_hash() {
    let full = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    assert_eq!(identity::short_hash(full), "abcdef01");
}

/// short_hash on a string shorter than SHORT_HASH_LEN returns the whole string.
#[test]
fn test_short_hash_short_input() {
    assert_eq!(identity::short_hash("abc"), "abc");
}

/// Unicode in descriptions does not break content hashing.
#[test]
fn test_content_hash_unicode() {
    let h1 = identity::content_hash_role(&[], "outcome", "Descripción con acentos");
    let h2 = identity::content_hash_role(&[], "outcome", "Description con acentos");
    assert_ne!(h1, h2);
    assert_eq!(h1.len(), 64);
}

/// Special YAML characters in descriptions are handled correctly.
#[test]
fn test_content_hash_yaml_special_chars() {
    let h1 = identity::content_hash_role(&[], "outcome", "description: with colon");
    let h2 = identity::content_hash_role(&[], "outcome", "description with colon");
    assert_ne!(h1, h2);
    assert_eq!(h1.len(), 64);
    assert_eq!(h2.len(), 64);
}

/// init is idempotent — calling it twice doesn't error.
#[test]
fn test_init_idempotent() {
    let tmp = TempDir::new().unwrap();
    let identity_dir = tmp.path().join("identity");
    identity::init(&identity_dir).unwrap();
    identity::init(&identity_dir).unwrap(); // second call should not fail
    assert!(identity_dir.join("roles").is_dir());
    assert!(identity_dir.join("objectives").is_dir());
    assert!(identity_dir.join("rewards").is_dir());
    assert!(identity_dir.join("agents").is_dir());
}

/// load_all_* on nonexistent directories returns empty vec (not error).
#[test]
fn test_load_all_nonexistent_dirs() {
    let tmp = TempDir::new().unwrap();
    let nowhere = tmp.path().join("nowhere");

    assert!(identity::load_all_roles(&nowhere).unwrap().is_empty());
    assert!(identity::load_all_objectives(&nowhere).unwrap().is_empty());
    assert!(identity::load_all_rewards(&nowhere).unwrap().is_empty());
    assert!(identity::load_all_agents(&nowhere).unwrap().is_empty());
}
