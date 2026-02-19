= Evolution & Improvement

The identity does not merely execute work. It learns from it.

Every completed task generates a signal—a scored reward measuring how well the agent performed against the task's requirements and the agent's own declared standards. These signals accumulate into performance records on agents, roles, and objectives. When enough data exists, an evolution cycle reads the aggregate picture and proposes structural changes: sharpen a role's description, tighten a objective's constraints, combine two high-performers into something new, retire what consistently underperforms. The changed entities receive new content-hash IDs, linked to their parents by lineage metadata. Better identities produce better work. Better work produces sharper rewards. The loop closes.

This is the autopoietic core of the identity system—a structured feedback loop where work produces the data that drives its own improvement.

== Reward <reward>

Reward is the act of scoring a completed task. It answers a concrete question: given what this agent was asked to do and the identity it was given, how well did it perform?

The evaluator is itself an LLM agent. It receives the full context of the work: the task definition (title, description, deliverables), the agent's identity (role and objective), any artifacts the agent produced, log entries from execution, and timing data (when the task started and finished). From this, it scores four dimensions:

#table(
  columns: (auto, auto, auto),
  [*Dimension*], [*Weight*], [*What it measures*],
  [Correctness], [40%], [Does the output satisfy the task's requirements and the role's desired outcome?],
  [Completeness], [30%], [Were all aspects of the task addressed? Are deliverables present?],
  [Efficiency], [15%], [Was the work done without unnecessary steps, bloat, or wasted effort?],
  [Style adherence], [15%], [Were project conventions followed? Were the objective's constraints respected?],
)

The weights are deliberate. Correctness dominates because wrong output is worse than incomplete output. Completeness follows because partial work still has value. Efficiency and style adherence matter but are secondary—a correct, complete solution with poor style is more useful than an elegant, incomplete one.

The four dimension scores are combined into a single weighted score between 0.0 and 1.0. This score is the fundamental unit of evolutionary pressure.

=== Three-level propagation <score-propagation>

A single reward does not merely update one record. It propagates to three levels:

+ *The agent's performance record.* The score is appended to the agent's reward history. The agent's average score and task count update.

+ *The role's performance record*—with the objective's ID recorded as `context_id`. This means the role's record knows not just its average score, but _which objective it was paired with_ for each reward.

+ *The objective's performance record*—with the role's ID recorded as `context_id`. Symmetrically, the objective knows which role it was paired with.

This three-level, cross-referenced propagation creates the data structure that makes synergy analysis possible. A role's aggregate score tells you how it performs _in general_. The context IDs tell you how it performs _with specific objectives_. The distinction matters: a role might score 0.9 with one objective and 0.5 with another. The aggregate alone would hide this.

=== What gets rewardd

Both done and failed tasks can be rewardd. This is intentional—there is useful signal in failure. Which agents fail on which kinds of tasks reveals mismatches between identity and work that evolution can address.

Human agents are tracked by the same reward machinery, but their rewards are excluded from the evolution signal. The system does not attempt to "improve" humans. Human reward data exists for reporting and trend analysis, not for evolutionary pressure.

== Performance Records and Aggregation <performance>

Every role, objective, and agent maintains a performance record: a task count, a running average score, and a list of reward references. Each reference carries the score, the task ID, a timestamp, and the crucial `context_id`—the ID of the paired entity.

From these records, two analytical tools emerge.

=== The synergy matrix <synergy>

The synergy matrix is a cross-reference of every (role, objective) pair that has been rewardd together. For each pair, it shows the average score and the number of rewards. `wg identity stats` renders this automatically.

High-synergy pairs—those scoring 0.8 or above—represent effective identity combinations worth preserving and expanding. Low-synergy pairs—0.4 or below—represent mismatches. Under-explored combinations with too few rewards are surfaced as hypotheses: try this pairing and see what happens.

The matrix is not a static report. It is a map of the identity's combinatorial identity space, updated with every reward. It tells you where your identity is strong, where it is weak, and where it has not yet looked.

=== Trend indicators

`wg identity stats` also computes directional trends. It splits each entity's recent rewards into first and second halves and compares the averages. If the second half scores more than 0.03 higher, the trend is _improving_. More than 0.03 lower, _declining_. Within 0.03, _flat_.

Trends answer the question that aggregate scores cannot: is this entity getting better or worse over time? A role with a middling 0.65 average but an improving trend is a better evolution candidate than one with a static 0.70. Trends make the temporal dimension of performance visible.

== Evolution <evolution>

Evolution is the process of improving identity entities based on accumulated reward data. Where reward extracts signal from individual tasks, evolution acts on the aggregate—reading the full performance picture and proposing structural changes to roles and objectives.

