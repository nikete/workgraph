# GEPA x Workgraph: Integration Points

[GEPA](https://gepa-ai.github.io/gepa/) (Genetic-Pareto, arxiv 2507.19457)
is a reflective prompt evolution framework that optimizes any text artifact
given a measurable evaluator. Its core loop: sample trajectories, reflect on
failures in natural language, propose targeted mutations, maintain a Pareto
frontier of attempts.

Workgraph (wg) manages a population of text-based agent identities (roles +
objectives) that are assigned to tasks, rewarded after completion, and evolved.

The two systems share deep structural parallels but occupy different levels
of the stack. GEPA optimizes a *single artifact* through iterative reflection.
Workgraph orchestrates a *population of artifacts* through task assignment,
evaluation, and evolutionary selection. The integration is GEPA as the inner
optimization loop inside workgraph's outer evolutionary loop.

---

## 1. Role descriptions as GEPA optimization targets

A wg `Role` is a text artifact: `description + skills + desired_outcome`. It
is already the kind of thing GEPA optimizes.

```python
import gepa.optimize_anything as oa

def evaluate_role(role_description: str) -> tuple[float, dict]:
    """Deploy a role in wg, run N tasks, return mean reward + diagnostics."""
    # 1. Write role_description to a temp YAML
    # 2. Create agent with this role + a fixed objective
    # 3. Assign to K ready tasks, let coordinator dispatch
    # 4. Wait for completion, collect rewards
    # 5. Return (mean_reward, {per_task_scores, failure_logs, artifacts})
    ...

result = oa.optimize_anything(
    seed_candidate=current_role.description,
    evaluator=evaluate_role,
    objective="Optimize this agent role description to maximize task performance",
    background=f"The role is paired with objective: {objective.description}",
    config=gepa.GEPAConfig(max_metric_calls=50),
)
```

This replaces wg's current single-shot evolver LLM call with GEPA's
multi-iteration reflective search. The evolver currently proposes one mutation
per `wg evolve` run. GEPA would explore dozens of variations, keeping the
Pareto-optimal ones.

**What wg provides that GEPA needs:** The evaluator function (task dispatch +
reward collection), the seed candidate (current role description), and the
background context (objective constraints, performance history).

**What GEPA provides that wg needs:** Multi-iteration reflective optimization
with ASI-driven mutations, rather than a single-shot LLM proposal.

---

## 2. Actionable Side Information = wg artifacts + logs

GEPA's key innovation is **ASI** -- diagnostic feedback that tells the
reflective LLM *why* a candidate failed, not just that it did. This is the
"text-optimization analogue of the gradient."

wg already produces rich ASI for every task:

| wg data | GEPA ASI role |
|---|---|
| `task.artifacts` (file paths) | Output traces -- what the agent produced |
| `task.log` entries | Execution traces -- what happened during the task |
| `reward.dimensions` | Per-dimension scores -- where specifically it fell short |
| `reward.notes` | Evaluator's natural-language diagnosis |
| `verify` field on task | Ground-truth checking criteria |
| VX `outcome:*` rewards | Real-world outcome data |

The evaluator wrapper would package all of this into GEPA's ASI format:

```python
def evaluate_role(role_description: str) -> tuple[float, dict]:
    # ... run tasks ...
    return mean_reward, {
        "per_task": [
            {
                "task": task.title,
                "score": reward.value,
                "correctness": reward.dimensions.get("correctness"),
                "efficiency": reward.dimensions.get("efficiency"),
                "notes": reward.notes,
                "log_tail": task.log[-5:],  # last 5 log entries
            }
            for task, reward in results
        ],
        "worst_dimension": min(avg_dimensions, key=avg_dimensions.get),
    }
```

The reflective LLM reads this and knows: "this role description scores 0.9 on
correctness but 0.5 on efficiency because log entries show the agent
over-generates boilerplate." That is a targeted mutation signal.

---

## 3. Pareto frontier = multi-dimensional rewards without collapse

wg currently collapses four reward dimensions to a weighted average:
`value = 0.4*correctness + 0.3*completeness + 0.15*efficiency + 0.15*style_adherence`

This is exactly the kind of collapse GEPA is designed to avoid. GEPA maintains
a **Pareto frontier**: any candidate that is best at *something* survives,
even if its average is lower.

Integration: instead of one `float` return from the evaluator, return
per-dimension scores:

```python
def evaluate_role(role_description: str) -> tuple[float, dict]:
    return composite_score, {
        "scores": {
            "correctness": avg_correctness,
            "completeness": avg_completeness,
            "efficiency": avg_efficiency,
            "style_adherence": avg_style,
        }
    }
```

GEPA would then maintain a Pareto frontier of role descriptions -- one
optimized for correctness, another for efficiency, a third balancing both.
The wg evolution system could then create multiple new roles from the frontier,
each filling a different niche in the identity space.

This directly addresses the Holmstrom-Milgrom (1991) concern from the
organizational economics review: multi-dimensional incentives should not be
collapsed to a single scalar because it distorts attention allocation. The
Pareto approach preserves dimensional diversity.

---

## 4. Objective tuning via GEPA

An `Objective` is also a text artifact: `description + acceptable_tradeoffs +
unacceptable_tradeoffs`. GEPA can optimize it:

```python
def evaluate_objective(obj_text: str) -> tuple[float, dict]:
    """Parse objective, pair with a fixed role, run tasks, measure rewards."""
    # Parse obj_text back into (description, accept, reject)
    # Create agent with fixed role + this objective
    # Assign to tasks, collect rewards
    # Return (mean_reward, ASI)
    ...

result = oa.optimize_anything(
    seed_candidate=yaml.dump({
        "description": objective.description,
        "acceptable_tradeoffs": objective.acceptable_tradeoffs,
        "unacceptable_tradeoffs": objective.unacceptable_tradeoffs,
    }),
    evaluator=evaluate_objective,
    background="Optimize this behavioral mandate for an AI coding agent",
)
```

This replaces wg's `objective-tuning` evolution strategy with GEPA's
reflective search. The current strategy is a single-shot LLM proposal to
"adjust tradeoffs." GEPA would systematically explore the tradeoff space.

---

## 5. Evaluator prompt optimization (meta-evaluation)

The evaluator itself is a text artifact -- the prompt assembled by
`render_evaluator_prompt()`. Its quality determines the quality of the
entire reward signal.

GEPA can optimize it, using VX outcome rewards as ground truth:

```python
def evaluate_evaluator_prompt(prompt_template: str) -> tuple[float, dict]:
    """Measure how well this evaluator prompt predicts real-world outcomes."""
    # For each task with both LLM reward and VX outcome reward:
    #   1. Re-run evaluation with this prompt template
    #   2. Compare LLM score to outcome score
    #   3. Correlation = evaluator quality
    return correlation, {"per_task_deltas": deltas}
```

This is the "who monitors the monitor" problem from Alchian & Demsetz (1972),
solved computationally: GEPA optimizes the evaluator prompt to maximize
agreement with ground-truth outcomes.

---

## 6. Multi-task mode = cross-task-type generalization

GEPA's **multi-task** and **generalization** modes map directly to wg's
task landscape:

| GEPA concept | wg analog |
|---|---|
| `dataset` (training examples) | Historical tasks the role has been assigned to |
| `valset` (validation set) | Held-out task types for generalization testing |
| Per-example scores | Per-task rewards |
| Cross-transfer between examples | A role improvement that helps on task A also helps on task B |

A role should generalize across task types, not overfit to one. GEPA's
generalization mode directly supports this:

```python
result = oa.optimize_anything(
    seed_candidate=role.description,
    evaluator=evaluate_on_task,
    dataset=historical_tasks[:20],  # training tasks
    valset=held_out_tasks[:5],      # unseen task types
    objective="Optimize role description to generalize across task types",
)
```

---

## 7. GEPA's merge = wg's crossover, but iterative

wg's `crossover` evolution strategy combines two high-performing roles in a
single LLM call. GEPA's `MergeProposer` does the same thing but within the
optimization loop -- it can attempt multiple merges, evaluate each, and keep
only the ones that improve the Pareto frontier.

The integration: when wg's evolver selects `crossover` as the strategy, it
delegates to GEPA with two seed candidates (the two parent roles) and lets
GEPA's merge proposer find the best combination:

```python
result = oa.optimize_anything(
    seed_candidate=parent_role_1.description,  # start from parent 1
    evaluator=evaluate_role,
    background=f"Also consider incorporating elements from this alternative: {parent_role_2.description}",
)
```

---

## 8. VX outcome metrics as GEPA evaluators

VX's outcome-based rewards (`source: "outcome:brier"`, `source: "outcome:sharpe"`)
are precisely the evaluator functions GEPA needs. They are:

- **Scalar-valued**: GEPA needs `str -> float`
- **Ground-truth-based**: computed from real-world data, not LLM judgment
- **Incentive-compatible**: proper scoring rules (Brier) cannot be gamed

This means GEPA can optimize role descriptions *directly against real-world
outcomes*, bypassing the LLM evaluator entirely:

```python
def evaluate_against_outcomes(role_description: str) -> tuple[float, dict]:
    # Run tasks, wait for VX outcome measurements
    # Return (outcome_score, {per_task_outcomes})
    ...
```

This closes the loop that the organizational economics review identified as
critical: Goodhart's Law says that LLM-on-LLM evaluation will be gamed by
shared blind spots. Optimizing directly against VX outcomes eliminates this
pathway entirely.

---

## 9. Callbacks = wg provenance logging

GEPA's callback system maps to wg's operation log:

| GEPA callback | wg log entry |
|---|---|
| `OptimizationStartEvent` | evolution run start |
| `CandidateAcceptedEvent` | new role/objective created |
| `CandidateRejectedEvent` | proposed variant discarded |
| `ParetoFrontUpdatedEvent` | frontier snapshot |
| `EvaluationEndEvent` | reward recorded |
| `MergeAcceptedEvent` | crossover succeeded |

Each GEPA optimization run would produce a full provenance trail stored in
`.workgraph/identity/evolution_runs/`, compatible with wg's existing
lineage system.

---

## 10. Stop conditions = wg evolution budget

| GEPA stop condition | wg analog |
|---|---|
| `MaxMetricCallsStopper(N)` | `wg evolve --budget N` |
| `NoImprovementStopper(patience=K)` | convergence detection (not yet in wg) |
| `ScoreThresholdStopper(0.95)` | "stop when role hits target performance" |
| `TimeoutStopCondition("2h")` | wall-clock budget (not yet in wg) |

Adding convergence detection and score thresholds to wg's evolution system
is a natural consequence of the GEPA integration.

---

## Architecture options

### Option A: GEPA as evolution backend (recommended)

Replace the single-shot evolver LLM call with a GEPA optimization run.
The `wg evolve` command becomes:

```
wg evolve --strategy mutation    # GEPA optimizes one role with ASI
wg evolve --strategy crossover   # GEPA merges two roles iteratively
wg evolve --strategy gap-analysis # GEPA generates from scratch (seedless)
wg evolve --strategy objective-tuning # GEPA optimizes one objective
```

Each strategy maps to a GEPA configuration:
- `mutation` = `optimize_anything` with seed + reflective mutation
- `crossover` = `optimize_anything` with merge proposer
- `gap-analysis` = `optimize_anything` with `seed_candidate=None`
- `objective-tuning` = `optimize_anything` targeting the objective text
- `retirement` = no GEPA involvement (selection only, no generation)

The `--budget N` flag maps to `MaxMetricCallsStopper(N)`, where each "metric
call" is one task-dispatch-and-reward cycle.

### Option B: GEPA adapter for wg

Implement a `WorkgraphAdapter` that plugs into GEPA's adapter system:

```python
class WorkgraphAdapter(gepa.GEPAAdapter):
    def __init__(self, wg_dir, role_id=None, objective_id=None):
        self.wg_dir = wg_dir
        ...

    def evaluate(self, batch, candidate, capture_traces):
        # Parse candidate text into role/objective YAML
        # Assign to batch of ready tasks
        # Wait for completion, collect rewards
        # Package as EvaluationBatch with ASI from logs/artifacts
        ...

    def make_reflective_dataset(self, candidate, eval_batch, components):
        # Format wg reward dimensions + notes as reflective examples
        ...
```

This would let GEPA users optimize workgraph identities through the standard
GEPA API, and would also appear in GEPA's adapter gallery alongside DSPy,
RAG, MCP, etc.

### Option C: Both

Option A for wg users (`wg evolve` uses GEPA internally).
Option B for GEPA users (`gepa.WorkgraphAdapter` exposes wg as a target).

---

## What this changes about the evolution system

| Current (single-shot evolver) | With GEPA |
|---|---|
| One LLM call per evolution run | N reflective iterations per run |
| Evolver proposes blind (no execution feedback) | Each iteration runs tasks and reads ASI |
| Scalar reward signal | Pareto frontier across dimensions |
| No convergence detection | `NoImprovementStopper` built in |
| No generalization testing | `valset` enables held-out task type validation |
| Single merge attempt for crossover | Multiple merge attempts, keeping best |
| Human must judge evolution quality | GEPA tracks improvement trajectory automatically |

The fundamental shift: wg's current evolver is **generative** (one-shot LLM
proposal). GEPA makes it **iterative** (propose, evaluate, reflect, repeat).
This is the difference between asking "what mutation would help?" and actually
trying mutations, observing their effect, and refining.

