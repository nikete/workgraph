= Executive Summary
<executive-summary>
Workgraph’s primitives—tasks, dependency edges, roles, objectives,
agents, a coordinator, rewards, and an evolve loop—are not
arbitrary design choices. They map precisely onto well-established
concepts from organizational theory, cybernetics, workflow science, and
distributed systems. This document develops a vocabulary and framework —
a "mathematics of organizations"—that helps users think rigorously
about how to structure work in workgraph.

#strong[Key findings:]

+ #strong[The task graph is a stigmergic medium.] Agents coordinate
  indirectly by reading and writing task state, exactly as ants
  coordinate via pheromone trails. No agent-to-agent communication is
  needed—the graph #emph[is] the communication channel.

+ #strong[`blocked_by` edges natively express the five basic workflow
  patterns] (Sequence, Parallel Split, Synchronization, Exclusive
  Choice, Simple Merge). `loops_to` back-edges add structured loops and
  arbitrary cycles. Advanced patterns (discriminators, cancellation,
  milestones) require coordinator logic.

+ #strong[The execute→reward→evolve loop is autopoietic.] The system
  literally produces the components (agent definitions) that produce the
  system (task completions that trigger rewards that trigger
  evolution). This is Maturana & Varela’s self-producing network,
  Luhmann’s operationally closed social system, and Argyris & Schön’s
  double-loop learning—all at once.

+ #strong[Fork-Join is the natural topology of `blocked_by` graphs.] The
  planner→N workers→synthesizer pattern is workgraph’s most fundamental
  parallel decomposition. It maps to MapReduce, scatter-gather, and
  WCP2+WCP3.

+ #strong[The coordinator is a cybernetic regulator] operating an OODA
  loop, subject to Ashby’s Law of Requisite Variety: the number of
  distinct roles must match or exceed the variety of task types, or the
  system becomes under-regulated.

+ #strong[Rewards solve the principal-agent problem.] The human
  principal delegates to autonomous agents under information asymmetry.
  Rewards are the monitoring mechanism; objectives are the bonding
  mechanism; evolution is the incentive-alignment mechanism.

+ #strong[Role design is an Inverse Conway Maneuver.] Conway’s Law
  predicts that system architecture mirrors org structure. In workgraph,
  deliberately designing roles shapes the task decomposition and
  therefore the output architecture.


#line(length: 100%, stroke: 0.5pt + luma(180))

