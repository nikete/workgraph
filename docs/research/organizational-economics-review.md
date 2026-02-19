# Organizational Economics and the Design of Workgraph + VX

A review of the economics and organizational theory literature relevant to the
workgraph (wg) identity/evolution system and the Veracity Exchange (VX), with
connections to system design and flagged naming mismatches.

---

## 1. Principal-Agent Theory and the Identity System

### The literature

The principal-agent problem (Jensen & Meckling, 1976; Hart & Holmstrom, 1987)
arises when a principal delegates work to an agent whose actions are
unobservable or unverifiable. Two canonical problems emerge: **adverse
selection** (the agent has private information about their type before
contracting) and **moral hazard** (the agent's effort is unobservable after
contracting).

Holmstrom's (1979) **Informativeness Principle** states that any performance
measure that reveals information about the agent's effort should be included in
the compensation contract, including relative performance evaluation that
filters out common noise.

### How wg implements this

The `Objective` struct is the principal's **behavioral contract**: it specifies
acceptable tradeoffs (negotiable latitude) and unacceptable tradeoffs (hard
constraints). This contract is injected into the agent's prompt at dispatch
time. The `Reward` struct is the principal's **performance signal**: a
four-dimensional score (correctness, completeness, efficiency,
style_adherence) that decomposes aggregate performance into informative
components, consistent with the Informativeness Principle.

The architecture maps cleanly onto the PA framework:

| PA concept | wg implementation |
|---|---|
| Principal | The human who defines roles, objectives, and triggers evolution |
| Agent | The `Agent` entity (role x objective pairing), executed by an LLM |
| Contract | `Objective.acceptable_tradeoffs` + `Objective.unacceptable_tradeoffs` |
| Effort | The LLM's work on a task (unobservable process, observable output) |
| Performance signal | `Reward { value, dimensions, notes }` |
| Relative evaluation | Synergy matrix comparing (role, objective) pair performance |

### Design implications

The four-dimensional reward decomposition is well-motivated by Holmstrom
(1979): each dimension (correctness, completeness, efficiency,
style_adherence) reveals independent information about which aspect of the
agent's "effort" was strong or weak. The synergy matrix provides the **relative
performance evaluation** that the Informativeness Principle recommends --
comparing how the same role performs with different objectives (and vice versa)
filters out entity-specific noise.

**Gap:** The current system has no mechanism for the agent to signal private
information back to the principal about task difficulty or ambiguity before
committing effort. In classical PA theory, menu contracts (offering multiple
effort-reward bundles) enable screening under adverse selection. A lightweight
analog might allow agents to report estimated difficulty during the claim phase,
enabling the evolution system to condition rewards on ex-ante difficulty
estimates.

---

## 2. Multi-task Incentives (Holmstrom & Milgrom, 1991)

### The literature

Holmstrom & Milgrom (1991) show that when an agent performs multiple tasks,
incentive pay serves not only to motivate effort but to **direct the allocation
of attention** among tasks. If only some task dimensions are measurable, strong
incentives on the measurable dimension cause the agent to neglect the
unmeasurable one ("teaching to the test"). The paper proves that **fixed wages
can be optimal** when measurement is unbalanced across tasks, and that **job
design** (restricting which tasks are bundled into a single role) is itself an
incentive instrument.

### Connection to wg

The wg identity system implements Holmstrom-Milgrom job design directly:

1. **Role as job design.** A `Role` bundles a specific set of skills and a
   desired outcome. Instead of giving every agent every possible task, the role
   narrows scope -- exactly the "restricting the task set" that H&M recommend
   as an incentive tool.

2. **Balanced multi-dimensional rewards.** The four reward dimensions with
   explicit weights (40/30/15/15) are a deliberate attempt to prevent
   "teaching to the test." If only `correctness` were rewarded, agents would
   neglect `style_adherence`. The dimensional breakdown makes the
   measurement less unbalanced.

3. **Objective as behavioral constraint.** The `unacceptable_tradeoffs` field
   on `Objective` is the mechanism for preventing agents from sacrificing
   unmeasured dimensions for measured ones. If "skipping tests" is an
   unacceptable tradeoff, the evaluator checks for it even if the test suite
   isn't in the measured artifacts.

### Design implications

H&M's key warning is that **incentive intensity should be balanced across
tasks**. The current 40/30/15/15 weighting is a reasonable first approximation
but is hardcoded. The literature suggests this should be adaptive: if the
evolution system observes that agents consistently score high on correctness but
low on efficiency, the weights should shift to rebalance attention. The
`objective-tuning` evolution strategy could naturally encompass weight
adjustment.