---

## Connection to the organizational economics literature

From the [organizational economics review](organizational-economics-review.md):

1. **March (1991), exploration vs exploitation.** GEPA's
   `EpsilonGreedyCandidateSelector` explicitly manages the explore/exploit
   tradeoff. This addresses the review's warning that wg's evolution system
   will naturally underexplore.

2. **Holmstrom & Milgrom (1991), multi-dimensional incentives.** GEPA's
   Pareto frontier preserves dimensional diversity instead of collapsing to
   a weighted average. This directly addresses the "teaching to the test" risk.

3. **Goodhart's Law.** GEPA + VX outcome evaluators optimize directly against
   real-world metrics, bypassing LLM-on-LLM shared blind spots.

4. **Nelson & Winter (1982), directed search.** GEPA's ASI-driven reflection
   is a more sophisticated version of Nelson & Winter's "routine-guided
   search" -- it does not mutate randomly but reads diagnostic feedback and
   proposes targeted improvements. This is Lamarckian evolution with a better
   feedback signal.

5. **Weitzman (1998), recombinant growth.** GEPA's merge proposer implements
   Weitzman's combinatorial process iteratively: instead of one crossover
   attempt, it can try many combinations and keep the Pareto-optimal ones.
   This increases the yield from the combinatorial identity space.

---

## Practical next steps

1. **Prototype the evaluator wrapper.** Write a Python function that takes a
   role description string, creates a wg role, assigns it to N tasks, waits
   for rewards, and returns `(mean_score, ASI_dict)`. This is the critical
   integration surface.

2. **Test with `optimize_anything` on a single role.** Pick the lowest-
   performing role in a live workgraph, use GEPA to optimize its description
   over 20-50 iterations, compare the optimized role's performance to the
   original.

3. **Implement the `WorkgraphAdapter`.** Package the evaluator wrapper as a
   GEPA adapter, enabling standard GEPA workflows.

4. **Wire into `wg evolve`.** Add a `--backend gepa` flag to `wg evolve`
   that routes evolution through GEPA instead of the single-shot evolver.

5. **Add Pareto frontier support to wg identity stats.** Display the
   per-dimension frontier alongside the existing synergy matrix.
