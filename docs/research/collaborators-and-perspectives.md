# Workgraph + VX: Collaborators and Intellectual Lineage

A document for the three of us -- situating what we each bring to the
workgraph/VX project and where the organizational economics literature
connects to our respective backgrounds.

---

## The people

### Erik Garrison

Erik is an Associate Professor of computational biology at the University of
Tennessee Health Science Center. He builds methods for reading genomes and
understanding their variation through **pangenomic graph models** -- the vg
toolkit, PGGB, freebayes, seqwish. He co-chairs the Human Pangenome Reference
Consortium's Pangenomes Working Group. His PhD at Cambridge (Wellcome Sanger
Institute, advised by Richard Durbin) formalized graphical pangenomics.

Before all that, Erik studied Social Studies at Harvard, with electives in
functional programming, theoretical computer science, peer-to-peer networks,
and linear algebra. His senior thesis examined the relationship between social
structure and communication technologies.

This trajectory -- from social structure to genome graphs -- is not a detour.
The core abstraction is the same: how do you represent a complex,
branching, recombining system where no single linear reference captures the
full picture? In genomics, the answer was to replace the linear reference
genome with a variation graph. In workgraph, the answer is to replace the
linear task list with a directed graph of dependencies, loop edges, and
agent identities that evolve.

Erik brings to workgraph the engineering discipline of building graph systems
that must be correct, reproducible, and auditable at scale. The content-hash
ID system in workgraph (SHA-256 of identity-defining fields, with lineage
metadata linking mutations to parents) mirrors how pangenome graphs track
variants: each variant is uniquely identified by its sequence content, not by
its position in an arbitrary reference coordinate system.

### Vaughn Tan

Vaughn is an Associate Professor of Strategy and Entrepreneurship at UCL
School of Management. His PhD in Organizational Behavior and Sociology from
Harvard examined how organizations design themselves to thrive under genuine
uncertainty (not merely risk). His book *The Uncertainty Mindset* (Columbia
University Press) draws on years of ethnographic fieldwork in elite culinary
R&D teams -- The Fat Duck Experimental Kitchen, ThinkFoodTank, The Cooking
Lab -- to develop a theory of organizational innovation under uncertainty.
Before academia, he worked at Google across advertising, Earth, Maps,
spaceflight, and Fusion Tables.

His most directly relevant academic contribution is "Using Negotiated Joining
to Construct and Fill Open-ended Roles in Elite Culinary Groups"
(*Administrative Science Quarterly*, 2015). The core finding: when an
organization operates under genuine uncertainty about what work needs to be
done, **roles should be intentionally left open-ended**. Instead of hiring
people into fixed job descriptions, the role is explicitly provisional. It is
then jointly constructed through repeated iterations of proposition,
validation, and selective integration. The role evolves in response to what the
new member discovers they can contribute.

This finding is operationalized directly in workgraph's identity system. A
`Role` defines a starting point -- description, skills, desired outcome -- but
the evolution system treats it as provisional: after sufficient reward data
accumulates, `wg evolve --strategy mutation` modifies the role description
to address observed weaknesses, and `wg evolve --strategy crossover` combines
elements from two roles to produce a novel hybrid. Roles in workgraph are
never frozen. They are Vaughn's open-ended roles made computational.

Vaughn also brings the crucial distinction between **risk** (known
probability distribution over known outcomes) and **uncertainty** (unknown
outcomes, unknown distributions). Most organizational design assumes risk.
Workgraph's evolution system -- and especially VX's outcome-based rewards --
are designed for uncertainty: you do not know in advance which role-objective
pairings will work, which metrics will matter, or how the task landscape
will shift.

He was recently a research fellow at the Future of Life Foundation's programme
on AI for human reasoning and at the Ethereum Foundation's protocols research
programme -- both directly relevant to the mechanism design questions that VX
raises.

### nikete

nikete works at GroupLang, studying how LLMs interacting with groups affect
incentives. PhD from the Australian National University, with a thesis on
algorithm and mechanism design for decision-making while preserving subjects'
autonomy. Publications span ML venues (NeurIPS, WWW, UAI, IJCAI) and medical
journals (*Critical Care Medicine*, *Chest*). Research sits at the
intersection of machine learning and economics: mechanism design, decision
elicitation, collective cognition, and the design of markets and institutions.

