# Brief for Erik — Feb 20, 2026

This covers what the fork changed relative to your upstream, why, and where your graph infrastructure and systems expertise would push things forward most.

## What the Fork Changed

### Terminology Alignment

We renamed the core abstractions to make the system legible to three research communities simultaneously — org theory (Vaughn's world), mechanism design/RL (nikete's world), and systems engineering (your world).

| Your upstream | Fork | Rationale |
|---|---|---|
| Agency | Identity | "Agency" means different things in org theory vs. AI. "Identity" captures what's durable: who you are persists across tasks. |
| Evaluation | Reward | Standard RL term. Also: evaluations are just one *source* of rewards — outcome metrics, manual scores, and VX backward inference are others. |
| Motivation | Objective | Less anthropomorphic, formally specifiable, comparable across agents. |
| score | value | RL convention. Avoids confusion with VX veracity scores. |
| PerformanceRecord | RewardHistory | Descriptive of contents. |
| AgencyStore | IdentityStore | Follows from Agency → Identity. |

The rename is complete across every function, struct, CLI command, config key, test, and doc. Backward compat via `#[serde(alias = "score")]` so existing YAML files still deserialize.

**All 2,366 tests pass.** No functionality was removed. The fork is a strict superset of upstream.

### New: `source` Field on Reward

This is the key architectural addition. The `Reward` struct now has:

```rust
#[serde(default = "default_reward_source")]
pub source: String,  // "llm", "manual", "outcome:<metric>", "backward_inference", custom
```

This opens the door to heterogeneous reward signals — not just LLM judgment, but deterministic outcome metrics, human scores, and VX market-derived values. The evolution system can (eventually) weight these differently based on provenance.

**CLI support:**
```bash
wg reward my-task --value 0.85 --source "outcome:test-pass-rate"
wg reward my-task --value 0.3 --source manual --notes "Wrong approach"
wg reward my-task --value 0.9 --dimensions '{"correctness":0.95,"efficiency":0.85}'
```

The `--value` path skips the LLM evaluator entirely and records directly. This is 96 lines in `reward.rs`.

### New: GEPA Backend for Evolution

`wg evolve --backend gepa` pipes role/objective/reward data as JSON to `python3 -m wg_gepa.evolve` and parses structured output. The standalone crate is at https://github.com/nikete/gepa-rs (Rust port, 1,080 lines, 36 tests).

This replaces single-shot Claude proposals with iterative reflective search — multiple rounds of propose-evaluate-reflect using Pareto frontiers.

### New: Peer Command

`wg peer` is a thin alias over `wg identity remote` — same operations, friendlier name for the federation workflow.

### Upstream Merge Status

As of today, the fork includes everything from your `main` up through commit `22ecc32` (2D graph layout, federation improvements). The merge required adapting all naming in 59 files. All 2,366 tests pass.

---

## What to Review

### 1. The `source` Field Design

`src/identity.rs` lines 203-230. The field is free-form `String` with a default of `"llm"`. Questions for you:

- **Serialization**: Should `source` be an enum instead of a free string? Enums give type safety but lose extensibility. The current design follows the same pattern as `executor: String` on Agent.
- **Indexing**: When reward files accumulate, will we need to query by source? If so, the YAML-per-file storage might need a source-based index or directory structure.
- **Content hash impact**: The `source` field is *not* part of the reward's content hash (rewards don't have content-hash IDs like roles do). Should it be?

### 2. Federation with Mixed Terminology

When pulling from an upstream project that still uses `score`/`evaluations`/`agency`, the `serde(alias)` annotations handle deserialization. But the federation transfer logic in `federation.rs` serializes using our names. This means:

- Pull from upstream → works (alias handles old field names)
- Push to upstream → the YAML will say `value:` not `score:`, `rewards:` not `evaluations:`

Is this acceptable? Should federation preserve the target's terminology when pushing? Or is the alias approach sufficient for interop?

### 3. GEPA Integration Surface

The GEPA backend in `evolve.rs` (lines ~900-1050) serializes roles/objectives/rewards into JSON and pipes to a Python subprocess. This follows your existing `call_gepa_backend()` pattern. But the JSON schema is ad-hoc — there's no formal contract between the Rust and Python sides.