**Teaching to the test risk:** LLM evaluators can only score what they can
observe in artifacts and logs. If an agent produces correct, complete output but
via an expensive, wasteful process that happens to be invisible in the
artifacts, `efficiency` will be mismeasured. VX's outcome-based rewards
(`source: "outcome:..."`) are the natural correction: real-world outcomes
reveal whether the efficient-looking output was genuinely efficient.

---

## 3. The Knowledge Problem (Hayek, 1945)

### The literature

Hayek's "The Use of Knowledge in Society" (1945) argues that the knowledge
relevant to economic coordination is dispersed, local, and often tacit.
Centralized planning fails because no single authority can collect and process
all relevant information. The **price system** succeeds because it aggregates
distributed knowledge into compact signals that coordinate decentralized
action without anyone needing to understand the full picture.

### Connection to wg and VX

The workgraph coordinator is deliberately **not** a central planner. It does
not decide which agent should work on which task (that is delegated to the
assigner agent), does not decide how to evaluate work (that is delegated to the
evaluator agent), and does not decide how to improve identities (that is
delegated to the evolver agent). The coordinator's role is Hayekian: it
maintains the **infrastructure for decentralized coordination** (the task
graph, the identity registry, the reward records) without making substantive
decisions.

The `Reward.value` field functions as a **local price signal**: a compact
scalar summary of a complex evaluation that allows downstream decisions
(assignment, evolution) without requiring the decision-maker to understand the
full evaluation context. The synergy matrix extends this to a
two-dimensional price surface over the (role x objective) identity space.

VX pushes this further toward Hayek's vision. The `source` field on `Reward`
means that rewards can come from **dispersed, heterogeneous sources** -- LLM
judgments, real-world outcomes, manual human assessments, peer suggestions --
each contributing local knowledge that no single evaluator possesses. The
peer exchange mechanism proposed in VX is essentially a **market for
improvement suggestions**, where credibility (the `PeerCredibility` struct)
serves as Hayek's price: a compact signal about a peer's reliability that
allows trust decisions without full information.

### Design implication

The architecture already embodies Hayekian decentralization well. One gap:
there is no **feedback channel from agents to the principal about task
specification quality**. Hayek emphasizes that knowledge flows in both
directions. Currently, if a task description is ambiguous or the required
skills are mislabeled, the agent has no structured way to signal this. A
"task quality" feedback signal from agents could be a valuable information
channel -- analogous to how prices convey supply-side information to buyers.

---

## 4. Team Production and Monitoring (Alchian & Demsetz, 1972)

### The literature

Alchian & Demsetz (1972) argue that firms exist because **team production** --
where the joint output exceeds the sum of individual outputs -- creates a
**metering problem**: individual contributions cannot be cleanly separated from
collective output. The solution is a **residual-claimant monitor** who
specializes in observing individual effort and is paid from the surplus their
monitoring extracts. The monitor is the manager; the manager's incentive to
monitor well comes from their residual claim.

### Connection to wg

The evaluator agent in wg is precisely the Alchian-Demsetz monitor. It
specializes in observing and scoring individual agent output. The evaluator
does not do the productive work itself -- it reads artifacts, logs, and context,
then produces a structured assessment.

The key design question Alchian & Demsetz raise is: **who monitors the
monitor?** In wg, the evaluator is itself an identity entity with a role and
objective, and the manual states that it "too can be rewarded" and "too can
be evolved." This creates a **recursive monitoring structure** where the
evolver can propose improvements to the evaluator's role. However, there is
currently no independent evaluation of evaluator quality -- the system relies
on the human principal noticing miscalibrated evaluations.

VX's outcome-based rewards provide a natural solution: if the evaluator gives
high internal rewards to work that performs poorly on real-world outcomes, the
divergence between `source: "llm"` rewards and `source: "outcome:..."` rewards
reveals evaluator miscalibration. This is precisely the "monitoring the
monitor" function that the literature says is necessary.

### Design implication

The current system handles the metering problem for individual tasks well.
The gap is in **team production** -- tasks that genuinely require multiple
agents to collaborate. When two agents work on related tasks in a dependency
chain, the downstream agent benefits from (or is harmed by) the upstream
agent's output. The current reward system scores each agent individually on
their own task, but cannot attribute the downstream agent's performance to
upstream quality. The `context_id` field on `RewardRef` records which
objective was paired with a role, but not which *other agents* contributed
to the task's input context.

---

## 5. Tournament Theory (Lazear & Rosen, 1981)

### The literature

