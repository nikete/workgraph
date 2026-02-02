//! Integration tests for the service daemon end-to-end flow.
//!
//! Tests:
//! 1. Auto-pickup via GraphChanged notification: start daemon, add task, verify pickup
//! 2. Fallback poll pickup: add task without notification, verify poll picks it up
//! 3. Dead-agent recovery: kill agent, verify daemon detects and re-spawns

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Get the path to the compiled `wg` binary (from target/debug or target/release).
fn wg_binary() -> PathBuf {
    // Use the binary built by `cargo test` in the same target directory
    let mut path = std::env::current_exe()
        .expect("could not get current exe path");
    // current_exe is something like target/debug/deps/integration_service-<hash>
    // Go up to target/debug/
    path.pop(); // remove the binary name
    if path.ends_with("deps") {
        path.pop(); // remove deps/
    }
    path.push("wg");
    assert!(
        path.exists(),
        "wg binary not found at {:?}. Run `cargo build` first.",
        path
    );
    path
}

/// Helper: run `wg` with given args in a specific workgraph directory.
fn wg_cmd(wg_dir: &Path, args: &[&str]) -> std::process::Output {
    let wg = wg_binary();
    Command::new(&wg)
        .arg("--dir")
        .arg(wg_dir)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("Failed to run wg {:?}: {}", args, e))
}

/// Helper: run `wg` and assert success, returning stdout as string.
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

/// Helper: initialize a fresh workgraph in a temp directory,
/// and configure a shell executor with the correct working_dir
/// so that the wrapper script's bare `wg` commands can find `.workgraph`.
fn setup_workgraph(tmp_root: &Path) -> PathBuf {
    let wg_dir = tmp_root.join(".workgraph");
    wg_ok(&wg_dir, &["init"]);

    // Create a shell executor config with working_dir set to the tmp root.
    // This ensures the wrapper script runs with cwd = tmp_root, so bare `wg`
    // commands (which default to .workgraph in cwd) find the right workgraph.
    let executors_dir = wg_dir.join("executors");
    fs::create_dir_all(&executors_dir).unwrap();
    let shell_config = format!(
        r#"[executor]
type = "shell"
command = "bash"
args = ["-c", "{{{{task_context}}}}"]
working_dir = "{}"

[executor.env]
TASK_ID = "{{{{task_id}}}}"
TASK_TITLE = "{{{{task_title}}}}"
"#,
        tmp_root.display()
    );
    fs::write(executors_dir.join("shell.toml"), shell_config).unwrap();

    wg_dir
}

/// Helper: generate a unique socket path for this test's temp directory.
/// Each test gets its own socket to avoid conflicts when running in parallel.
fn socket_path_for(tmp_root: &Path) -> String {
    format!("{}/wg-test.sock", tmp_root.display())
}

/// Helper: add a task with a shell exec command.
fn add_shell_task(wg_dir: &Path, task_id: &str, title: &str, exec_cmd: &str) {
    // wg add doesn't support --exec directly, so we add the task then patch the JSONL
    wg_ok(wg_dir, &["add", title, "--id", task_id]);

    // Patch the graph to add exec field
    let graph_path = wg_dir.join("graph.jsonl");
    let content = fs::read_to_string(&graph_path).unwrap();
    let mut new_lines = Vec::new();
    for line in content.lines() {
        if line.contains(&format!("\"id\":\"{}\"", task_id)) {
            // Parse, add exec, re-serialize
            let mut val: serde_json::Value = serde_json::from_str(line).unwrap();
            val["exec"] = serde_json::Value::String(exec_cmd.to_string());
            new_lines.push(serde_json::to_string(&val).unwrap());
        } else {
            new_lines.push(line.to_string());
        }
    }
    fs::write(&graph_path, new_lines.join("\n") + "\n").unwrap();
}

