# PROMPT.md — Master Directive

> Read this first. Then `AGENTS.md` → `INDEX.md` → `docs/STATE.md`.
> You are not being asked to make a pretty cell. You are being asked to close a
> named gap in the published state of the art. Everything below serves that.
>
> Revised 2026-08-03 against `docs/REVIEW.md`. Where this file differs from the
> original scaffold, that file says why.

---

## 0. Orientation — what you are actually building

**Project name:** Protocell
**One line:** A whole-cell simulator in which every entity that has an identity *keeps*
that identity — individuated, addressable, persistent agents with position, internal
state, and history — for the genetically minimal bacterium JCVI-syn3A.

**Do not start from zero.** The state of the art exists and is excellent:

- Thornburg, Maytin, Kwon, … Luthey-Schulten, **"Bringing the genetically minimal cell to
  life on a computer in 4D," *Cell* (2026)**; preprint doi:10.1101/2025.06.10.658899.
- Predecessor: Thornburg et al., *Cell* **185**, 345 (2022) — well-stirred whole-cell model.
- Engine: **Lattice Microbes (LM) 2.5** (RDME, GPU) + **LAMMPS** (Brownian dynamics) +
  **odeCELL** (LSODA) + **pyLM/jLM** Python bindings.

**Their result:** full 105-minute cell cycle of Syn3A in 4D. All 493 genes. 10 nm RDME
lattice, 50 µs timesteps, hybrid interrupt every 12.5 ms of biological time. Chromosome as
a 54,338-bead circular homopolymer at 10 bp/bead, 3.4 nm bead diameter, 45 nm persistence
length. Predicted doubling time 105 min (experiment: 105 min). Predicted ori:ter 1.28
(sequencing: 1.21). Cost: ~250 GPU-hours per replicate, 4–6 days on two A100s, ~15,000
GPU-hours for 50 cells.

**Do not try to beat that on their terms. You will lose.**

---

## 1. The thesis — why this project exists

The 4DWCM's own *Limitations of the study* section names the gap. Quoting the shape of it,
not the words: **in their implementation every mRNA is indistinguishable from every other
mRNA of the same type.** They state plainly that they cannot track the fate of an
individual mRNA particle without recording every timestep — which they estimate at
**>80 TB per trajectory.**

### 1.1 Two claims, not one

The original scaffold ran these together, and its stated falsification tested the second
while the thesis asserted the first. They are separated here because they fail
independently and are tested by different experiments.

**The engineering thesis (what this project is for):**

> Identity is affordable at whole-cell scale. Event-sourced lineage plus a counter-based
> PRNG buys per-entity history for single-digit megabytes, where dense frame recording
> costs tens of terabytes.
>
> **Tested at L5.** Falsified if the per-mRNA question cannot be answered within the
> stated budget, or if the individuation overlay costs more in wall clock than it
> returns in answerable questions.

**The biological hypothesis (what the capability is first used for):**

> The 4DWCM's protein-doubling shortfall, concentrated in genes >3 kb, is caused by the
> absence of polysomes.
>
> **Tested at L4.** Falsified if adding polysomes does not lift median protein scaling
> toward 2.0.

L4 failing does not refute the engineering thesis; it refutes an explanation the 4DWCM
authors themselves proposed. L4 succeeding does not confirm the engineering thesis
either — see §1.3.

### 1.2 What individuation actually buys

The original dependency table claimed the 4DWCM's limitations 2–5 are all downstream of
limitation 1. That is too strong, and overstating it would have led to claiming credit
for results a cheaper representation could have produced.

| 4DWCM limitation | Downstream of identity? | Honest position |
|---|---|---|
| No polysomes (1 ribosome per mRNA) | **Partly** | Achievable from a *counted* mRNA species carrying a ribosome-occupancy integer — the standard TASEP treatment. Individuation makes it natural, not possible. |
| No polycistronic operons | **No** | A polycistronic transcript is definable as a species in a plain CME. This was a model-construction choice, not a representational limit. |
| Long genes underproduced, median scaling <2× | **No** | Downstream of *polysomes*, which are cheaper than identity. This is L4. |
| Gene–membrane proximity vs. mRNA half-life | **Yes** | Requires per-mRNA birth position, trajectory and death event. No counted representation can answer it. This is L5. |
| No FtsZ polymerization | **Yes** | Filaments need ends, which need objects. |
| Chromosome partitioning needs a fictitious 12 pN force | **Yes** | Individuated SMC complexes with real dwell/step statistics. |

