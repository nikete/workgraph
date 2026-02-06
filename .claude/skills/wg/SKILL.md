---
name: wg
description: Use this skill for task coordination with workgraph (wg). Triggers include "workgraph", "wg", task graphs, multi-step projects, tracking dependencies, coordinating agents, or when you see a .workgraph directory.
---

# workgraph

Structured task coordination for complex work. Use this when you need to break down, track, and execute multi-step projects.

## When to use workgraph

- Projects with multiple dependent tasks
- Work that spans multiple sessions
- Coordinating between humans and AI agents
- Anything where you need to track "what's done, what's next, what's blocked"

## Quick check

```bash
wg ready    # what can I work on?
wg list     # all tasks
wg analyze  # project health
```

## The protocol

### Starting work

1. Check what's available:
   ```bash
   wg ready
   ```

2. Claim a task:
   ```bash
   wg claim <task-id> --actor claude
   ```

3. Understand the task:
   ```bash
   wg show <task-id>      # full details
   wg context <task-id>   # inputs from dependencies
   ```

### While working

Log progress (helps with context recovery if interrupted):
```bash
wg log <task-id> "Completed X, now working on Y"
```

If you produce output files:
```bash
wg artifact <task-id> path/to/output
```

### Finishing

Success:
```bash
wg done <task-id>
```

Failed (can retry later):
```bash
wg fail <task-id> --reason "why it failed"
```

Need to stop mid-task:
```bash
wg unclaim <task-id>
```

### Verified tasks

Some tasks require human review before completion. These use a submit/approve workflow:

```bash
# Create a verified task
wg add "Critical feature" --verify "Check tests pass and code reviewed"

# Agent works and submits (can't use wg done)
wg submit <task-id>  # → status: pending-review [R]

# Reviewer approves or rejects
wg approve <task-id>              # → status: done
wg reject <task-id> --reason "..."  # → status: open (retry)
```

**Status indicators:**
- `[R]` = pending-review (awaiting verification)

**Auto-submit:** When spawned agents complete verified tasks, the wrapper automatically uses `submit` instead of `done`.

### Discovering new work

Add tasks as you discover them:
```bash
wg add "New task title" --blocked-by current-task
```

Check impact:
```bash
wg impact <task-id>  # what depends on this?
```

## Planning work

Break down a goal:
```bash
wg add "Goal: Ship the feature"
wg add "Design the API"
wg add "Implement backend" --blocked-by design-the-api
wg add "Write tests" --blocked-by implement-backend
wg add "Update docs" --blocked-by implement-backend
```

Add metadata:
```bash
wg add "Complex task" \
  --hours 4 \
  --skill rust \
  --deliverable src/feature.rs \
  --blocked-by prerequisite-task \
  --model sonnet
```

Check the plan:
```bash
wg critical-path  # longest chain
wg bottlenecks    # what to prioritize
wg forecast       # when will it be done?
```

## Analysis commands

| Command | What it tells you |
|---------|-------------------|
| `wg ready` | Tasks you can work on now |
| `wg list` | All tasks with status |
| `wg show <id>` | Full task details |
| `wg why-blocked <id>` | Why can't this start? |
| `wg impact <id>` | What depends on this? |
| `wg bottlenecks` | Highest-impact tasks |
| `wg critical-path` | Longest dependency chain |
| `wg forecast` | Completion estimate |
| `wg analyze` | Full health report |
| `wg context <id>` | Available inputs |
| `wg trajectory <id>` | Optimal claim order |

## Key behaviors

1. **Always claim before working** - prevents conflicts with other agents
2. **Log as you go** - helps recovery if interrupted
3. **Mark done immediately** - unblocks dependent tasks
4. **Add tasks as you discover them** - keep the graph current
5. **Check `wg ready` after completing** - see what's unblocked

## Multi-agent coordination

If multiple agents are working:
- Claims are atomic (no double-work)
- Use `wg coordinate` to see parallel opportunities
- Each agent should have a unique actor ID

## Agent Service

The service daemon is the recommended way to run multi-agent workflows. It combines agent spawning, lifecycle management, and the coordinator into a single background process.

### Quick start

```bash
wg service start     # start daemon — auto-spawns agents on ready tasks
wg agents            # see what's running
wg tui               # interactive dashboard
wg service stop      # stop daemon when done
```