== 1. Stigmergy: The Task Graph as Coordination Medium
<stigmergy-the-task-graph-as-coordination-medium>
=== 1.1 What is Stigmergy?
<what-is-stigmergy>
Stigmergy (from Greek #emph[stigma] "mark" + #emph[ergon] "work") is
indirect coordination between agents through traces left in a shared
environment. The term was coined by Pierre-Paul Grassé in 1959 to
explain how termites coordinate mound construction without a central
plan: each termite reads the current state of the structure and responds
with an action that modifies that structure, which in turn stimulates
further action by other termites.

As Heylighen (2016) defines it: "A process is stigmergic if the work
done by one agent provides a stimulus that entices other agents to
continue the job."

There are two fundamental types:

#align(center)[#table(
  columns: 4,
  align: (col, row) => (auto,auto,auto,auto,).at(col),
  inset: 6pt,
  [Type], [Definition], [Example], [Persistence],
  [#strong[Sematectonic]],
  [The work product itself serves as the stimulus],
  [Termite mounds: the shape of the partial structure tells the next
  termite what to do],
  [Permanent (structural)],
  [#strong[Marker-based]],
  [A separate signal (marker) is deposited, distinct from the work
  product],
  [Ant pheromone trails: the chemical trail is not the food, but a
  signal about the food],
  [Transient (decays)],
)
]

=== 1.2 Workgraph is a Stigmergic System
<workgraph-is-a-stigmergic-system>
A workgraph task graph is a stigmergic medium. Agents do not communicate
with each other directly—they read and write to the shared graph, and
the graph’s state stimulates their actions.

#align(center)[#table(
  columns: 2,
  align: (col, row) => (auto,auto,).at(col),
  inset: 6pt,
  [Stigmergy Concept], [Workgraph Equivalent],
  [#strong[Shared environment]],
  [The task graph (`.workgraph/graph.jsonl`)],
  [#strong[Sematectonic trace]],
  [A completed task’s artifacts—the code, docs, or other work product
  left behind #emph[is] the stimulus for downstream tasks],
  [#strong[Marker-based trace]],
  [Task status changes (`Open`→`Done`, `Failed`), dependency edges,
  reward scores],
  [#strong[Pheromone decay]],
  [Stale assignment detection (dead agent checks), task expiration],
  [#strong[Stigmergic coordination]],
  [The coordinator polls the graph for "ready" tasks (all `blocked_by`
  satisfied)—it reads the markers],
  [#strong[Self-reinforcing trails]],
  [Tasks with good reward scores reinforce the role/objective
  patterns that produced them (via evolve)],
)
]

This is not a metaphor. It is a precise structural correspondence. The
defining characteristic of stigmergy—that agents coordinate through a
shared medium rather than through direct communication—is exactly how
workgraph agents operate. Agent A completes task X, modifying the graph
(setting status to `Done`, recording artifacts). Agent B, working on
task Y with `blocked_by = [X]`, is now unblocked. B never spoke to A.
The graph mediated the coordination.

=== 1.3 Real-World Stigmergic Systems
<real-world-stigmergic-systems>
Wikipedia is the canonical human example of stigmergy. An editor sees a
stub article (the trace), is stimulated to expand it, and leaves a more
complete article (a new trace) that stimulates further refinement.
Open-source development works identically: a bug report (marker)
stimulates a patch (sematectonic), which stimulates a review (marker),
which stimulates a merge (sematectonic).

The theoretical literature connects stigmergy to self-organization,
emergence, and scalability. Stigmergic systems scale better than
centrally planned systems because adding agents does not increase
communication overhead—the coordination cost is absorbed by the shared
medium.

=== 1.4 Implications for Workgraph Users
<implications-for-workgraph-users>
- #strong[The task graph is your communication channel.] Write
  descriptive task titles, clear descriptions, and meaningful log
  entries—these are the "pheromone trails" that guide downstream
  agents.
- #strong[Task decomposition is environment design.] How you break work
  into tasks determines the stigmergic landscape agents navigate.
  Fine-grained tasks create more frequent, smaller traces.
  Coarse-grained tasks create fewer, larger traces.
- #strong[Reward records are marker traces.] They don’t change the
  work product but signal information about its quality, guiding the
  evolve loop toward better agent configurations.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 2. Workflow Patterns: What `blocked_by` and `loops_to` Can Express
<workflow-patterns-what-blocked_by-and-loops_to-can-express>
=== 2.1 The Workflow Patterns Catalog
<the-workflow-patterns-catalog>
The Workflow Patterns Initiative, established by Wil van der Aalst,
Arthur ter Hofstede, Bartek Kiepuszewski, and Alistair Barros,
catalogued 43 control-flow patterns that recur across business process
modeling systems. The original 2003 paper identified 20; a 2006 revision
expanded this to 43. The initiative also catalogued 43 Resource Patterns
and 40 Data Patterns.

These patterns provide a precise vocabulary for what any workflow system
can and cannot express.

=== 2.2 Patterns Natively Supported by `blocked_by`
<patterns-natively-supported-by-blocked_by>
#align(center)[#table(
  columns: 4,
  align: (col, row) => (auto,auto,auto,auto,).at(col),
  inset: 6pt,
  [Pattern], [ID], [Workgraph Expression], [Example],
  [#strong[Sequence]],
  [WCP1],
  [`B.blocked_by = [A]`],
  [`write-code → review-code`],
  [#strong[Parallel Split]],
  [WCP2],
  [Multiple tasks sharing the same predecessor: `B.blocked_by = [A]`,
  `C.blocked_by = [A]`],
  [`plan → {implement-frontend, implement-backend}`],
  [#strong[Synchronization] (AND-join)],
  [WCP3],
  [`D.blocked_by = [B, C]`],
  [`{frontend, backend} → integration-test`],
  [#strong[Simple Merge]],
  [WCP5],
  [Single successor of multiple predecessors, where only one fires],
  [`{hotfix, feature} → deploy` (only one path active)],
  [#strong[Implicit Termination]],
  [WCP11],
  [Tasks with no successors simply complete],
  [Leaf tasks in the graph],
)
]

These five patterns—the DAG patterns—are the bread and butter of
`blocked_by` graphs.

=== 2.3 Patterns Added by `loops_to`
<patterns-added-by-loops_to>
#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Pattern], [ID], [Workgraph Expression],
  [#strong[Arbitrary Cycles]],
  [WCP10],
  [`loops_to` edges create back-edges with guards and max iterations],
  [#strong[Structured Loop]],
  [WCP21],
  [`loops_to` with a guard condition (pre-test or post-test)],
)
]

=== 2.4 Patterns Requiring Coordinator Logic (Idioms)
<patterns-requiring-coordinator-logic-idioms>
These patterns cannot be expressed with static edges alone but can be
achieved through coordinator behavior or conventions:

#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Pattern], [ID], [Idiom],
  [#strong[Exclusive Choice]],
  [WCP4],
  [A coordinator task rewards a condition and creates only the
  appropriate successor task],
  [#strong[Multi-Choice]],
  [WCP6],
  [A coordinator task selectively creates subsets of successor tasks],
  [#strong[Discriminator]],
  [WCP9],
  [A join task is manually marked ready after the first of N
  predecessors completes],
  [#strong[Multiple Instance (runtime)]],
  [WCP14-15],
  [The coordinator dynamically creates N task copies at runtime based on
  data],
  [#strong[Deferred Choice]],
  [WCP16],
  [Multiple tasks created; coordinator cancels the unchosen ones],
  [#strong[Cancel Task/Region]],
  [WCP19/25],
  [`wg abandon <task-id>`—terminal status that unblocks dependents],
  [#strong[Milestone]],
  [WCP18],
  [A task checks the status of a non-predecessor ("is task X done?")
  before proceeding],
)
]

=== 2.5 Resource Patterns and the Identity
<resource-patterns-and-the-identity>
Beyond control-flow, Van der Aalst’s Resource Patterns describe how work
is distributed to agents. Several map directly:

#align(center)[#table(
  columns: 2,
  align: (col, row) => (auto,auto,).at(col),
  inset: 6pt,
  [Resource Pattern], [Workgraph Equivalent],
  [#strong[Role-Based Distribution] (WRP2)],
  [Tasks matched to agents by role],
  [#strong[Capability-Based Distribution] (WRP8)],
  [Task `skills` matched against role capabilities],
  [#strong[Automatic Execution] (WRP11)],
  [`wg service start`—the coordinator auto-assigns and spawns agents],
  [#strong[History-Based Distribution] (WRP6)],
  [Reward-informed agent selection in auto-assign tasks],
  [#strong[Organizational Distribution] (WRP9)],
  [The identity structure (roles, objectives) determines the
  distribution],
)
]

=== 2.6 Summary: Expressiveness Hierarchy
<summary-expressiveness-hierarchy>
```
blocked_by alone:        WCP1-3, WCP5, WCP11 (basic DAG patterns)
+ loops_to:              + WCP10, WCP21 (cycles and structured loops)
+ coordinator logic:     + WCP4, WCP6, WCP9, WCP14-16, WCP18-20, WCP25
+ resource patterns:     + WRP2, WRP6, WRP8, WRP9, WRP11
```

The design principle: #strong[edges express structure; the coordinator
expresses policy.]

#line(length: 100%, stroke: 0.5pt + luma(180))

== 3. Fork-Join, MapReduce, and Scatter-Gather
<fork-join-mapreduce-and-scatter-gather>
=== 3.1 The Three Parallel Decomposition Patterns
<the-three-parallel-decomposition-patterns>
These three patterns represent variations of the same fundamental idea —
parallel decomposition with subsequent aggregation—originating from
different fields:

#align(center)[#table(
  columns: 4,
  align: (col, row) => (auto,auto,auto,auto,).at(col),
  inset: 6pt,
  [Pattern], [Structure], [Origin], [Key Distinction],
  [#strong[Fork-Join]],
  [A task forks into N subtasks; a join barrier waits for all N to
  complete],
  [OS/concurrency theory (Conway 1963, Lea 2000)],
  [Strict barrier synchronization. All forks must join.],
  [#strong[MapReduce]],
  [A map phase applies a function to each element in parallel; a reduce
  phase aggregates results],
  [Dean & Ghemawat 2004, functional programming],
  [Data-parallel. Decomposition driven by data partitioning, not task
  structure. Includes shuffle/sort between map and reduce.],
  [#strong[Scatter-Gather]],
  [A request is scattered to N recipients; responses are gathered by an
  aggregator],
  [Enterprise Integration Patterns (Hohpe & Woolf 2003)],
  [Message-oriented. Recipients may be heterogeneous. Aggregation may
  accept partial results.],
)
]

=== 3.2 Fork-Join in Workgraph
<fork-join-in-workgraph>
Fork-Join is the natural topology of `blocked_by` graphs:

```
           ┌─── worker-1 ───┐
planner ───┼─── worker-2 ───┼─── synthesizer
           └─── worker-3 ───┘
```

```bash
wg add "Plan the work" --id planner
wg add "Worker 1" --id worker-1 --blocked-by planner
wg add "Worker 2" --id worker-2 --blocked-by planner
wg add "Worker 3" --id worker-3 --blocked-by planner
wg add "Synthesize results" --id synthesizer --blocked-by worker-1 worker-2 worker-3
```

This is WCP2 (Parallel Split) composed with WCP3 (Synchronization). It
is workgraph’s most fundamental parallel pattern. Every fan-out from a
single task is a fork; every convergence point with multiple
`blocked_by` entries is a join.

=== 3.3 MapReduce in Workgraph
<mapreduce-in-workgraph>
MapReduce adds data-parallel semantics to fork-join. In workgraph, this
is expressed as:

+ A #strong[planner] task that analyzes input data and produces a
  decomposition
+ N #strong[map] tasks (one per data partition), each `blocked_by` the
  planner
+ A #strong[reduce] task that `blocked_by` all map tasks and aggregates
  results

The coordinator creates the N map tasks dynamically based on the
planner’s output. The "shuffle" phase is implicit—each reduce task’s
description specifies which map outputs it consumes.

This is workgraph’s most common pattern for parallelizable research,
analysis, and implementation tasks.

=== 3.4 Scatter-Gather in Workgraph
<scatter-gather-in-workgraph>
Scatter-Gather differs from fork-join in two ways: recipients may be
heterogeneous (different roles), and the aggregator may not require all
responses. In workgraph:

- #strong[Heterogeneous scatter]: Assign different roles to the worker
  tasks. A security analyst, a performance engineer, and a UX reviewer
  all examine the same codebase.
- #strong[Partial gather]: The synthesizer task can be unblocked by
  marking incomplete worker tasks as `Abandoned` (a terminal status).
  This is an idiom for the Discriminator pattern (WCP9).

=== 3.5 Work-Stealing
<work-stealing>
Doug Lea’s Fork/Join framework introduced work-stealing: idle threads
steal tasks from busy threads’ queues, achieving dynamic load balancing
without central scheduling. The workgraph coordinator does something
similar—when an agent finishes a task, the coordinator assigns it the
next ready task regardless of which "queue" it originated from. The
coordinator’s `max_agents` parameter is the thread pool size.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 4. Pipeline and Assembly Line Patterns
<pipeline-and-assembly-line-patterns>
=== 4.1 The Pipeline Pattern
<the-pipeline-pattern>
A pipeline is a serial chain of specialized processing stages:

```
analyst → implementer → reviewer → deployer
```

Each stage transforms inputs into outputs consumed by the next stage.
This maps directly to manufacturing and operations concepts:

#align(center)[#table(
  columns: 2,
  align: (col, row) => (auto,auto,).at(col),
  inset: 6pt,
  [Manufacturing Concept], [Workgraph Expression],
  [#strong[Assembly line]],
  [A chain of tasks with sequential `blocked_by` edges, each assigned to
  a different specialized role],
  [#strong[Work station]],
  [A role—the specialized capability at each pipeline stage],
  [#strong[Work-in-progress (WIP)]],
  [Tasks in `InProgress` status—the items currently being processed],
  [#strong[Throughput]],
  [Rate of task completion—how many tasks move through the pipeline
  per unit time],
  [#strong[Bottleneck]],
  [The pipeline stage with the longest average task duration
  (identifiable from `started_at`/`completed_at` timestamps)],
  [#strong[WIP limit]],
  [`max_agents` parameter—limits how many tasks are simultaneously
  in-progress],
)
]

=== 4.2 Pipeline vs. Fork-Join
<pipeline-vs.-fork-join>
These two patterns are complementary, not competing:

#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Dimension], [Pipeline], [Fork-Join],
  [#strong[Parallelism type]],
  [Task-level (different stages run concurrently on different work
  items)],
  [Data-level (same operation applied to multiple items
  simultaneously)],
  [#strong[Role assignment]],
  [Different role per stage],
  [Same role for all workers],
  [#strong[When to use]],
  [Work requires sequential specialized transformation],
  [Work is decomposable into independent parallel units],
  [#strong[Workgraph shape]],
  [Long chain],
  [Wide diamond],
)
]

=== 4.3 Combined Patterns
<combined-patterns>
Real workflows combine both. A common workgraph pattern:

```
         ┌─── implement-module-1 ───┐
plan ────┼─── implement-module-2 ───┼─── integrate ─── review ─── deploy
         └─── implement-module-3 ───┘
```

The middle is fork-join (parallel implementation); the overall shape is
a pipeline (plan → implement → integrate → review → deploy). Each stage
can have a different role:

- `plan`: architect role
- `implement-module-*`: implementer role
- `integrate`: implementer role
- `review`: reviewer role
- `deploy`: operator role

This is the Inverse Conway Maneuver in action—the role assignments
shape the pipeline stages, which shape the output architecture.

=== 4.4 Theory of Constraints
<theory-of-constraints>
Eliyahu Goldratt’s Theory of Constraints (1984) applies directly to
pipelines:

+ #strong[Identify] the bottleneck—the pipeline stage with the lowest
  throughput
+ #strong[Exploit] the bottleneck—ensure it’s never idle (keep it fed
  with ready tasks)
+ #strong[Subordinate] everything else to the bottleneck—upstream
  stages should not overproduce
+ #strong[Elevate] the bottleneck—add more agents to that role, or
  split the role into finer-grained specializations
+ #strong[Repeat]—the bottleneck shifts; find the new one

In workgraph terms: if the `reviewer` role is the bottleneck, either
assign more agents to that role, or decompose review into sub-roles
(security review, code style review, correctness review) that can run in
parallel.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 5. Autopoiesis: The Self-Producing Identity
<autopoiesis-the-self-producing-identity>
=== 5.1 The Concept
<the-concept>
Autopoiesis (from Greek #emph[auto] "self" + #emph[poiesis]
"production") was introduced by Chilean biologists Humberto Maturana and
Francisco Varela in 1972 to characterize the self-maintaining chemistry
of living cells. An autopoietic system is:

#quote(block: true)[
"A network of inter-related component-producing processes such that the
components in interaction generate the same network that produced them."
]

Key properties:

#align(center)[#table(
  columns: 2,
  align: (col, row) => (auto,auto,).at(col),
  inset: 6pt,
  [Property], [Definition],
  [#strong[Self-production]],
  [The system’s processes produce the components that constitute the
  system],
  [#strong[Operational closure]],
  [Internal operations only produce operations of the same type; the
  system’s boundary is maintained from within],
  [#strong[Structural coupling]],
  [While operationally closed, the system is coupled to its environment
 —perturbations trigger internal structural changes, but the
  environment does not #emph[determine] internal states],
  [#strong[Structural determinism]],
  [The system’s current structure determines what perturbations it can
  respond to and how],
)
]

=== 5.2 Luhmann’s Social Systems Theory
<luhmanns-social-systems-theory>
Niklas Luhmann (1984) adapted autopoiesis for sociology with a radical
move: #strong[social systems are made of communications, not people.]
People are in the #emph[environment] of social systems, not their
components. A social system is autopoietic because each communication
connects to previous communications and stimulates subsequent ones —
communications producing communications.

This reframing is strikingly applicable to workgraph: the #emph[system]
is the network of task state transitions and rewards, not the agents
themselves. Agents are in the environment of the workgraph system. What
matters is the network of communications: "task X is done" triggers
"task Y is ready" triggers "agent A starts work" triggers "task Y is
in-progress"—communications producing communications.

=== 5.3 The Evolve Loop is Autopoietic
<the-evolve-loop-is-autopoietic>
The execute→reward→evolve→execute cycle maps precisely onto
autopoietic self-production:

```
execute (agents produce artifacts)
   ↓
reward (artifacts produce reward scores)
   ↓
evolve (scores produce new role/objective definitions)
   ↓
agents formed from new definitions → assigned to future tasks
   ↓
execute (cycle repeats)
```

#align(center)[#table(
  columns: 2,
  align: (col, row) => (auto,auto,).at(col),
  inset: 6pt,
  [Autopoietic Property], [Workgraph Manifestation],
  [#strong[Self-production]],
  [The evolve step produces new agent definitions (modified roles,
  objectives) that are themselves the components that execute the next
  cycle. The system literally produces the components that produce the
  system.],
  [#strong[Operational closure]],
  [Agents interact only through the task graph. All "communication" is
  mediated by task state changes. The internal logic (role definitions,
  objective constraints, reward rubrics) is self-referential.],
  [#strong[Structural coupling]],
  [The task graph is coupled to the external codebase/project. Changes
  in the environment (new bugs, new requirements) perturb the system by
  adding new tasks, but the system’s internal structure determines how
  it responds.],
  [#strong[Cognition]],
  [Maturana and Varela argued that #emph[living is cognition]—the
  capacity to maintain autopoiesis in a changing environment is a form
  of knowing. The reward system is the identity’s cognition—its
  capacity to sense whether autopoiesis is being maintained (are tasks
  being completed successfully?) and adapt accordingly.],
  [#strong[Temporalization]],
  [Tasks are momentary events. Once completed, they are consumed. The
  system must continuously produce new tasks (or loop back via
  `loops_to`) to maintain itself. A workgraph with no open tasks has
  ceased its autopoiesis.],
)
]

=== 5.4 Practical Implications
<practical-implications>
The autopoietic framing suggests several design principles:

+ #strong[The identity is alive only while tasks flow.] An idle identity
  with no open tasks is a dead system. The `loops_to` mechanism keeps
  the identity alive by re-activating tasks.
+ #strong[Evolution is not optional—it is survival.] An identity that
  does not evolve in response to reward feedback will become
  structurally coupled to an environment that has moved on. The evolve
  step is the autopoietic system’s metabolism.
+ #strong[Perturbations enter through tasks, not through agents.] New
  requirements, bug reports, and changing priorities are perturbations
  that enter the system as new tasks. The system’s response is
  determined by its current structure (which agents exist, what roles
  they have, what objectives constrain them).
+ #strong[Self-reference is a feature, not a bug.] The evolve step
  modifying the very agents that will execute the next cycle is
  self-referential. This is what makes the system autopoietic. The
  self-mutation safety guard (evolver cannot modify its own role without
  human approval) is the autopoietic system’s immune response —
  preventing pathological self-modification.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 6. Cybernetics and Control Theory
<cybernetics-and-control-theory>
=== 6.1 Core Concepts
<core-concepts>
Cybernetics (from Greek #emph[kybernetes] "steersman") is the study of
regulatory systems—feedback loops, circular causality, and the science
of control and communication. Founded by Norbert Wiener (1948) and W.
Ross Ashby (1956), it provides the mathematical framework for
understanding how systems maintain stability in changing environments.

#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Concept], [Author], [Definition],
  [#strong[Negative feedback]],
  [Wiener (1948)],
  [A signal from the output is fed back to the input to reduce deviation
  from a desired state. The basis of homeostasis.],
  [#strong[Positive feedback]],
  [Wiener (1948)],
  [Output amplifies input, producing exponential growth or runaway
  change.],
  [#strong[Law of Requisite Variety]],
  [Ashby (1956)],
  ["Only variety can absorb variety." A regulator must have at least as
  many states as the system it regulates.],
  [#strong[Homeostasis]],
  [Cannon (1932)],
  [A system maintains internal stability through negative feedback,
  adjusting to external perturbations.],
  [#strong[OODA Loop]],
  [Boyd (1976)],
  [Observe→Orient→Decide→Act. Competitive advantage from completing the
  loop faster.],
  [#strong[Double-loop learning]],
  [Argyris & Schön (1978)],
  [Single-loop: adjust actions to reduce error. Double-loop: question
  the governing variables themselves.],
  [#strong[Second-order cybernetics]],
  [von Foerster (1974)],
  [The cybernetics of cybernetics—the observer is part of the
  system.],
)
]

=== 6.2 The Coordinator as Cybernetic Regulator
<the-coordinator-as-cybernetic-regulator>
The workgraph coordinator operates a control loop:

```
       ┌──────────────────────────────────────┐
       │                                      │
       ▼                                      │
   [Observe]                             [Feedback]
   Poll graph for ready tasks            Reward scores
   Check agent status (alive/dead)       Task completion/failure
       │                                      │
       ▼                                      │
   [Orient]                                   │
   Match tasks to agents by capability        │
   Check capacity (max_agents)                │
       │                                      │
       ▼                                      │
   [Decide]                                   │
   Select agent for task (auto-assign)        │
   Priority ordering of ready tasks           │
       │                                      │
       ▼                                      │
   [Act] ─────────────────────────────────────┘
   Spawn agent on task
   Detect/cleanup dead agents
```

This is simultaneously: - An #strong[OODA loop] (Boyd): Observe the
graph state → Orient to available agents and tasks → Decide on
assignment → Act by spawning - A #strong[negative feedback loop]
(Wiener): Failed tasks trigger re-assignment or retry, reducing
deviation from the goal state (all tasks done) - A #strong[homeostatic
regulator] (Cannon/Ashby): The coordinator maintains steady throughput
despite perturbations (agent failures, new tasks added, changing
priorities)

=== 6.3 Ashby’s Law of Requisite Variety
<ashbys-law-of-requisite-variety>
Ashby’s Law states: #strong["Only variety can absorb variety."] A
regulator must have at least as many response options as the system has
disturbance types. Formally: V(Regulator) ≥ V(Disturbances).

Applied to workgraph quantitatively:

- Let #strong[V] \= the number of distinct task types in the graph
  (identified by required skills, complexity, domain)
- Let #strong[R] \= the number of distinct roles in the identity

#strong[Ashby’s Law requires R ≥ V] for adequate regulation. If V grows
(new kinds of work emerge) and R does not, the system becomes
under-regulated—agents will be assigned to tasks they lack the
capability for, producing poor results.

The `evolve` mechanism is precisely how the system increases its
requisite variety: when rewards reveal that existing roles cannot
handle certain task types, the evolver creates new roles (increasing R)
to match the growing variety of disturbances (V).

#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Requisite Variety Violation], [Symptom in Workgraph], [Fix],
  [Too few roles for task variety],
  [Low reward scores on certain task types],
  [`wg evolve --strategy gap-analysis`],
  [Too many roles (over-regulation)],
  [Roles with zero task assignments, wasted identity complexity],
  [`wg evolve --strategy retirement`],
  [Objective too restrictive],
  [Tasks that require speed are assigned agents with "never rush"
  constraints],
  [Tune acceptable/unacceptable tradeoffs],
)
]

=== 6.4 Single-Loop vs. Double-Loop Learning
<single-loop-vs.-double-loop-learning>
#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Learning Type], [Mechanism], [Workgraph Equivalent],
  [#strong[Single-loop]],
  [Adjust actions within existing framework to reduce error],
  [Rewards adjust which agent is assigned to which task type. Same
  roles, same objectives, different assignment.],
  [#strong[Double-loop]],
  [Question and modify the framework itself],
  [The `evolve` step modifies roles and objectives themselves. The
  governing variables change.],
)
]

Single-loop: "This agent performed poorly on this task. Assign a
different agent next time." Double-loop: "The role definition itself is
wrong. Modify the role’s skills and desired outcome."

Argyris and Schön argued that organizations that cannot double-loop
learn become rigid and eventually fail. In workgraph, an identity that
only re-assigns tasks without evolving its roles and objectives will
plateau in performance.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 7. The Viable System Model
<the-viable-system-model>
=== 7.1 Beer’s Five Systems
<beers-five-systems>
Stafford Beer’s Viable System Model (VSM) describes the organizational
structure of any autonomous system capable of surviving in a changing
environment. The model is #strong[recursive]: every viable system
contains viable systems and is contained within a viable system.

#align(center)[#table(
  columns: 4,
  align: (col, row) => (auto,auto,auto,auto,).at(col),
  inset: 6pt,
  [System], [Name], [Function], [Key Principle],
  [#strong[S1]],
  [Operations],
  [The parts that #emph[do things]. Multiple S1 units operate
  semi-autonomously.],
  [Autonomy of operational units],
  [#strong[S2]],
  [Coordination],
  [Prevents oscillation and conflict between S1 units. Scheduling,
  protocols, standards.],
  [Anti-oscillatory damping],
  [#strong[S3]],
  [Operational Control],
  [Optimizes the "here and now" across all S1 units. Resource
  allocation, synergy.],
  [Internal optimization],
  [#strong[S3\*]],
  [Audit Channel],
  [Sporadic checks that bypass normal reporting.],
  [Independent verification],
  [#strong[S4]],
  [Intelligence],
  [Scans the external environment for threats and opportunities. Models
  possible futures.],
  [Adaptation, strategic sensing],
  [#strong[S5]],
  [Policy / Identity],
  [Defines the organization’s identity, purpose, and ground rules.
  Balances S3 (stability) and S4 (adaptation).],
  [Organizational closure],
)
]

The critical homeostatic balance is the #strong[S3-S4 homeostat]: S3
wants stability and optimization of current operations; S4 wants
exploration and adaptation. S5 mediates this tension.

=== 7.2 Mapping to Workgraph
<mapping-to-workgraph>
#align(center)[#table(
  columns: 2,
  align: (col, row) => (auto,auto,).at(col),
  inset: 6pt,
  [VSM System], [Workgraph Equivalent],
  [#strong[S1 (Operations)]],
  [#strong[Agents] executing tasks. Each agent (role + objective) is a
  semi-autonomous operational unit.],
  [#strong[S2 (Coordination)]],
  [#strong[`blocked_by` dependency edges] and #strong[task status
  transitions]. These protocols prevent agents from clashing—an agent
  cannot start a task until dependencies are satisfied. The
  coordinator’s scheduling logic is S2.],
  [#strong[S3 (Control)]],
  [#strong[The coordinator] (`wg service start`). It allocates agents to
  tasks, monitors throughput, detects dead agents, and optimizes
  resource utilization across all S1 units.],
  [#strong[S3\* (Audit)]],
  [#strong[Rewards]. Sporadic, independent assessment of agent
  performance that bypasses normal task-completion reporting. The
  reward system provides a check that cannot be gamed by the agent
  reporting its own success.],
  [#strong[S4 (Intelligence)]],
  [#strong[The `evolve` mechanism]. It scans performance data (the
  "environment" of reward scores) for patterns and generates
  adaptations (new roles, modified objectives). Also: any human
  operator reviewing the graph and adding tasks based on environmental
  scanning.],
  [#strong[S5 (Policy)]],
  [#strong[Objectives] and #strong[project-level configuration]
  (CLAUDE.md, the root of the task tree). These define the ground rules
  under which all agents operate—what is acceptable, what is not, what
  the system’s identity and purpose are.],
  [#strong[Recursion]],
  [Workgraph’s task nesting. A high-level task can contain subtasks,
  each potentially a mini-viable-system with its own agents and
  coordination.],
)
]

=== 7.3 The S3-S4 Balance in Practice
<the-s3-s4-balance-in-practice>
In workgraph, the S3-S4 tension manifests as:

- #strong[S3 pull (stability)]: "Keep using the existing roles and
  objectives—they’re working fine. Optimize assignment. Don’t change
  what isn’t broken."
- #strong[S4 pull (adaptation)]: "The task landscape is changing. New
  types of work need new roles. Evolve the identity."

The human operator is S5, mediating this tension. The evolve mechanism’s
self-mutation safety guard (requiring human approval for changes to the
evolver’s own role) is the S5 function enforcing identity preservation.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 8. The Principal-Agent Problem
<the-principal-agent-problem>
=== 8.1 The Problem
<the-problem>
The principal-agent problem, formalized by Ross (1973) and Jensen &
Meckling (1976), arises when a #strong[principal] delegates work to an
#strong[agent] who has different interests and more information than the
principal.

Two core information asymmetries:

#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Problem], [Timing], [Description],
  [#strong[Adverse selection]],
  [Before delegation],
  [The agent has private information about their capabilities that the
  principal cannot observe. The principal may select the wrong agent.],
  [#strong[Moral hazard]],
  [After delegation],
  [The agent’s actions are not fully observable. The agent may cut
  corners or pursue private objectives.],
)
]

Identity costs (Jensen & Meckling 1976) \= Monitoring costs + Bonding
costs + Residual loss.

=== 8.2 Workgraph as an Identity Relationship
<workgraph-as-an-identity-relationship>
This mapping is unusually precise because workgraph literally has
primitives called "agents," "rewards," and "objectives"—the
vocabulary of identity theory.

#align(center)[#table(
  columns: 2,
  align: (col, row) => (auto,auto,).at(col),
  inset: 6pt,
  [Identity Theory Concept], [Workgraph Equivalent],
  [#strong[Principal]],
  [The human operator who defines the task graph and configures the
  identity],
  [#strong[Agent]],
  [The workgraph agent (role + objective)—literally named],
  [#strong[Delegation]],
  [`wg service start --max-agents N`—the principal delegates work to
  autonomous agents],
  [#strong[Moral hazard]],
  [The agent might produce low-quality output, hallucinate, or take
  shortcuts not visible from task completion status alone],
  [#strong[Adverse selection]],
  [Assigning the wrong role+objective pairing to a task—the agent
  lacks the capability, but this is not apparent until after execution],
  [#strong[Monitoring costs]],
  [#strong[Rewards]—the computational cost of assessing agent
  output quality after every task],
  [#strong[Bonding costs]],
  [#strong[Objectives]—the agent is constrained by its objective
  document to act in the principal’s interest. Acceptable/unacceptable
  tradeoffs are the bonding contract.],
  [#strong[Incentive alignment]],
  [#strong[The evolve mechanism]—agents that perform well have their
  patterns reinforced; agents that perform poorly are evolved or
  retired. This is performance-based selection.],
  [#strong[Residual loss]],
  [The gap between what the principal would produce and what the agent
  actually produces. Minimized by iterating reward→evolve.],
  [#strong[Repeated games]],
  [The reward history builds "reputation" that informs future
  assignment and evolution. Long-term relationships (many tasks by the
  same agent) build trust (`TrustLevel::Verified`).],
  [#strong[Screening]],
  [The coordinator’s auto-assign capability-matching: skills on the task
  matched against skills on the role.],
)
]

=== 8.3 Mechanism Design Implications
<mechanism-design-implications>
Identity theory suggests specific design principles for workgraph:

+ #strong[Invest in monitoring (rewards) proportional to risk.]
  High-stakes tasks deserve more thorough reward. Low-stakes tasks
  can be spot-checked.
+ #strong[Make bonding explicit.] The objective’s
  `unacceptable_tradeoffs` should list the specific failure modes the
  principal fears most. "Never skip tests" is a bonding clause.
+ #strong[Align incentives through evolution.] The evolve mechanism
  should explicitly reward the behaviors the principal values. If
  correctness matters more than speed, the reward rubric should
  weight correctness heavily (it does—40% by default).
+ #strong[Screen before delegating.] Auto-assign should match task
  skills against agent capabilities, not assign randomly. This reduces
  adverse selection.
+ #strong[Build trust incrementally.] New agents should start with
  low-stakes tasks. The `TrustLevel` field (Unknown → Provisional →
  Verified) formalizes this progression.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 9. Conway’s Law and the Inverse Conway Maneuver
<conways-law-and-the-inverse-conway-maneuver>
=== 9.1 Conway’s Law
<conways-law>
#quote(block: true)[
"Organizations which design systems are constrained to produce designs
which are copies of the communication structures of these
organizations."—Melvin Conway, "How Do Committees Invent?" (1968)
]

Conway’s argument: a system design is decomposed into parts, each
assigned to a team. The teams must communicate to integrate the parts.
Therefore, the interfaces between the system’s parts will mirror the
communication channels between the teams. This is a #emph[constraint],
not a choice.

=== 9.2 The Inverse Conway Maneuver
<the-inverse-conway-maneuver>
Coined by Jonny LeRoy and Matt Simons (2010): if org structure shapes
system architecture, then #strong[deliberately designing org structure
can drive desired system architecture]. Rather than accepting that your
system mirrors your org chart, you restructure teams to produce the
architecture you want.

=== 9.3 Mapping to Workgraph
<mapping-to-workgraph-1>
#align(center)[#table(
  columns: 2,
  align: (col, row) => (auto,auto,).at(col),
  inset: 6pt,
  [Conway’s Law Concept], [Workgraph Equivalent],
  [#strong[Organization structure]],
  [The set of roles and how they are assigned to agents],
  [#strong[Communication channels]],
  [`blocked_by` edges between tasks assigned to different roles],
  [#strong[System architecture]],
  [The task graph structure—how work is decomposed and connected],
  [#strong[Conway’s constraint]],
  [The task decomposition will mirror the role decomposition. If you
  have "frontend" and "backend" roles, you get tasks that split along
  that boundary, producing a system with that split.],
  [#strong[Inverse Conway Maneuver]],
  [Deliberately designing roles and objectives to produce the desired
  task decomposition (and therefore system architecture)],
)
]

=== 9.4 The Inverse Conway Maneuver in Practice
<the-inverse-conway-maneuver-in-practice>
#strong[Example: Microservices via roles]

If you want a microservices architecture, define one role per service
domain: - `user-service-developer` role - `payment-service-developer`
role - `notification-service-developer` role

Tasks will naturally be decomposed along service boundaries, and the
resulting code will have clean service interfaces—because the
dependency edges between tasks assigned to different roles become the
API contracts between services.

#strong[Example: Monolith via cross-cutting roles]

If you want a cohesive monolith, define cross-cutting roles: -
`backend-developer` role (handles all backend work) -
`frontend-developer` role (handles all frontend work)

Tasks will be decomposed by layer, not by domain, producing a layered
monolith.

The profound implication: #strong[in workgraph, the role ontology IS the
org chart, and the task graph IS the system architecture.] Conway’s Law
predicts they will converge. The Inverse Conway Maneuver says: design
the roles first, and the task graph (and resulting code) will follow.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 10. Team Topologies
<team-topologies>
=== 10.1 The Framework
<the-framework>
Team Topologies (Skelton & Pais, 2019) provides a practical framework
for organizing technology teams, built on Conway’s Law and cognitive
load theory.

#strong[Four team types:]

#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Team Type], [Purpose], [Cognitive Load Strategy],
  [#strong[Stream-aligned]],
  [Aligned to a single valuable stream of work (product, service, user
  journey). The primary type—most teams should be this.],
  [Owns and delivers end-to-end; minimizes handoffs],
  [#strong[Platform]],
  [Provides internal services that accelerate stream-aligned teams.
  Treats offerings as products with internal customers.],
  [Reduces cognitive load of other teams by providing self-service
  capabilities],
  [#strong[Enabling]],
  [Specialists who help stream-aligned teams acquire missing
  capabilities. Cross-cuts multiple teams.],
  [Temporarily increases capability of other teams, then steps back],
  [#strong[Complicated-subsystem]],
  [Maintains a part of the system requiring heavy specialist knowledge
  (ML model, codec, financial engine).],
  [Isolates specialist knowledge so others don’t need it],
)
]

#strong[Three interaction modes:]

#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Mode], [Description], [Duration],
  [#strong[Collaboration]],
  [Two teams work closely together for a defined period (joint
  exploration). High bandwidth.],
  [Temporary (weeks)],
  [#strong[X-as-a-Service]],
  [One team provides, another consumes, with a clear API/contract. Low
  overhead.],
  [Ongoing (steady-state)],
  [#strong[Facilitating]],
  [One team helps and mentors another. One-way knowledge transfer.],
  [Temporary (until capability transferred)],
)
]

=== 10.2 Mapping to Workgraph Roles
<mapping-to-workgraph-roles>
#align(center)[#table(
  columns: 2,
  align: (col, row) => (auto,auto,).at(col),
  inset: 6pt,
  [Team Topologies Concept], [Workgraph Equivalent],
  [#strong[Stream-aligned team]],
  [A role assigned to a stream of related tasks. The "default" role
  type.],
  [#strong[Platform team]],
  [A role whose tasks produce shared infrastructure that unblocks other
  agents’ tasks. Platform tasks appear as `blocked_by` dependencies for
  stream-aligned tasks.],
  [#strong[Enabling team]],
  [A role whose tasks improve other roles/agents—writing
  documentation, creating templates, establishing patterns. Maps
  naturally to the evolve mechanism.],
  [#strong[Complicated-subsystem team]],
  [A role with specialized capabilities, assigned to tasks that other
  agents should not attempt.],
  [#strong[Collaboration mode]],
  [Two agents sharing `blocked_by` edges on overlapping tasks during a
  discovery phase.],
  [#strong[X-as-a-Service mode]],
  [Clean `blocked_by` edges: platform tasks complete, stream-aligned
  tasks consume their outputs.],
  [#strong[Facilitating mode]],
  [An enabling agent’s tasks are prerequisites for another agent’s
  improvement.],
  [#strong[Cognitive load]],
  [The number and complexity of tasks assigned to a single agent.
  Overload signals the need for role decomposition.],
  [#strong[Team API]],
  [The interface between roles—defined by what outputs one role
  produces that another role’s tasks consume.],
)
]

=== 10.3 Practical Guidance for Workgraph Users
<practical-guidance-for-workgraph-users>
+ #strong[Most roles should be stream-aligned.] If you have a "build the
  feature" type of work, that’s stream-aligned. Don’t over-specialize.
+ #strong[Create platform roles for shared infrastructure.] If multiple
  stream-aligned agents need the same tooling/setup, create a platform
  role whose tasks they all depend on.
+ #strong[Use enabling roles sparingly.] An "evolver" that reviews the
  identity and proposes improvements is an enabling role. It shouldn’t
  exist permanently—it should work itself out of a job.
+ #strong[Complicated-subsystem roles protect cognitive load.] If a task
  requires deep ML expertise, create a specialized role rather than
  expecting a general-purpose role to handle it.
+ #strong[Interaction modes evolve.] Two agents might collaborate on
  initial exploration, then shift to X-as-a-Service once interfaces
  stabilize. The task graph structure should reflect this evolution.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 11. Organizational Theory Primitives
<organizational-theory-primitives>
=== 11.1 Division of Labor
<division-of-labor>
Adam Smith’s pin factory (1776): splitting work into specialized steps
increases productivity. In workgraph, this maps to:

- #strong[Task decomposition]: Breaking a large task into smaller
  subtasks, each with a specific focus
- #strong[Role specialization]: Defining roles with narrow skill sets
  (analyst, implementer, reviewer) rather than one generalist role
- #strong[The pipeline pattern]: Sequential stages of specialized work
  (Section 4)

The tradeoff: over-specialization increases coordination costs (more
`blocked_by` edges, more handoffs, more potential for misalignment).
This is the fundamental tension in organizational design, and it applies
directly to workgraph identity design.

=== 11.2 Span of Control
<span-of-control>
The number of subordinates a manager can effectively supervise. In
workgraph: the number of agents a single coordinator tick can
effectively manage. The `max_agents` parameter is the span of control.

Research suggests 5-9 direct reports as optimal for human managers
(Urwick, 1956). For workgraph, the constraint is computational: how many
agents can the coordinator monitor, reward, and evolve without losing
oversight quality.

=== 11.3 Coordination Costs
<coordination-costs>
Every dependency edge (`blocked_by`) is a coordination point. The total
coordination cost of a task graph scales with the number of edges, not
the number of tasks. This connects to Brooks’s Law: "Adding manpower to
a late software project makes it later"—because the number of
communication channels grows as n(n-1)/2 with n participants.

In workgraph terms: adding more agents (higher `max_agents`) only helps
if the task graph has enough parallelism to exploit. If the graph is a
serial chain, more agents are wasted. If the graph is a wide diamond
(fork-join), more agents directly increase throughput—up to the point
where coordination overhead dominates.

=== 11.4 Transaction Cost Economics
<transaction-cost-economics>
Oliver Williamson’s Transaction Cost Economics (1975, 1985) asks: when
should work be done inside the organization ("make") vs. outside
("buy")? The answer depends on:

#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Factor], [Favors "Make" (Internal)], [Favors "Buy" (External)],
  [#strong[Asset specificity]],
  [High (specialized knowledge needed)],
  [Low (commodity work)],
  [#strong[Uncertainty]],
  [High (requirements change frequently)],
  [Low (well-defined)],
  [#strong[Frequency]],
  [High (recurring work)],
  [Low (one-off)],
)
]

In workgraph, this maps to the choice between: - #strong[Internal
agents] (identity-defined roles with specialized capabilities) for
recurring, high-specificity work - #strong[External agents] (human
operators, one-off shell executors) for infrequent, well-defined tasks

The `executor` field on agents (`claude`, `matrix`, `email`, `shell`)
represents this make-vs-buy boundary.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 12. Synthesis: Cross-Cutting Connections
<synthesis-cross-cutting-connections>
These frameworks are not independent—they are deeply interconnected
views of the same underlying organizational dynamics.

=== 12.1 The Grand Unification
<the-grand-unification>
```
                    STRUCTURE                        DYNAMICS
                    --------                         --------
Conway's Law ───── Role ontology ─────────── Inverse Conway Maneuver
                       │                            │
Team Topologies ── Team types ────────────── Interaction mode evolution
                       │                            │
Workflow Patterns ─ blocked_by edges ────── Coordinator dispatch logic
                       │                            │
Division of Labor ─ Task decomposition ──── Pipeline & Fork-Join
                       │                            │
                       ▼                            ▼
                   THE TASK GRAPH              THE EVOLVE LOOP
                       │                            │
Stigmergy ──────── Shared medium ──────────── Self-organizing traces
                       │                            │
Identity Theory ──── Principal delegates ────── Monitor + Evolve = Alignment
                       │                            │
Cybernetics ────── Feedback loops ─────────── Requisite Variety
                       │                            │
VSM ────────────── S1-S5 hierarchy ────────── S3-S4 balance
                       │                            │
Autopoiesis ────── Self-production ────────── Operational closure
```

=== 12.2 Key Structural Identities
<key-structural-identities>
Several deep identities connect these frameworks:

+ #strong[VSM + Cybernetics + OODA]: Beer’s VSM is explicitly
  cybernetic. S3 is a negative feedback regulator; S4 is the
  Observe/Orient function; S5 is the governing variable for double-loop
  learning. The coordinator’s OODA loop IS the S3 regulation cycle.

+ #strong[Stigmergy + Autopoiesis]: Both describe systems that maintain
  themselves without central control. Stigmergy is the #emph[mechanism]
  (indirect coordination through traces); autopoiesis is the
  #emph[property] (self-production). A stigmergic system that produces
  its own traces is autopoietic.

+ #strong[Conway’s Law + Team Topologies + Resource Patterns]: Conway’s
  Law is the theoretical prediction; Team Topologies is the practical
  prescription; Workflow Resource Patterns are the formal specification.
  All three say: #emph[how you assign people to work determines the
  structure of what gets built.]

+ #strong[Principal-Agent + Rewards + Cybernetics]: Identity theory
  identifies the #emph[problem] (misaligned incentives under information
  asymmetry); cybernetics provides the #emph[solution architecture]
  (feedback loops); rewards are the #emph[implementation] of both
  monitoring (identity theory) and negative feedback (cybernetics).

+ #strong[Fork-Join + Workflow Patterns + `blocked_by`]: Fork-Join is
  the computational realization of WCP2+WCP3, which are the two most
  fundamental `blocked_by` graph topologies.

+ #strong[Autopoiesis + Evolve + Double-Loop Learning]: The
  execute→reward→evolve→execute cycle is simultaneously autopoietic
  (self-producing), double-loop (questioning governing variables), and
  cybernetic (feedback-driven regulation). This is the single most
  theoretically dense primitive in workgraph.

#line(length: 100%, stroke: 0.5pt + luma(180))

== 13. Practical Recommendations
<practical-recommendations>
=== 13.1 Identity Design Checklist
<identity-design-checklist>
Based on the theoretical frameworks above, here is a checklist for
designing a workgraph identity:

#align(center)[#table(
  columns: 4,
  align: (col, row) => (auto,auto,auto,auto,).at(col),
  inset: 6pt,
  [Step], [Framework], [Question], [Action],
  [1],
  [Conway’s Law],
  [What system architecture do I want?],
  [Design roles to mirror the desired decomposition],
  [2],
  [Requisite Variety],
  [Do I have enough roles for the variety of tasks?],
  [Count task types, ensure ≥1 role per type],
  [3],
  [Team Topologies],
  [Which role is stream-aligned? Platform? Enabling?],
  [Label roles by type; most should be stream-aligned],
  [4],
  [Division of Labor],
  [How fine-grained should specialization be?],
  [Balance specialization against coordination cost],
  [5],
  [Principal-Agent],
  [What failure modes do I fear most?],
  [Encode them in objectives as `unacceptable_tradeoffs`],
  [6],
  [Cybernetics],
  [Is the feedback loop working?],
  [Enable auto-reward; run evolve periodically],
  [7],
  [VSM S3-S4],
  [Am I balancing stability and adaptation?],
  [Don’t evolve too often (S3) or too rarely (S4)],
)
]

=== 13.2 Pattern Selection Guide
<pattern-selection-guide>
#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Situation], [Pattern], [Workgraph Expression],
  [Sequential specialized stages],
  [Pipeline (Section 4)],
  [Serial `blocked_by` chain, different roles per stage],
  [Independent parallelizable work],
  [Fork-Join (Section 3)],
  [Fan-out from planner, fan-in to synthesizer],
  [Data-parallel analysis],
  [MapReduce (Section 3)],
  [Planner decomposes → N workers → reducer aggregates],
  [Heterogeneous parallel review],
  [Scatter-Gather (Section 3)],
  [Multiple reviewer roles examine same artifact],
  [Iterative refinement],
  [Structured Loop (Section 2)],
  [`loops_to` with guard and max\_iterations],
  [Recurring process],
  [Autopoietic cycle (Section 5)],
  [`loops_to` chain forming a full cycle],
)
]

=== 13.3 Anti-Patterns
<anti-patterns>
#align(center)[#table(
  columns: 4,
  align: (col, row) => (auto,auto,auto,auto,).at(col),
  inset: 6pt,
  [Anti-Pattern], [Theory Violated], [Symptom], [Fix],
  [One role for all tasks],
  [Requisite Variety],
  [Low scores on specialized tasks],
  [Add specialized roles],
  [Too many roles],
  [Parsimony / coordination cost],
  [Roles with zero tasks; confusion in assignment],
  [Retire unused roles],
  [No rewards],
  [Identity Theory (no monitoring)],
  [Quality drift; no evolution signal],
  [Enable auto-reward],
  [Evolving every cycle],
  [VSM S3-S4 imbalance],
  [Instability; roles changing faster than agents can adapt],
  [Evolve periodically, not continuously],
  [Serial pipeline where fork-join fits],
  [Division of Labor mismatch],
  [Slow throughput on parallelizable work],
  [Decompose into parallel tasks],
  [Monolithic tasks],
  [No division of labor],
  [Single agent bottleneck; no parallelism],
  [Break into subtasks with dependencies],
  [Circular `blocked_by` without `loops_to`],
  [Workflow Patterns (unbounded cycle)],
  [Deadlock or infinite loop],
  [Use `loops_to` with guards and max\_iterations],
)
]

#line(length: 100%, stroke: 0.5pt + luma(180))

== 14. Appendix: Comparative Tables
<appendix-comparative-tables>
=== Framework-to-Primitive Mapping
<framework-to-primitive-mapping>
#align(center)[#table(
  columns: 10,
  align: (col, row) => (auto,auto,auto,auto,auto,auto,auto,auto,auto,auto,).at(col),
  inset: 6pt,
  [Framework], [Tasks], [`blocked_by`], [`loops_to`], [Roles],
  [Objectives], [Agents], [Coordinator], [Rewards], [Evolve],
  [Stigmergy],
  [Traces in environment],
  [Sematectonic coordination],
  [Feedback trail],
  [—],
  [—],
  [Stimulated actors],
  [—],
  [Marker traces],
  [Self-reinforcing trails],
  [Workflow Patterns],
  [Activities],
  [Control-flow edges],
  [Back-edges (WCP10/21)],
  [Resource roles (WRP2)],
  [—],
  [Resources],
  [Engine],
  [—],
  [—],
  [Fork-Join/MapReduce],
  [Map/fork units],
  [Join barriers],
  [—],
  [—],
  [—],
  [Worker threads],
  [Scheduler],
  [—],
  [—],
  [Autopoiesis],
  [Momentary events],
  [Process network],
  [Self-production cycle],
  [System components],
  [System boundary],
  [Environment],
  [—],
  [Cognition],
  [Self-production],
  [Cybernetics],
  [System states],
  [Causal chains],
  [Feedback loops],
  [Regulator variety],
  [Constraints],
  [Regulated units],
  [Regulator (OODA)],
  [Feedback signal],
  [Variety amplification],
  [VSM],
  [S1 operations],
  [S2 coordination],
  [S3 audit cycle],
  [S1 capabilities],
  [S5 policy],
  [S1 units],
  [S3 control],
  [S3\* audit],
  [S4 intelligence],
  [Identity Theory],
  [Delegated work],
  [Contract terms],
  [Repeated games],
  [Agent type],
  [Bonding contract],
  [Agent],
  [Principal],
  [Monitoring],
  [Incentive alignment],
  [Conway’s Law],
  [System components],
  [Interfaces],
  [—],
  [Team capabilities],
  [—],
  [Teams],
  [—],
  [—],
  [Inverse Conway],
  [Team Topologies],
  [Work streams],
  [Team interactions],
  [—],
  [Team types],
  [—],
  [Teams],
  [—],
  [—],
  [Topology evolution],
  [Org Theory],
  [Labor units],
  [Coordination channels],
  [—],
  [Specializations],
  [Values],
  [Workers],
  [Manager],
  [Performance review],
  [Restructuring],
)
]

=== Theoretical Density of Workgraph Primitives
<theoretical-density-of-workgraph-primitives>
#align(center)[#table(
  columns: 3,
  align: (col, row) => (auto,auto,auto,).at(col),
  inset: 6pt,
  [Primitive], [Frameworks That Map To It], [Theoretical "Load"],
  [#strong[Tasks]],
  [All 10 frameworks],
  [The universal unit of work],
  [#strong[`blocked_by`]],
  [Workflow Patterns, Fork-Join, Stigmergy, Conway’s Law, Coordination
  Costs],
  [The structural backbone],
  [#strong[`loops_to`]],
  [Workflow Patterns (WCP10/21), Cybernetics (feedback), Autopoiesis
  (self-production), Identity Theory (repeated games)],
  [Enables dynamics],
  [#strong[Roles]],
  [Team Topologies, Conway’s Law, Resource Patterns, VSM (S1), Requisite
  Variety, Division of Labor],
  [The competency model],
  [#strong[Objectives]],
  [Identity Theory (bonding), VSM (S5 policy), Cybernetics (constraints)],
  [The value system],
  [#strong[Agents]],
  [Identity Theory (literally), Stigmergy (stimulated actors), VSM (S1
  units), Team Topologies (teams)],
  [The executing entity],
  [#strong[Coordinator]],
  [Cybernetics (regulator), VSM (S3), OODA Loop, Identity Theory
  (principal’s delegate)],
  [The control system],
  [#strong[Rewards]],
  [Identity Theory (monitoring), Cybernetics (feedback signal), VSM (S3\*
  audit), Autopoiesis (cognition)],
  [The sensing mechanism],
  [#strong[Evolve]],
  [Autopoiesis (self-production), Cybernetics (double-loop learning,
  variety amplification), VSM (S4 intelligence), Identity Theory
  (incentive alignment)],
  [The adaptation mechanism],
)
]

#line(length: 100%, stroke: 0.5pt + luma(180))

== 15. Sources
<sources>
=== Organizational Theory
<organizational-theory>
- Smith, A. (1776). #emph[An Inquiry into the Nature and Causes of the
  Wealth of Nations]. Book I, Chapter 1: "Of the Division of Labour."
- Simon, H.A. (1947). #emph[Administrative Behavior]. Macmillan.
  \[Bounded rationality, satisficing\]
- Williamson, O.E. (1975). #emph[Markets and Hierarchies]. Free Press.
  \[Transaction cost economics\]
- Williamson, O.E. (1985). #emph[The Economic Institutions of
  Capitalism]. Free Press.
- Urwick, L.F. (1956). "The Manager’s Span of Control." #emph[Harvard
  Business Review], 34(3), 39-47.
- Brooks, F.P. (1975). #emph[The Mythical Man-Month]. Addison-Wesley.

=== Workflow Patterns
<workflow-patterns>
- van der Aalst, W.M.P., ter Hofstede, A.H.M., Kiepuszewski, B., &
  Barros, A.P. (2003). "Workflow Patterns." #emph[Distributed and
  Parallel Databases], 14(1), 5-51.
- Russell, N., ter Hofstede, A.H.M., van der Aalst, W.M.P., & Mulyar, N.
  (2006). "Workflow Control-Flow Patterns: A Revised View." BPM Center
  Report BPM-06-22.
- Russell, N., van der Aalst, W.M.P., & ter Hofstede, A.H.M. (2016).
  #emph[Workflow Patterns: The Definitive Guide]. MIT Press.
- Russell, N., ter Hofstede, A.H.M., Edmond, D., & van der Aalst, W.M.P.
  (2005). "Workflow Resource Patterns." In #emph[Advanced Information
  Systems Engineering (CAiSE)], Springer.

=== Parallel Decomposition
<parallel-decomposition>
- Dean, J. & Ghemawat, S. (2004). "MapReduce: Simplified Data Processing
  on Large Clusters." #emph[OSDI ’04], 137-150.
- Lea, D. (2000). "A Java Fork/Join Framework." #emph[ACM Java Grande
  Conference], 36-43.
- Hohpe, G. & Woolf, B. (2003). #emph[Enterprise Integration Patterns].
  Addison-Wesley.
- Conway, M.E. (1963). "A Multiprocessor System Design." #emph[AFIPS
  Fall Joint Computer Conference]. \[The original fork-join concept\]
- Blumofe, R.D. & Leiserson, C.E. (1999). "Scheduling Multithreaded
  Computations by Work Stealing." #emph[JACM], 46(5), 720-748.

=== Stigmergy
<stigmergy>
- Grassé, P.-P. (1959). "La reconstruction du nid et les coordinations
  interindividuelles chez Bellicositermes natalensis et Cubitermes sp."
  #emph[Insectes Sociaux], 6(1), 41-80.
- Theraulaz, G. & Bonabeau, E. (1999). "A Brief History of Stigmergy."
  #emph[Artificial Life], 5(2), 97-116.
- Heylighen, F. (2016). "Stigmergy as a Universal Coordination Mechanism
  I: Definition and Components." #emph[Cognitive Systems Research], 38,
  4-13.
- Heylighen, F. (2016). "Stigmergy as a Universal Coordination Mechanism
  II: Varieties and Evolution." #emph[Cognitive Systems Research], 38,
  50-59.
- Elliott, M. (2006). "Stigmergic Collaboration: The Evolution of Group
  Work." #emph[M/C Journal], 9(2).
- Bolici, F., Howison, J., & Crowston, K. (2016). "Stigmergic
  Coordination in FLOSS Development Teams." #emph[Cognitive Systems
  Research], 38, 14-22.

=== Autopoiesis
<autopoiesis>
- Maturana, H.R. & Varela, F.J. (1972/1980). #emph[Autopoiesis and
  Cognition: The Realization of the Living]. D. Reidel.
- Varela, F.J., Maturana, H.R., & Uribe, R. (1974). "Autopoiesis: The
  Organization of Living Systems." #emph[BioSystems], 5(4), 187-196.
- Maturana, H.R. & Varela, F.J. (1987). #emph[The Tree of Knowledge].
  Shambhala.
- Luhmann, N. (1984/1995). #emph[Social Systems]. Stanford University
  Press.
- Mingers, J. (2002). "Can Social Systems Be Autopoietic?"
  #emph[Sociological Review], 50(2), 278-299.

=== Cybernetics
<cybernetics>
- Wiener, N. (1948). #emph[Cybernetics: Or Control and Communication in
  the Animal and the Machine]. MIT Press.
- Ashby, W.R. (1956). #emph[An Introduction to Cybernetics]. Chapman &
  Hall.
- Ashby, W.R. (1958). "Requisite Variety and Its Implications for the
  Control of Complex Systems." #emph[Cybernetica], 1(2), 83-99.
- Boyd, J. (1976/1986). "Patterns of Conflict." \[Unpublished briefing\]
- von Foerster, H. (1974). #emph[Cybernetics of Cybernetics]. University
  of Illinois.
- Argyris, C. & Schön, D.A. (1978). #emph[Organizational Learning: A
  Theory of Action Perspective]. Addison-Wesley.
- Beer, S. (1959). #emph[Cybernetics and Management]. English
  Universities Press.

=== Viable System Model
<viable-system-model>
- Beer, S. (1972). #emph[Brain of the Firm]. Allen Lane / Penguin Press.
- Beer, S. (1979). #emph[The Heart of Enterprise]. Wiley.
- Beer, S. (1985). #emph[Diagnosing the System for Organizations].
  Wiley.
- Espejo, R. & Harnden, R. (1989). #emph[The Viable System Model:
  Interpretations and Applications]. Wiley.

=== Principal-Agent Theory
<principal-agent-theory>
- Ross, S.A. (1973). "The Economic Theory of Identity: The Principal’s
  Problem." #emph[AER], 63(2), 134-139.
- Jensen, M.C. & Meckling, W.H. (1976). "Theory of the Firm: Managerial
  Behavior, Identity Costs and Ownership Structure." #emph[JFE], 3(4),
  305-360.
- Holmström, B. (1979). "Moral Hazard and Observability." #emph[Bell
  Journal of Economics], 10(1), 74-91.
- Eisenhardt, K.M. (1989). "Identity Theory: An Assessment and Review."
  #emph[AMR], 14(1), 57-74.
- Laffont, J.-J. & Martimort, D. (2002). #emph[The Theory of Incentives:
  The Principal-Agent Model]. Princeton University Press.

=== Conway’s Law
<conways-law-1>
- Conway, M.E. (1968). "How Do Committees Invent?" #emph[Datamation],
  14(4), 28-31.
- LeRoy, J. & Simons, M. (2010). "The Inverse Conway Maneuver."
  #emph[Cutter IT Journal], 23(12).
- MacCormack, A., Rusnak, J., & Baldwin, C. (2012). "Exploring the
  Duality between Product and Organizational Architectures."
  #emph[Research Policy], 41(8), 1309-1324.

=== Team Topologies
<team-topologies-1>
- Skelton, M. & Pais, M. (2019). #emph[Team Topologies: Organizing
  Business and Technology Teams for Fast Flow]. IT Revolution Press.

=== Theory of Constraints
<theory-of-constraints-1>
- Goldratt, E.M. (1984). #emph[The Goal]. North River Press.