/// Helper: read task status from graph using `wg show --json`.
/// Returns "unknown" on any error (task not found, parse error, etc.)
fn task_status(wg_dir: &Path, task_id: &str) -> String {
    let output = wg_cmd(wg_dir, &["show", task_id, "--json"]);
    if !output.status.success() {
        return "unknown".to_string();
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    match serde_json::from_str::<serde_json::Value>(&stdout) {
        Ok(val) => val["status"].as_str().unwrap_or("unknown").to_string(),
        Err(_) => "unknown".to_string(),
    }
}

/// Helper: send GraphChanged notification via IPC.
fn notify_graph_changed(wg_dir: &Path) {
    let state_path = wg_dir.join("service").join("state.json");
    if let Ok(content) = fs::read_to_string(&state_path) {
        if let Ok(state) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(socket_path) = state["socket_path"].as_str() {
                if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(socket_path) {
                    let _ = writeln!(stream, r#"{{"cmd":"graph_changed"}}"#);
                    let _ = stream.flush();
                    // Read response
                    let mut reader = BufReader::new(&stream);
                    let mut response = String::new();
                    let _ = reader.read_line(&mut response);
                }
            }
        }
    }
}

/// Helper: stop the service daemon.
fn stop_service(wg_dir: &Path) {
    let _ = wg_cmd(wg_dir, &["service", "stop", "--force"]);
}

/// Helper: wait for a condition with timeout, polling at interval.
fn wait_for<F>(timeout: Duration, poll_ms: u64, mut condition: F) -> bool
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    while start.elapsed() < timeout {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(poll_ms));
    }
    false
}

/// Helper: get the number of coordinator ticks from the coordinator state file.
fn coordinator_ticks(wg_dir: &Path) -> u64 {
    let state_path = wg_dir.join("service").join("coordinator-state.json");
    if let Ok(content) = fs::read_to_string(&state_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            return val["ticks"].as_u64().unwrap_or(0);
        }
    }
    0
}

/// Test 1: End-to-end auto-pickup flow via GraphChanged notification.
///
/// Scenario:
/// 1. Initialize a workgraph
/// 2. Start the service daemon with shell executor, poll_interval=300s (slow),
///    max_agents=2
/// 3. Add a task with a shell exec command (echo done)
/// 4. Trigger GraphChanged notification
/// 5. Verify the daemon picks up the task within a few seconds
///    (much faster than the 300s poll interval, proving GraphChanged path works)
/// 6. Wait for the task to complete
#[test]
fn test_auto_pickup_via_graph_changed() {
    let tmp = tempfile::tempdir().unwrap();
    let wg_dir = setup_workgraph(tmp.path());

    // Start the service with a long poll interval so we can distinguish
    // GraphChanged fast-path from the slow poll.
    let socket = socket_path_for(tmp.path());
    let out = wg_ok(
        &wg_dir,
        &[
            "service",
            "start",
            "--socket",
            &socket,
            "--executor",
            "shell",
            "--max-agents",
            "2",
            "--interval",
            "300", // 5 minutes - we should never wait this long
        ],
    );
    assert!(
        out.contains("Service started") || out.contains("started"),
        "Service did not start: {}",
        out
    );

    // Give daemon a moment to initialize
    std::thread::sleep(Duration::from_millis(500));

    // Add a task with exec command. `wg add` triggers notify_graph_changed
    // automatically, but since we patch the file after, we also trigger manually.
    add_shell_task(&wg_dir, "test-task-1", "Test Task 1", "echo done");

    // Re-notify so the coordinator sees the patched task with exec field.
    notify_graph_changed(&wg_dir);

    // Wait for the task to be picked up (status changes from open to in-progress)
    // This should happen within a few seconds via the GraphChanged path,
    // NOT the 300s poll interval.
    let picked_up = wait_for(Duration::from_secs(10), 200, || {
        let status = task_status(&wg_dir, "test-task-1");
        status == "in-progress" || status == "done" || status == "pending-review"
    });

    assert!(
        picked_up,
        "Task was not picked up within 10s. Status: {}. This should have been instant via GraphChanged.",
        task_status(&wg_dir, "test-task-1")
    );

    // Verify coordinator ran at least one tick
    assert!(
        coordinator_ticks(&wg_dir) >= 1,
        "Coordinator should have ticked at least once"
    );

    // Wait for the shell command to complete (echo is instant, wrapper needs a moment)
    let completed = wait_for(Duration::from_secs(15), 500, || {
        let status = task_status(&wg_dir, "test-task-1");
        status == "done" || status == "pending-review"
    });

    assert!(
        completed,
        "Task should have completed. Status: {}",
        task_status(&wg_dir, "test-task-1")
    );

    // Cleanup
    stop_service(&wg_dir);
}

