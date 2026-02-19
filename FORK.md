# Fork: VX-Adapted Workgraph

## Why this fork exists

This fork adapts [graphwork/workgraph](https://github.com/graphwork/workgraph) for use with the
[Veracity Exchange (VX)](https://github.com/nikete/veracity) -- a standalone tool that scores
workflow outputs against real-world outcomes and facilitates peer exchange of improvements.

Two categories of changes were made:

1. **Terminology alignment** -- upstream uses ad-hoc naming ("agency", "motivation", "evaluation",
   "score"). This fork renames these to match established reinforcement learning and multi-agent
   systems literature, making the codebase legible to researchers in those fields and consistent
   with how VX models the same concepts.

2. **Pluggable reward sources** -- upstream hardcodes LLM-judged evaluation as the only reward
   signal. This fork adds a `source` field to the `Reward` struct so that VX (or any external
   system) can contribute outcome-based, manual, or custom reward signals alongside LLM judgments.

All 1798 upstream tests pass. No upstream functionality was removed.

Based on upstream commit `a1fdaa1` (graphwork/workgraph main).

---

## Change 1: Terminology

### `Agency` -> `Identity`

The upstream "agency" module manages agent identities, roles, and behavioral objectives. The
upstream code itself already used "identity" in comments and function names
(`render_identity_prompt`, "identity-defining fields", "evolutionary identity system"). The
term "agency" is overloaded in AI (rational agency, multi-agent systems) and doesn't describe
what this module actually does: maintain an identity registry. Renamed to `identity` throughout.

### `Motivation` -> `Objective`

The upstream `Motivation` struct defines: a description, acceptable tradeoffs, and unacceptable
tradeoffs. In BDI (Belief-Desire-Intention) agent architecture -- the standard framework for
autonomous agents -- this maps to an *objective*: the goal specification that constrains how an
agent makes tradeoff decisions. "Motivation" is a psychology term that doesn't appear in the
multi-agent systems literature.

### `Evaluation` -> `Reward`

The upstream `Evaluation` struct represents the scalar feedback signal returned after an agent
completes a task. In reinforcement learning, this is a *reward*. The upstream struct contains
everything a reward signal has: a numeric value, dimensional breakdown, the identity of the
reward function, and metadata. "Evaluation" is ambiguous -- it could mean the process, the
rubric, or the result. "Reward" is precise and universally understood in RL.

### `Evaluation.score` -> `Reward.value`

The scalar field was renamed from `score` to `value` to avoid confusion with unrelated priority
scores used elsewhere in the codebase (task-matching scores in `match_cmd.rs`, assignment
priority in `next.rs`). In user-facing contexts (CLI flags, display output), the canonical term
is **reward** (e.g. `--below-reward`, `Avg reward: 0.85`). The field name `value` is internal.
Existing JSON files with `"score"` key are accepted via `#[serde(alias = "score")]`.

### `PerformanceRecord` -> `RewardHistory`

The aggregated statistics struct (mean reward, count, individual reward references) is a reward
history -- standard RL terminology for an experience summary.

### `avg_score` -> `mean_reward`

Standard statistical and RL naming for the arithmetic mean of accumulated rewards.

### What was NOT renamed

| Term | Why it stays |
|---|---|
| `evaluator` field and config keys | Identifies *who* computes the reward -- a clear role description in any framework |
| `evolve` / `evolution` | The system genuinely implements evolutionary computation (mutation, crossover, tournament selection). Papers like EvoPrompting and PromptBreeder use identical terminology |
| `Role` | Standard in both organizational and multi-agent systems literature |
| `performance` field on Role/Objective/Agent | Plain English field name; renaming would cascade through too many accessor patterns for minimal clarity gain |
| `score` on `TaskCandidate`, match/priority contexts | Unrelated to rewards -- these are task-matching priority scores |

---

## Change 2: Pluggable reward sources (`source` field)

The `Reward` struct now includes a `source: String` field tracking how each reward was computed.
Upstream only supports LLM-judged evaluation; this fork makes the reward source explicit so that
VX and other external systems can contribute rewards.

```rust
#[serde(default = "default_reward_source")]
pub source: String,
```

The default is `"llm"`, preserving backward compatibility. Supported conventions:

| Source | Meaning |
|---|---|
| `"llm"` | LLM-judged evaluation (upstream default) |
| `"outcome:<metric>"` | Real-world outcome (e.g. `"outcome:sharpe"`, `"outcome:neg_mse"`, `"outcome:brier"`) |
| `"manual"` | Human-assigned reward |
| `"backward_inference"` | FLIP-style backward inference reward |
| Any other string | Custom reward function |

This is the primary integration point for VX: it can write rewards with
`source: "outcome:..."` that the evolution system uses alongside LLM judgments.

---

## File mapping

| Upstream | Fork |
|---|---|
| `src/agency.rs` | `src/identity.rs` |
| `src/commands/agency_init.rs` | `src/commands/identity_init.rs` |
| `src/commands/agency_stats.rs` | `src/commands/identity_stats.rs` |
| `src/commands/motivation.rs` | `src/commands/objective.rs` |
| `src/commands/evaluate.rs` | `src/commands/reward.rs` |
| `tests/integration_agency.rs` | `tests/integration_identity.rs` |
| `tests/integration_agency_edge_cases.rs` | `tests/integration_identity_edge_cases.rs` |
| `tests/evaluation_recording.rs` | `tests/reward_recording.rs` |
| `docs/AGENCY.md` | `docs/IDENTITY.md` |
| `docs/manual/03-agency.typ` | `docs/manual/03-identity.typ` |

## CLI changes

| Upstream | Fork |
|---|---|
| `wg agency init` | `wg identity init` |
| `wg agency stats` | `wg identity stats` |
| `wg motivation add/list/show/edit/rm` | `wg objective add/list/show/edit/rm` |
| `wg mot` (alias) | `wg obj` (alias) |
| `wg evaluate <task>` | `wg reward <task>` |
| `wg replay --below-score` | `wg replay --below-reward` |

## On-disk path changes

| Upstream | Fork |
|---|---|
| `.workgraph/agency/` | `.workgraph/identity/` |
| `.workgraph/identity/motivations/` | `.workgraph/identity/objectives/` |
| `.workgraph/identity/evaluations/` | `.workgraph/identity/rewards/` |

## Naming convention

- **Internal code**: struct fields use concise names (`reward.value`, `history.mean_reward`)
- **User-facing**: CLI flags and display output use the canonical literature term (`--below-reward`, `Avg reward: 0.85`)
- **JSON serialization**: field names match struct fields (`"value"`, `"mean_reward"`), with `#[serde(alias = "score")]` for backward compatibility