The daemon watches your task graph and automatically:
1. Spawns agents on ready tasks (up to `max_agents`)
2. Detects dead agents (process exit or stale heartbeat)
3. Unclaims dead agents' tasks so others can pick them up
4. Spawns new agents when tasks become unblocked

### When to use the service vs manual claim

Use `wg service start` when:
- Running multiple agents in parallel
- Want automatic task pickup as work becomes ready
- Need heartbeat monitoring and dead agent detection
- Hands-off operation of a task graph

Use manual `wg claim` when:
- Working interactively on a single task
- Testing or debugging a specific task
- Don't need process management

### Configuration

The service reads from `.workgraph/config.toml`:

```toml
[coordinator]
max_agents = 4         # max parallel agents (default: 4)
poll_interval = 60     # seconds between coordinator ticks (default: 60)
executor = "claude"    # executor for spawned agents (default: "claude")
model = "opus-4-5"    # model override for all spawned agents (optional)

[agent]
executor = "claude"
model = "opus-4-5"
heartbeat_timeout = 5  # minutes before agent is considered dead (default: 5)
```

Set config via CLI:
```bash
wg config --max-agents 8
wg config --model sonnet
wg config --poll-interval 120
wg config --executor shell
```

CLI flags on `wg service start` override config.toml:
```bash
wg service start --max-agents 8 --executor shell --interval 120 --model haiku
```

### Managing the service

```bash
wg service start              # start background daemon
wg service stop               # stop daemon (agents continue independently)
wg service stop --kill-agents # stop daemon and kill all agents
wg service stop --force       # SIGKILL daemon immediately
wg service status             # PID, uptime, agent summary, coordinator state
wg service reload             # re-read config.toml without restart
wg service reload --max-agents 8 --model haiku  # apply specific overrides
wg service install            # generate systemd user service file
```

### Spawning agents manually

You can also spawn agents for specific tasks directly:

```bash
wg spawn <task-id> --executor <name> [--timeout <duration>] [--model <model>]
```

This claims the task, starts the executor process, registers the agent, and returns immediately.

```bash
wg spawn implement-feature --executor claude
wg spawn simple-task --executor claude --model haiku
```

### Model selection

Models are selected in priority order (highest first):

1. `--model` flag on `wg spawn`
2. Task's `--model` property (set at creation with `wg add`)
3. Coordinator config (`coordinator.model` in config.toml)
4. Agent config default (`agent.model` in config.toml)

```bash
# Set per-task model at creation
wg add "Simple fix" --model haiku
wg add "Complex design" --model opus

# Override at spawn time
wg spawn my-task --executor claude --model haiku

# Set coordinator default for all auto-spawned agents
wg config --model sonnet
wg service reload
```

**Cost tips:** Use **haiku** for simple formatting/linting, **sonnet** for typical coding, **opus** for complex reasoning and architecture.

### Agent management

```bash
wg agents              # list all agents
wg agents --alive      # running only
wg agents --dead       # dead only
wg agents --working    # actively working on a task
wg agents --idle       # waiting for work
wg agents --json       # JSON for scripting
```

Kill agents:
```bash
wg kill <agent-id>         # graceful: SIGTERM → wait → SIGKILL
wg kill <agent-id> --force # immediate SIGKILL
wg kill --all              # kill all running agents
```

Killing an agent automatically unclaims its task.

### Dead agent detection

Agents send heartbeats while working. The service daemon automatically detects dead agents (process exited or heartbeat stale) and unclaims their tasks. You can also check manually:

```bash
wg dead-agents --check        # check for dead agents (read-only)
wg dead-agents --cleanup      # mark dead and unclaim their tasks
wg dead-agents --remove       # remove dead agents from registry
wg dead-agents --threshold 10 # custom timeout in minutes
```

### The TUI

Interactive terminal dashboard for monitoring:

```bash
wg tui [--refresh-rate 2000]  # default: 2000ms
```

**Views:**
- **Dashboard** — tasks (left panel) and agents (right panel) with status bars
- **Graph Explorer** — dependency DAG tree with task status and agent indicators
- **Log Viewer** — real-time agent output with auto-scroll

**Keybindings:**