Should we define a shared schema (JSON Schema or protobuf) for the evolve interface? This matters if GEPA evolves independently of workgraph.

---

## Where Your Skills Add Most Value

### 1. Lineage Graph Scaling (Highest Impact)

As GEPA runs multi-iteration evolution, the lineage graph grows fast. Each mutation creates a new entity with a content-hash ID linked to its parent(s). With `--budget 10` across 5 roles, you get 50 new entities per evolution run.

**Your pangenomics experience is directly applicable.** The lineage graph is structurally identical to a haplotype variation graph: each entity is a "variant" identified by content, linked to ancestors. The questions are the same:

- Can `load_roles()` handle thousands of generations without degradation? Currently it reads all YAML files in a directory.
- Should the lineage graph have its own index structure (like a GFA-style graph) rather than being reconstructed from entity metadata?
- When does garbage collection of retired entities matter? Your `wg gc` command handles terminal tasks — should there be a `wg identity gc` for retired roles with no living descendants?

**Concrete suggestion:** Run `wg evolve --strategy all --budget 50` in a loop 20 times and profile. Identify where the lineage graph becomes the bottleneck.

### 2. Content-Hash Correctness Under Evolution

You designed the SHA-256 content-hash identity scheme. The fork preserves it exactly, but the new reward `source` field introduces a subtlety: two rewards for the same task with different sources (LLM vs. outcome) are semantically different signals about the same work. The `record_reward` function updates `mean_reward` by averaging all rewards regardless of source.

**Question for you:** Should `mean_reward` in `RewardHistory` be computed over all sources equally, or should there be per-source aggregation? This is an identity-system design question that interacts with the content-hash scheme — if `RewardHistory` gains per-source breakdowns, the hash of the parent entity changes.

### 3. Federation Protocol Formalization

The federation system you built (scan, pull, push, remote, merge) is powerful but the protocol is implicit in the code. As the system gets used across multiple projects:

- What happens when two projects evolve the same role independently and then try to merge? The current `merge_performance` deduplicates by task_id, but what about conflicting role descriptions?
- Should there be a federation version/epoch to detect divergent evolution?
- Your experience with distributed genome data (PGGB, seqwish) dealing with multiple assemblies of the same biological structure maps directly here.

### 4. The `forecast.rs` Stack Overflow

The DAG assumptions survey found that `find_longest_path_from()` at `forecast.rs:370-408` has no visited set and will stack overflow on any cycle. With loop edges now in the system, this is a live bug. A quick fix: add a `HashSet<&str>` visited guard, same pattern as `reward_loop_edges` in `graph.rs`.

### 5. Executor Generalization

The amplifier integration proposal identifies a clean refactor: add a `prompt_mode` field to the executor config that decouples how prompts are delivered (stdin, CLI arg, file, API) from which executor runs them. This would make it trivial to add new executors without touching the dispatch logic. Your original executor design in `service/executor.rs` is the right place to do this — it's ~94 lines of change.

### 6. Trace Function Robustness

The trace function system you built (extract → parameterize → instantiate) is impressive but the parameter extraction in `trace_extract.rs` relies on heuristic pattern matching. With your graph algorithms background:

- Could trace extraction be formalized as a graph homomorphism problem? A trace is a subgraph of the completed work graph; a trace function is the quotient graph with parameters at the equivalence class boundaries.
- This would make trace functions composable — instantiate one trace function inside another's parameter slot.

---

## Quick Orientation to the Fork

```bash
git remote -v                    # origin = nikete/workgraph, upstream = graphwork/workgraph
git log --oneline -10            # recent fork history
cargo test                       # 2,366 tests
```

Key fork-specific files:
- `FORK.md` — why we forked, what changed
- `src/identity.rs:203-230` — the Reward struct with `source` field
- `src/commands/reward.rs:188-280` — manual reward injection path
- `src/commands/evolve.rs:900-1050` — GEPA backend
- `src/gepa.rs` — 6-line re-export of gepa-rs crate
- `tests/integration_fork_features.rs` — 20 tests for fork-specific features
- `docs/research/collaborators-and-perspectives.md` — the three-person design mapping
