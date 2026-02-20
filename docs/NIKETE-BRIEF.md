# Self-Brief for nikete — Feb 20, 2026

State of your contributions, what's unfinished, and where your mechanism design / ML / economics skills should focus next.

## Current State of Your Work

### Completed

1. **Terminology alignment** — Full rename across 107 files, 2,366 tests passing. The system now speaks RL/MAS language: identity, reward, objective, value, mean_reward. Backward compat via serde aliases.

2. **Pluggable reward sources** — `source: String` on Reward, `default_reward_source()` → `"llm"`, manual injection via `wg reward --value`. The data model is ready for heterogeneous reward signals.

3. **GEPA-rs crate** — Standalone Rust port at github.com/nikete/gepa-rs (1,080 lines, 36 tests). Integrated into workgraph via `--backend gepa` on evolve.

4. **Upstream merge** — Fork is current with Erik's upstream through commit `22ecc32`. Federation, loop convergence, trace functions, 2D layout — all ported with terminology adaptation.

5. **Integration tests** — 20 new tests for fork-specific features (peer command, manual rewards, evolve strategies, GEPA backend validation).

6. **Research documents** — Collaborators mapping, organizational economics review (766 lines connecting to 13 foundational results), GEPA integration analysis.

### Unfinished

These are listed in priority order based on your unique skills.

#### 1. Distillation LLM Call (Replay Pipeline)

The capture → distill → replay pipeline is 2/3 complete. The distill prompt builder works, `--dry-run` shows the prompt, but the actual LLM call prints "LLM integration not yet implemented." Wiring this up following the evaluator pattern in `reward.rs` (spawn Claude with `--print`, parse structured output) is straightforward — maybe 40 lines.

**Status:** The `TraceEvent` enum, stream-json parser, canon system, and run management all work. 52 tests pass. Just the LLM bridge is missing.

#### 2. VX Design Document Recovery

The original `docs/design-veracity-exchange.md` (737 lines with concrete Rust struct definitions) was lost when the fork repo was deleted. The concepts survive across `veracity-exchange-deep-dive.md` and `veracity-exchange-integration.md`, but the typed implementation plan needs reconstruction.

**What was in it:** Outcome recording structs, the scoring market protocol, backward inference mechanics, the trust/reputation system, and the bridge from VX scores to workgraph Reward records.

#### 3. `parse_stream_json()` Timestamp Bug