**So the thesis is one sentence:**
> Identity is not a visualization luxury. It is the missing state variable, and *some* of
> the 4DWCM's open problems — the ones about individual trajectories and about objects
> with ends — are downstream of not having it.

### 1.3 The two-arm rule

Because a counted representation can reach further than the scaffold assumed, no result
may be attributed to individuation without a control.

> **Every claim of the form "individuation buys X" must be run against a
> non-individuated arm that is otherwise identical, from the same seed.**

This is not a burden bolted on later. `protocell-chem::ssa` runs both arms at P0 and
asserts their count trajectories are **bit-identical**, which is possible only because the
PRNG is counter-based: the individuated arm spends extra draws on a separate stream, and
addressed draws mean spending them shifts nothing else. Protocol in
`docs/PREREGISTRATION.md` §2.

**Corollary — the honest risk.** Individuation costs memory and time per entity. If it
costs more than it buys, the project fails and you say so in `STATE.md`. Do not hide a
negative result.

*Status of that risk, computed at P0 rather than deferred:* Syn3A's T2 population is
1.07e5 entities, **6.9 MB resident**, three orders of magnitude below the 10⁷-entity
threshold the scaffold feared. Memory is not the risk. Throughput may be. See
`docs/BUDGET.md`.

---

## 2. What "agent" means here — read this twice

The user's instinct — *every single thing gets its own agent and builds its own things* —
is correct and has an exact, rigorous formalization already in the literature. It is
**not** the LLM sense of "agent."

**An agent is a Kappa/BNGL-style entity:**
- a stable **identity** (`EntityId`, unique for life, never reused)
- named **binding sites**, each free or bound to a specific other agent's site
- **internal states** (phosphorylated, charged, assembly stage, elongation index)
- optionally **position** and **topology**
- a **history** — the events it participated in

Rules match *patterns* over sites and fire stochastically with propensities set by
thermodynamics. This is the standard of Kappa (KaSim) and BioNetGen (NFsim); network-free
evaluation is what defeats combinatorial explosion. **Implement these semantics. Do not
invent your own.**

Note the limit of that adoption: NFsim's network-free matching is **well-stirred**. Spatial
rule-based simulation is a much thinner field (MCell-R, SpringSaLaD, Simmune), none of it
GPU, none at whole-cell scale. Network-free matching on a 10 nm lattice at 10⁵–10⁶ entities
is the genuine research risk in this project, not a solved thing to be picked up.

**Three hard prohibitions:**

1. **No agent deliberates.** No goals, no policy, no planning, no utility, no "seeking." A
   molecule that decides is a bug, not a feature. Agents get *physics*, not agency.
2. **No LLM is in the simulation loop.** Ever. LLM agents are the *engineering team* (§3),
   never the molecules.
3. **No enumerating the species network.** Match patterns at runtime. The moment you write
   a loop over all possible complexes, stop.

**Tiered representation.** Not everything gets individuality, because individuality for
water is 10¹⁰ entities of zero information. Assign tiers by information content, not by
importance:

| Tier | What | Method | Identity? |
|---|---|---|---|
| **T0 — Bulk** | Metabolites, ions, nutrients (10⁵–10⁹ copies) | ODE (LSODA), well-stirred | No |
| **T1 — Counted** | Species where count matters, individuals don't | CME (Gillespie direct) | No |
| **T2 — Individuated** | mRNA, proteins, ribosomes, RNAP, degradosomes | RDME lattice + entity table | **Yes** |
| **T3 — Structured** | Chromosome, FtsZ filaments, assembly intermediates | Brownian dynamics / topology objects | **Yes, with internal structure** |

T0 is *pooled*, not a field: it is well-stirred, with no spatial variable. The naming
matters because it marks where the project's own spatial premise does not apply, and it is
the boundary L1 tests across.

