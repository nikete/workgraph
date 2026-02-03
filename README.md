# workgraph

A task graph for getting things done. Works for humans, works for AI agents, works for both at once.

## What is this?

You've got tasks. Some block others. Multiple people (or AIs) need to coordinate without stepping on each other. Workgraph handles that.

```bash
wg init
wg add "Design the API"
wg add "Build the backend" --blocked-by design-the-api
wg add "Write tests" --blocked-by build-the-backend

wg ready        # what can I work on?
wg claim design-the-api --actor erik
wg done design-the-api   # automatically unblocks the next task
```

Tasks flow through `open → in-progress → done`. Dependencies are respected. No one works on the same thing twice.

## Install

From source:

```bash
git clone https://github.com/graphwork/workgraph
cd workgraph
cargo install --path .
```

Or directly via cargo:

```bash
cargo install --git https://github.com/graphwork/workgraph
```

Verify it works:

```bash
wg --help
```

## Setup

### 1. Initialize in your project

```bash
cd your-project
wg init
```

This creates `.workgraph/` with your task graph.

### 2. Add some tasks

```bash
# Simple task
wg add "Set up CI pipeline"

# Task with a blocker
wg add "Deploy to staging" --blocked-by set-up-ci-pipeline

# Task with metadata
wg add "Implement auth" \
  --hours 8 \
  --skill rust \
  --skill security \
  --deliverable src/auth.rs
```

### 3. Register yourself (or your agent)

```bash
# Human
wg actor add erik --name "Erik" --role engineer -c rust -c python

# AI agent
wg actor add claude --name "Claude" --role agent -c coding -c testing -c docs
```

### 4. Start working

```bash
wg ready                         # see what's available
wg claim set-up-ci-pipeline --actor erik
# ... do the work ...
wg done set-up-ci-pipeline       # unblocks deploy-to-staging
```

## Using with AI Coding Assistants

Workgraph includes a skill definition that teaches AI assistants the full protocol.

### Claude Code

From the workgraph directory, install the skill:

```bash
# Personal (all your projects)
cp -r .claude/skills/wg ~/.claude/skills/

# Or project-specific
cp -r .claude/skills/wg /path/to/your-project/.claude/skills/
```

The skill has YAML frontmatter so Claude auto-detects when to use it. You can also invoke explicitly with `/wg`.

### OpenCode / Codex / Other Agents

```bash
cat .claude/skills/wg/SKILL.md >> /path/to/your-project/AGENTS.md
```

Or just add the core instruction:

```markdown
## Task Management

Use workgraph (`wg`) for task coordination:
- `wg ready` to see available tasks
- `wg claim <task> --actor <name>` before working
- `wg done <task>` when complete
- `wg fail <task> --reason "..."` if blocked

See `wg --help` for all commands.
```

### What the skill teaches

The full skill covers:
- When to use workgraph (multi-step projects, cross-session work, coordination)
- The claim/work/done protocol
- Progress logging and artifact tracking
- Planning with dependencies
- Analysis commands for project health
- Agent service (spawn, kill, monitoring)
- Multi-agent coordination rules

It's designed to be self-contained - an agent can read it and immediately know how to participate.

## Agentic workflows

### Pattern 1: Human plans, agent executes

You define the work, agent does it:

```bash
# You: create the plan
wg add "Refactor auth module" --skill rust
wg add "Update tests" --blocked-by refactor-auth-module --skill testing
wg add "Update docs" --blocked-by refactor-auth-module --skill docs

# Agent: execute
wg agent --actor claude --max-tasks 10
```

The agent will work through ready tasks, respecting dependencies.

### Pattern 2: Agent plans and executes

Let the agent figure out what needs doing:

```markdown
# In CLAUDE.md or your prompt:

Break down this goal into tasks using workgraph:
1. Analyze what needs to be done
2. Create tasks with `wg add`, linking dependencies with --blocked-by
3. Work through them with `wg ready` / `wg claim` / `wg done`
4. If you discover more work, add it to the graph
```

### Pattern 3: Top-level coordinator

One agent manages the work, spawns sub-agents for execution:

```markdown
# Coordinator prompt:

You are a project coordinator. Your job:
1. Check `wg ready` for available work
2. For each ready task, spawn a sub-agent to handle it
3. Sub-agents should `wg claim`, do the work, then `wg done` or `wg fail`
4. Monitor progress with `wg list` and `wg analyze`
5. Replan if needed - add tasks, adjust dependencies
6. Continue until `wg ready` returns nothing and all tasks are done
```

### Pattern 4: Parallel agents

Multiple agents working simultaneously:

