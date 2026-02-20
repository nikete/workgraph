# Return Brief for Vaughn — Feb 20, 2026

Welcome back. A lot happened in the last week. This document covers what changed, what to review, and where your skills would have the most impact right now.

## What Changed in the Agent/Identity Concept

### Terminology Alignment (Feb 19)

We forked from upstream and renamed the core abstractions to align with RL/multi-agent systems literature. The goal: make the system legible to three research communities simultaneously (organizational behavior, mechanism design, multi-agent RL).

| Before (upstream) | After (our fork) | Why |
|---|---|---|
| Agency | Identity | "Agency" overloaded — means something different in org theory vs. AI. "Identity" captures the durable aspect: who you are persists across tasks. |
| Evaluation | Reward | Standard RL term. Evaluations are just one *source* of rewards. |
| Motivation | Objective | Less anthropomorphic. Objectives can be formally specified and compared. |
| score | value | RL convention. Also avoids confusion with VX veracity scores. |
| avg_score | mean_reward | Precise statistical language. |
| PerformanceRecord | RewardHistory | Descriptive of what it actually contains. |

The rename is mechanical and complete — every function, struct, CLI command, config key, and test has been updated. Backward compatibility is maintained via `#[serde(alias = "score")]` on deserialization.

### New: Pluggable Reward Sources (Feb 19)

The `Reward` struct now has a `source: String` field. This is the most conceptually significant change. Previously, all evaluations came from the LLM evaluator. Now rewards can come from:

- `"llm"` — LLM evaluator (the default, unchanged behavior)
- `"manual"` — human judgment via `wg reward --value 0.85`
- `"outcome:<metric>"` — deterministic outcome metrics (test pass rate, P&L, MSE)
- `"backward_inference"` — VX-style retroactive scoring
- Custom sources

**Why this matters for your work:** This is the bridge between your uncertainty-aware role design and Nik's incentive architecture. A role's performance history now carries provenance about *how* each reward was generated. An LLM's subjective assessment of "good work" is qualitatively different from a CI pipeline reporting test pass rates. The evolution system can (and should) weight these differently.

**CLI:**
```bash
wg reward my-task --value 0.85 --source "outcome:test-pass-rate"
wg reward my-task --value 0.3 --source manual --notes "Attempted wrong approach"
```

### New: GEPA Backend for Evolution (Feb 19-20)

The `wg evolve` command now supports `--backend gepa` alongside the default Claude backend. GEPA (Generalized Evolutionary Prompt Architecture) replaces single-shot LLM proposals with iterative reflective search — multiple rounds of propose-evaluate-reflect.

The `gepa-rs` crate is a standalone Rust port: https://github.com/nikete/gepa-rs

**What to review:** Does iterative optimization risk premature convergence? The single-shot Claude evolver was "dumb" in a good way — it proposed changes based on a holistic reading of the situation. GEPA is smarter but might optimize toward local optima.

### New: Federation System (Feb 19)

Roles, objectives, and agents can now be shared across projects:

```bash
wg peer add team-b /path/to/their/project
wg identity pull team-b           # import their roles
wg identity push team-b           # share ours
wg identity merge /store-a /store-b  # combine multiple stores
```

Content-hash deduplication means the same role imported from two sources is recognized as identical. Performance histories are merged (union of reward refs, deduped by task_id).

### New: Loop Convergence (Feb 19)

Autopoietic loops (task A produces work evaluated by task B which re-opens task A) can now break early with `wg done --converged`. The `converged` tag prevents loop edges from firing.

### New: Trace Functions (Feb 19)

Completed task traces can be extracted into parameterized YAML templates and instantiated into new task subgraphs. This is pattern reuse — a successful workflow becomes a reusable template.

### New: ObjectiveTuning Strategy (Feb 19-20)

`wg evolve --strategy objective-tuning` adjusts objectives based on accumulated reward data. Previously evolution only mutated roles and ran gap-analysis. Now objectives themselves can evolve.

---

## What to Review

### Priority 1: The Reward Source Semantics

Read `src/identity.rs` lines 203-230 (the `Reward` struct) and `src/commands/reward.rs` lines 188-280 (the manual reward path). The question for you:

**When a role accumulates rewards from heterogeneous sources (LLM judgments, outcome metrics, manual scores), how should the evolution system weight them?** Currently `_summarize_historical()` in the GEPA evaluator treats all rewards equally. Your ethnographic intuition about which signals actually indicate role fit vs. which are noise would be valuable here.