All trace events parsed by `parse_stream_json()` get the same timestamp (the parse-time timestamp, not the event's original timestamp). This means replay ordering is wrong for any trace captured in real-time. The fix: extract the timestamp from the JSON event's own `timestamp` field if present.

#### 4. Duplicated Functions

`load_eval_scores()` and `collect_transitive_dependents()` are duplicated between modules. These should be consolidated into shared utilities.

---

## Where Your Mechanism Design Skills Matter Most

### 1. Reward Source Weighting in Evolution (Highest Priority)

The `source` field exists but the evolution system treats all rewards equally. This is the immediate gap where your expertise is irreplaceable.

**The problem:** An LLM evaluator saying "0.8" and a CI pipeline reporting "0.8 test pass rate" carry fundamentally different information. The LLM judgment is subjective, potentially biased toward surface quality, and not incentive-compatible (the evaluator has no stake in accuracy). The outcome metric is objective but narrow — passing tests doesn't mean the code is well-designed.

**What mechanism design says:**
- Proper scoring rules (Brier, log) guarantee that an honest reporter maximizes expected payoff. LLM evaluators aren't strategic agents, but they have systematic biases that function like dishonest reporting.
- The VX design uses outcome scoring as the "ground truth" anchor. But outcome metrics themselves can be gamed (Goodhart's Law). The defense is diversity of outcome metrics — harder to game many metrics simultaneously.
- Holmstrom (1979) multi-task model: when agents have multiple dimensions of effort, rewarding only measurable dimensions distorts effort away from unmeasurable ones. This is exactly the `dimensions: HashMap<String, f64>` field — which dimensions get measured affects which work gets done.

**Concrete next step:** Implement `_summarize_historical()` in the GEPA evaluator to weight rewards by source. Proposal:
- `"outcome:*"` sources get weight 1.0 (ground truth)
- `"llm"` sources get weight 0.7 (informative but biased)
- `"manual"` sources get weight 0.5 (sparse, potentially inconsistent)
- Unknown sources get weight 0.3

Then validate: does this weighting improve evolution quality compared to equal weights? This is an empirical question with a formal framework behind it.

### 2. VX Outcome Scoring Implementation

The `source: "outcome:brier"` convention exists in the data model but there's no implementation that computes Brier scores against realized outcomes. Building this requires:

- An `wg outcome record <task> --metric brier --predicted 0.8 --realized true` command
- A bridge that converts outcome records into Reward records with `source: "outcome:brier"`
- Proper scoring rule validation: does the scoring function satisfy the incentive compatibility condition?

**Which task domains admit proper scoring?** This is the theoretical question you're uniquely positioned to answer. Binary outcomes (did the code pass CI?) are straightforward — Brier/log scores apply. Continuous outcomes (how much did latency improve?) need different rules. Multi-dimensional outcomes (correctness AND efficiency AND maintainability) may not admit a single proper scoring rule at all — this is where your PhD work on multi-attribute mechanism design matters.

### 3. Exploration Budget Mechanism

March (1991) predicts that adaptive systems naturally shift toward exploitation (refining what works) at the expense of exploration (discovering what's missing). The organizational economics review you wrote identifies this as a core risk.

**Current state:** The `all` strategy runs all six evolution strategies with equal weight. There's no mechanism to detect or correct exploration underinvestment.

**Your contribution:** Design an exploration budget. Options:
- **Minimum gap-analysis frequency**: Every Nth evolution run must include gap-analysis regardless of other strategy performance.
- **Diversity threshold**: If the role population's pairwise semantic distance falls below a threshold, force exploration.
- **Regret-based switching**: Treat strategy selection as a multi-armed bandit. Use EXP3 or similar to balance exploitation (mutation/crossover/retirement, which refine existing entities) with exploration (gap-analysis, which creates new ones).

The multi-armed bandit framing is exactly your wheelhouse — you've published on this.

### 4. Incentive Compatibility Audit of the Evolution System

The evolution system has a subtle incentive problem: the evolver agent proposes changes to roles, and the evaluator agent scores the resulting work. If both are the same LLM (which they are, by default), there's a self-reinforcing loop — the evolver learns to propose changes that make the evaluator happy, not changes that improve real outcomes.

**This is the LLM-on-LLM evaluation blind spot** you identified in the VX deep dive. The structural fix is outcome-based rewards (VX), but until VX is built, what intermediate measures help?

Options:
- Use different models for evolving and evaluating (already supported via `evaluator_model` config)
- Add noise to evaluation to prevent the evolver from overfitting to evaluator preferences
- Weight outcome-source rewards higher in evolution decisions (see point 1 above)

### 5. Latent Payoff Handling

Many task outcomes only become observable much later — code quality reveals itself over months of maintenance, not at completion time. The VX design calls these "latent payoffs." Your FORK.md mentions `"backward_inference"` as a reward source convention.

**The mechanism design question:** How should the evolution system handle rewards that arrive late? If a role was retired between task completion and outcome observation, should the reward still update the (now-retired) role's history? Should it propagate to the role's descendants via lineage?

This is a discounting/attribution problem with formal solutions in the repeated games literature. Your contribution: design the reward propagation rule for latent payoffs and implement it as a `wg reward --backfill` mode.

---

## Practical Next Steps (ordered by impact / effort)

| # | Task | Effort | Impact | Why You |
|---|---|---|---|---|
| 1 | Wire distill LLM call | ~40 lines | Completes replay pipeline | Follows your existing code patterns |
| 2 | Implement reward source weighting in GEPA evaluator | ~80 lines | Evolution quality improvement | Mechanism design expertise |
| 3 | Reconstruct VX design doc | ~500 lines | Strategic clarity for all three collaborators | You're the only one who had the original |
| 4 | `wg outcome record` command | ~200 lines | First concrete VX component | Proper scoring rules are your domain |
| 5 | Exploration budget (MAB-based strategy selection) | ~150 lines | Prevents exploitation drift | Published on bandits |
| 6 | Fix timestamp bug in `parse_stream_json()` | ~10 lines | Replay correctness | Quick win, you wrote the parser |
| 7 | Latent payoff propagation design | Design doc | Enables delayed outcome rewards | Repeated games literature |

---

## Key Files You Own

```
src/commands/reward.rs:188-280    # manual reward path (yours)
src/gepa.rs                       # GEPA re-export (yours)
src/commands/evolve.rs:900-1050   # GEPA backend integration (yours)
FORK.md                           # fork rationale (yours)
tests/integration_fork_features.rs # fork-specific tests (yours)
docs/research/collaborators-and-perspectives.md
docs/research/organizational-economics-review.md
docs/research/gepa-integration.md
docs/research/veracity-exchange-deep-dive.md
docs/research/veracity-exchange-integration.md
```

## The Question Only You Can Answer

> When the evolution system accumulates rewards from heterogeneous sources — LLM judgment, outcome metrics, manual scores, VX backward inference — under what conditions does the resulting selection signal remain incentive-compatible? When does Goodhart's Law bite, and what's the structural defense?

This is the mechanism design question at the heart of the project. Everything else is engineering in service of getting this right.
