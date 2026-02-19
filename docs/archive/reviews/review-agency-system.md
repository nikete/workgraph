# Review: Identity System

**Scope:** ~8,328 lines across 10 files (including tests)
**Date:** 2026-02-11

## 1. Data Model & Entity Relationships

### Core Entities

```
Role (YAML, content-hash ID)
├── id: SHA-256(skills + desired_outcome + description)
├── name, description, desired_outcome
├── skills: Vec<SkillRef>  (Name | File | Url | Inline)
├── performance: RewardHistory
└── lineage: Lineage

Objective (YAML, content-hash ID)
├── id: SHA-256(acceptable_tradeoffs + unacceptable_tradeoffs + description)
├── name, description
├── acceptable_tradeoffs, unacceptable_tradeoffs
├── performance: RewardHistory
└── lineage: Lineage

Agent (YAML, content-hash ID)
├── id: SHA-256(role_id + objective_id)
├── role_id, objective_id
├── name
├── performance: RewardHistory
└── lineage: Lineage

Reward (JSON)
├── id, task_id, agent_id, role_id, objective_id
├── score: f64, dimensions: HashMap<String, f64>
├── notes, evaluator, timestamp
```

### Relationships

```
Agent = Role + Objective  (deterministic pairing)
Task.agent → Agent.id      (optional assignment on task)

Reward references: task_id, agent_id, role_id, objective_id
  - Recording a reward updates performance on all three entities
  - Role.performance.rewards[].context_id = objective_id
  - Objective.performance.rewards[].context_id = role_id

Lineage: parent_ids[], generation, created_by
  - Mutation: 1 parent, generation+1
  - Crossover: 2 parents, max(parent_gens)+1
  - Manual creation: generation=0, created_by="human"
```

### Storage Layout

```
.workgraph/identity/
├── roles/{sha256}.yaml
├── objectives/{sha256}.yaml
├── agents/{sha256}.yaml
├── rewards/eval-{task_id}-{timestamp}.json
└── evolver-skills/*.md
```

### Lifecycle

1. **Seed** — `wg identity init` creates starter roles (Programmer, Reviewer, Documenter, Architect) and objectives (Careful, Fast, Thorough, Balanced)
2. **Create** — `wg role add` / `wg objective add` / `wg agent create`
3. **Assign** — `wg assign <task> <agent-hash>` sets `task.agent`
4. **Execute** — Coordinator resolves Agent → Role + Objective → identity prompt injected into agent spawn
5. **Reward** — `wg reward <task>` spawns LLM evaluator, records Reward, updates performance on role/objective/agent
6. **Evolve** — `wg evolve` spawns LLM evolver that proposes create/modify/retire operations based on performance data
7. **Stats** — `wg identity stats` shows leaderboards, synergy matrix, tag breakdowns, under-explored combinations

## 2. File Size & Complexity Analysis

### Actual Line Counts (code vs tests)

| File | Total | Code | Tests | Test% |
|------|------:|-----:|------:|------:|
| `identity.rs` | 2,346 | 1,327 | 1,019 | 43% |
| `evolve.rs` | 2,677 | 1,121 | 1,556 | 58% |
| `identity_stats.rs` | 675 | 552 | 123 | 18% |
| `agent_crud.rs` | 634 | 495 | 139 | 22% |
| `objective.rs` | 447 | 294 | 153 | 34% |
| `role.rs` | 394 | 349 | 45 | 11% |
| `reward.rs` | 370 | 312 | 58 | 16% |
| `assign.rs` | 310 | 119 | 191 | 62% |
| `skills.rs` | 256 | 163 | 93 | 36% |
| `match_cmd.rs` | 219 | 140 | 79 | 36% |
| **Total** | **8,328** | **4,872** | **3,456** | **41%** |

The high test ratio is good. The actual production code is ~4,900 lines, which is reasonable for the scope.

### Is identity.rs too large?

**Not critically, but it could benefit from splitting.** The 1,327 lines of production code in `identity.rs` contain several distinct concerns:

