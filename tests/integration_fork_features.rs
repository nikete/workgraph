//! Integration tests for fork-specific features not present in upstream.
//!
//! Covers:
//! 1. Peer command (add, list, remove, show, status) — alias for identity remote
//! 2. Manual reward injection (--value, --source, --dimensions, --notes)
//! 3. Evolve strategy: objective-tuning
//! 4. Evolve --backend argument validation

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

use workgraph::graph::{Node, Status, Task, WorkGraph};
use workgraph::identity::{self, Agent, Lineage, RewardHistory, SkillRef};
use workgraph::parser::save_graph;

// ===========================================================================
// Helpers (standard pattern — each integration test file redefines these)
// ===========================================================================

fn wg_binary() -> PathBuf {
    let mut path = std::env::current_exe().expect("could not get current exe path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("wg");
    assert!(
        path.exists(),
        "wg binary not found at {:?}. Run `cargo build` first.",
        path
    );
    path
}

fn wg_cmd(wg_dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(wg_binary())
        .arg("--dir")
        .arg(wg_dir)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("Failed to run wg {:?}: {}", args, e))
}

fn wg_ok(wg_dir: &Path, args: &[&str]) -> String {
    let output = wg_cmd(wg_dir, args);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        output.status.success(),
        "wg {:?} failed.\nstdout: {}\nstderr: {}",
        args,
        stdout,
        stderr
    );
    stdout
}

fn wg_json(wg_dir: &Path, args: &[&str]) -> serde_json::Value {
    let mut full_args = vec!["--json"];
    full_args.extend_from_slice(args);
    let output = wg_ok(wg_dir, &full_args);
    serde_json::from_str(&output).unwrap_or_else(|e| {
        panic!("Failed to parse JSON.\nError: {}\nOutput: {}", e, output)
    })
}

fn make_task(id: &str, title: &str, status: Status) -> Task {
    Task {
        id: id.to_string(),
        title: title.to_string(),
        status,
        ..Task::default()
    }
}

fn setup_workgraph(dir: &Path, tasks: Vec<Task>) {
    fs::create_dir_all(dir).unwrap();
    let path = dir.join("graph.jsonl");
    let mut graph = WorkGraph::new();
    for task in tasks {
        graph.add_node(Node::Task(task));
    }
    save_graph(&graph, &path).unwrap();
}

/// Set up identity with a role, objective, and agent. Returns (agent_id, role_id, objective_id).
fn setup_identity(dir: &Path) -> (String, String, String) {
    let identity_dir = dir.join("identity");
    identity::init(&identity_dir).unwrap();

    let role = identity::build_role(
        "Implementer",
        "Writes production-quality Rust code",
        vec![SkillRef::Name("rust".to_string())],
        "Working, tested code",
    );
    let role_id = role.id.clone();
    identity::save_role(&role, &identity_dir.join("roles")).unwrap();

    let objective = identity::build_objective(
        "Quality First",
        "Prioritise correctness over speed",
        vec!["Slower delivery".to_string()],
        vec!["Skipping tests".to_string()],
    );
    let obj_id = objective.id.clone();
    identity::save_objective(&objective, &identity_dir.join("objectives")).unwrap();

    let agent_id = identity::content_hash_agent(&role_id, &obj_id);
    let agent = Agent {
        id: agent_id.clone(),
        role_id: role_id.clone(),
        objective_id: obj_id.clone(),
        name: "test-agent".to_string(),
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
    identity::save_agent(&agent, &identity_dir.join("agents")).unwrap();

    (agent_id, role_id, obj_id)
}

/// Create a done task assigned to the given agent.
fn make_done_task(id: &str, title: &str, agent_id: &str) -> Task {
    let mut task = make_task(id, title, Status::Done);
    task.agent = Some(agent_id.to_string());
    task.started_at = Some("2026-01-01T00:00:00Z".to_string());
    task.completed_at = Some("2026-01-01T01:00:00Z".to_string());
    task.log = vec![workgraph::graph::LogEntry {
        timestamp: "2026-01-01T00:30:00Z".to_string(),
        actor: Some(agent_id.to_string()),
        message: "Completed the task successfully.".to_string(),
    }];
    task
}

// ===========================================================================
// 1. PEER COMMAND TESTS
// ===========================================================================

/// `wg peer add` creates a remote entry, `wg peer list` shows it.
#[test]
fn peer_add_and_list() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    fs::create_dir_all(&wg_dir).unwrap();
    setup_workgraph(&wg_dir, vec![]);

    // Create a target store to point at
    let target = tmp.path().join("remote-store").join("identity");
    identity::init(&target).unwrap();

    wg_ok(
        &wg_dir,
        &[
            "peer",
            "add",
            "upstream",
            target.parent().unwrap().to_str().unwrap(),
            "--description",
            "Our upstream peer",
        ],
    );

    let output = wg_ok(&wg_dir, &["peer", "list"]);
    assert!(
        output.contains("upstream"),
        "peer list should contain 'upstream', got: {}",
        output
    );
}