```bash
# Terminal 1
wg agent --actor claude-1

# Terminal 2
wg agent --actor claude-2

# Terminal 3
wg agent --actor claude-3
```

Each agent claims different tasks. The claim mechanism prevents conflicts.

### Pattern 5: Mixed human + AI

```bash
# Human claims the design work
wg claim design-api --actor erik

# Agent handles implementation once design is done
wg agent --actor claude
```

The agent waits for your work to complete before touching dependent tasks.

## Service

The service daemon automates agent spawning and lifecycle management. Start it once and it continuously picks up ready tasks, spawns agents, and cleans up dead ones.

### Quick start

```bash
wg service start
```

That's it. The daemon watches your task graph and auto-spawns agents on ready tasks (up to `max_agents` in parallel). When a task completes and unblocks new ones, the daemon picks those up too.

Monitor what's happening:

```bash
wg service status    # daemon info, agent summary, coordinator state
wg agents            # list all agents
wg tui               # interactive dashboard
```

Stop the daemon when you're done:

```bash
wg service stop              # stop daemon (agents keep running)
wg service stop --kill-agents  # stop daemon and all agents
```

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

Set config values with:

```bash
wg config --max-agents 8
wg config --set-model sonnet
wg config --poll-interval 120
wg config --executor shell
```

CLI flags on `wg service start` override config.toml:

```bash
wg service start --max-agents 8 --executor shell --interval 120 --model haiku
```

### Managing the service

| Command | What it does |
|---------|-------------|
| `wg service start` | Start the background daemon |
| `wg service stop` | Stop daemon (agents continue independently) |
| `wg service stop --kill-agents` | Stop daemon and kill all running agents |
| `wg service stop --force` | Immediately SIGKILL the daemon |
| `wg service status` | Show daemon PID, uptime, agent summary, coordinator state |
| `wg service reload` | Re-read config.toml without restarting |
| `wg service install` | Generate a systemd user service file |

Reload lets you change settings at runtime:

```bash
wg service reload                              # re-read config.toml
wg service reload --max-agents 8 --model haiku # apply specific overrides
```

### Agent management

List and filter agents:

```bash
wg agents              # all agents
wg agents --alive      # running agents only
wg agents --dead       # dead agents only
wg agents --working    # actively working on a task
wg agents --idle       # waiting for work
wg agents --json       # JSON output for scripting
```

Kill agents:

```bash
wg kill agent-7          # graceful: SIGTERM → wait → SIGKILL
wg kill agent-7 --force  # immediate SIGKILL
wg kill --all            # kill all running agents
```

Killing an agent automatically unclaims its task so another agent can pick it up.

**Dead agent detection:** Agents send heartbeats while working. If an agent's process exits or its heartbeat goes stale (default: 5 minutes), the coordinator marks it dead and unclaims its task. You can also check manually:

```bash
wg dead-agents --check     # check for dead agents (read-only)
wg dead-agents --cleanup   # mark dead and unclaim their tasks
wg dead-agents --remove    # remove dead agents from registry
```

### Model selection

Models are selected in priority order:

1. `--model` flag on `wg spawn` (highest priority)
2. Task's `--model` property (set at creation)
3. Coordinator config (`coordinator.model` in config.toml)
4. Agent config default (`agent.model` in config.toml)

```bash
# Set model per-task
wg add "Simple fix" --model haiku
wg add "Complex design" --model opus

# Override at spawn time
wg spawn my-task --executor claude --model haiku

# Set coordinator default (applies to all auto-spawned agents)
wg config --set-model sonnet
wg service reload
```

**Cost tips:** Use **haiku** for simple formatting/linting, **sonnet** for typical coding, **opus** for complex reasoning and architecture.

### The TUI

Launch the interactive terminal dashboard:

```bash
wg tui [--refresh-rate 2000]  # default: 2000ms refresh
```

The TUI has three views:

**Dashboard** — split-pane showing tasks (left) and agents (right) with status bars.

**Graph Explorer** — tree view of the dependency DAG with task status and active agent indicators.

**Log Viewer** — real-time tailing of agent output with auto-scroll.

#### Keybindings

**Global:**

| Key | Action |
|-----|--------|
| `q` | Quit |
| `?` | Show help overlay |
| `Esc` | Back / close overlay |

**Dashboard:**

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch panel (Tasks ↔ Agents) |
| `j` / `k` or `↑` / `↓` | Scroll up / down |
| `Enter` | Drill into selected item |
| `g` | Open graph explorer |
| `r` | Refresh data |