| Key | Action |
|-----|--------|
| `q` | Quit |
| `?` | Help overlay |
| `Esc` | Back / close |
| `Tab` / `Shift+Tab` | Switch panel |
| `j`/`k` or `↑`/`↓` | Navigate |
| `Enter` | Drill into item |
| `g` | Graph explorer (dashboard) / jump to top (log viewer) |
| `G` | Jump to bottom (log viewer, enables auto-scroll) |
| `h`/`l` or `←`/`→` | Collapse/expand (graph explorer) |
| `d` | Toggle tree/DAG view (graph explorer) |
| `a` | Cycle to next active-agent task (graph explorer) |
| `r` | Refresh |
| `PageDown`/`PageUp` | Scroll half viewport (log viewer) |

### Executor configuration

Executors define how agents run. Place configs in `.workgraph/executors/`:

**Claude executor** (`claude.toml`):
```toml
[executor]
type = "claude"
command = "claude"
args = ["--print", "--dangerously-skip-permissions"]

[executor.prompt_template]
template = """
Working on: {{task_id}} - {{task_title}}
{{task_description}}

Context: {{task_context}}

When done: wg done {{task_id}}
"""
```

**Shell executor** (`shell.toml`) — uses task's `exec` field:
```toml
[executor]
type = "shell"
command = "bash"
```

### Troubleshooting

**Daemon logs:** `.workgraph/service/daemon.log` (rotates at 10 MB, keeps one backup).

```bash
wg service status  # shows recent errors
```

**Common issues:**
- **Agents not spawning** — check `wg service status` for coordinator state; ensure `max_agents` isn't reached (`wg agents --alive`); ensure tasks exist in `wg ready`
- **Agent marked dead prematurely** — increase `heartbeat_timeout` in config.toml
- **Config changes not taking effect** — run `wg service reload` after editing config.toml
- **Stale socket** — if daemon didn't clean up, check `wg service status` then `wg service stop` or remove the socket manually

**State files** in `.workgraph/service/`:

| File | Purpose |
|------|---------|
| `state.json` | Daemon PID, socket path, start time |
| `daemon.log` | Persistent daemon logs |
| `coordinator-state.json` | Effective config and runtime metrics |
| `registry.json` | Agent registry |

### Example workflow

```bash
# Set up tasks
wg add "Task A" --model haiku
wg add "Task B" --model haiku
wg add "Task C" --blocked-by task-a --model haiku

# Start service (spawns A and B in parallel, then C when A completes)
wg service start --max-agents 2

# Monitor
wg tui

# When done
wg service stop
```

## Agency: roles, motivations, and evolution

The agency system gives agents distinct identities (role + motivation) that shape how they approach tasks. Evaluations measure output quality, and evolution improves roles and motivations over time based on performance data.

### Concepts

- **Role** — defines *what* an agent is good at: skills, desired outcome, description
- **Motivation** — defines *how* an agent approaches work: acceptable/unacceptable tradeoffs
- **Identity** — a (role, motivation) pair assigned to a task
- **Evaluation** — scores a completed task's output against its identity's criteria
- **Evolution** — creates improved roles/motivations based on evaluation data

### Creating roles and motivations

```bash
# Create a role with skills and desired outcome
wg role add "Code Reviewer" \
  --outcome "Catch bugs and improve code quality" \
  --skill rust --skill testing \
  --description "Reviews PRs for correctness and style"

# Create a motivation with tradeoff boundaries
wg motivation add "Quality First" \
  --accept "Slower delivery" \
  --reject "Skipping tests" \
  --description "Prioritise correctness over speed"
```

### Managing roles and motivations

```bash
wg role list [--json]           # list all roles
wg role show <id> [--json]      # full role details
wg role edit <id>               # open in $EDITOR
wg role lineage <id> [--json]   # evolutionary ancestry
wg role rm <id>                 # delete a role

wg motivation list [--json]
wg motivation show <id> [--json]
wg motivation edit <id>
wg motivation lineage <id> [--json]
wg motivation rm <id>
```

### Assigning identity to tasks

```bash
# Manual: specify both role and motivation
wg assign <task-id> --role <role-id> --motivation <motivation-id>

# Manual: specify role, auto-select best motivation
wg assign <task-id> --role <role-id>

# Automatic: match role and motivation from task skills/tags
wg assign <task-id>

# Clear identity from a task
wg assign <task-id> --clear
```

### Evaluating completed tasks