Evolution is triggered manually by running `wg evolve`. This is a deliberate design choice. The system accumulates reward data automatically (via the coordinator's auto-reward feature), but the decision to act on that data belongs to the human. Evolution is powerful enough to reshape the identity's identity space. It should not run unattended.

=== The evolver agent

The evolver is itself an LLM agent. It receives a comprehensive performance summary: every role and objective with their scores, dimension breakdowns, generation numbers, lineage, and the synergy matrix. It also receives strategy-specific guidance documents from `.workgraph/identity/evolver-skills/`—prose procedures for each type of evolutionary operation.

The evolver can have its own identity identity—a role and objective that shape how it approaches improvement. A cautious evolver objective that rejects aggressive changes will produce different proposals than an experimental one. The evolver's identity is configured in `config.toml` and injected into its prompt, just like any other agent.

=== Strategies <strategies>

Six strategies define the space of evolutionary operations:

*Mutation.* The most common operation. Take an existing role or objective and modify it to address specific weaknesses. If a role scores poorly on completeness, the evolver might sharpen its desired outcome or add a skill reference that emphasizes thoroughness. The mutated entity receives a new content-hash ID—it is a new entity, linked to its parent by lineage.

*Crossover.* Combine traits from two high-performing entities into a new one. If two roles each excel on different dimensions, crossover attempts to produce a child that inherits the strengths of both. The new entity records both parents in its lineage.

*Gap analysis.* Create entirely new roles or objectives for capabilities the identity lacks. If tasks requiring a skill no agent possesses consistently fail or go unmatched, gap analysis proposes a new role to fill that space.

*Retirement.* Remove consistently poor-performing entities. This is pruning—clearing out identities that reward has shown to be ineffective. Retired entities are not deleted; they are renamed to `.yaml.retired` and preserved for audit.

*Objective tuning.* Adjust the trade-offs on an existing objective. Tighten a constraint that rewards show is being violated. Relax one that is unnecessarily restrictive. This is a targeted form of mutation specific to the objective's acceptable and unacceptable trade-off lists.

*All.* Use every strategy as appropriate. The evolver reads the full performance picture and proposes whatever mix of operations it deems most impactful. This is the default.

Each strategy can be selected individually via `wg evolve --strategy mutation` or combined as the default `all`. Strategy-specific guidance documents in the evolver-skills directory give the evolver detailed procedures for each approach.

=== Mechanics

When `wg evolve` runs, the following sequence executes:

+ All roles, objectives, and rewards are loaded. Human-agent rewards are filtered out—they would pollute the signal, since human performance does not reflect the effectiveness of a role-objective prompt.

+ A performance summary is built: role-by-role and objective-by-objective scores, dimension averages, generation numbers, lineage, and the synergy matrix.

+ The evolver prompt is assembled: system instructions, the evolver's own identity (if configured), meta-agent assignments (so the evolver knows which entities serve coordination roles), the chosen strategy, budget constraints, retention heuristics (a prose policy from configuration), the performance summary, and strategy-specific skill documents.

+ The evolver agent runs and returns structured JSON: a list of operations (create, modify, or retire) with full entity definitions and rationales.

+ Operations are applied sequentially. Budget limits are enforced—if the evolver proposes more operations than the budget allows, only the first N are applied. After each operation, the local state is reloaded so subsequent operations can reference newly created entities.

+ A run report is saved to `.workgraph/identity/evolution_runs/` with the full transcript: what was proposed, what was applied, and why.

=== How modified entities are born

When the evolver proposes a `modify_role` operation, the system does not edit the existing role in place. It creates a _new_ role with the modified fields, computes a fresh content-hash ID from the new content, and writes it as a new YAML file. The original role remains untouched.

The new role's lineage records its parent: the ID of the role it was derived from, a generation number one higher than the parent's, the evolver run ID as the creator, and a timestamp. For crossover operations, the lineage records multiple parents and takes the highest generation among them.

This is where content-hash IDs and immutability pay off. The original entity is a mathematical fact—its hash proves it has not been tampered with. The child is a new fact, with a provable link to its origin. You can walk the lineage chain from any entity back to its manually-created ancestor at generation zero.

== Safety Guardrails <safety>

Evolution is powerful. The guardrails are proportional.

*The last remaining role or objective cannot be retired.* The identity must always have at least one of each. This prevents an overzealous evolver from pruning the identity into nonexistence.

*Retired entities are preserved, not deleted.* The `.yaml.retired` suffix removes them from active duty but keeps them on disk for audit, rollback, or lineage inspection.

*Dry run.* `wg evolve --dry-run` renders the full evolver prompt and shows it without executing. You see exactly what the evolver would see. This is the first thing to run when experimenting with evolution.

*Budget limits.* `--budget N` caps the number of operations applied per run. Start small—two or three operations—review the results, iterate. The evolver may propose ten changes, but you decide how many land.

*Self-mutation deferral.* The evolver's own role and objective are valid mutation targets—the system should be able to improve its own improvement mechanism. But self-modification without oversight is dangerous. When the evolver proposes a change to its own identity, the operation is not applied directly. Instead, a review meta-task is created in the workgraph with a `verify` field requiring human approval. The proposed operation is embedded in the task description as JSON. A human must inspect the change and apply it manually.

== Lineage <lineage>

Every role, objective, and agent tracks its evolutionary history through a lineage record: parent IDs, generation number, creator identity, and timestamp.

Generation zero entities are the seeds—created by humans via `wg role add`, `wg objective add`, or `wg identity init`. They have no parents. Their `created_by` field reads `"human"`.

Generation one entities are the first children of evolution. A mutation from a generation-zero role produces a generation-one role with a single parent. A crossover of two generation-zero roles produces a generation-one role with two parents. Each subsequent evolution increments from the highest parent's generation.

The `created_by` field on evolved entities records the evolver run ID: `"evolver-run-20260115-143022"`. Combined with the run reports saved in `evolution_runs/`, this creates a complete audit trail: you can trace any entity to the exact evolution run that created it, see what performance data the evolver was working from, and read the rationale for the change.

Lineage commands—`wg role lineage`, `wg objective lineage`, `wg agent lineage`—walk the chain. Agent lineage is the most interesting: it shows not just the agent's own history but the lineage of its constituent role and objective, revealing the full evolutionary tree that converged to produce that particular identity.

== The Autopoietic Loop <autopoiesis>

Step back from the mechanics and see the shape of the whole.

Work enters the system as tasks. The coordinator dispatches agents—each carrying an identity composed of a role and a objective—to execute those tasks. When a task completes, auto-reward creates an reward meta-task. The evaluator agent scores the work across four dimensions. Scores propagate to the agent, the role, and the objective. Over time, performance records accumulate. Trends emerge. The synergy matrix fills in.

When the human decides enough signal has accumulated, `wg evolve` runs. The evolver reads the full performance picture and proposes changes. A role that consistently scores low on efficiency gets its description sharpened to emphasize economy. A objective whose constraints are too tight gets its trade-offs relaxed. Two high-performing roles get crossed to produce a child that inherits both strengths. A consistently poor performer gets retired.

The changed entities—new roles, new objectives—are paired into new agents. These agents are dispatched to the next round of tasks. Their work is rewardd. Their rewards feed the next evolution cycle.

```
        ┌──────────┐
        │  Tasks   │
        └────┬─────┘
             │ dispatch
             ▼
        ┌──────────┐
        │  Agents  │ ◄── roles + objectives
        └────┬─────┘
             │ execute
             ▼
        ┌──────────┐
        │   Work   │
        └────┬─────┘
             │ reward
             ▼
        ┌──────────┐
        │  Scores  │ ──► performance records
        └────┬─────┘     synergy matrix
             │ evolve    trend indicators
             ▼
        ┌──────────┐
        │  Better  │
        │  roles & │ ──► new agents
        │  motiv.  │
        └────┬─────┘
             │
             └──────────► back to dispatch
```

The meta-agents—the assigner that picks which agent gets which task, the evaluator that scores the work, the evolver that proposes changes—are themselves identity entities with roles and objectives. They too can be rewardd. They too can be evolved. The evolver can propose improvements to the evaluator's role. It can propose improvements to _its own_ role, subject to the self-mutation safety check that routes such proposals through human review.

This is what makes the system autopoietic: it does not just produce work, it produces the conditions for better work. It does not just execute, it reflects on execution and restructures itself in response. The identity space of the identity—the set of roles, objectives, and their pairings—is not static. It is a living population subject to selective pressure from the reward signal and evolutionary operations from the evolver.

But the human hand is always on the wheel. Evolution is a manual trigger, not an automatic process. The human decides when to evolve, reviews what the evolver proposes (especially via `--dry-run`), sets budget limits, and must personally approve any self-mutations. The system improves itself, but only with permission.

== Practical Guidance <practical>

*When to evolve.* Wait until you have at least five to ten rewards per role before running evolution. Fewer than that, and the evolver is working from noise rather than signal. `wg identity stats` shows reward counts and trends—use it to judge readiness.

*Start with dry run.* Always run `wg evolve --dry-run` first. Read the prompt. Understand what the evolver sees. This also serves as a diagnostic: if the performance summary looks thin, you need more rewards before evolving.

*Use budgets.* `--budget 2` or `--budget 3` for early runs. Review each operation's rationale. As you build confidence in the evolver's judgment, you can increase the budget or omit it.

*Targeted strategies.* If you know what the problem is—roles scoring low on a specific dimension, objectives with constraints that are too strict—use a targeted strategy. `--strategy mutation` for improving existing entities. `--strategy objective-tuning` for adjusting trade-offs. `--strategy gap-analysis` when tasks are going unmatched.

*Seed, then evolve.* `wg identity init` creates four starter roles and four starter objectives. These are generic seeds—competent but not specialized. Run them through a few task cycles, accumulate rewards, then evolve. The starters are generation zero. Evolution produces generation one, two, and beyond—each generation shaped by the actual work your project requires.

*Watch the synergy matrix.* The matrix reveals which role-objective pairings work well together and which do not. High-synergy pairs should be preserved. Low-synergy pairs are candidates for mutation or retirement. Under-explored combinations are experiments waiting to happen—assign them to tasks and see what the rewards say.

*Lineage as audit.* When an agent produces unexpectedly good or bad work, trace its lineage. Which evolution run created its role? What performance data informed that mutation? The lineage chain, combined with evolution run reports, makes every identity decision traceable.