**Graph Explorer:**

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Navigate up / down |
| `h` / `l` or `←` / `→` | Collapse / expand subtree |
| `Enter` | View task details or jump to agent log |
| `a` | Cycle to next task with active agents |
| `r` | Refresh graph |

**Log Viewer:**

| Key | Action |
|-----|--------|
| `j` / `k` or `↑` / `↓` | Scroll one line |
| `PageDown` / `PageUp` | Scroll half viewport |
| `g` | Jump to top (disable auto-scroll) |
| `G` | Jump to bottom (enable auto-scroll) |

### Troubleshooting

**Daemon logs:** Check `.workgraph/service/daemon.log` for errors. The daemon logs with timestamps and rotates at 10 MB (keeps one backup at `daemon.log.1`).

```bash
# Recent errors are also shown in status
wg service status
```

**Common issues:**

- **"Socket already exists"** — A previous daemon didn't clean up. Check if it's still running with `wg service status`, then `wg service stop` or manually remove the stale socket.
- **Agents not spawning** — Check `wg service status` for coordinator state. Verify `max_agents` isn't already reached with `wg agents --alive`. Ensure there are tasks in `wg ready`.
- **Agent marked dead prematurely** — Increase `heartbeat_timeout` in config.toml if agents do long-running work without heartbeating.
- **Config changes not taking effect** — Run `wg service reload` after editing `config.toml`. CLI flag overrides on `wg service start` take precedence over the file.
- **Daemon won't start** — Check if another daemon is already running. Look at `.workgraph/service/state.json` for stale PID info.

**State files:** The service stores runtime state in `.workgraph/service/`:

| File | Purpose |
|------|---------|
| `state.json` | Daemon PID, socket path, start time |
| `daemon.log` | Persistent daemon logs |
| `coordinator-state.json` | Effective config and runtime metrics |
| `registry.json` | Agent registry (IDs, PIDs, tasks, status) |

## The recommended flow

For most projects:

1. **Plan first**: Sketch out the major tasks and dependencies
   ```bash
   wg add "Goal task"
   wg add "Step 1"
   wg add "Step 2" --blocked-by step-1
   wg add "Step 3" --blocked-by step-2
   # ... etc
   ```

2. **Check the structure**:
   ```bash
   wg analyze        # health check
   wg critical-path  # what's the longest chain?
   wg bottlenecks    # what should we prioritize?
   ```

3. **Execute**: Either manually or with agents
   ```bash
   wg agent --actor claude --once  # one task at a time, review between
   # or
   wg agent --actor claude         # let it run
   ```

4. **Adapt**: As you learn more, update the graph
   ```bash
   wg add "New thing we discovered" --blocked-by whatever
   wg fail stuck-task --reason "Need to rethink this"
   wg retry stuck-task  # when ready to try again
   ```

5. **Ship**: When `wg ready` is empty and everything important is done, you're there.

## Key concepts

**Tasks** have a status (`open`, `in-progress`, `done`, `failed`, `abandoned`) and can block other tasks.

**Actors** are humans or AI agents. They claim tasks to work on them.

**The graph** is tasks connected by "blocked-by" relationships. A task is blocked until all its blockers are done.

**Context flow**: Tasks can declare inputs (what they need) and deliverables (what they produce). Use `wg context <task>` to see what's available.

**Trajectories**: For AI agents, `wg trajectory <task>` suggests the best order to claim related tasks, minimizing context switches.

## Analysis commands

```bash
wg ready           # what can be worked on now?
wg list            # all tasks
wg show <id>       # full task details
wg why-blocked <id> # trace the blocker chain
wg impact <id>     # what depends on this?
wg bottlenecks     # tasks blocking the most work
wg critical-path   # longest dependency chain
wg forecast        # when will we be done?
wg analyze         # comprehensive health report
```

## Storage

Everything lives in `.workgraph/graph.jsonl`. One JSON object per line. Human-readable, git-friendly, easy to hack on.

```jsonl
{"kind":"task","id":"design-api","title":"Design the API","status":"done"}
{"kind":"task","id":"build-backend","title":"Build the backend","status":"open","blocked_by":["design-api"]}
{"kind":"actor","id":"claude","name":"Claude","role":"agent","capabilities":["coding","testing"]}
```

Configuration is in `.workgraph/config.toml`:

```toml
[agent]
executor = "claude"
model = "opus-4-5"
interval = 10

[project]
name = "My Project"
```

## More docs

- [docs/COMMANDS.md](docs/COMMANDS.md) - Complete command reference
- [docs/AGENT-GUIDE.md](docs/AGENT-GUIDE.md) - Deep dive on agent operation

## License

MIT