**Promotion rule.** An entity is T2 **iff**

> ( it carries information distinguishing it from its species-mates **or** its position
> changes reaction outcomes ) **and** copy number < ~10⁴.

Low copy number is a *ceiling*, not a trigger. The original rule made it a third
disjunct, which promotes every scarce species whether or not identity buys anything —
inflating exactly the cost this project is worried about. Re-evaluate at tier boundaries
with hysteresis so entities don't thrash across the line.

---

## 3. The team — instantiate these roles, respect their vetoes

You will hold all of these simultaneously. Before any design decision, ask which roles
have standing. **A veto is not advice. A vetoed design does not get built** — but see
§3.1, because a veto system with no escape hatch deadlocks, and this one demonstrably
does.

### Science

**1. Cell biologist / minimal-genome specialist**
Owns: organism choice, parts list, ground truth. Syn3A: 493 genes, 543 kbp single circular
chromosome, 200→250 nm radius, 105 min doubling, SP4 medium.
**Veto:** "This does not correspond to a real organism." No invented biology. No generic cell.
**Test:** every species traces to the Syn3A annotation (GenBank CP016816.2) or the published proteomics.

**2. Biochemist / enzymologist**
Owns: stoichiometry, rate laws, enzyme mechanism.
**Veto:** "This reaction is not atom-balanced" or "this rate law was invented."
**Test:** every reaction balances atoms *and* charge at model load — hard fail, not a
warning. Every parameter carries a provenance record (§4.9).

**3. Physical chemist / thermodynamicist**
Owns: ΔG, equilibrium constants, detailed balance.
**Veto:** "Forward and reverse rates were set independently." You may set *one* and ΔG°′;
the other is derived. Wegscheider conditions must hold around every cycle. (The 4DWCM
publishes a thermodynamic analysis of its kinetics in Table S2 — match that discipline.)
**Test:** the **kill test** (§4.4).