1. **Type definitions** (lines 1-170) — Role, Objective, Agent, Reward, SkillRef, etc.
2. **Skill resolution** (lines 172-255) — resolve_skill, resolve_all_skills, URL fetching
3. **Prompt rendering** (lines 260-480) — render_identity_prompt, render_evaluator_prompt
4. **Content hashing** (lines 486-558) — content_hash_role/objective/agent
5. **Prefix-matching lookups** (lines 560-606) — find_role_by_prefix, find_objective_by_prefix
6. **Storage I/O** (lines 610-798) — load/save/load_all for roles, objectives, rewards, agents
7. **Lineage queries** (lines 800-875) — ancestry tree walking
8. **Reward recording** (lines 880-968) — record_reward with cross-entity updates
9. **Starter data** (lines 970-1124) — starter_roles, starter_objectives, seed_starters
10. **Task output capture** (lines 1130-1322) — capture_task_output, git diff, artifact manifest

These are logically distinct modules lumped into one file. The file isn't unmanageable, but a split along these boundaries would improve navigability.

**Recommended split if pursuing:**
- `identity/types.rs` — struct definitions
- `identity/storage.rs` — load/save/load_all/find_by_prefix
- `identity/prompt.rs` — render_identity_prompt, render_evaluator_prompt
- `identity/reward.rs` — record_reward, recalculate_mean_reward
- `identity/capture.rs` — capture_task_output and helpers
- `identity/mod.rs` — re-exports, content_hash functions, starters

### Is evolve.rs too complex?

**The production logic (1,121 lines) is not complex, but it's long due to structural repetition.** The file follows a clean pipeline:

1. Load data → build performance summary → build prompt → spawn Claude → parse output → apply operations

The bulk comes from:
- 6 `apply_*` functions that are structurally similar (create/modify/retire × role/objective)
- A large `build_evolver_prompt` that's just string concatenation
- `build_performance_summary` which is also string formatting

The 1,556 lines of tests (58%) are thorough and useful. The production code itself is linear and easy to follow — no deep nesting or complex control flow.

**Verdict:** Leave as-is. The repetition is local and readable. Extracting shared helpers would add indirection without much benefit.

## 3. What's Actually Used vs Theoretical

### Actively Used (integrated into service/coordinator loop)

- **Agent identity resolution** — `render_identity_prompt` is called by the executor when spawning agents with `task.agent` set. This is the core integration point.
- **Agent CRUD** — creating/listing agents pairs them with tasks
- **Assign** — sets `task.agent` field, used by manual assignment and potentially the coordinator's assigner
- **Role/Objective CRUD** — needed to create the entities agents are composed of
- **Reward recording** — updates performance on role/objective/agent

### Likely Underused

- **`wg evolve`** — Requires enough rewards to produce meaningful performance data. Very few projects will accumulate enough data to make the evolver useful. The entire evolution system (evolve.rs, evolver-skills/) is 2,677 lines of speculative infrastructure.
- **`wg identity stats`** — Same issue: needs reward data to show anything interesting. 675 lines.
- **`wg match`** — Uses the `Actor` graph node (different from `Agent`), skill-matching against actors in the graph. This is an older/parallel concept to the identity system. Only 219 lines, low cost.
- **`wg skills list/find`** — Utility for looking at task skills. Lightweight, fine.
- **Lineage tracking** — Generation tracking, ancestry trees, parent_ids. Adds complexity to the data model for a feature (evolutionary history visualization) that only matters after many evolution cycles.
- **Synergy matrix** — O(roles × objectives) analysis. Interesting concept, but only useful with substantial reward data.
- **Task output capture** — `capture_task_output` writes git diffs and artifact manifests for the evaluator. Well-designed but only used when rewards are triggered.

### Two Actor Systems

There are **two parallel identity/capability systems** that don't interact:

1. **`Actor` (graph.rs)** — Stored as graph nodes, has capabilities, trust_level, actor_type. Used by `wg match`.
2. **`Agent` (identity.rs)** — Stored in identity/agents/, composed of Role + Objective, has performance tracking. Used by `wg assign`, identity prompts, rewards.

These serve overlapping purposes and could be confusing. The `Actor` system appears to be an older approach, and `Agent` is the current one.

## 4. Duplication

### extract_json() is duplicated

`extract_json()` appears in both:
- `src/commands/evolve.rs:678` — for parsing evolver output
- `src/commands/reward.rs:279` — for parsing evaluator output

Both implementations are identical. Should be extracted to a shared utility.