nikete's specific contributions to this project:

1. **The VX fork** -- renaming workgraph's internal terminology to match
   reinforcement learning and multi-agent systems literature (agency ->
   identity, motivation -> objective, evaluation -> reward, score -> value),
   and adding the `source` field to `Reward` for pluggable reward functions.

2. **The Veracity Exchange (VX)** -- a standalone tool for scoring workflow
   outputs against real-world outcomes and facilitating peer exchange of
   improvements. This is mechanism design applied to agent coordination:
   how do you elicit truthful outcome reports, incentivize useful improvement
   suggestions, and aggregate dispersed information about what works?

3. **The organizational economics review** -- connecting workgraph's design
   to 13 foundational results from economics (in
   `docs/research/organizational-economics-review.md`), identifying where the
   system aligns with theory and where the literature warns of risks.

nikete brings the formal framework: when is the reward system incentive-
compatible? When do proper scoring rules apply? What does mechanism design
say about the VX peer exchange? What are the conditions under which the
evolution system's selection signal is informative?

---

## How the backgrounds interlock

The three backgrounds map onto three distinct layers of the system:

| Layer | Person | Discipline | What they bring |
|---|---|---|---|
| **Graph infrastructure** | Erik | Computational biology, graph algorithms | Correct, reproducible, auditable graph representations; content-hash identity; variation-as-graph |
| **Organizational design** | Vaughn | Organizational behavior, ethnography | Open-ended roles, negotiated joining, uncertainty mindset; why roles should evolve, not be fixed |
| **Incentive architecture** | nikete | Mechanism design, ML, economics | Proper scoring rules, incentive compatibility, reward source diversity; when and why the evolution signal is trustworthy |

These are complementary, not overlapping. Erik's question is "how do you
represent it?" Vaughn's is "how should the organization work?" nikete's is
"are the incentives right?"

---

## Where each person's literature connects

### Erik's world: graphs, variation, and recombination

Erik's pangenomics work is built on the insight that a single linear reference
genome is a lossy representation of population-level variation. The variation
graph preserves all observed variants as branches in a graph, with each
variant identified by its sequence content (not by coordinates in an
arbitrary reference).

Workgraph's identity system uses the same principle. Each role, objective,
and agent is identified by a SHA-256 content hash of its identity-defining
fields. Mutations and crossovers produce new entities with new hashes, linked
to parents by lineage metadata -- exactly how a variation graph tracks derived
haplotypes.

The economics literature most relevant to Erik's contribution:

- **Weitzman (1998), "Recombinant Growth"**: Innovation as combinatorial
  recombination of existing ideas. The `crossover` evolution strategy is
  literally Weitzman's recombinant process: two high-performing roles are
  combined to produce a child that inherits elements of both. The identity
  space (roles x objectives) is Weitzman's combinatorial idea space.

- **Nelson & Winter (1982), "An Evolutionary Theory of Economic Change"**:
  Organizational routines as heritable information subject to variation and
  selection. In workgraph, roles are the routines; content-hash IDs are the
  genes; the evolver is the search process. Nelson & Winter describe their
  framework as Lamarckian (directed variation in response to adversity),
  which matches workgraph's evolver: it reads performance data and proposes
  *targeted* mutations, not random ones.

### Vaughn's world: uncertainty, roles, and organizational adaptation

Vaughn's ethnographic finding -- that elite innovation teams use intentionally
open-ended roles, constructed through negotiated joining -- maps directly onto
workgraph's identity evolution cycle:

1. A role is created with an initial description (the **provisional role**)
2. An agent carrying that role is assigned to tasks and rewarded
3. The evolver reads performance data and proposes modifications
4. The modified role is a new entity (new content hash) linked to the
   original by lineage

This is negotiated joining made computational. The "negotiation" is between
the human principal (who triggers evolution and reviews proposals) and the
evolver agent (who reads the data and proposes changes). The role is never
frozen.

The economics literature most relevant to Vaughn's contribution:

- **March (1991), "Exploration and Exploitation"**: Adaptive systems tend to
  overexploit (refine existing capabilities) at the expense of exploration
  (discovering new ones). Vaughn's uncertainty mindset is precisely the
  antidote: organizations that embrace uncertainty maintain higher exploration
  rates. In workgraph, the `gap-analysis` evolution strategy is the explicit
  exploration mechanism. March's warning is that it will be used too rarely.