/// Test 2: Fallback poll pickup.
///
/// Scenario:
/// 1. Start service with a short poll_interval (2s)
/// 2. Write a task directly to the graph file (bypassing wg add, so no
///    GraphChanged notification is sent)
/// 3. Verify the background poll picks up the task within the poll interval
#[test]
fn test_fallback_poll_pickup() {
    let tmp = tempfile::tempdir().unwrap();
    let wg_dir = setup_workgraph(tmp.path());

    // Start service with a short poll interval for this test
    let socket = socket_path_for(tmp.path());
    let out = wg_ok(
        &wg_dir,
        &[
            "service",
            "start",
            "--socket",
            &socket,
            "--executor",
            "shell",
            "--max-agents",
            "2",
            "--interval",
            "2", // 2 second poll interval
        ],
    );
    assert!(
        out.contains("Service started") || out.contains("started"),
        "Service did not start: {}",
        out
    );

    // Wait for initial tick to complete
    std::thread::sleep(Duration::from_secs(1));

    // Write a task directly to graph.jsonl, bypassing `wg add`
    // This means NO GraphChanged notification is sent to the daemon.
    let graph_path = wg_dir.join("graph.jsonl");
    let task_json = serde_json::json!({
        "kind": "task",
        "id": "poll-task",
        "title": "Poll Test Task",
        "description": "Added directly to test poll fallback",
        "status": "open",
        "blocked_by": [],
        "tags": [],
        "skills": [],
        "inputs": [],
        "deliverables": [],
        "artifacts": [],
        "log": [],
        "retry_count": 0,
        "exec": "echo 'poll task done'"
    });
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&graph_path)
        .unwrap();
    writeln!(file, "{}", serde_json::to_string(&task_json).unwrap()).unwrap();

    // Wait for the poll interval to pick it up.
    // With a 2s poll, it should be picked up within ~5s.
    let picked_up = wait_for(Duration::from_secs(10), 300, || {
        let status = task_status(&wg_dir, "poll-task");
        status == "in-progress" || status == "done" || status == "pending-review"
    });

    assert!(
        picked_up,
        "Task was not picked up by poll within 10s. Status: {}",
        task_status(&wg_dir, "poll-task")
    );

    // Verify coordinator ticks progressed (should have ticked multiple times)
    assert!(
        coordinator_ticks(&wg_dir) >= 2,
        "Coordinator should have ticked at least twice (initial + poll). Ticks: {}",
        coordinator_ticks(&wg_dir)
    );

    // Wait for completion
    let completed = wait_for(Duration::from_secs(10), 500, || {
        let status = task_status(&wg_dir, "poll-task");
        status == "done" || status == "pending-review"
    });

    assert!(
        completed,
        "Poll task should have completed. Status: {}",
        task_status(&wg_dir, "poll-task")
    );

    // Cleanup
    stop_service(&wg_dir);
}

