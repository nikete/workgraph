//! Submit command - mark work complete, awaiting review
//!
//! For tasks with --verify set, agents must use submit instead of done.
//! This sets the task to PendingReview status.

use anyhow::{Context, Result};
use chrono::Utc;
use std::path::Path;
use workgraph::agency::capture_task_output;
use workgraph::graph::{LogEntry, Status};
use workgraph::parser::{load_graph, save_graph};
use workgraph::query;

use super::graph_path;

pub fn run(dir: &Path, task_id: &str, actor: Option<&str>) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    let mut graph = load_graph(&path).context("Failed to load graph")?;

    let task = graph
        .get_task_mut(task_id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", task_id))?;

    // Only allow submit from InProgress
    if task.status != Status::InProgress {
        anyhow::bail!(
            "Cannot submit task '{}': status is {:?}, expected InProgress",
            task_id,
            task.status
        );
    }

    // Check for unresolved blockers
    let blockers = query::blocked_by(&graph, task_id);
    if !blockers.is_empty() {
        let blocker_list: Vec<String> = blockers
            .iter()
            .map(|t| format!("  - {} ({}): {:?}", t.id, t.title, t.status))
            .collect();
        anyhow::bail!(
            "Cannot submit task '{}': blocked by {} unresolved task(s):\n{}",
            task_id,
            blockers.len(),
            blocker_list.join("\n")
        );
    }

    // Re-acquire mutable reference after immutable borrow
    let task = graph.get_task_mut(task_id).unwrap();

    // Set status to PendingReview
    task.status = Status::PendingReview;

    // Add log entry
    task.log.push(LogEntry {
        timestamp: Utc::now().to_rfc3339(),
        actor: actor.map(String::from),
        message: "Work submitted for review".to_string(),
    });

    save_graph(&graph, &path).context("Failed to save graph")?;
    super::notify_graph_changed(dir);

    println!("Submitted task '{}' for review", task_id);
    if let Some(ref verify) = graph.get_task(task_id).and_then(|t| t.verify.clone()) {
        println!("Verification criteria: {}", verify);
    }

    // Capture task output (git diff, artifacts, log) for evaluation.
    // When auto_evaluate is enabled, the coordinator creates an evaluation task
    // in the graph that becomes ready once this task completes; the captured
    // output feeds that evaluator.
    if let Some(task) = graph.get_task(task_id) {
        match capture_task_output(dir, task) {
            Ok(output_dir) => {
                eprintln!("Output captured to {}", output_dir.display());
            }
            Err(e) => {
                eprintln!("Warning: output capture failed: {}", e);
            }
        }
    }

    Ok(())
}