- **Aghion & Tirole (1997), "Formal and Real Authority"**: Delegating
  authority increases initiative but decreases control. Workgraph's
  `auto_assign` and `auto_reward` are delegation of formal authority to LLM
  agents. Vaughn's insight about when to delegate (uncertainty work) and
  when not to (stable, well-understood work) directly informs when
  auto-assign is appropriate.

- **Holmstrom & Milgrom (1991), "Multitask Principal-Agent"**: Job design
  (restricting which tasks are bundled) is itself an incentive instrument.
  Vaughn's open-ended roles challenge this: under uncertainty, over-specifying
  the role is counterproductive because you do not know what the job will
  require. The tension between H&M's "narrow the role for incentive clarity"
  and Vaughn's "leave the role open for adaptability" is the central design
  question for the identity system. The answer depends on the task type --
  and the system should support both.

### nikete's world: mechanisms, incentives, and scoring rules

nikete's work on mechanism design and decision elicitation provides the formal
framework for asking whether workgraph's reward and evolution systems are
*incentive-compatible* -- whether agents (and, in VX, human peers) have
correct incentives to report truthfully and exert genuine effort.

The key contributions:

- **Generative governance**: nikete's work on language model contracts --
  using LLMs to execute natural-language agreements into state transitions --
  is directly relevant to how workgraph's objectives function. An `Objective`
  is a natural-language contract ("acceptable tradeoffs: X; unacceptable: Y")
  that is evaluated by an LLM evaluator. This is a language model contract in
  embryonic form.

- **Advice auctions and decision markets**: VX's peer exchange, where external
  peers contribute improvement suggestions and earn credibility from their
  accuracy, is structurally an advice auction. nikete's research on decision
  markets -- which "seek to both predict and decide the future" rather than
  merely predict it -- maps directly: VX peers are not just predicting which
  improvements will work, they are proposing changes that will be adopted.

- **Proper scoring rules**: The `source: "outcome:brier"` convention in the
  VX fork uses a mathematically principled, incentive-compatible metric.
  Brier (1950) showed that the quadratic scoring rule incentivizes honest
  probability reporting. This is the formal guarantee that outcome-based
  rewards cannot be gamed by a dishonest forecasting agent -- exactly the
  property needed when VX accepts reward signals from external sources.

The economics literature most relevant to nikete's contribution:

- **Holmstrom (1979), Informativeness Principle**: Any measure that reveals
  information about effort should be included in the contract. The four
  reward dimensions (correctness, completeness, efficiency, style_adherence)
  are an application of this principle -- each reveals independent
  information. VX's outcome rewards add a fifth signal (real-world impact)
  that is maximally informative because it cannot be Goodharted by the same
  mechanism that produces the work.

- **Goodhart's Law / Campbell's Law / Kerr's Folly**: The three pathologies
  of measurement-as-target. LLM-on-LLM evaluation is specifically vulnerable
  to shared blind spots (a form of Goodhart). VX's outcome-based rewards are
  the structural mitigation -- they are computed from reality, not from LLM
  judgment.

- **Lazear & Rosen (1981), Tournament Theory**: Tournament selection is
  efficient when absolute output is hard to measure but relative ranking is
  easy -- but it encourages selfish behavior in human participants. LLM
  agents cannot strategically manipulate tournaments (they lack that kind
  of agency), but human participants in VX's peer exchange can. The mechanism
  design must account for this asymmetry.

---

## The harmonization task

Erik and Vaughn built workgraph as a practical system for coordinating AI
agents with composable, evolving identities. The original naming reflected
the system's organic development: "agency" for the identity module,
"motivation" for the behavioral contract, "evaluation" for the feedback
signal, "score" for the scalar value.

The fork aligns these names with the literatures that study the same concepts
formally:

| Original (intuitive) | Fork (literature-aligned) | Literature |
|---|---|---|
| Agency | Identity | Organizational identity theory (Albert & Whetten, 1985) |
| Motivation | Objective | BDI agent architecture (Rao & Georgeff, 1995); optimization theory |
| Evaluation | Reward | Reinforcement learning (Sutton & Barto, 2018) |
| score | value | RL conventions; avoids collision with priority scores |
| PerformanceRecord | RewardHistory | RL experience replay terminology |
| avg_score | mean_reward | Standard statistical and RL naming |