/// Test 3: Dead-agent recovery.
///
/// Scenario:
/// 1. Start service with shell executor, short poll interval, short heartbeat timeout
/// 2. Add a long-running task (sleep 300)
/// 3. Wait for daemon to pick it up and spawn an agent
/// 4. Kill the agent process
/// 5. Trigger a coordinator tick (or wait for poll)
/// 6. Verify the daemon detects the dead agent:
///    a. Task status returns to "open"
///    b. Agent is marked as dead in the registry
/// 7. Verify the daemon re-spawns a new agent on the task
#[test]
fn test_dead_agent_recovery() {
    let tmp = tempfile::tempdir().unwrap();
    let wg_dir = setup_workgraph(tmp.path());

    // Use a 1-minute heartbeat timeout. We'll rely on process-exit detection.
    // The daemon's poll_interval of 2s ensures frequent checks.
    let config_content = r#"
[agent]
heartbeat_timeout = 5

[coordinator]
max_agents = 2
poll_interval = 2
executor = "shell"
"#;
    fs::write(wg_dir.join("config.toml"), config_content).unwrap();

    // Start service
    let socket = socket_path_for(tmp.path());
    let out = wg_ok(
        &wg_dir,
        &[
            "service",
            "start",
            "--socket",
            &socket,
            "--executor",
            "shell",
            "--max-agents",
            "2",
            "--interval",
            "2",
        ],
    );
    assert!(
        out.contains("Service started") || out.contains("started"),
        "Service did not start: {}",
        out
    );

    std::thread::sleep(Duration::from_millis(500));

    // Add a long-running task
    add_shell_task(
        &wg_dir,
        "long-task",
        "Long Running Task",
        "sleep 300",
    );
    notify_graph_changed(&wg_dir);

    // Wait for the task to be picked up
    let picked_up = wait_for(Duration::from_secs(10), 200, || {
        task_status(&wg_dir, "long-task") == "in-progress"
    });
    assert!(
        picked_up,
        "Long task was not picked up. Status: {}",
        task_status(&wg_dir, "long-task")
    );

    // Find the agent's PID from the registry
    let registry_path = wg_dir.join("service").join("registry.json");
    let registry_content = fs::read_to_string(&registry_path).unwrap();
    let registry: serde_json::Value = serde_json::from_str(&registry_content).unwrap();

    let agents = registry["agents"].as_object().unwrap();
    let agent_entry = agents
        .values()
        .find(|a| a["task_id"].as_str() == Some("long-task") && a["status"].as_str() != Some("dead"))
        .expect("Alive agent for long-task not found in registry");
    let agent_pid = agent_entry["pid"].as_u64().unwrap() as i32;
    let agent_id = agent_entry["id"].as_str().unwrap().to_string();

    // Kill the agent process (SIGKILL - immediate death).
    unsafe {
        libc::kill(agent_pid, libc::SIGKILL);
    }

    // Give the kernel a moment to process the signal.
    std::thread::sleep(Duration::from_millis(500));

    // Trigger a coordinator tick to detect the dead agent.
    // The coordinator checks is_process_alive(pid) first; for zombies,
    // this may still return true. In that case, heartbeat timeout will
    // catch it on a subsequent tick. Trigger multiple ticks.
    notify_graph_changed(&wg_dir);
    std::thread::sleep(Duration::from_millis(500));
    notify_graph_changed(&wg_dir);

    // Wait for the original agent to be marked as dead.
    // This may happen via process-exit detection or heartbeat timeout.
    // The task should either go back to "open" or be immediately re-claimed
    // by a new agent (status "in-progress" with a different agent).
    let recovered = wait_for(Duration::from_secs(15), 300, || {
        // Check if the original agent was marked dead
        if let Ok(content) = fs::read_to_string(&registry_path) {
            if let Ok(reg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(agent) = reg["agents"].get(&agent_id) {
                    return agent["status"].as_str() == Some("dead");
                }
            }
        }
        false
    });
    assert!(
        recovered,
        "Original agent should have been detected as dead"
    );

    // Verify the original agent is marked as dead in the registry
    let registry_content = fs::read_to_string(&registry_path).unwrap();
    let registry: serde_json::Value = serde_json::from_str(&registry_content).unwrap();
    let original_agent = &registry["agents"][&agent_id];
    assert_eq!(
        original_agent["status"].as_str().unwrap_or(""),
        "dead",
        "Original agent should be marked as dead"
    );

    // Wait for the task to be re-claimed by a new agent.
    // After the dead agent is cleaned up, the task goes to "open",
    // then the coordinator spawns a new agent (back to "in-progress").
    // Trigger a tick to ensure re-spawn.
    notify_graph_changed(&wg_dir);

    let re_spawned = wait_for(Duration::from_secs(10), 300, || {
        // Check if a NEW agent is working on the task
        if let Ok(content) = fs::read_to_string(&registry_path) {
            if let Ok(reg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(agents) = reg["agents"].as_object() {
                    return agents.values().any(|a| {
                        a["task_id"].as_str() == Some("long-task")
                            && a["id"].as_str() != Some(&agent_id)
                            && a["status"].as_str() != Some("dead")
                    });
                }
            }
        }
        false
    });
    assert!(
        re_spawned,
        "A new agent should have been spawned for the task after recovery"
    );

    // Verify the task is in-progress with a new agent
    assert_eq!(
        task_status(&wg_dir, "long-task"),
        "in-progress",
        "Task should be in-progress with new agent"
    );

    // Verify a new agent was spawned (different from the original)
    let registry_content = fs::read_to_string(&registry_path).unwrap();
    let registry: serde_json::Value = serde_json::from_str(&registry_content).unwrap();
    let agents = registry["agents"].as_object().unwrap();

    // Should have at least 2 agents now (original dead + new one)
    assert!(
        agents.len() >= 2,
        "Should have at least 2 agents (original dead + respawned). Got: {}",
        agents.len()
    );

    // Find the new alive agent for long-task
    let new_agent = agents
        .values()
        .find(|a| {
            a["task_id"].as_str() == Some("long-task")
                && a["id"].as_str() != Some(&agent_id)
                && a["status"].as_str() != Some("dead")
        });
    assert!(
        new_agent.is_some(),
        "Should have a new agent working on long-task"
    );

    let new_agent = new_agent.unwrap();
    assert_ne!(
        new_agent["pid"].as_u64().unwrap() as i32,
        agent_pid,
        "New agent should have a different PID"
    );

    // Cleanup: kill the new agent too before stopping service
    let new_pid = new_agent["pid"].as_u64().unwrap() as i32;
    unsafe {
        libc::kill(new_pid, libc::SIGKILL);
    }
    std::thread::sleep(Duration::from_millis(200));

    stop_service(&wg_dir);
}