Lazear & Rosen (1981) show that rank-order tournaments -- where pay depends on
ordinal rank rather than absolute output -- can achieve the same efficiency as
piece-rate compensation. Tournaments are preferred when **absolute output is
hard to measure but relative ranking is easy**. The key variable is the **prize
spread**: larger spreads induce more effort but also more risk-taking.

Tournaments have a significant downside: they encourage **selfish behavior**
and discourage knowledge sharing, since other agents are competitors.

### Connection to wg

The wg evolution system implements tournament selection explicitly. The
retirement strategy selects the lowest-performing entities for removal, and
the mutation/crossover strategies favor high-performing entities as parents.
The synergy matrix is a leaderboard that ranks (role, objective) pairings.

However, wg's implementation is closer to **evolutionary tournament selection**
than to Lazear-Rosen labor tournaments:

| Lazear-Rosen | wg evolution |
|---|---|
| Workers compete for a fixed prize | Identities compete for survival/reproduction |
| Winner gets promoted | High-performing identity gets mutated/crossed to produce offspring |
| Loser gets nothing | Low-performing identity gets retired |
| Workers know they are competing | LLM agents have no awareness of competition |
| Selfish behavior is a risk | Not applicable -- LLM agents cannot strategically shirk |

The last point is crucial: the strategic behavior problems that Lazear & Rosen
worry about (sabotage, knowledge hoarding, excessive risk-taking) do not arise
when agents are LLMs executing prompt-injected identities. LLM agents cannot
strategically manipulate the tournament. This makes tournament-style selection
**safer in the AI agent setting** than in human organizations.

### Design implication

For the VX peer exchange, tournament concerns do re-emerge. Human peers
participating in the improvement marketplace *can* behave strategically:
withholding good suggestions, sabotaging competitors' credibility scores, or
free-riding on others' contributions. The VX trust market design should
account for the Lazear-Rosen findings on tournament pathologies in human
settings.

---

## 6. Evolutionary Economics (Nelson & Winter, 1982)

### The literature

Nelson & Winter's *An Evolutionary Theory of Economic Change* (1982)
reconceptualizes firms as collections of **organizational routines** -- the
economic analog of genes. Routines are:
- **Repetitive** (stable behavioral patterns)
- **Heritable** (passed from one generation of the organization to the next)
- **Subject to variation** (through search and learning)
- **Subject to selection** (through profitability-based competition)

Crucially, Nelson & Winter describe their evolutionary process as
**Lamarckian**: organizations can inherit "acquired characters" (learned
improvements), and variation arises partly in response to adversity (directed
search, not random mutation).

### Connection to wg

The wg identity system is a remarkably direct implementation of Nelson &
Winter's framework:

| Nelson & Winter concept | wg implementation |
|---|---|
| Organizational routine | `Role` (description + skills + desired_outcome) |
| Gene / hereditable information | Content-hash ID (SHA-256 of identity-defining fields) |
| Organism / vehicle | `Agent` (role x objective pairing) |
| Fitness / profitability | `mean_reward` on `RewardHistory` |
| Mutation / search | `wg evolve --strategy mutation` |
| Recombination | `wg evolve --strategy crossover` |
| Selection environment | Task performance + evaluator judgment |
| Extinction | `wg evolve --strategy retirement` |
| Lamarckian inheritance | Yes: the evolver reads performance data and *directs* mutation toward observed weaknesses |

The Lamarckian character of wg's evolution is important. Unlike biological
evolution (random mutation + selection), wg's evolver is an intelligent agent
that reads performance data and proposes targeted improvements. A role scoring
low on efficiency gets its description "sharpened to emphasize economy" -- this
is directed, adversity-responsive variation, exactly the Lamarckian process
Nelson & Winter describe.

### Naming note

Nelson & Winter use **"routine"** for the hereditable unit. In wg, the closest
analog is `Role` (not "routine"). This is a reasonable divergence -- "routine"
in Nelson & Winter refers to broad behavioral patterns, while `Role` in wg
specifically defines functional identity. The wg term is more precise for its
domain. No rename recommended.

Nelson & Winter use **"search"** for the process of variation. In wg, this is
`evolve`. "Search" in the N&W sense is broader than evolutionary
operations -- it includes any organizational process that explores new
capabilities. The wg term is narrower but accurate for what it does.

---

## 7. Exploration vs. Exploitation (March, 1991)

### The literature

March (1991) formalizes the tension between **exploration** (search for new
possibilities, experimentation, risk, discovery) and **exploitation**
(refinement of existing capabilities, efficiency, selection, execution).
Adaptive systems tend to shift toward exploitation because its returns are
more proximate, more predictable, and more precisely measurable. But long-run
survival requires sustained exploration.