/// `wg peer remove` deletes a previously added remote.
#[test]
fn peer_add_then_remove() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    fs::create_dir_all(&wg_dir).unwrap();
    setup_workgraph(&wg_dir, vec![]);

    let target = tmp.path().join("remote-store").join("identity");
    identity::init(&target).unwrap();

    wg_ok(
        &wg_dir,
        &[
            "peer",
            "add",
            "upstream",
            target.parent().unwrap().to_str().unwrap(),
        ],
    );

    // Verify it exists
    let output = wg_ok(&wg_dir, &["peer", "list"]);
    assert!(output.contains("upstream"));

    // Remove
    wg_ok(&wg_dir, &["peer", "remove", "upstream"]);

    // Verify gone
    let output = wg_ok(&wg_dir, &["peer", "list"]);
    assert!(
        !output.contains("upstream"),
        "peer list should not contain 'upstream' after remove, got: {}",
        output
    );
}

/// `wg peer remove` on non-existent remote fails gracefully.
#[test]
fn peer_remove_nonexistent_fails() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    fs::create_dir_all(&wg_dir).unwrap();
    setup_workgraph(&wg_dir, vec![]);

    let output = wg_cmd(&wg_dir, &["peer", "remove", "doesnt-exist"]);
    assert!(
        !output.status.success(),
        "removing non-existent peer should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found"),
        "error should mention 'not found', got: {}",
        stderr
    );
}

/// `wg peer status` with no remotes gives clean output.
#[test]
fn peer_status_no_remotes() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    fs::create_dir_all(&wg_dir).unwrap();
    setup_workgraph(&wg_dir, vec![]);

    let output = wg_ok(&wg_dir, &["peer", "status"]);
    // Should not panic; either shows empty list or a "no remotes" message
    assert!(
        output.contains("No remotes") || output.is_empty() || output.contains("[]"),
        "peer status should handle empty remotes gracefully, got: {}",
        output
    );
}

/// `wg peer add` for duplicate name fails.
#[test]
fn peer_add_duplicate_fails() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    fs::create_dir_all(&wg_dir).unwrap();
    setup_workgraph(&wg_dir, vec![]);

    let target = tmp.path().join("remote-store").join("identity");
    identity::init(&target).unwrap();

    wg_ok(
        &wg_dir,
        &[
            "peer",
            "add",
            "upstream",
            target.parent().unwrap().to_str().unwrap(),
        ],
    );

    // Adding same name again should fail
    let output = wg_cmd(
        &wg_dir,
        &[
            "peer",
            "add",
            "upstream",
            target.parent().unwrap().to_str().unwrap(),
        ],
    );
    assert!(
        !output.status.success(),
        "adding duplicate peer should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already exists"),
        "error should mention 'already exists', got: {}",
        stderr
    );
}

// ===========================================================================
// 2. MANUAL REWARD TESTS (--value, --source, --dimensions, --notes)
// ===========================================================================

/// `wg reward <task> --value 0.85` injects a manual reward without calling LLM.
#[test]
fn reward_manual_value_basic() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (agent_id, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![make_done_task("t1", "Test Task", &agent_id)]);

    let output = wg_ok(&wg_dir, &["reward", "t1", "--value", "0.85"]);

    assert!(
        output.contains("Reward Complete (manual)"),
        "should show manual reward completion, got: {}",
        output
    );
    assert!(
        output.contains("0.85"),
        "should show the reward value, got: {}",
        output
    );
    assert!(
        output.contains("manual"),
        "should show source as manual, got: {}",
        output
    );

    // Verify reward file was saved (record_reward saves into the identity tree)
    let identity_dir = wg_dir.join("identity");
    let has_reward_file = walkdir(identity_dir.as_path(), "yaml");
    assert!(
        !has_reward_file.is_empty(),
        "at least one reward yaml file should exist in identity dir"
    );
}

/// Recursively find files with a given extension under a directory.
fn walkdir(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                result.extend(walkdir(&path, ext));
            } else if path.extension().is_some_and(|e| e == ext) {
                result.push(path);
            }
        }
    }
    result
}

