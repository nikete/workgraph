use anyhow::Result;

const QUICKSTART_TEXT: &str = r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                         WORKGRAPH AGENT QUICKSTART                           ║
╚══════════════════════════════════════════════════════════════════════════════╝

CORE WORKFLOW: ready → claim → log → done
─────────────────────────────────────────
  wg ready                    # See tasks available to work on
  wg claim <task-id>          # Claim a task (sets status to in-progress)
  wg log <task-id> "message"  # Log progress as you work
  wg done <task-id>           # Mark task complete (use 'wg submit' if verified)

DISCOVERING & ADDING WORK
─────────────────────────────────────────
  wg list                     # List all tasks
  wg list --status open       # Filter by status (open, in-progress, done, etc.)
  wg show <task-id>           # View task details and context
  wg add "Title" -d "Desc"    # Add new task
  wg add "X" --blocked-by Y   # Add task blocked by another

TASK STATE COMMANDS
─────────────────────────────────────────
  wg done <task-id>           # Complete (non-verified tasks)
  wg submit <task-id>         # Submit for review (verified tasks)
  wg fail <task-id> --reason  # Mark failed (can be retried)
  wg abandon <task-id>        # Give up permanently

CONTEXT & ARTIFACTS
─────────────────────────────────────────
  wg context <task-id>        # See context from dependencies
  wg artifact <task-id> path  # Record output file/artifact
  wg log <task-id> --list     # View task's progress log

SERVICE MODE (Autonomous Agents)
─────────────────────────────────────────
  wg service start            # Start the agent coordinator daemon
  wg service status           # Check daemon status
  wg tui                      # Interactive dashboard
  wg agents                   # List running agents

TIPS
─────────────────────────────────────────
• Run 'wg log' BEFORE starting work to track progress
• Use 'wg context' to understand what dependencies produced
• If 'wg done' fails, the task may require verification - use 'wg submit'
• Check 'wg blocked <task-id>' if a task isn't appearing in ready list
"#;

pub fn run(json: bool) -> Result<()> {
    if json {
        let output = serde_json::json!({
            "workflow": {
                "steps": ["ready", "claim", "log", "done"],
                "description": "Core workflow for completing tasks"
            },
            "commands": {
                "discovery": {
                    "ready": "See tasks available to work on",
                    "list": "List all tasks",
                    "show": "View task details and context",
                    "add": "Add a new task"
                },
                "work": {
                    "claim": "Claim a task for work",
                    "log": "Log progress as you work",
                    "context": "See context from dependencies",
                    "artifact": "Record output file/artifact"
                },
                "completion": {
                    "done": "Mark task complete (non-verified)",
                    "submit": "Submit for review (verified tasks)",
                    "fail": "Mark failed (can be retried)",
                    "abandon": "Give up permanently"
                },
                "service": {
                    "service start": "Start the agent coordinator daemon",
                    "service status": "Check daemon status",
                    "tui": "Interactive dashboard",
                    "agents": "List running agents"
                }
            },
            "tips": [
                "Run 'wg log' BEFORE starting work to track progress",
                "Use 'wg context' to understand what dependencies produced",
                "If 'wg done' fails, the task may require verification - use 'wg submit'",
                "Check 'wg blocked <task-id>' if a task isn't appearing in ready list"
            ]
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", QUICKSTART_TEXT.trim());
    }

    Ok(())
}
