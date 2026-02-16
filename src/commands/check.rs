use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use workgraph::check::{LoopEdgeIssueKind, check_all};

#[derive(Serialize)]
struct CheckJsonOutput {
    ok: bool,
    cycles: Vec<Vec<String>>,
    orphan_refs: Vec<workgraph::check::OrphanRef>,
    loop_edge_issues: Vec<workgraph::check::LoopEdgeIssue>,
    stale_assignments: Vec<workgraph::check::StaleAssignment>,
    stuck_blocked: Vec<workgraph::check::StuckBlocked>,
    node_count: usize,
    loop_edge_count: usize,
    warnings: usize,
    errors: usize,
}

pub fn run(dir: &Path, json: bool) -> Result<()> {
    let (graph, _path) = super::load_workgraph(dir)?;
    let result = check_all(&graph);

    let warnings =
        result.cycles.len() + result.stale_assignments.len() + result.stuck_blocked.len();
    let errors = result.orphan_refs.len() + result.loop_edge_issues.len();
    let loop_edge_count: usize = graph.tasks().map(|t| t.loops_to.len()).sum();

    if json {
        let output = CheckJsonOutput {
            ok: result.ok,
            cycles: result.cycles,
            orphan_refs: result.orphan_refs,
            loop_edge_issues: result.loop_edge_issues,
            stale_assignments: result.stale_assignments,
            stuck_blocked: result.stuck_blocked,
            node_count: graph.len(),
            loop_edge_count,
            warnings,
            errors,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Cycles are warnings (allowed for recurring tasks)
    if !result.cycles.is_empty() {
        eprintln!("Warning: Cycles detected (this is OK for recurring tasks):");
        for cycle in &result.cycles {
            eprintln!("  {}", cycle.join(" -> "));
        }
    }

    // Stale assignments are warnings
    if !result.stale_assignments.is_empty() {
        eprintln!(
            "Warning: Stale assignments (task is open but has an agent assigned — agent may have died):"
        );
        for stale in &result.stale_assignments {
            eprintln!("  {} (assigned to '{}')", stale.task_id, stale.assigned);
        }
    }

    // Stuck blocked tasks are warnings
    if !result.stuck_blocked.is_empty() {
        eprintln!(
            "Warning: Stuck blocked tasks (all dependencies are terminal but task is still blocked):"
        );
        for stuck in &result.stuck_blocked {
            eprintln!(
                "  {} (blocked by: {})",
                stuck.task_id,
                stuck.blocked_by_ids.join(", ")
            );
        }
    }

    // Orphan references are errors
    if !result.orphan_refs.is_empty() {
        eprintln!("Error: Orphan references:");
        for orphan in &result.orphan_refs {
            eprintln!(
                "  {} --[{}]--> {} (not found)",
                orphan.from, orphan.relation, orphan.to
            );
        }
    }

    // Loop edge issues are errors
    if !result.loop_edge_issues.is_empty() {
        eprintln!("Error: Loop edge issues:");
        for issue in &result.loop_edge_issues {
            let desc = match &issue.kind {
                LoopEdgeIssueKind::TargetNotFound => {
                    format!(
                        "{} --[loops_to]--> {} (target not found)",
                        issue.from, issue.target
                    )
                }
                LoopEdgeIssueKind::ZeroMaxIterations => {
                    format!(
                        "{} --[loops_to]--> {} (max_iterations is 0, loop will never fire)",
                        issue.from, issue.target
                    )
                }
                LoopEdgeIssueKind::GuardTaskNotFound(guard_task) => {
                    format!(
                        "{} --[loops_to]--> {} (guard references non-existent task '{}')",
                        issue.from, issue.target, guard_task
                    )
                }
                LoopEdgeIssueKind::SelfLoop => {
                    format!(
                        "{} --[loops_to]--> {} (self-loop: task would immediately re-open on completion)",
                        issue.from, issue.target
                    )
                }
            };
            eprintln!("  {}", desc);
        }
    }

    // Count loop edges for info
    if loop_edge_count > 0 && result.loop_edge_issues.is_empty() {
        println!("Loop edges: {} edge(s), all valid", loop_edge_count);
    }

    if errors > 0 {
        anyhow::bail!("Found {} error(s) and {} warning(s)", errors, warnings);
    } else if warnings > 0 {
        println!("Graph OK: {} nodes, {} warning(s)", graph.len(), warnings);
    } else {
        println!("Graph OK: {} nodes, no issues found", graph.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::graph_path;
    use super::*;
    use tempfile::TempDir;
    use workgraph::graph::{LoopEdge, Node, Task};
    use workgraph::parser::save_graph;

    fn make_task(id: &str, title: &str) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            ..Task::default()
        }
    }

    fn setup_graph(dir: &Path, graph: &workgraph::graph::WorkGraph) {
        std::fs::create_dir_all(dir).unwrap();
        let path = graph_path(dir);
        save_graph(graph, &path).unwrap();
    }

    #[test]
    fn test_check_ok_clean_graph() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".workgraph");

        let mut graph = workgraph::graph::WorkGraph::new();
        graph.add_node(Node::Task(make_task("t1", "Task 1")));
        graph.add_node(Node::Task(make_task("t2", "Task 2")));
        setup_graph(&dir, &graph);

        let result = run(&dir, false);
        assert!(result.is_ok(), "clean graph should pass check");
    }

    #[test]
    fn test_check_fails_on_orphan_refs() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".workgraph");

        let mut graph = workgraph::graph::WorkGraph::new();
        let mut t1 = make_task("t1", "Task 1");
        t1.blocked_by = vec!["nonexistent".to_string()];
        graph.add_node(Node::Task(t1));
        setup_graph(&dir, &graph);

        let result = run(&dir, false);
        assert!(result.is_err(), "orphan refs should fail check");
    }

    #[test]
    fn test_check_warns_on_cycles_but_no_error_alone() {
        // Cycles are treated as warnings, not errors, in the command layer.
        // However, cycles in blocked_by also create orphan-like issues only
        // if the target doesn't exist. With valid nodes that have cycles,
        // the check should still succeed (cycles are just warnings).
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".workgraph");

        let mut graph = workgraph::graph::WorkGraph::new();
        let mut t1 = make_task("t1", "Task 1");
        t1.blocked_by = vec!["t2".to_string()];
        let mut t2 = make_task("t2", "Task 2");
        t2.blocked_by = vec!["t1".to_string()];
        graph.add_node(Node::Task(t1));
        graph.add_node(Node::Task(t2));
        setup_graph(&dir, &graph);

        // Cycles are warnings, not errors — run should succeed
        // (the command only bails on errors > 0, not warnings)
        let result = run(&dir, false);
        assert!(
            result.is_ok(),
            "cycles alone should not cause check failure (they are warnings)"
        );
    }

    #[test]
    fn test_check_fails_on_loop_edge_issues() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".workgraph");

        let mut graph = workgraph::graph::WorkGraph::new();
        let mut t1 = make_task("t1", "Task 1");
        t1.loops_to = vec![LoopEdge {
            target: "nonexistent".to_string(),
            guard: None,
            max_iterations: 3,
            delay: None,
        }];
        graph.add_node(Node::Task(t1));
        setup_graph(&dir, &graph);

        let result = run(&dir, false);
        assert!(result.is_err(), "loop edge issues should fail check");
    }

    #[test]
    fn test_check_fails_when_not_initialized() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".workgraph");
        // Don't create anything — dir doesn't even exist

        let result = run(&dir, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not initialized"));
    }

    #[test]
    fn test_check_ok_with_valid_loop_edges() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".workgraph");

        let mut graph = workgraph::graph::WorkGraph::new();
        let t1 = make_task("t1", "Task 1");
        let mut t2 = make_task("t2", "Task 2");
        t2.loops_to = vec![LoopEdge {
            target: "t1".to_string(),
            guard: None,
            max_iterations: 5,
            delay: None,
        }];
        graph.add_node(Node::Task(t1));
        graph.add_node(Node::Task(t2));
        setup_graph(&dir, &graph);

        let result = run(&dir, false);
        assert!(result.is_ok(), "valid loop edges should pass check");
    }

    #[test]
    fn test_check_warns_on_stale_assignments_but_no_error() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join(".workgraph");

        let mut graph = workgraph::graph::WorkGraph::new();
        let mut t1 = make_task("t1", "Task 1");
        t1.assigned = Some("agent-dead".to_string());
        graph.add_node(Node::Task(t1));
        setup_graph(&dir, &graph);

        // Stale assignments are warnings, not errors — run should succeed
        let result = run(&dir, false);
        assert!(
            result.is_ok(),
            "stale assignments alone should not cause check failure (they are warnings)"
        );
    }
}