Key finding: **adaptive processes are self-destructive** -- they increase
exploitation at the expense of exploration, becoming effective in the short
run but brittle in the long run.

### Connection to wg

The wg evolution system makes the exploration-exploitation tradeoff explicit
through its six strategies:

| Strategy | March category |
|---|---|
| `mutation` | Exploitation (refining existing capabilities) |
| `crossover` | Mixed (combining known strengths = exploitation; novel combinations = exploration) |
| `gap-analysis` | Exploration (creating entirely new capabilities) |
| `retirement` | Selection (intensifying exploitation by pruning) |
| `objective-tuning` | Exploitation (adjusting constraints on existing objectives) |
| `all` | The evolver decides the balance |

March's central warning applies directly: if the evolver predominantly chooses
mutation, crossover, and retirement (all exploitation-heavy), the identity
space will converge on a narrow optimum and lose the diversity needed to adapt
to novel tasks. The `gap-analysis` strategy is the primary exploration
mechanism, and its usage frequency relative to exploitation strategies
determines the system's long-run adaptability.

### Design implication

The current system gives the human full control over when to evolve and which
strategy to use. This is conservative (prevents runaway exploitation) but also
means exploration depends on human judgment. March's model suggests that
**explicit diversity maintenance mechanisms** -- analogous to maintaining
minimum exploration rates -- could be valuable. For example:

- A minimum fraction of evolution runs must use `gap-analysis`
- The system could flag when identity diversity (measured as the number of
  distinct active roles/objectives) falls below a threshold
- Crossover could be biased toward pairing *dissimilar* parents, increasing
  the exploratory component

The VX peer exchange is a natural exploration mechanism: external suggestions
introduce variation from outside the local identity space, injecting diversity
that local evolution alone might never produce.

---

## 8. Recombinant Growth (Weitzman, 1998)

### The literature

Weitzman (1998) models innovation as a **combinatorial process**: new ideas are
produced by recombining existing ideas. Given *n* ideas, there are *n(n-1)/2*
possible pairings. The key insight is that while the space of possible
combinations grows super-exponentially, the **R&D resources** needed to
investigate each combination are the binding constraint, so long-run growth
converges to exponential.

### Connection to wg

The wg identity space is exactly Weitzman's combinatorial model. With *r*
roles and *o* objectives, there are *r x o* possible agents. Each agent is a
"combination" of a role-idea and an objective-idea. The crossover strategy
produces new roles by recombining two existing roles -- the direct analog of
Weitzman's recombinant process.

Weitzman's binding constraint -- R&D resources needed to evaluate each
combination -- maps to the evaluation cost in wg. Each new agent identity
must be assigned to tasks, executed, and rewarded before its quality is known.
This evaluation cost is the bottleneck, not the combinatorial space itself.

### Design implication

Weitzman's framework suggests that the system should **track which (role,
objective) pairings have been evaluated** and prioritize assigning untested
combinations. The synergy matrix already tracks this, but the assigner agent
does not currently use "coverage of untested pairings" as an explicit
assignment criterion. Adding this would operationalize Weitzman's insight that
the constraint is in processing the abundance of possibilities, not in
generating them.

---

## 9. Incomplete Contracts and Organizational Boundaries (Hart & Moore, 1990)

### The literature

Grossman & Hart (1986) and Hart & Moore (1990) develop the **property rights
theory of the firm**: when contracts are incomplete (cannot specify actions for
every contingency), ownership of assets determines **residual rights of
control** -- the right to make decisions in uncontracted-for situations.
Ownership matters because it determines bargaining power, which in turn
affects ex-ante investment incentives. The **hold-up problem** arises when
one party can exploit the other's sunk investments by renegotiating terms.

Hart & Moore's key result: **complementary assets should be under common
ownership** to prevent hold-up and encourage relationship-specific investments.

### Connection to wg and VX

The `Objective` struct is an explicitly **incomplete contract** in the
Hart-Moore sense. It specifies:
- `acceptable_tradeoffs` -- situations where the agent has discretion
- `unacceptable_tradeoffs` -- bright-line constraints

But it does not and cannot specify the agent's action for every possible task
state. The "residual rights of control" -- what the agent does when the
objective is silent -- are determined by the agent's model weights and the
role description. This is a feature, not a bug: the whole point of delegating
to an LLM agent is that it exercises judgment in situations the objective
doesn't anticipate.

For VX, Hart-Moore's property rights framework is directly relevant to the
**peer exchange boundary problem**: when an external peer contributes an
improvement suggestion, who owns the intellectual property? The `visibility`
field proposed in the VX design (Private / PublicPrompt / Public) is a
mechanism for **controlling residual rights** over task information, analogous
to how asset ownership controls residual rights over physical assets.