```bash
# Evaluate a done/pending-review task (spawns a Claude evaluator)
wg evaluate <task-id>

# Preview what would be evaluated without running
wg evaluate <task-id> --dry-run

# Use a specific model for evaluation
wg evaluate <task-id> --model haiku

# JSON output
wg evaluate <task-id> --json
```

Evaluations produce a score (0.0-1.0) with optional dimension breakdowns (correctness, completeness, efficiency, style_adherence) and are recorded in `.workgraph/agency/evaluations/`.

### Evolving roles and motivations

```bash
# Run a full evolution cycle (all strategies)
wg evolve

# Dry-run to preview what would happen
wg evolve --dry-run

# Use a specific strategy
wg evolve --strategy mutation
wg evolve --strategy crossover
wg evolve --strategy gap-analysis
wg evolve --strategy retirement
wg evolve --strategy motivation-tuning

# Limit operations and choose model
wg evolve --budget 3 --model sonnet

# JSON output
wg evolve --json
```

Evolution strategies:
- **mutation** — tweak an existing role/motivation to improve weak dimensions
- **crossover** — combine two roles into a new hybrid
- **gap-analysis** — create new roles/motivations for uncovered task types
- **retirement** — archive poor-performing roles/motivations
- **motivation-tuning** — adjust tradeoff boundaries based on evaluation data
- **all** — apply whichever strategies the evolver deems most impactful (default)

### Agency stats

```bash
# Performance overview: leaderboards, synergy matrix, trends
wg agency stats

# With minimum evaluation threshold for under-explored detection
wg agency stats --min-evals 5

# JSON output
wg agency stats --json
```

### Example workflow: full agency cycle

```bash
# 1. Set up roles and motivations
wg role add "Implementer" --outcome "Working, tested code" --skill rust
wg motivation add "Quality First" --accept "Slower delivery" --reject "Skipping tests"

# 2. Assign identity to a task
wg assign implement-feature --role implementer --motivation quality-first

# 3. Agent works on the task and completes it
wg claim implement-feature --actor claude
# ... do the work ...
wg done implement-feature

# 4. Evaluate the output
wg evaluate implement-feature

# 5. Check agency performance
wg agency stats

# 6. Run evolution to improve roles/motivations
wg evolve --strategy mutation --budget 2
```

## All commands

```
wg init              # start a workgraph
wg add <title>       # create task
wg done <id>         # complete task
wg submit <id>       # submit for review (verified tasks)
wg approve <id>      # approve pending review
wg reject <id>       # reject and return to open
wg fail <id>         # mark failed
wg abandon <id>      # give up on task
wg retry <id>        # retry failed task
wg claim <id>        # take a task
wg unclaim <id>      # release a task
wg reclaim <id>      # take from dead agent
wg log <id> <msg>    # add progress note
wg show <id>         # task details
wg list              # all tasks
wg ready             # available tasks
wg blocked <id>      # direct blockers
wg why-blocked <id>  # full blocker chain
wg impact <id>       # dependents
wg context <id>      # available inputs
wg trajectory <id>   # optimal claim order
wg bottlenecks       # high-impact tasks
wg critical-path     # longest chain
wg forecast          # completion estimate
wg velocity          # completion rate
wg aging             # task age distribution
wg workload          # actor assignments
wg analyze           # health report
wg actor add <id>    # register actor
wg actor list        # list actors
wg artifact <id> <p> # record output
wg exec <id>         # run task command
wg agent --actor <x> # autonomous loop
wg config            # view/set config
wg spawn <id>        # spawn agent for task
wg agents            # list running agents
wg kill <agent-id>   # terminate agent
wg service start     # start service daemon
wg service stop      # stop service daemon
wg service status    # daemon status
wg service reload    # reload config
wg service install   # generate systemd service
wg dead-agents       # dead agent detection
wg tui               # interactive dashboard
wg role add <name>   # create a role
wg role list         # list roles
wg role show <id>    # role details
wg role edit <id>    # edit role in $EDITOR
wg role lineage <id> # role ancestry
wg role rm <id>      # delete role
wg motivation add    # create a motivation
wg motivation list   # list motivations
wg motivation show   # motivation details
wg motivation edit   # edit motivation
wg motivation lineage # motivation ancestry
wg motivation rm     # delete motivation
wg assign <id>       # assign identity to task
wg evaluate <id>     # evaluate completed task
wg evolve            # run evolution cycle
wg agency stats      # agency performance stats
```

All commands support `--json` for structured output.