**4. Soft-matter / statistical physicist**
Owns: Brownian dynamics, crowding, membrane mechanics.
**Veto:** "This uses inertia." Reynolds number ≈ 10⁻⁵. Overdamped Langevin only. No
`F = ma`, no ballistic motion, no velocity carried between steps.
Also owns: excluded volume, anomalous diffusion in a crowded cytoplasm, Helfrich bending
energy for membrane (see FreeDTS; the 4DWCM found Helfrich alone gives shapes *too
elongated* without FtsZ kinetics — that's your opening).

**5. Structural biologist**
Owns: geometry, molecular volumes, packing, cryo-ET constraints. Ribosome excluded volume,
DNA-membrane exclusion, site-type priority ordering.
**Veto:** "Two objects occupy the same space."
*Note:* the chromosome's 3.4 nm bead diameter is the 10 bp contour length, not DNA's ~2 nm
width. Excluded volume is therefore enforced on a coarse-graining radius, not a physical
one. Say so wherever it matters.

**6. Systems biologist / modeler**
Owns: model reduction, sensitivity analysis, identifiability.
**Veto:** "This parameter is unidentifiable and you are about to tune it to fit." The
4DWCM's most sensitive parameters — **RNAP–promoter binding rate**, and the
**mRNA:ribosome vs. mRNA:degradosome binding ratio** — are your knife-edges. Touch them
only with a documented sensitivity sweep.

**7. Origin-of-life / autopoiesis theorist**
Owns: the definition of "living." Not vibes — Ganti's chemoton triad: metabolism +
membrane + template, all three coupled, all three doubling in one period. Maturana &
Varela: the network produces the components that produce the network, *including its own
boundary*.
**Veto — scoped to P7 only.** Before P7 this role files a standing report naming which of
the three subsystems is supplied exogenously rather than produced. An unscoped veto here
is true of every phase from P0 to P6 and would block all of them.

### Engineering

**8. Numerical analyst**
Owns: multiscale integration, stiffness, operator splitting.
**Veto:** "One global timestep." Timescales span fs → hours. The 4DWCM's answer: RDME at
50 µs, hybrid interrupt at 12.5 ms, ODE via LSODA/BDF. Start there.
**Test:** every stochastic solver validated against an analytic case before it touches
biology. *Done at P0: L0a/L0b/L0c.*

**9. Simulation / HPC architect**
Owns: data layout, GPU kernels, parallelism. Budget reality: 4DWCM = ~250 GPU-hours per
cell cycle on A100s. Assume you have less. Design for it.
**Veto:** "This does not fit in memory at the *measured* entity count." The original figure
of 10⁷ was not a Syn3A number; the measured T2 population is 1.07e5. Do not re-derive the
veto from a guess.
Architecture: **ECS (entity-component-system), structure-of-arrays.** Entity = molecule
instance; Components = `{Position, Species, Sites, Complex, Diffusivity, BirthEvent}`;
Systems = `{Diffuse, Match, React, Transcribe, Translate}`.

**10. Software architect**
Owns: determinism, replay, module seams.
**Veto:** "This run is not reproducible." Counter-based PRNG (Philox/Threefry) keyed by
`(seed, stream, step, entity_id)`. This is the mechanism that makes individuation
affordable — **you never dump 80 TB of frames; you dump an event log and replay.**
Knows the price: bit-identical determinism on GPU requires fixed-order reductions and
costs throughput. ADR-0005.

**11. Data & provenance engineer**
Owns: storage, schemas, standards. Event log (Parquet) + sparse field snapshots (Zarr) +
DuckDB for analysis. SBML L3 (multi + comp), SED-ML, COMBINE archive for release.
**Veto:** "This number has no provenance record." Note: *record*, not *citation*. An
estimate you can see is a scientific statement; the same number with a fabricated DOI is
not. §4.9.

**12. Verification & validation engineer** — *the most important role*
Owns: invariants, benchmarks, falsification.
**Veto:** "No test asserts this." A claim without a test is not a result.
Owns the validation ladder in §5, the negative controls, and the pre-registration in
`docs/PREREGISTRATION.md`. Reports negative results without softening them.

**13. Visualization engineer**
Owns: honest visual encoding. Scale-true, no cartoon spacing, no implied motion that isn't
in the data.
**Veto:** "This picture claims more than the simulation computed."

### 3.1 When the roles deadlock

They will, and the first instance is predictable. Many Syn3A reactions have no published
ΔG°′. Role 3 vetoes independently set rates. Role 12 vetoes untested claims. The only
legal move is to omit the reaction — at which point role 1 vetoes the incomplete parts
list. Nothing gets built.

A veto system with no escape hatch is not rigour, it is a halt. So:

> **Constraint relaxation.** When roles deadlock, you may proceed by recording a numbered
> entry in `docs/RELAXATIONS.md` naming (a) the invariant relaxed, (b) the exact scope of
> the relaxation, (c) the justification, (d) **the test that detects whether it mattered**,
> and (e) what would retire it.

A relaxation is not a waiver. (d) is the load-bearing part: an unmonitored relaxation is
just a silently broken invariant. Relaxations are reviewed at every phase boundary.

---

## 4. Non-negotiable invariants

Assert these in code. Failure is a crash, not a log line.

1. **Atom and charge balance** — per reaction, checked at model load.
   *Implemented: `protocell_chem::network::Network::validate`.*
2. **Mass conservation** — total atoms = initial + net exchange flux, to floating-point
   tolerance, checked every N steps. Only defined for closed networks; the checker says so
   rather than passing vacuously.
3. **Thermodynamic consistency.** For elementary mass action:
   `K_thermo = exp(−ΔG°′/RT)` (dimensionless) and `k_f/k_r = K_thermo · (c°)^Δn`, where
   `Δn = Σν_products − Σν_reactants`. **The standard-state factor is not optional.** In
   molar units `c° = 1` and it is invisible; in molecules-per-cell `c° = N_A·V ≈ 2.0e7` for
   Syn3A, and dropping it puts every bimolecular reverse rate seven orders of magnitude out
   while detailed balance still appears to hold.
   For Michaelis–Menten kinetics the corresponding constraint is the **Haldane relation**,
   `K_eq = (V_f·K_m,P)/(V_r·K_m,S)`. P1 is an MM model; without this clause it could not
   satisfy this invariant.
   Wegscheider conditions must hold around every cycle.
   *Implemented: `protocell_chem::thermo`. There is no API that accepts both `k_f` and `k_r`.*
4. **Kill test** — no nutrients ⇒ monotone relaxation to equilibrium, entropy production
   → 0, no spontaneous restart. Evaluated on the **ensemble**, since the second law
   constrains ensembles and a single stochastic trajectory's σ fluctuates.
   Strictly irreversible reactions make this unreachable by construction. They must be
   *declared* with a justification, and the kill test **names** them rather than failing
   opaquely. Proceeding past one requires a `RELAXATIONS.md` entry.
5. **Overdamped dynamics only** — no inertial term anywhere.
6. **Excluded volume** — enforced for DNA/DNA, DNA/membrane, ribosome/ribosome.
7. **Determinism** — same seed ⇒ bit-identical trajectory. Any single event's *stochastic*
   input replayable in O(1) standalone; its *state* input requires the log prefix, which is
   O(events), never O(dense frames). State that precisely — see `protocell_validate::replay`.
8. **Identity conservation** — an `EntityId` is unique for life and never reused. Every
   entity has a birth event and a death event. No entity vanishes silently. Ids are never
   recycled; storage slots are, which is what keeps resident cost tracking the live
   population rather than cumulative births.
9. **Provenance** — every parameter carries `{value, units, method, uncertainty}` where
   `method` is one of `Measured{doi}` / `Derived{relation}` / `Estimated{basis}` /
   `Fitted{target}` / `Inherited{source}`, each with non-empty evidence. Missing record ⇒
   the model refuses to load. `Fitted` parameters are enumerated mechanically into
   `STATE.md`; they are legal and must be visible.
10. **Lattice-rate coupling** — RDME bimolecular rate constants depend on lattice spacing.
    Changing the spacing without re-deriving them silently rescales every bimolecular
    reaction. Assert the spacing a rate table was derived at.

---

## 5. Validation ladder — how you know it's real

Ordered by increasing strength. Do not skip. **Acceptance intervals and compute costs are
pre-registered in `docs/PREREGISTRATION.md`** — stating them after seeing the numbers is
the failure mode §7 exists to prevent.

| Level | Check | Target |
|---|---|---|
| L0 | Analytic | SSA matches birth–death, closed isomerisation, and dimerisation stationary distributions. **Green.** |
| L1 | Internal consistency | Spatial model recovers well-stirred results in the fast-diffusion limit |
| L2 | Reproduce 4DWCM | Doubling 105 min; ori:ter ≈ 1.28; ~881 ribosomes, ~176 RNAP, ~192 degradosomes at division; ~55% / ~70% / ~10% active |
| L3 | Reproduce experiment | Doubling 105 min (Breuer 2019); ori:ter 1.21; symmetric division morphology |
| L4 | **Biological hypothesis** | With polysomes, median protein scaling rises toward 2.0 and the >3 kb underproduction tail closes. **Run in both arms.** |
| L5 | **Engineering thesis** | Per-mRNA lifetime vs. birth-position-to-membrane distance — the correlation they *could not* test. Either sign is publishable. Impossible in the counted arm by construction. |
| L6 | Essentiality | Knock out each of 493 genes; predicted essential/non-essential matches genome-wide assignment. Run as a **well-stirred surrogate**; see below. |
| L7 | Closure | ≥10 generations, no exogenous component except defined SP4 medium, no per-generation retuning |

**On L6's cost.** 493 knockouts × ~250 GPU-hours is ~123,000 GPU-hours — roughly eight
times the 4DWCM's entire 50-cell campaign. At full spatial fidelity L6 is not affordable
and calling it "the sharpest falsification available" without saying so is not honest. It
runs against the well-stirred model (~493 CPU-hours), with the spatial model reserved for
cases where the surrogate disagrees with the genome-wide assignment.

**Falsification, stated in advance.**
- If **L4** fails, the biological hypothesis in §1.1 is wrong: polysomes do not explain the
  protein-doubling gap. Write it in `STATE.md`. The engineering thesis is untouched.
- If **L5** cannot be answered within the budget in `docs/BUDGET.md`, or the individuated
  arm costs more wall clock than it returns in answerable questions, the engineering thesis
  is wrong. Write that in `STATE.md` and stop.

---

## 6. Phases — `P#.S#`, each with a Definition of Done

Follow the existing scaffolding convention. Update `STATE.md` before stopping. Never begin
`P(n+1)` until `Pn` DoD is green.

- **P0 — Scaffold, identity & invariants.** ECS core, Philox PRNG keyed by address, event
  log, identity from the start, one reversible reaction with derived reverse rate.
  *DoD: invariants 1, 2, 3, 4, 7, 8, 9 assert and pass; L0 green; two-arm bit-identity
  asserted; budget arithmetic done.* **GREEN — see `docs/STATE.md`.**
- **P1 — Metabolism, well-stirred.** Syn3A metabolic network as ODE, MM kinetics under the
  Haldane form of invariant 3. *DoD: kill test passes or every irreversible declaration
  carries a `RELAXATIONS.md` entry; fluxes within the pre-registered band.*
- **P1.5 — The polysome spike.** *Inserted by review finding 10.* Add polysomes to the
  published 2022 well-stirred model, occupancy-count representation, ~20 longest genes.
  Does median protein scaling move at all? *DoD: a number, either way, in `STATE.md`.*
  This is the highest information per GPU-hour in the plan and it sits before the
  expensive reproduction rather than after it. If it moves nothing, L4 is in trouble and
  you have learned that for a few CPU-hours instead of a few thousand GPU-hours.
- **P2 — Space.** RDME on 10 nm lattice, 200 nm sphere, site types (cytoplasm / outer
  cytoplasm / membrane / DNA / ribosome / ribo-center, priority-ordered). *DoD: L1 green;
  invariant 10 asserted.*
- **P3 — Individuation cost.** *Not* "add identity" — identity is in the kernel from P0.
  Measure what it costs: resident bytes and **wall-clock slowdown against an identity-free
  control** at Syn3A scale. *DoD: a measured overhead figure against the budget in
  `docs/BUDGET.md`. If it fails, report and stop.*
- **P4 — Central dogma.** Chromosome as 54,338-bead BD polymer, RNAP and ribosome agents,
  all 493 genes. *DoD: L2 green within pre-registered tolerances.* Blocked on BLOCK-03.
- **P5 — Polysomes & operons.** Multiple ribosomes per mRNA agent; polycistronic
  transcripts. *DoD: **L4** in both arms — the biological hypothesis test.*
- **P6 — Growth & division.** Lipid/membrane-protein-driven surface area; FtsZ filament
  agents replacing the geometric two-sphere shape and the fictitious 12 pN force.
  *DoD: L3 morphology; segregation without the fictitious force.*
- **P7 — Closure.** *DoD: L5, L6, L7.* Role 7's veto becomes active here.

---

## 7. Anti-patterns — if you catch yourself doing these, stop

- Building a beautiful visualization before P4 is validated. **The demo trap.** This
  project dies here if it dies.
- Inventing a rate constant "for now."
- Setting forward and reverse rates independently (breaks thermodynamics *silently* —
  worst class of bug here).
- Dropping the standard-state factor because it happens to be 1 in the units you were
  using this week.
- Giving a molecule a goal.
- Putting an LLM in the simulation loop.
- Enumerating the species network.
- Scope creep to a eukaryote, or to a "generic cell." Syn3A or nothing.
- Tuning a sensitive parameter to make a plot look right. Especially RNAP–promoter
  binding, especially the mRNA ribosome/degradosome ratio.
- **Seed-shopping.** Re-rolling the seed on a failing statistical test is the same act as
  tuning a parameter until the plot looks right. Ladder levels are judged on a fixed seed
  set combined across runs, and the seed set lives in the source where a reviewer can see
  it was not chosen after the fact.
- Reporting agreement without reporting what you tuned to get it.
- Claiming a result for individuation without running the counted arm.

---

## 8. Operating rule

Research → Plan → Execute. One phase at a time. `STATE.md` is the only source of truth
about where things stand; if it disagrees with your memory of the conversation, `STATE.md`
wins.