Hart & Moore's **complementary assets** result suggests that roles and
objectives that are strongly complementary in the synergy matrix should be
"owned" together -- i.e., kept as a stable agent pairing rather than
frequently reassigned. The current system treats assignment as ephemeral (per
task), but the literature suggests that high-synergy pairings should be
preserved and protected from disruption.

---

## 10. Formal and Real Authority (Aghion & Tirole, 1997)

### The literature

Aghion & Tirole (1997) distinguish between **formal authority** (the
contractual right to decide) and **real authority** (effective control over
decisions, determined by information structure). Delegating formal authority to
a subordinate increases their initiative but decreases the principal's control.
Factors that increase real authority of subordinates include: **principal
overload**, **urgency**, and **reputation**.

### Connection to wg

The wg auto-assign system is a textbook implementation of Aghion-Tirole
delegation:

| A&T concept | wg implementation |
|---|---|
| Formal authority to assign tasks | The human who runs `wg assign` manually |
| Delegated formal authority | `auto_assign = true` delegates to the assigner agent |
| Real authority | The assigner agent has real authority because it has the information (performance records, capability matching) |
| Initiative from delegation | Agents are dispatched faster because assignment doesn't wait for human review |
| Loss of control from delegation | The human cannot veto individual assignments without disabling auto-assign |
| Overload driving delegation | The human cannot manually assign hundreds of tasks per day |

The same pattern applies to auto-reward (delegating evaluation authority to the
evaluator agent) and evolution (delegating organizational redesign authority
to the evolver agent). Each delegation increases throughput but decreases human
control.

### Design implication

Aghion & Tirole emphasize that the **optimal degree of delegation depends on
the principal's overload**. The wg config options (`auto_assign`,
`auto_reward`) are binary -- delegation is either on or off. A more
nuanced design might allow **conditional delegation**: auto-assign for low-
stakes tasks but require human approval for high-stakes ones, with a
configurable threshold. This would match the literature's prediction that
delegation should increase with the principal's opportunity cost of personal
attention.

---

## 11. Measurement Distortion (Goodhart, Campbell, Kerr)

### The literature

Three related results warn about the pathologies of performance measurement:

- **Goodhart's Law** (1975): "When a measure becomes a target, it ceases to
  be a good measure."
- **Campbell's Law** (1971): "The more any quantitative social indicator is
  used for social decision-making, the more subject it will be to corruption
  pressures."
- **Kerr's Folly** (1975): Organizations frequently reward behavior A while
  hoping for behavior B, because they measure A more easily than B.

### Connection to wg

The wg system is exposed to all three pathologies:

**Goodhart's Law in LLM evaluation.** The evaluator agent scores tasks on four
dimensions that the evolver uses as the selection signal. If the LLM evaluator
is itself an LLM, and the work being evaluated was produced by an LLM with
the same training distribution, there is a risk of **evaluator-producer
collusion through shared biases** -- not strategic collusion, but statistical:
both LLMs may share the same blind spots about what constitutes "good" code.
The evaluator may give high scores to output that *looks* correct to an LLM
but is subtly wrong in ways that require domain expertise to detect.

**Campbell's Law in evolution.** The evolution system uses `mean_reward` as the
primary selection signal. Over generations, roles and objectives may be
"optimized" to produce output that scores well on the evaluator's rubric
rather than output that is genuinely good. This is the AI analog of teaching
to the test.

**Kerr's Folly in dimensional weighting.** The 40/30/15/15 weighting rewards
correctness heavily and efficiency lightly. If the organization actually needs
efficient agents (e.g., agents that minimize token costs), the weighting
rewards what is easy to measure (correctness) while hoping for what is hard
to measure (efficiency).

### VX as the primary mitigation

VX's outcome-based rewards are the natural antidote to all three pathologies:

1. **Goodhart:** Outcome metrics (`outcome:sharpe`, `outcome:brier`) are
   computed from real-world data, not LLM judgment. They cannot be Goodharted
   by the same mechanism that produces the work.
2. **Campbell:** Outcome-informed evolution weights real-world impact alongside
   LLM scores, reducing corruption of the selection signal.
3. **Kerr:** If the organization hopes for real-world impact but measures LLM
   approval, VX adds the real-world measure that aligns the reward system with
   the actual goal.

### Design implication

The system should be designed to make it easy to **add outcome-based reward
sources** and to give them **increasing weight** in the evolution signal over
time, as the mapping between tasks and outcomes becomes better understood.
The `source` field on `Reward` enables this structurally; what is needed is
a configurable weighting scheme in the evolution system that can blend LLM
rewards with outcome rewards.

