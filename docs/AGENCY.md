# Agency System

The agency system gives workgraph the ability to assign **composable identities** to AI agents. Instead of every agent being a generic assistant, the agency system lets you define *what* an agent does (its **role**) and *why* it acts the way it does (its **motivation**), then pair them into a reusable **agent** that can be assigned to tasks, evaluated, and evolved over time.

## Core Concepts

The agency system is built on three composable primitives:

### Role

A role defines **what** an agent does. It captures capabilities, skills, and the desired outcome of the agent's work.

| Field             | Description                                         | Identity-defining? |
|-------------------|-----------------------------------------------------|--------------------|
| `name`            | Human-readable label (e.g. "Programmer")            | No                 |
| `description`     | What this role is about                             | Yes                |
| `skills`          | List of skill references (see [Skill System](#skill-system)) | Yes       |
| `desired_outcome` | What good output looks like                         | Yes                |
| `performance`     | Aggregated evaluation scores                        | No (mutable)       |
| `lineage`         | Evolutionary history                                | No (mutable)       |

### Motivation

A motivation defines **why** an agent acts the way it does — its priorities, acceptable trade-offs, and hard constraints.

| Field                    | Description                                      | Identity-defining? |
|--------------------------|--------------------------------------------------|--------------------|
| `name`                   | Human-readable label (e.g. "Careful")            | No                 |
| `description`            | What this motivation prioritizes                 | Yes                |
| `acceptable_tradeoffs`   | Compromises the agent may make                   | Yes                |
| `unacceptable_tradeoffs` | Hard constraints the agent must never violate    | Yes                |
| `performance`            | Aggregated evaluation scores                     | No (mutable)       |
| `lineage`                | Evolutionary history                             | No (mutable)       |

### Agent

An agent is a **named pairing** of exactly one role and one motivation. It is the unit that gets assigned to tasks and tracked over time.

| Field            | Description                                |
|------------------|--------------------------------------------|
| `name`           | Human-readable label                       |
| `role_id`        | Content-hash ID of the role                |
| `motivation_id`  | Content-hash ID of the motivation          |
| `performance`    | Agent-level aggregated evaluation scores   |
| `lineage`        | Evolutionary history                       |

The separation matters: the same role can be paired with different motivations to produce agents that do the same *thing* but with different *priorities*. A "Programmer" role paired with a "Careful" motivation produces a different agent than when paired with a "Fast" motivation.

## Content-Hash IDs

Every role, motivation, and agent is identified by a **SHA-256 content hash** of its identity-defining fields. This has several important properties:

- **Deterministic:** The same content always produces the same ID.
- **Deduplication:** You cannot create two entities with identical content — the hash collision is detected.
- **Immutability of identity:** Changing an identity-defining field (e.g. a role's `desired_outcome`) produces a *new* entity with a *new* ID. The old entity remains unchanged.

### What gets hashed

| Entity     | Hashed fields                                                  |
|------------|----------------------------------------------------------------|
| Role       | `skills` + `desired_outcome` + `description`                  |
| Motivation | `acceptable_tradeoffs` + `unacceptable_tradeoffs` + `description` |
| Agent      | `role_id` + `motivation_id`                                    |

Fields like `name`, `performance`, and `lineage` are **mutable** — changing them does not change the entity's ID.

### Short hashes

For display purposes, IDs are shown as 8-character prefixes (e.g. `a3f7c21d`). All commands that accept an ID also accept a unique prefix.

## Lifecycle

### 1. Create roles and motivations

Define the building blocks of agent identity:

```bash
# Create a role
wg role add "Programmer" \
  --outcome "Working, tested code" \
  --skill code-writing \
  --skill testing \
  --description "Writes, tests, and debugs code"

# Create a motivation
wg motivation add "Careful" \
  --accept "Slow" \
  --accept "Verbose" \
  --reject "Unreliable" \
  --reject "Untested" \
  --description "Prioritizes reliability and correctness above speed"
```

Alternatively, seed the built-in starters:

```bash
wg agency init
```

This creates four starter roles (Programmer, Reviewer, Documenter, Architect) and four starter motivations (Careful, Fast, Thorough, Balanced).

### 2. Pair into agents

Combine a role and motivation into a named agent:

```bash
wg agent create "Careful Programmer" --role <role-hash> --motivation <motivation-hash>
```

### 3. Assign to tasks

Bind an agent to a specific task in the graph:

```bash
wg assign <task-id> <agent-hash>
```

When the service dispatches that task, the agent's role and motivation are injected into the prompt as an identity section covering skills, desired outcome, acceptable trade-offs, and hard constraints.

### 4. Evaluate

After a task completes, evaluations score the agent's work across four dimensions:

- **correctness** (40%) — Does the output match the desired outcome?
- **completeness** (30%) — Were all aspects of the task addressed?
- **efficiency** (15%) — Was work done without unnecessary steps?
- **style_adherence** (15%) — Were project conventions and motivation constraints followed?

Evaluations are recorded as JSON files in `.workgraph/agency/evaluations/` and the scores propagate to the agent, role, and motivation performance records.

### 5. Evolve

Use performance data to improve the agency over time:

```bash
wg evolve [--strategy <strategy>] [--budget <N>] [--dry-run]
```

See [Evolution](#evolution) for details.

## CLI Reference

### `wg role`

Manage roles — the "what" of agent identity.

| Command                     | Description                                         |
|-----------------------------|-----------------------------------------------------|
| `wg role add <name>`        | Create a new role                                   |
| `wg role list [--json]`     | List all roles                                      |
| `wg role show <id> [--json]`| Show details of a role                              |
| `wg role edit <id>`         | Edit a role in `$EDITOR` (re-hashes on save)        |
| `wg role rm <id>`           | Delete a role                                       |
| `wg role lineage <id> [--json]` | Show evolutionary ancestry of a role            |

**`wg role add` options:**

```
--outcome <text>       Desired outcome (required)
--skill <spec>         Skill reference (repeatable; see Skill System)
--description <text>   Role description
```

**Example:**

```bash
$ wg role add "Security Auditor" \
    --outcome "Security findings report with severity ratings" \
    --skill security-audit \
    --skill "review:https://example.com/security-checklist.md" \
    --description "Reviews code for security vulnerabilities"
Created role 'Security Auditor' (b2e4f1a9) at .workgraph/agency/roles/b2e4f1a9...yaml
```

### `wg motivation`

Manage motivations — the "why" of agent identity.

| Command                            | Description                                    |
|------------------------------------|------------------------------------------------|
| `wg motivation add <name>`         | Create a new motivation                        |
| `wg motivation list [--json]`      | List all motivations                           |
| `wg motivation show <id> [--json]` | Show details of a motivation                   |
| `wg motivation edit <id>`          | Edit a motivation in `$EDITOR` (re-hashes on save) |
| `wg motivation rm <id>`            | Delete a motivation                            |
| `wg motivation lineage <id> [--json]` | Show evolutionary ancestry of a motivation  |

**`wg motivation add` options:**

```
--accept <text>        Acceptable trade-off (repeatable)
--reject <text>        Unacceptable trade-off / hard constraint (repeatable)
--description <text>   Motivation description
```

**Example:**

```bash
$ wg motivation add "Quality First" \
    --accept "Slower delivery" \
    --accept "More verbose output" \
    --reject "Skipping tests" \
    --reject "Ignoring edge cases" \
    --description "Prioritise correctness over speed"
Created motivation: Quality First (c7d3e8f2)
```

### `wg agent`

Manage agents — named role+motivation pairings.

| Command                              | Description                                    |
|--------------------------------------|------------------------------------------------|
| `wg agent create <name>`            | Create a new agent from a role+motivation pair |
| `wg agent list [--json]`            | List all agents                                |
| `wg agent show <id> [--json]`       | Show agent details including role/motivation   |
| `wg agent rm <id>`                  | Delete an agent                                |
| `wg agent lineage <id> [--json]`    | Show agent + role + motivation ancestry        |
| `wg agent performance <id> [--json]`| Show evaluation history for an agent           |

**`wg agent create` options:**

```
--role <hash>          Role ID or prefix (required)
--motivation <hash>    Motivation ID or prefix (required)
```

**Example:**

```bash
$ wg agent create "Careful Coder" --role b2e4f1a9 --motivation c7d3e8f2
Created agent 'Careful Coder' (1a2b3c4d) at .workgraph/agency/agents/1a2b3c4d...yaml
  role:       Security Auditor (b2e4f1a9)
  motivation: Quality First (c7d3e8f2)
```

### `wg assign`

Assign an agent to a task or clear the assignment.

```bash
wg assign <task-id> <agent-hash>    # Assign agent to task
wg assign <task-id> --clear         # Remove assignment
```

### `wg evolve`

Trigger an evolution cycle to improve roles and motivations based on performance data.

```bash
wg evolve [--strategy <strategy>] [--budget <N>] [--model <model>] [--dry-run] [--json]
```

**Options:**

| Option               | Description                                              |
|----------------------|----------------------------------------------------------|
| `--strategy <name>`  | Evolution strategy (default: `all`)                      |
| `--budget <N>`       | Maximum number of operations to apply                    |
| `--model <model>`    | LLM model for the evolver agent                          |
| `--dry-run`          | Show the evolver prompt without executing                |
| `--json`             | Output results as JSON                                   |

See [Evolution](#evolution) for strategy details.

### `wg agency stats`

Display aggregated performance statistics across the entire agency.

```bash
wg agency stats [--json] [--min-evals <N>]
```

**Output includes:**

- Overall counts (roles, motivations, evaluations, average score)
- Role leaderboard (sorted by average score, with trend indicators)
- Motivation leaderboard (sorted by average score, with trend indicators)
- Synergy matrix (average scores for each role x motivation pair)
- Tag breakdown (scores by role x task-tag and motivation x task-tag)
- Under-explored combinations (role x motivation pairs with fewer than `--min-evals` evaluations)

## Skill System

Skills define the capabilities attached to a role. There are four types of skill references:

### `Name` (tag-only)

A simple string label. Acts as a tag for matching and display, with no associated content.

```bash
wg role add "Coder" --skill rust --skill testing --outcome "Working code"
```

### `File`

A path to a file containing skill instructions. Supports absolute paths, relative paths (resolved from the project root), and `~` expansion.

```bash
wg role add "Coder" --skill "coding:file:///home/user/skills/rust-style.md" --outcome "Idiomatic Rust"
```

### `Url`

A URL to fetch skill content from (requires the `matrix-lite` feature).

```bash
wg role add "Reviewer" --skill "review:https://example.com/review-checklist.md" --outcome "Review report"
```

### `Inline`

Skill content embedded directly in the specification.

```bash
wg role add "Writer" --skill "tone:inline:Write in a clear, technical style" --outcome "Documentation"
```

### Resolution

When a task is dispatched with an agent identity, all skill references on the role are resolved:

- `Name` references pass through as-is
- `File` references read the file content
- `Url` references fetch the URL content
- `Inline` references use the content directly

Resolved skills are injected into the agent's prompt. Skills that fail to resolve produce a warning but do not block task execution.

## Evolution

The evolution system improves agency performance by analyzing evaluation data and proposing changes to roles and motivations. It works by spawning an LLM-powered "evolver agent" that reads performance summaries and proposes structured operations.

### Strategies

| Strategy              | Description                                                    |
|-----------------------|----------------------------------------------------------------|
| `mutation`            | Modify a single existing role to improve weak dimensions       |
| `crossover`           | Combine traits from two high-performing roles into a new one   |
| `gap-analysis`        | Create entirely new roles/motivations for unmet needs          |
| `retirement`          | Remove consistently poor-performing roles/motivations          |
| `motivation-tuning`   | Adjust trade-offs and constraints on existing motivations      |
| `all`                 | Use all strategies as appropriate (default)                    |

### Operation types

The evolver produces structured JSON with these operations:

| Operation              | Effect                                                         |
|------------------------|----------------------------------------------------------------|
| `create_role`          | Creates a brand-new role (typically from gap-analysis)         |
| `modify_role`          | Mutates or crosses over an existing role into a new one        |
| `create_motivation`    | Creates a new motivation                                       |
| `modify_motivation`    | Tunes an existing motivation into a new variant                |
| `retire_role`          | Retires a poor-performing role (renames `.yaml` to `.yaml.retired`) |
| `retire_motivation`    | Retires a poor-performing motivation                           |

### Safety guardrails

- The last remaining role or motivation cannot be retired
- Retired entities are preserved as `.yaml.retired` files, not deleted
- `--dry-run` shows the full evolver prompt without making changes
- `--budget` limits the number of operations applied

### Evolver skills

The evolver agent loads strategy-specific skill documents from `.workgraph/agency/evolver-skills/`:

- `role-mutation.md`
- `role-crossover.md`
- `gap-analysis.md`
- `retirement.md`
- `motivation-tuning.md`

These documents provide detailed procedures and guidelines for each strategy.

### Example

```bash
# See what the evolver would propose
wg evolve --dry-run

# Run a mutation-only evolution with at most 3 changes
wg evolve --strategy mutation --budget 3

# Run a full evolution cycle
wg evolve
```

## Performance Tracking

Evaluations create a three-level performance record:

### Evaluation flow

1. A task completes and an evaluation is created (scoring correctness, completeness, efficiency, style_adherence)
2. The evaluation is saved as JSON in `.workgraph/agency/evaluations/`
3. The **agent's** performance record is updated (task count, average score, evaluation history)
4. The **role's** performance record is updated, with the motivation ID stored as `context_id`
5. The **motivation's** performance record is updated, with the role ID stored as `context_id`

### Performance records

Each entity (role, motivation, agent) maintains a `PerformanceRecord`:

```yaml
performance:
  task_count: 5
  avg_score: 0.82
  evaluations:
    - score: 0.85
      task_id: "implement-feature-x"
      timestamp: "2025-01-15T10:30:00Z"
      context_id: "<motivation_id>"  # on roles; role_id on motivations
```

The `context_id` cross-references create a performance matrix: you can see how a role performs with different motivations, and vice versa. The `wg agency stats` command uses this to build a synergy matrix showing which role+motivation combinations perform best.

### Trend indicators

The stats command computes trend indicators by comparing the first and second halves of recent scores:

- **up** — second half averages >0.03 higher than first half
- **down** — second half averages >0.03 lower
- **flat** — difference within 0.03

## Lineage

Every role, motivation, and agent tracks its evolutionary history through the `lineage` field:

```yaml
lineage:
  parent_ids:
    - "a1b2c3d4..."   # mutation: single parent
    # or two parents for crossover
  generation: 2
  created_by: "evolver-run-20250115-143022"
  created_at: "2025-01-15T14:30:22Z"
```

### Fields

| Field        | Description                                                    |
|--------------|----------------------------------------------------------------|
| `parent_ids` | Empty for manually created entities. Single parent for mutations, multiple for crossover. |
| `generation` | 0 for manually created, incrementing for evolved entities      |
| `created_by` | `"human"` for manually created, `"evolver-{run_id}"` for evolved |
| `created_at` | Timestamp of creation                                          |

### Ancestry queries

The `lineage` subcommand walks the parent chain to reconstruct the full evolutionary tree:

```bash
# Role ancestry
wg role lineage <role-id>

# Motivation ancestry
wg motivation lineage <motivation-id>

# Agent ancestry (shows agent + role ancestry + motivation ancestry)
wg agent lineage <agent-id>
```

Example output:

```
Lineage for role: b2e4f1a9 (Security Auditor v3)

b2e4f1a9 (Security Auditor v3) [gen 2] created by: evolver-run-20250115 <- [c3d4e5f6]
  c3d4e5f6 (Security Auditor v2) [gen 1] created by: evolver-run-20250110 <- [d4e5f6a7]
    d4e5f6a7 (Security Auditor) [gen 0 (root)] created by: human
```

## Storage Layout

All agency data lives under `.workgraph/agency/`:

```
.workgraph/agency/
  roles/
    <sha256-hash>.yaml           # Role definitions
    <sha256-hash>.yaml.retired   # Retired roles
  motivations/
    <sha256-hash>.yaml           # Motivation definitions
    <sha256-hash>.yaml.retired   # Retired motivations
  agents/
    <sha256-hash>.yaml           # Agent definitions
  evaluations/
    eval-<task-id>-<timestamp>.json  # Evaluation records
  evolver-skills/
    role-mutation.md             # Strategy-specific guidance
    role-crossover.md
    gap-analysis.md
    retirement.md
    motivation-tuning.md
```

Roles, motivations, and agents are stored as YAML. Evaluations are stored as JSON. All filenames are based on the entity's content-hash ID.