/// `wg reward --value --source --dimensions --notes` all work together.
#[test]
fn reward_manual_with_all_options() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (agent_id, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![make_done_task("t1", "Complex Task", &agent_id)]);

    let output = wg_ok(
        &wg_dir,
        &[
            "reward",
            "t1",
            "--value",
            "0.92",
            "--source",
            "outcome:test-pass-rate",
            "--dimensions",
            r#"{"correctness":0.95,"efficiency":0.88}"#,
            "--notes",
            "All tests passed, efficient implementation",
        ],
    );

    assert!(output.contains("0.92"), "should show value, got: {}", output);
    assert!(
        output.contains("outcome:test-pass-rate"),
        "should show custom source, got: {}",
        output
    );
    assert!(
        output.contains("correctness"),
        "should show dimensions, got: {}",
        output
    );
}

/// `wg reward --value` with --json outputs structured JSON.
#[test]
fn reward_manual_json_output() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (agent_id, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![make_done_task("t1", "JSON Task", &agent_id)]);

    let json = wg_json(
        &wg_dir,
        &[
            "reward",
            "t1",
            "--value",
            "0.75",
            "--source",
            "manual",
            "--notes",
            "Test note",
        ],
    );

    assert_eq!(json["task_id"], "t1");
    assert_eq!(json["value"], 0.75);
    assert_eq!(json["source"], "manual");
    assert_eq!(json["evaluator"], "manual");
    assert_eq!(json["notes"], "Test note");
    assert!(
        json["reward_id"].as_str().is_some(),
        "should have a reward_id"
    );
    assert!(json["path"].as_str().is_some(), "should have a path");
}

/// `wg reward --value` with invalid --dimensions JSON fails gracefully.
#[test]
fn reward_manual_invalid_dimensions_fails() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (agent_id, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![make_done_task("t1", "Bad Dims", &agent_id)]);

    let output = wg_cmd(
        &wg_dir,
        &[
            "reward",
            "t1",
            "--value",
            "0.5",
            "--dimensions",
            "not-valid-json",
        ],
    );
    assert!(
        !output.status.success(),
        "invalid dimensions JSON should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to parse --dimensions"),
        "error should mention dimensions parsing, got: {}",
        stderr
    );
}

/// `wg reward --value` on an open (not done) task fails.
#[test]
fn reward_manual_on_open_task_fails() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (_, _, _) = setup_identity(&wg_dir);
    setup_workgraph(
        &wg_dir,
        vec![make_task("t1", "Open Task", Status::Open)],
    );

    let output = wg_cmd(&wg_dir, &["reward", "t1", "--value", "0.5"]);
    assert!(
        !output.status.success(),
        "rewarding an open task should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("must be done or failed"),
        "error should mention status requirement, got: {}",
        stderr
    );
}

/// `wg reward --value` on a task with no agent still saves the reward (with warning).
#[test]
fn reward_manual_no_agent_saves_with_warning() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    fs::create_dir_all(&wg_dir).unwrap();

    // Done task with no agent
    let task = make_task("t1", "Unassigned Task", Status::Done);
    setup_workgraph(&wg_dir, vec![task]);

    let output = wg_cmd(&wg_dir, &["reward", "t1", "--value", "0.6"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "manual reward without agent should succeed.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    // Should warn about no identity
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("no identity assigned")
            || combined.contains("no assigned agent")
            || combined.contains("Warning"),
        "should warn about missing agent, got stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}

/// `wg reward --value` on a failed task works (failed tasks are valid reward targets).
#[test]
fn reward_manual_on_failed_task() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (agent_id, _, _) = setup_identity(&wg_dir);

    let mut task = make_task("t1", "Failed Task", Status::Failed);
    task.agent = Some(agent_id.clone());
    setup_workgraph(&wg_dir, vec![task]);

    let output = wg_ok(
        &wg_dir,
        &[
            "reward",
            "t1",
            "--value",
            "0.1",
            "--notes",
            "Task failed but attempted",
        ],
    );
    assert!(
        output.contains("Reward Complete"),
        "should complete reward on failed task, got: {}",
        output
    );
}

/// `wg reward --dry-run` shows info without creating files.
#[test]
fn reward_dry_run_no_side_effects() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (agent_id, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![make_done_task("t1", "Dry Run Task", &agent_id)]);

    let output = wg_ok(&wg_dir, &["reward", "t1", "--dry-run"]);
    assert!(
        output.contains("Dry Run"),
        "should show dry run header, got: {}",
        output
    );
    assert!(
        output.contains("Evaluator Prompt"),
        "should show the evaluator prompt, got: {}",
        output
    );

    // Verify no reward files created
    let rewards_dir = wg_dir.join("identity").join("rewards");
    if rewards_dir.exists() {
        let files: Vec<_> = fs::read_dir(&rewards_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml"))
            .collect();
        assert!(
            files.is_empty(),
            "dry run should not create reward files"
        );
    }
}

// ===========================================================================
// 3. EVOLVE STRATEGY TESTS
// ===========================================================================

/// `wg evolve --strategy objective-tuning --dry-run` is accepted as a valid strategy.
#[test]
fn evolve_objective_tuning_strategy_accepted() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (_, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![]);

    // dry-run should succeed (strategy is valid, even if there's nothing to evolve)
    let output = wg_cmd(
        &wg_dir,
        &["evolve", "--strategy", "objective-tuning", "--dry-run"],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The command should not fail due to strategy parsing —
    // it may fail for other reasons (e.g. no claude CLI), but not for invalid strategy
    assert!(
        !stderr.contains("Unknown strategy"),
        "objective-tuning should be a valid strategy.\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
}

/// `wg evolve --strategy invalid-strategy` fails with a helpful error.
#[test]
fn evolve_invalid_strategy_fails() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (_, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![]);

    let output = wg_cmd(&wg_dir, &["evolve", "--strategy", "bogus-strategy"]);
    assert!(
        !output.status.success(),
        "invalid strategy should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unknown strategy") || stderr.contains("bogus-strategy"),
        "error should mention the invalid strategy, got: {}",
        stderr
    );
}