---

## 12. Bounded Rationality and Satisficing (Simon, 1955)

### The literature

Simon (1955) argues that human decision-makers cannot optimize because they
face **bounded rationality**: limited information, limited cognitive capacity,
and time pressure. Instead, they **satisfice** -- setting an aspiration level
and choosing the first option that meets it. Organizations help overcome
bounded rationality by establishing **procedural rationality**: formal
processes, role definitions, and information channels that structure
decision-making.

### Connection to wg

The wg identity system is an implementation of Simon's procedural rationality
for AI agents:

- **Roles** narrow the decision space: instead of facing every possible task
  as a general-purpose agent, the role description focuses attention on
  specific competencies and desired outcomes.
- **Objectives** set aspiration levels: the acceptable/unacceptable tradeoffs
  define what "good enough" means for this agent, implementing Simon's
  satisficing criterion.
- **The coordinator's dispatch loop** is a formal procedure that structures
  when and how decisions are made, preventing the chaos of uncoordinated
  action.

LLMs are in some sense the ultimate boundedly rational agents: they have vast
but imperfect knowledge, limited context windows, and no ability to extend
their own reasoning time. The identity system's role as cognitive scaffolding
is well-motivated by Simon's framework.

---

## 13. Proper Scoring Rules, Prediction Markets, and VX

### The literature

A **proper scoring rule** (Brier, 1950; Good, 1952) is a reward function where
the forecaster maximizes expected reward only by reporting their true
probability estimate. The Brier score (mean squared error of probabilistic
predictions) is the canonical example. Proper scoring rules are the foundation
of **prediction markets** and **forecasting tournaments** (the Good Judgment
Project, Metaculus).

The key property is **incentive compatibility**: truthful reporting is a
dominant strategy. This is the forecasting analog of the VCG mechanism in
auction theory (Vickrey, 1961; Clarke, 1971; Groves, 1973).

### Connection to VX

VX's `source: "outcome:brier"` convention directly references proper scoring
rules. When VX writes a reward with `source: "outcome:brier"`, it is using a
mathematically principled, incentive-compatible metric to score predictions
against real-world outcomes.

The VX peer exchange is structurally similar to a **prediction market**: peers
submit improvement suggestions (predictions about which changes will improve
outcomes), and the `PeerCredibility` score tracks their accuracy over time.
The literature on prediction markets strongly supports this design -- they
consistently outperform individual forecasters and have been shown to
aggregate dispersed information efficiently (Surowiecki, 2004; Tetlock &
Gardner, 2015).

### Design implication

VX should ensure that outcome metrics used as reward sources satisfy the
**proper scoring rule** property whenever the task domain permits it. For
probabilistic predictions, this means using Brier or log scores, not ad-hoc
accuracy thresholds. For continuous outcomes (e.g., portfolio returns), the
metric should be chosen so that honest effort is the dominant strategy --
i.e., agents cannot game the metric by exploiting systematic biases.

The `outcome:brier` convention already signals this intent. Consider adding
`outcome:log_score` for logarithmic scoring and documenting which metrics
satisfy the proper scoring property.

---

## 14. Naming Mismatches and Recommendations

### Currently well-aligned terms

| wg/VX term | Literature term | Status |
|---|---|---|
| `Reward` | Reward (RL) | Aligned |
| `Reward.value` | Scalar reward signal | Aligned |
| `RewardHistory` | Reward history / experience summary | Aligned |
| `mean_reward` | Mean reward / average return | Aligned |
| `Objective` | Objective (BDI agents, optimization) | Aligned |
| `Role` | Role (organizational theory, MAS) | Aligned |
| `Identity` (module) | Identity (organizational identity literature) | Aligned |
| `evolve` / evolution | Evolutionary computation / evolutionary economics | Aligned |
| `mutation`, `crossover`, `retirement` | Standard evolutionary computation terms | Aligned |
| `source` on Reward | Standard metadata provenance | Aligned |

### Potential mismatches to consider