### Priority 2: Evolution Strategy Balance

Read `src/commands/evolve.rs` — the strategy enum and the dispatch logic. The six strategies are: mutation, crossover, gap-analysis, retirement, objective-tuning, all.

**Is the exploration/exploitation balance right?** March (1991) predicts systems will overuse mutation (refining what works) and underuse gap-analysis (discovering what's missing). Does the `all` strategy's equal weighting make sense, or should gap-analysis be privileged?

### Priority 3: Federation and Role Portability

Read `docs/design/agency-federation.md` and `src/federation.rs`.

**When a role is pulled from another project, does it retain its "provisional" character?** Your negotiated-joining finding says roles should be jointly constructed through iteration. But federation treats roles as static artifacts to copy. Should there be a "re-negotiation" step when importing a role?

### Priority 4: Organizational Patterns

Read `docs/research/organizational-patterns.typ` (the updated version with trace column and framework rows). This formalizes the connection between workgraph primitives and organizational theory patterns (stigmergy, viable systems, autopoiesis).

**Does the formalization actually capture what you observed in culinary teams?** The viable systems model row and the autopoietic loop row are the ones most informed by your work.

---

## Where Your Skills Add Most Value Now

### 1. Evolution Philosophy Oversight (Highest Impact)

The system is at a critical juncture. GEPA integration means evolution is becoming *more capable* — it can now do multi-round reflective optimization instead of one-shot proposals. More capability means more risk of the system optimizing away the very uncertainty-tolerance that makes it valuable.

**Concrete task:** Review 3-5 evolution outputs (run `wg evolve --dry-run` on a project with some reward history) and assess: do the proposed mutations look like something that could emerge from negotiated joining, or do they look like mechanical optimization?

### 2. Reward Source Taxonomy Design

The `source` field is a free-form string right now. We need a principled taxonomy of reward sources that maps onto your uncertainty/risk distinction:

- **Uncertainty-appropriate sources:** LLM judgment, peer review, retrospective assessment
- **Risk-appropriate sources:** test pass rates, benchmark scores, deterministic metrics
- **Hybrid:** outcome metrics that have both signal and noise

**Concrete task:** Propose a categorization scheme. Which sources should carry more weight in evolution decisions? Should the evolution system treat a role with 10 outcome-metric rewards differently from one with 10 LLM-judgment rewards?

### 3. Federation as Organizational Knowledge Transfer

Federation is currently "copy roles between projects." But your research on how elite teams actually transfer knowledge suggests this might be too simple. When The Fat Duck sends a cook to Noma, they don't just transfer a job description — there's a re-negotiation process.

**Concrete task:** Design the "import and adapt" workflow. When `wg identity pull` brings in a role from another project, what information should prompt the user to re-evaluate the role's fit? Should imported roles start at generation 0 (fresh) or preserve their lineage?

### 4. Convergence Criteria for Autopoietic Loops

The `--converged` flag is currently binary and manual. Your work on how real teams know when iteration is "done enough" could inform automated convergence detection.

**Concrete task:** What signals from reward history should trigger convergence? Diminishing marginal improvement? Increasing evaluator disagreement? Stable role descriptions across iterations?

### 5. Organizational Patterns Validation

The patterns document maps formal models (viable systems, stigmergy, autopoiesis) to workgraph primitives. This needs ethnographic validation.

**Concrete task:** For each pattern, write a paragraph assessing whether the formalization captures real organizational dynamics you've observed, or whether it's a "tidy abstraction" that misses something important.

---

## Quick Orientation

```bash
cd /path/to/workgraph
git log --oneline -15          # see recent history
wg quickstart                  # orient in any project
cargo test                     # 2,366 tests, all passing
```

Key files to read first:
1. `FORK.md` — why we forked, what changed
2. `docs/IDENTITY.md` — the identity system design (renamed from AGENCY.md)
3. `docs/research/collaborators-and-perspectives.md` — how your work maps to the system
4. `docs/research/organizational-economics-review.md` — literature review connecting to your citations
5. `docs/research/gepa-integration.md` — GEPA integration analysis

The test suite is the best documentation of behavior. For any feature, find the corresponding `tests/integration_*.rs` file.