### Lineage display code is duplicated

The ancestry tree display code (formatting nodes with generation indents, short parent hashes) is near-identical in:
- `role.rs:run_lineage()`
- `objective.rs:run_lineage()`
- `agent_crud.rs:run_lineage()` (for both role and objective ancestry)

### RewardHistory initialization is repeated everywhere

```rust
RewardHistory {
    task_count: 0,
    mean_reward: None,
    rewards: vec![],
}
```

Appears 20+ times. Should have `impl Default for RewardHistory`.

### find_*_by_prefix pattern

`find_role_by_prefix`, `find_objective_by_prefix`, `find_agent_by_prefix` are structurally identical. Could be a generic function over a trait, but this is mild — the functions are short.

## 5. Specific Issues

### Content-hash IDs create a UX problem

Entity IDs are full SHA-256 hashes (64 hex chars). This means:
- `wg role show a3f7c21d...` requires copying long hashes
- Prefix matching (`find_role_by_prefix`) helps, but 8-char prefixes are still unfriendly
- The `short_hash` display function shows 8 chars, but actual IDs are 64

The tradeoff is content-addressability (same content = same ID, deduplication is free) vs usability. For a CLI tool with typically <20 entities, slug-based IDs (like task IDs) would be much more ergonomic.

### Retirement just renames files

`retire_role`/`retire_objective` renames `.yaml` to `.yaml.retired`. This means:
- Retired entities are invisible to `load_all_*` (good)
- But they accumulate as dead files
- No way to list or unretire them
- No cascading cleanup of agents that reference retired roles/objectives

### Reward recording silently ignores missing entities

In `record_reward()` (identity.rs:943-966), if a role or objective doesn't exist, the function silently skips updating that entity's performance. This could mask data integrity issues — rewards for deleted roles would record successfully but performance would only update on the entities that still exist.

### Reward model is tightly coupled to LLM

Both `wg reward` and `wg evolve` directly shell out to `claude --print --dangerously-skip-permissions`. This means:
- No support for other reward methods (manual scoring, different LLMs)
- `--dangerously-skip-permissions` is hardcoded
- No retry logic on LLM failures

## 6. Recommendations

### High-value, Low-effort

1. **Extract `extract_json()` to a shared utility** — Remove duplication between reward.rs and evolve.rs. ~10 minutes.

2. **Add `Default` impl for `RewardHistory`** — Reduces boilerplate across the codebase. ~5 minutes.

3. **Consolidate lineage display** — Extract a `format_ancestry_tree()` function in identity.rs, call from role/objective/agent_crud lineage commands. ~30 minutes.

### Medium-effort

4. **Clarify Actor vs Agent** — Document which system is canonical. If `Actor` (graph nodes) is legacy, deprecate `wg match` or migrate it to use `Agent`. If both are needed, document when to use which.

5. **Add `wg identity retired list`** — Show retired entities, allow unretire. Low complexity but helps with the retirement UX gap.

### Consider for Simplification

6. **Defer evolution complexity** — The evolve.rs system (2,677 lines) is sophisticated but requires substantial reward data to be useful. Consider feature-gating it or marking it as experimental. The core identity value (identity prompts for agents) works without evolution.

7. **Consider splitting identity.rs** — If the file continues to grow, split along the boundaries described in section 2. Not urgent at current size.

8. **Don't split evolve.rs** — Despite being the largest file, its linear structure and heavy test coverage make it manageable as-is.

## 7. Summary

The identity system is well-designed and well-tested (41% test coverage by line count). The core loop — Role + Objective → Agent → identity prompt → reward → performance tracking — is clean and functional. The main concerns are:

1. **Speculative complexity** — Evolution, synergy matrices, and lineage tracking add ~3,350 lines for features that require accumulated reward data most projects won't have. The ROI is unclear.
2. **Two identity systems** — Actor (graph nodes) and Agent (identity entities) serve overlapping purposes without clear demarcation.
3. **Minor duplication** — extract_json, lineage display, RewardHistory init could be consolidated.
4. **Code-to-test ratio is healthy** — 41% tests indicates good engineering practice. Tests are thorough and cover edge cases.

Overall assessment: **solid foundation, some premature generality in the evolution layer, a few easy wins on deduplication.**