| Current term | Literature term | Discussion |
|---|---|---|
| `evaluator` | **Monitor** (Alchian & Demsetz) | The evaluator is structurally the A&D monitor -- a specialized agent observing individual performance. "Evaluator" is defensible (it evaluates), but "monitor" has a precise meaning in organizational economics. **No rename recommended** -- "evaluator" is clearer in the AI context and already well-established in the codebase. |
| `gap-analysis` (strategy) | **Exploration** (March, 1991) | The gap-analysis strategy is the primary exploration mechanism. In March's framework, it is the canonical "exploration" operation. The current name describes *what* it does (finds gaps); March's term describes *why* it matters (maintains diversity). Both are informative. **No rename recommended.** |
| `acceptable_tradeoffs` | **Discretion** or **latitude** (Aghion & Tirole) | In the formal/real authority literature, the space where the agent can exercise judgment is called their "discretion" or "latitude." The current name is descriptive and clear. **No rename recommended.** |
| `unacceptable_tradeoffs` | **Bright-line rules** / **hard constraints** (contract theory) | Contract theory calls these "bright-line rules" or "hard constraints" that cannot be waived. The current name is clear. **No rename recommended.** |
| `TrustLevel` | **Reputation** (repeated games literature) | The trust level enum (Verified/Provisional/Unknown) is closer to a reputation mechanism than a trust measure in the game-theory sense. The economics literature on repeated games (Kreps & Wilson, 1982; Mailath & Samuelson, 2006) uses "reputation" for the belief others hold about an agent's type. "Trust" is fine colloquially but may confuse readers familiar with the trust games literature (Berg, Dickhaut & McCabe, 1995), where "trust" specifically means willingness to make oneself vulnerable. **Minor note; no rename necessary**, but VX documentation should clarify that `TrustLevel` tracks reputation (track record) not trust (vulnerability acceptance). |
| `synergy matrix` | **Complementarity matrix** or **interaction effects** (personnel economics) | In personnel economics (Ichniowski, Shaw & Prennushi, 1997), the interaction between organizational practices is called "complementarity." The synergy matrix measures exactly this: how much better (or worse) a role performs when paired with a specific objective. "Synergy" is used more in management consulting; "complementarity" has a precise economic meaning. **Consider noting the connection in documentation**, but the current term is fine for user-facing purposes. |
| `capabilities` (agent field) | **Skills** or **human capital** (labor economics) | In labor economics, agent capabilities are called "skills" or "human capital." In wg, `capabilities` are flat routing tags for task matching, while `skills` are prompt content injected into the agent context. This distinction is wg-specific and potentially confusing to economists who would expect the two to be the same concept. **Consider documenting the distinction** clearly, as it is a deliberate design choice with no clean analog in the literature. |
| `assigner` | **Allocator** or **matchmaker** (matching theory) | In the matching theory literature (Roth, 1984; Roth & Sotomayor, 1990), the mechanism that assigns workers to tasks is called a "matching mechanism" or "allocator." "Assigner" is clear but the connection to matching theory (and its impossibility results, e.g. that stable matching may not be compatible with incentive compatibility) could be noted. **No rename recommended.** |

### One flagged naming concern

**`performance` field on Role/Objective/Agent:** This field holds a
`RewardHistory` struct. In the terminology alignment pass, `PerformanceRecord`
was renamed to `RewardHistory`, but the *field name* accessing it was kept as
`performance` (per the FORK.md: "renaming would cascade through too many
accessor patterns for minimal clarity gain"). This creates a mild inconsistency:
you write `role.performance.mean_reward`, mixing the old term (`performance`)
with the new term (`mean_reward`). In the economics literature, this would be
called the agent's **track record** or **performance history**. The current
naming is a pragmatic compromise and not misleading, but worth noting for
future cleanup if a larger refactor occurs.

---

## 15. Synthesis: What the Literature Says About wg + VX Design

### What the system gets right

1. **Multi-dimensional rewards** (Holmstrom & Milgrom): The four-dimension
   decomposition prevents "teaching to the test" and reveals which aspects of
   performance are strong or weak.

2. **Decentralized coordination** (Hayek): The coordinator maintains
   infrastructure without making substantive decisions; specialized agents
   (assigner, evaluator, evolver) handle domain-specific judgment.

3. **Evolutionary identity management** (Nelson & Winter): The Lamarckian
   evolution system -- directed mutation based on performance data, with
   lineage tracking -- is a faithful implementation of evolutionary economics
   applied to organizational routines.

4. **Content-hash immutability**: Once an identity is created and evaluated,
   its performance record is permanently and verifiably attached to that exact
   specification. This prevents the retroactive revision that plagues human
   performance evaluation.

5. **Explicit behavioral contracts** (Hart & Moore): The Objective struct
   makes the principal's preferences explicit, injected, and evaluable --
   moving beyond implicit cultural norms to verifiable specifications.

6. **Separation of assignment and evaluation** (A&D monitoring): The evaluator
   does not assign tasks, and the assigner does not evaluate. This separation
   prevents conflicts of interest that arise when the same entity both
   delegates and judges.

### What the literature warns about

1. **Goodhart's Law is the primary risk.** LLM-on-LLM evaluation (LLM
   evaluator scoring LLM agent output) is vulnerable to shared blind spots.
   **VX outcome rewards are the critical mitigation.** The system should be
   designed to increase the weight of outcome-based rewards over time.