/// `wg evolve --backend gepa` validates the GEPA backend is available.
#[test]
fn evolve_backend_gepa_validates() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (_, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![]);

    // If python3 gepa module is not installed, this should fail with a helpful error
    // (not a panic or generic error)
    let output = wg_cmd(&wg_dir, &["evolve", "--backend", "gepa", "--dry-run"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Either succeeds (gepa installed) or fails with a clear message
    if !output.status.success() {
        assert!(
            stderr.contains("gepa") || stderr.contains("python") || stderr.contains("import"),
            "GEPA unavailable error should mention gepa/python.\nstdout: {}\nstderr: {}",
            stdout,
            stderr
        );
    }
    // If it succeeds, that's fine too — the backend was available
}

/// All valid strategy names are accepted by `wg evolve --strategy`.
#[test]
fn evolve_all_valid_strategies_accepted() {
    let strategies = [
        "mutation",
        "crossover",
        "gap-analysis",
        "retirement",
        "objective-tuning",
        "all",
    ];

    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (_, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![]);

    for strategy in &strategies {
        let output = wg_cmd(&wg_dir, &["evolve", "--strategy", strategy, "--dry-run"]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("Unknown strategy"),
            "strategy '{}' should be accepted, got stderr: {}",
            strategy,
            stderr
        );
    }
}

// ===========================================================================
// 4. REWARD SOURCE FIELD TESTS
// ===========================================================================

/// Manual reward defaults source to "manual" when --source is not specified.
#[test]
fn reward_source_defaults_to_manual() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (agent_id, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![make_done_task("t1", "Source Test", &agent_id)]);

    let json = wg_json(&wg_dir, &["reward", "t1", "--value", "0.7"]);
    assert_eq!(
        json["source"], "manual",
        "default source for --value should be 'manual'"
    );
}

/// Custom --source overrides the default.
#[test]
fn reward_custom_source_override() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (agent_id, _, _) = setup_identity(&wg_dir);
    setup_workgraph(&wg_dir, vec![make_done_task("t1", "Custom Source", &agent_id)]);

    let json = wg_json(
        &wg_dir,
        &[
            "reward",
            "t1",
            "--value",
            "0.9",
            "--source",
            "outcome:ci-pass",
        ],
    );
    assert_eq!(
        json["source"], "outcome:ci-pass",
        "should use custom source"
    );
}

// ===========================================================================
// 5. REWARD + IDENTITY INTEGRATION
// ===========================================================================

/// Manual reward updates the agent's performance record (task_count, mean_reward).
#[test]
fn reward_manual_updates_performance() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = tmp.path().join(".workgraph");
    let (agent_id, _, _) = setup_identity(&wg_dir);

    // Create two done tasks
    setup_workgraph(
        &wg_dir,
        vec![
            make_done_task("t1", "Task One", &agent_id),
            make_done_task("t2", "Task Two", &agent_id),
        ],
    );

    // Reward both
    wg_ok(&wg_dir, &["reward", "t1", "--value", "0.8"]);
    wg_ok(&wg_dir, &["reward", "t2", "--value", "0.6"]);

    // Verify the agent's performance was updated by loading it
    let identity_dir = wg_dir.join("identity");
    let agents_dir = identity_dir.join("agents");
    let agent = identity::find_agent_by_prefix(&agents_dir, &agent_id).unwrap();
    assert_eq!(
        agent.performance.task_count, 2,
        "agent should have 2 tasks recorded"
    );
    assert!(
        agent.performance.mean_reward.is_some(),
        "agent should have a mean_reward"
    );
    let mean = agent.performance.mean_reward.unwrap();
    assert!(
        (mean - 0.7).abs() < 0.01,
        "mean_reward should be ~0.7 (avg of 0.8 and 0.6), got {}",
        mean
    );
}
