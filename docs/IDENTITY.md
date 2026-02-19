# Identity System

The identity system gives workgraph agents composable identities. Instead of every agent being a generic assistant, you define **roles** (what an agent does), **objectives** (why it acts that way), and pair them into **agents** that are assigned to tasks, rewarded, and evolved over time.

Agents can be **human or AI**. The difference is the executor: AI agents use `claude` (or similar), human agents use `matrix`, `email`, or `shell`. Both share the same identity model — roles, objectives, capabilities, trust levels, and performance tracking all work uniformly regardless of who (or what) is doing the work.

## Core Concepts

### Role

A role defines **what** an agent does.

| Field | Description | Identity-defining? |
|-------|-------------|--------------------|
| `name` | Human-readable label (e.g. "Programmer") | No |
| `description` | What this role is about | Yes |
| `skills` | List of skill references (see [Skill System](#skill-system)) | Yes |
| `desired_outcome` | What good output looks like | Yes |
| `performance` | Aggregated reward scores | No (mutable) |
| `lineage` | Evolutionary history | No (mutable) |

### Objective

An objective defines **why** an agent acts the way it does.

| Field | Description | Identity-defining? |
|-------|-------------|--------------------|
| `name` | Human-readable label (e.g. "Careful") | No |
| `description` | What this objective prioritizes | Yes |
| `acceptable_tradeoffs` | Compromises the agent may make | Yes |
| `unacceptable_tradeoffs` | Hard constraints the agent must never violate | Yes |
| `performance` | Aggregated reward scores | No (mutable) |
| `lineage` | Evolutionary history | No (mutable) |

### Agent

An agent is the **unified identity** in workgraph — it can represent a human or an AI. For AI agents, it is a named pairing of a role and an objective. For human agents, role and objective are optional.

| Field | Description |
|-------|-------------|
| `name` | Human-readable label |
| `role_id` | Content-hash ID of the role (required for AI, optional for human) |
| `objective_id` | Content-hash ID of the objective (required for AI, optional for human) |
| `capabilities` | Skills/capabilities for task matching (e.g., `rust`, `testing`) |
| `rate` | Hourly rate for cost forecasting |
| `capacity` | Maximum concurrent task capacity |
| `trust_level` | `Verified`, `Provisional` (default), or `Unknown` |
| `contact` | Contact info — email, Matrix ID, etc. (primarily for human agents) |
| `executor` | How this agent receives work: `claude` (default), `matrix`, `email`, `shell` |
| `performance` | Agent-level aggregated reward scores |
| `lineage` | Evolutionary history |

The same role paired with different objectives produces different agents. A "Programmer" role with a "Careful" objective produces a different agent than with a "Fast" objective.

Human agents are distinguished by their executor. Agents with a human executor (`matrix`, `email`, `shell`) don't need a role or objective — they're real people who bring their own judgment. AI agents (executor `claude`) require both, because the role and objective are injected into the prompt to shape behavior.

## Content-Hash IDs

Every role, objective, and agent is identified by a **SHA-256 content hash** of its identity-defining fields.

- **Deterministic**: Same content → same ID
- **Deduplication**: Can't create two entities with identical content
- **Immutable identity**: Changing an identity-defining field produces a *new* entity. The old one stays.

| Entity | Hashed fields |
|--------|---------------|
| Role | `skills` + `desired_outcome` + `description` |
| Objective | `acceptable_tradeoffs` + `unacceptable_tradeoffs` + `description` |
| Agent | `role_id` + `objective_id` |

For display, IDs are shown as 8-character prefixes (e.g. `a3f7c21d`). All commands accept unique prefixes.

## The Full Identity Loop

The identity system runs as a loop: assign identity → execute task → reward → evolve. Each step can be manual or automated.

```
┌─────────────┐     ┌───────────┐     ┌───────────┐     ┌──────────┐
│  1. Assign  │────>│ 2. Execute│────>│3. Reward│────>│ 4. Evolve│
│  identity   │     │   task    │     │  results  │     │  identity  │
│  to task    │     │  (agent   │     │  (score   │     │  (create │
│             │     │   runs)   │     │   agent)  │     │   new    │
│  wg assign  │     │  wg spawn │     │ wg reward│    │  roles)  │
└─────────────┘     └───────────┘     └───────────┘     └──────────┘
       ▲                                                      │
       └──────────────────────────────────────────────────────┘
                    performance data feeds back
```

### Manual loop

```bash
# 1. Assign
wg assign my-task a3f7c21d

# 2. Execute (service handles this)
wg service start

# 3. Reward
wg reward my-task

# 4. Evolve
wg evolve
```

### Automated loop

```bash
# Enable auto-assign and auto-reward
wg config --auto-assign true --auto-reward true

# The coordinator creates assign-{task} and reward-{task} meta-tasks automatically
# Just start the service and add work:
wg service start
wg add "Implement feature X" --skill rust

# Evolution is still manual (run when you have enough rewards):
wg evolve
```

## Lifecycle

### 1. Create roles and objectives

```bash
# Create a role
wg role add "Programmer" \
  --outcome "Working, tested code" \
  --skill code-writing \
  --skill testing \
  --description "Writes, tests, and debugs code"

# Create an objective
wg objective add "Careful" \
  --accept "Slow" \
  --accept "Verbose" \
  --reject "Unreliable" \
  --reject "Untested" \
  --description "Prioritizes reliability and correctness above speed"
```

Or seed the built-in starters:

```bash
wg identity init
```

This creates four starter roles (Programmer, Reviewer, Documenter, Architect) and four starter objectives (Careful, Fast, Thorough, Balanced).

### 2. Pair into agents

```bash
# AI agent (role + objective required)
wg agent create "Careful Programmer" --role <role-hash> --objective <objective-hash>

# AI agent with operational fields
wg agent create "Careful Programmer" \
  --role <role-hash> \
  --objective <objective-hash> \
  --capabilities rust,testing \
  --rate 50.0

# Human agent (role + objective optional)
wg agent create "Erik" \
  --executor matrix \
  --contact "@erik:server" \
  --capabilities rust,python,architecture \
  --trust-level verified
```

### 3. Assign to tasks

```bash
wg assign <task-id> <agent-hash>
```

When the service spawns that task, the agent's role and objective are rendered into the prompt as an identity section:

```markdown
# Task Assignment

## Agent Identity

### Role: Programmer
Writes, tests, and debugs code

#### Skills
- code-writing
- testing

#### Desired Outcome
Working, tested code

### Operational Parameters
#### Acceptable Trade-offs
- Slow
- Verbose

#### Non-negotiable Constraints
- Unreliable
- Untested
```

### 4. Reward

After a task completes, reward the agent's work:

```bash
wg reward <task-id>
wg reward <task-id> --evaluator-model opus
wg reward <task-id> --dry-run    # preview the evaluator prompt
```

The evaluator scores across four dimensions:

| Dimension | Weight | Description |
|-----------|--------|-------------|
| `correctness` | 40% | Does the output match the desired outcome? |
| `completeness` | 30% | Were all aspects of the task addressed? |
| `efficiency` | 15% | Was work done without unnecessary steps? |
| `style_adherence` | 15% | Were project conventions and objective constraints followed? |

The evaluator receives:
- The task definition (title, description, deliverables)
- The agent's identity (role + objective)
- Task artifacts and log entries
- The reward rubric

It outputs a JSON reward:
```json
{
  "score": 0.85,
  "dimensions": {
    "correctness": 0.9,
    "completeness": 0.85,
    "efficiency": 0.8,
    "style_adherence": 0.75
  },
  "notes": "Implementation is correct and complete. Minor efficiency issue..."
}
```

Scores propagate to three levels:
1. The **agent's** performance record
2. The **role's** performance record (with `objective_id` as context)
3. The **objective's** performance record (with `role_id` as context)

### 5. Evolve

Use performance data to improve the identity:

```bash
wg evolve                                     # full cycle, all strategies
wg evolve --strategy mutation --budget 3      # targeted changes
wg evolve --model opus                        # use specific model
wg evolve --dry-run                           # preview without applying
```

## CLI Reference

### `wg role`

| Command | Description |
|---------|-------------|
| `wg role add <name> --outcome <text> [--skill <spec>] [-d <text>]` | Create a new role |
| `wg role list` | List all roles |
| `wg role show <id>` | Show details |
| `wg role edit <id>` | Edit in `$EDITOR` (re-hashes on save) |
| `wg role rm <id>` | Delete a role |
| `wg role lineage <id>` | Show evolutionary ancestry |

### `wg objective`

Also aliased as `wg mot`.

| Command | Description |
|---------|-------------|
| `wg objective add <name> --accept <text> --reject <text> [-d <text>]` | Create a new objective |
| `wg objective list` | List all objectives |
| `wg objective show <id>` | Show details |
| `wg objective edit <id>` | Edit in `$EDITOR` (re-hashes on save) |
| `wg objective rm <id>` | Delete an objective |
| `wg objective lineage <id>` | Show evolutionary ancestry |

### `wg agent`

| Command | Description |
|---------|-------------|
| `wg agent create <name> [OPTIONS]` | Create an agent (see options below) |
| `wg agent list` | List all agents |
| `wg agent show <id>` | Show details with resolved role/objective |
| `wg agent rm <id>` | Remove an agent |
| `wg agent lineage <id>` | Show agent + role + objective ancestry |
| `wg agent performance <id>` | Show reward history |

**`wg agent create` options:**

| Option | Description |
|--------|-------------|
| `--role <ID>` | Role ID or prefix (required for AI agents) |
| `--objective <ID>` | Objective ID or prefix (required for AI agents) |
| `--capabilities <SKILLS>` | Comma-separated skills for task matching |
| `--rate <FLOAT>` | Hourly rate for cost tracking |
| `--capacity <FLOAT>` | Maximum concurrent task capacity |
| `--trust-level <LEVEL>` | `verified`, `provisional`, or `unknown` |
| `--contact <STRING>` | Contact info (email, Matrix ID, etc.) |
| `--executor <NAME>` | Executor backend: `claude` (default), `matrix`, `email`, `shell` |

### `wg assign`

```bash
wg assign <task-id> <agent-hash>    # assign agent to task
wg assign <task-id> --clear         # remove assignment
```

### `wg reward`

```bash
wg reward <task-id> [--evaluator-model <model>] [--dry-run]
```

### `wg evolve`

```bash
wg evolve [--strategy <name>] [--budget <N>] [--model <model>] [--dry-run]
```

### `wg identity stats`

```bash
wg identity stats [--min-evals <N>]
```

Shows: role leaderboard, objective leaderboard, synergy matrix, tag breakdown, under-explored combinations.

## Skill System

Skills define capabilities attached to a role. Four types of skill references:

### Name (tag-only)

Simple string label. No content, just matching and display.

```bash
wg role add "Coder" --skill rust --skill testing --outcome "Working code"
```

### File

Path to a file containing skill instructions. Supports absolute paths, relative paths, and `~` expansion.

```bash
wg role add "Coder" --skill "coding:file:///home/user/skills/rust-style.md" --outcome "Idiomatic Rust"
```

### Url

URL to fetch skill content from.

```bash
wg role add "Reviewer" --skill "review:https://example.com/checklist.md" --outcome "Review report"
```

### Inline

Skill content embedded directly.

```bash
wg role add "Writer" --skill "tone:inline:Write in a clear, technical style" --outcome "Documentation"
```

### Resolution

When a task is dispatched with an agent identity, all skill references on the role are resolved:
- `Name` → passes through as-is
- `File` → reads file content
- `Url` → fetches URL content
- `Inline` → uses content directly

Skills that fail to resolve produce a warning but don't block execution.

## Evolution

The evolution system improves identity performance by analyzing reward data and proposing changes. It spawns an LLM-powered "evolver agent" that reads performance summaries and proposes structured operations.

### Strategies

| Strategy | Description |
|----------|-------------|
| `mutation` | Modify a single existing role to improve weak dimensions |
| `crossover` | Combine traits from two high-performing roles into a new one |
| `gap-analysis` | Create entirely new roles/objectives for unmet needs |
| `retirement` | Remove consistently poor-performing roles/objectives |
| `objective-tuning` | Adjust trade-offs and constraints on existing objectives |
| `all` | Use all strategies as appropriate (default) |

### Operations

The evolver outputs structured JSON operations:

| Operation | Effect |
|-----------|--------|
| `create_role` | Creates a new role (typically from gap-analysis) |
| `modify_role` | Mutates or crosses over an existing role into a new one |
| `create_objective` | Creates a new objective |
| `modify_objective` | Tunes an existing objective into a new variant |
| `retire_role` | Retires a poor-performing role (renamed to `.yaml.retired`) |
| `retire_objective` | Retires a poor-performing objective |

### Safety guardrails

- The last remaining role or objective cannot be retired
- Retired entities are preserved as `.yaml.retired` files, not deleted
- `--dry-run` shows the full evolver prompt without making changes
- `--budget` limits the number of operations applied

### Evolver identity and meta-agent configuration

The evolver itself can have an agent identity. Configure meta-agents in config.toml:

```toml
[identity]
evolver_model = "opus"           # model for the evolver agent
evolver_agent = "abc123..."      # content-hash of evolver agent identity
assigner_model = "haiku"         # model for assigner agents
assigner_agent = "def456..."     # content-hash of assigner agent identity
evaluator_model = "opus"         # model for evaluator agents
evaluator_agent = "ghi789..."    # content-hash of evaluator agent identity
retention_heuristics = "Retire roles scoring below 0.3 after 10 rewards"
```

Or via CLI:

```bash
wg config --evolver-model opus --evolver-agent abc123
wg config --assigner-model haiku --assigner-agent def456
wg config --evaluator-model opus --evaluator-agent ghi789
wg config --retention-heuristics "Retire roles scoring below 0.3 after 10 rewards"
```

The evolver prompt includes:
- Performance summaries for all roles and objectives
- Strategy-specific skill documents from `.workgraph/identity/evolver-skills/`
- The evolver's own identity (if configured)
- References to the assigner, evaluator, and evolver agent hashes
- Retention heuristics (if configured)

### Evolver skills

Strategy-specific guidance documents live in `.workgraph/identity/evolver-skills/`:

- `role-mutation.md` — procedures for improving a single role
- `role-crossover.md` — procedures for combining two roles
- `gap-analysis.md` — procedures for identifying missing capabilities
- `retirement.md` — procedures for removing underperformers
- `objective-tuning.md` — procedures for adjusting trade-offs

## Performance Tracking

### Reward flow

1. Task completes → reward is created (4 dimensions + overall score)
2. Reward saved as YAML in `.workgraph/identity/rewards/`
3. **Agent's** performance record updated (task count, avg score, eval history)
4. **Role's** performance record updated (with objective_id as `context_id`)
5. **Objective's** performance record updated (with role_id as `context_id`)

### Performance records

Each entity maintains a `RewardHistory`:

```yaml
performance:
  task_count: 5
  mean_reward: 0.82
  rewards:
    - score: 0.85
      task_id: "implement-feature-x"
      timestamp: "2026-01-15T10:30:00Z"
      context_id: "<objective_id>"  # on roles; role_id on objectives
```

The `context_id` cross-references create a performance matrix: how a role performs with different objectives, and vice versa. `wg identity stats` uses this to build a synergy matrix.

### Trend indicators

`wg identity stats` computes trends by comparing first and second halves of recent scores:

- **up** — second half averages >0.03 higher
- **down** — second half averages >0.03 lower
- **flat** — difference within 0.03

## Lineage

Every role, objective, and agent tracks evolutionary history:

```yaml
lineage:
  parent_ids:
    - "a1b2c3d4..."   # single parent for mutation, two for crossover
  generation: 2
  created_by: "evolver-run-20260115-143022"
  created_at: "2026-01-15T14:30:22Z"
```

| Field | Description |
|-------|-------------|
| `parent_ids` | Empty for manual, single for mutation, multiple for crossover |
| `generation` | 0 for manual, incrementing for evolved |
| `created_by` | `"human"` for manual, `"evolver-{run_id}"` for evolved |
| `created_at` | Timestamp |

### Viewing lineage

```bash
wg role lineage <id>
wg objective lineage <id>
wg agent lineage <id>        # shows agent + role + objective ancestry
```

## Storage Layout

```
.workgraph/identity/
├── roles/
│   ├── <sha256>.yaml            # Role definitions
│   └── <sha256>.yaml.retired    # Retired roles
├── objectives/
│   ├── <sha256>.yaml            # Objective definitions
│   └── <sha256>.yaml.retired    # Retired objectives
├── agents/
│   └── <sha256>.yaml            # Agent definitions (role+objective pairs)
├── rewards/
│   └── eval-<task-id>-<timestamp>.yaml  # Reward records
└── evolver-skills/
    ├── role-mutation.md
    ├── role-crossover.md
    ├── gap-analysis.md
    ├── retirement.md
    └── objective-tuning.md
```

Roles, objectives, and agents are stored as YAML. Rewards are stored as YAML. All filenames are based on the entity's content-hash ID.

## Configuration Reference

```toml
[identity]
auto_reward = false              # auto-create reward tasks on completion
auto_assign = false                # auto-create assignment tasks for ready work
assigner_model = "haiku"           # model for assigner agents
evaluator_model = "opus"           # model for evaluator agents
evolver_model = "opus"             # model for evolver agents
assigner_agent = ""                # content-hash of assigner agent
evaluator_agent = ""               # content-hash of evaluator agent
evolver_agent = ""                 # content-hash of evolver agent
retention_heuristics = ""          # prose policy for retirement decisions
```

```bash
# CLI equivalents
wg config --auto-reward true
wg config --auto-assign true
wg config --assigner-model haiku
wg config --evaluator-model opus
wg config --evolver-model opus
wg config --assigner-agent abc123
wg config --evaluator-agent def456
wg config --evolver-agent ghi789
wg config --retention-heuristics "Retire roles scoring below 0.3 after 10 rewards"
```