2. **Exploration will be underproduced** (March). The evolution system will
   naturally converge toward exploitation (mutation, retirement) because their
   returns are more predictable. **Explicit exploration budgets** (minimum
   gap-analysis frequency, diversity thresholds) are needed.

3. **The hold-up problem applies to VX peer exchange** (Hart & Moore). Once
   a peer contributes a valuable improvement suggestion, the recipient could
   exploit it without reciprocating. The trust/credibility mechanism must
   account for this.

4. **Tournament pathologies re-emerge with human peers** (Lazear & Rosen).
   While LLM agents cannot strategically manipulate their rankings, human
   participants in VX can. The peer exchange design should account for
   strategic behavior, sabotage, and free-riding.

5. **Multi-task incentive balance should be adaptive** (H&M). The hardcoded
   40/30/15/15 reward weights may not match the organization's actual
   priorities. Making weights configurable per-objective or per-task-type
   would align the incentive structure with revealed preferences.

---

## References

- Aghion, P. & Tirole, J. (1997). [Formal and Real Authority in Organizations](https://people.duke.edu/~qc2/BA532/1997%20JPE%20Aghion%20and%20Tirole.pdf). *Journal of Political Economy*, 105(1), 1-29.
- Alchian, A. & Demsetz, H. (1972). [Production, Information Costs, and Economic Organization](https://josephmahoney.web.illinois.edu/BA549_Fall%202010/Session%205/Alchian_Demsetz%20(1972).pdf). *American Economic Review*, 62(5), 777-795.
- Brier, G. W. (1950). Verification of forecasts expressed in terms of probability. *Monthly Weather Review*, 78(1), 1-3.
- Campbell, D. T. (1971). Methods for the experimenting society. Presentation to the American Psychological Association.
- Goodhart, C. A. E. (1975). Problems of monetary management: The UK experience. *Papers in Monetary Economics*, Reserve Bank of Australia.
- Grossman, S. & Hart, O. (1986). The Costs and Benefits of Ownership. *Journal of Political Economy*, 94(4), 691-719.
- Hart, O. & Moore, J. (1990). [Property Rights and the Nature of the Firm](https://www.semanticscholar.org/paper/Property-Rights-and-the-Nature-of-the-Firm-Hart-Moore/b35af322333811bd16eb5b569466ad76909c0a20). *Journal of Political Economy*, 98(6), 1119-1158.
- Hayek, F. A. (1945). [The Use of Knowledge in Society](https://www.jstor.org/stable/1809376). *American Economic Review*, 35(4), 519-530.
- Holmstrom, B. (1979). Moral Hazard and Observability. *Bell Journal of Economics*, 10(1), 74-91.
- Holmstrom, B. (1982). Moral Hazard in Teams. *Bell Journal of Economics*, 13(2), 324-340.
- Holmstrom, B. & Milgrom, P. (1991). [Multitask Principal-Agent Analyses](https://people.duke.edu/~qc2/BA532/1991%20JLEO%20Holmstrom%20Milgrom.pdf). *Journal of Law, Economics, and Organization*, 7, 24-52.
- Jensen, M. & Meckling, W. (1976). Theory of the Firm. *Journal of Financial Economics*, 3(4), 305-360.
- Kerr, S. (1975). On the Folly of Rewarding A While Hoping for B. *Academy of Management Journal*, 18(4), 769-783.
- Lazear, E. & Rosen, S. (1981). [Rank-Order Tournaments as Optimum Labor Contracts](https://www.nber.org/papers/w0401). *Journal of Political Economy*, 89(5), 841-864.
- March, J. G. (1991). [Exploration and Exploitation in Organizational Learning](http://www.iot.ntnu.no/innovation/norsi-pims-courses/Levinthal/March%20(1991).pdf). *Organization Science*, 2(1), 71-87.
- Nelson, R. & Winter, S. (1982). [An Evolutionary Theory of Economic Change](https://www.hup.harvard.edu/books/9780674272286). Harvard University Press.
- Simon, H. A. (1955). A Behavioral Model of Rational Choice. *Quarterly Journal of Economics*, 69(1), 99-118.
- Weitzman, M. L. (1998). [Recombinant Growth](https://scholar.harvard.edu/files/weitzman/files/recombinant_growth.pdf). *Quarterly Journal of Economics*, 113(2), 331-360.