The purpose is not pedantic renaming. It is to make the system **legible**
to three communities simultaneously:

1. **Organizational theorists** (Vaughn's world): who study how roles,
   incentives, and adaptation work in human organizations
2. **ML/RL researchers** (nikete's world): who study reward signals, policy
   optimization, and multi-agent coordination
3. **Systems engineers** (Erik's world): who build correct, reproducible
   infrastructure for representing complex variation

When a role's `mean_reward` is 0.72 and its trend indicator points down, an
organizational theorist reads "this specialist is declining," an RL researcher
reads "this policy's expected return is falling," and a systems engineer reads
"this variant has lower fitness than its parent." Same data, same struct, same
field name -- three communities can reason about it without translation.

VX extends this further. The `source` field on `Reward` makes the system
legible to a fourth community: **economists and mechanism designers** who
study how to aggregate dispersed information from strategic agents. When a
reward has `source: "outcome:brier"`, an economist reads "this is an
incentive-compatible proper scoring rule applied to realized outcomes" --
a precise, theory-backed statement about the quality of the signal.

---

## What each of us should watch for

**Erik**: The graph infrastructure must remain correct under evolution.
When `wg evolve --strategy crossover` produces a child role from two parents,
the lineage graph grows. Are the lineage queries efficient? Can the system
handle thousands of generations without performance degradation? These are
the same scaling questions that arise in pangenome graphs with many
haplotypes.

**Vaughn**: The evolution system embodies the uncertainty mindset -- but does
it preserve it? March (1991) warns that adaptive systems naturally shift
toward exploitation. If the evolver predominantly uses `mutation` and
`retirement` (both exploitation-heavy), the identity space will converge and
lose the diversity needed for genuine adaptation. The `gap-analysis` strategy
is the exploration mechanism. Is it being used enough? Should there be a
minimum exploration budget? Vaughn's ethnographic instinct for when an
organization is "freezing up" is the check the system needs.

**nikete**: The reward system must be incentive-compatible end-to-end. The
LLM evaluator is not a strategic agent, but it can be systematically biased.
VX outcome rewards are the correction, but only if the outcome metrics
themselves satisfy the proper scoring rule property. Which task domains
admit proper scoring? What happens when outcomes are delayed (latent payoffs)?
When does the evolution system need to discount older rewards? These are
mechanism design questions that have formal answers in the literature.

---

## References

The full organizational economics review is at
`docs/research/organizational-economics-review.md`. Key references specific
to each person's contribution:

**Erik's thread:**
- Nelson, R. & Winter, S. (1982). *An Evolutionary Theory of Economic Change*. Harvard University Press.
- Weitzman, M. L. (1998). Recombinant Growth. *Quarterly Journal of Economics*, 113(2), 331-360.

**Vaughn's thread:**
- March, J. G. (1991). Exploration and Exploitation in Organizational Learning. *Organization Science*, 2(1), 71-87.
- Aghion, P. & Tirole, J. (1997). Formal and Real Authority in Organizations. *Journal of Political Economy*, 105(1), 1-29.
- Holmstrom, B. & Milgrom, P. (1991). Multitask Principal-Agent Analyses. *Journal of Law, Economics, and Organization*, 7, 24-52.
- Tan, V. Y. H. (2015). Using Negotiated Joining to Construct and Fill Open-ended Roles in Elite Culinary Groups. *Administrative Science Quarterly*, 60(1), 103-132.

**nikete's thread:**
- Holmstrom, B. (1979). Moral Hazard and Observability. *Bell Journal of Economics*, 10(1), 74-91.
- Kerr, S. (1975). On the Folly of Rewarding A While Hoping for B. *Academy of Management Journal*, 18(4), 769-783.
- Lazear, E. & Rosen, S. (1981). Rank-Order Tournaments as Optimum Labor Contracts. *Journal of Political Economy*, 89(5), 841-864.
- Brier, G. W. (1950). Verification of forecasts expressed in terms of probability. *Monthly Weather Review*, 78(1), 1-3.
