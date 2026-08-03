# REVIEW.md — Review of the original scaffold, 2026-08-03

The scaffold as delivered was five markdown files, 399 lines, no code. It had unusually
good instincts: falsification pre-registered, a standing commitment to report the
negative result, a named list of knife-edge parameters forbidden from tuning, and an
anti-patterns section that correctly identified the demo trap. Most of that survives
unchanged into the current `PROMPT.md`.

This file records what was wrong with it and what was done about each item, so that the
revisions are auditable rather than silent. Findings are in descending order of cost.

---

## 1. The stated falsification did not test the stated thesis

**Was.** Thesis (§1): *identity is the missing state variable.* Falsification (§5):
*if polysomes and operons do not close the protein-doubling gap, the thesis is wrong.*

Those are different claims, and L4 does not discriminate:

- L4 passes → polysomes matter. Says nothing about whether individuation was needed.
- L4 fails → "polysomes explain the long-gene deficit" is wrong. The engineering thesis
  is untouched.

The dependency table also overstated its case. Item 3 (polycistronic operons) is **not**
downstream of individuation — a polycistronic transcript is definable as a species in a
plain count-based CME. Item 2 (polysomes) is only partly downstream: ribosome occupancy
can be carried as an integer internal state on a counted mRNA species (`mRNA_k`,
k = 0…L/80), which is bounded and does not explode. That is the standard TASEP treatment
of translation, and it delivers polysomes with no individuation at all.

What genuinely requires identity is item 1 itself and item 5 — the per-mRNA
birth-position-to-lifetime correlation. That is **L5**.

**Now.** The thesis is split in two (`PROMPT.md` §1): an engineering thesis tested at L5,
and a biological hypothesis tested at L4. Every claim of the form "individuation buys X"
must be run against a non-individuated arm that is otherwise identical
(`docs/PREREGISTRATION.md` §2). The two-arm machinery exists at P0 —
`protocell-chem::ssa` runs counted and individuated arms from one seed and asserts the
count trajectories are bit-identical, so individuation is a measurable overhead rather
than a confound.

## 2. Missing prior art that already solves individuation

**Was.** Nothing in the five files mentioned particle-based reaction–diffusion —
Smoldyn, MCell, ReaDDy, eGFRD, Spatiocyte — all of which track individual particles with
position and identity by construction. The thesis was framed as though identity were
unexplored. It is not; it is the standard trade-off, and the 4DWCM chose RDME for speed
knowing the price.

**Now.** `docs/BASELINE.md` §3 covers the particle-based family and TASEP. The
contribution is reframed: identity-preserving methods exist and do not reach whole-cell
scale; the work is making them affordable there. `INDEX.md`'s "adopt, don't reinvent"
rule is applied consistently rather than only to Kappa/BNGL.

Related risk, previously unnamed: NFsim's network-free matching is **well-stirred**.
Spatial rule-based simulation is a much thinner field (MCell-R, SpringSaLaD, Simmune),
none of it GPU, none at whole-cell scale. The intersection of network-free matching, a
10 nm RDME lattice, GPU, and 10⁵–10⁶ entities is the real research risk. It is now in the
risk register.

## 3. The affordability claim had no arithmetic, and its load-bearing constraint was unstated

**Was.** Everything rested on "event log instead of an 80 TB frame dump", with no
bytes/event, no events/second, and no resulting volume anywhere.

**Now.** Computed: `cargo run -p protocell-cli -- budget`, recorded in `docs/BUDGET.md`.
The naive design — logging particle motion — is **215 TB per cycle, ~3× worse than the
80 TB dump it replaces**. The actual design is **8 MB, 1.0e-7 of it**. The gap between
those two numbers is the whole architecture, and it exists only under a constraint the
scaffold never stated:

> Everything reconstructible must be a pure function of `(seed, stream, step, entity_id)`,
> and only non-reconstructible facts go in the log.

That is now an enforced property of `protocell-core::rng` with its two consequences
spelled out (no state-dependent RNG consumption; distinct purposes get distinct streams,
never sequential draws). `docs/DECISIONS.md` ADR-0002.

Also corrected: P3's Definition of Done measured *log size* while the risk register
worried about *resident memory and throughput*. Different quantities; it measured the
easy one. P3 now measures wall-clock slowdown against an identity-free control.

## 4. The project-killing risk was deferred to P3 when it is an hour of arithmetic

**Was.** §1's "honest risk" — individuation costing more than it buys — was to be
measured at P3, most of a year in.

**Now.** Computed at P0. Syn3A's T2 population is **1.07e5 entities, 6.9 MB resident** —
three orders of magnitude below the HPC role's own 10⁷-entity veto threshold, which was
not a Syn3A figure. Memory is not the risk. Throughput may still be, and that is what P3
now measures. `docs/BUDGET.md`.

## 5. The tier promotion rule over-promoted (spec bug)

**Was.** T2 if **(a)** copy number < 10⁴ **or (b)** carries distinguishing information
**or (c)** position changes outcomes. The `or` on (a) promotes every low-abundance
species regardless of whether identity buys anything — free EF-Tu at 10³ copies gets an
entity record for no information gain, inflating the very cost the project feared.

**Now.** `T2 iff (b or c) and copy number < ~10⁴`. (a) is a ceiling, not a trigger.

## 6. Invariant 3 was dimensionally wrong and blocked P1

**Was.** `k_f/k_r = exp(−ΔG°′/RT)`. True only for unimolecular reactions. For Δn ≠ 0,
`K_eq` carries units of `(c°)^Δn`.

In molar units `c° = 1` and the error is invisible. In molecules-per-cell — the natural
unit for a stochastic whole-cell model — `c° = N_A·V ≈ 2.0e7` for a 200 nm Syn3A cell,
and every bimolecular reverse rate is wrong by seven orders of magnitude while detailed
balance still *looks* satisfied.

Worse: P1 is a metabolic ODE model, written in Michaelis–Menten form. MM rate laws have
no `k_f`/`k_r` pair, so **P1 could not satisfy P1's own invariant.**

**Now.** `protocell-chem::thermo` carries `c°` explicitly via `ConcentrationUnit`, and
the Haldane relation `K_eq = (V_f·K_m,P)/(V_r·K_m,S)` is implemented and tested as the MM
form of invariant 3. There is deliberately no API that accepts both `k_f` and `k_r`:
setting them independently is unrepresentable rather than merely forbidden.

## 7. The kill test forced total reversibility

**Was.** Invariant 4 required monotone relaxation with entropy production → 0. Any
strictly irreversible reaction runs to completion instead of equilibrating and pins σ at
infinity. Whole-cell models are full of steps declared irreversible for convenience, so
this was not hypothetical — invariant 4 silently committed the project to a ΔG°′ for
every reaction, many of which are unpublished.

**Now.** Irreversibility must be *declared* with a justification, which is recorded. The
kill test **names** the irreversible reactions rather than failing opaquely, and the
route to proceeding is a numbered entry in `docs/RELAXATIONS.md`. R-0001 covers it.

## 8. Determinism versus GPU was an unpriced constraint

**Was.** Invariant 7 (bit-identical) and role 9 (GPU kernels) are in tension:
`atomicAdd` reduction order is not deterministic. Solvable — fixed-order reductions,
fixed-point accumulators — at a typical cost of 10–30% throughput, on a project whose
premise is "assume you have less than 250 GPU-hours".

**Now.** In the risk register and in ADR-0005, with the propensity-sum ordering
requirement stated at the point in the code where it will be violated.

## 9. Phase 3 was sequenced before the thing it operated on

**Was.** P3 (individuation) preceded P4 (central dogma), but P3's DoD was "a named mRNA
is traceable end-to-end" — and mRNA does not exist until P4. Combined with the rule
against starting P(n+1) early, this forced a stub transcription system that P4 would
throw away.

**Now.** Identity is a kernel property, present from P0 (`protocell-core::entity`). P3
retains the *measurement*: what individuation costs in resident bytes and wall clock.

## 10. The validation ladder was uncosted, and L6/L7 were unaffordable

**Was.** L6 (essentiality, 493 knockouts) at ~250 GPU-h each is **~123,000 GPU-hours**,
roughly 8× the entire 4DWCM 50-cell campaign, and was listed as "the sharpest
falsification available" with no cost note. L7 (≥10 generations) is ~2,500 GPU-h per
lineage. L2 and L3 had no tolerance bands at all, so "reproduce" would be decided after
seeing the numbers — the exact failure mode §7 warned against.

**Now.** `docs/PREREGISTRATION.md` states acceptance intervals in advance and costs every
level. L6 runs as a well-stirred surrogate (~493 CPU-hours) with the spatial model
reserved for disagreements.

Also: P4's DoD was "L2 green" — reproduce a *Cell* paper in full — gated ahead of P5
where the thesis actually lives. The risk register rated that Medium; it is the largest
risk in the project. A de-risking spike now sits at **P1.5**: add polysomes to the
published 2022 well-stirred model for the ~20 longest genes and see whether median
protein scaling moves at all. Highest information per GPU-hour in the plan, and it tests
the biological half of the thesis without a multi-person-year reproduction in front of it.

## 11. The role protocol deadlocked with no escape hatch

**Was.** Thirteen roles, absolute vetoes, no tiebreaker. A concrete deadlock arrives in
P1: many Syn3A reactions have no published ΔG°′; role 3 vetoes independently set rates;
role 12 vetoes untested claims; so the only legal move is to omit the reaction — at which
point role 1 vetoes the incomplete parts list. Nothing gets built.

Separately, role 7 (autopoiesis) held a veto — "this is a chemical reactor, not a cell" —
true of P0 through P6 inclusive, so it blocked everything.

**Now.** `AGENTS.md` defines a **constraint relaxation**: name the invariant relaxed, its
scope, the justification, and the test that detects whether it mattered, in
`docs/RELAXATIONS.md`. Role 7's veto is scoped to P7; before that it files a standing
report.

## 12. Missing dependencies and paperwork

- **Are the 4DWCM parameter tables and initial conditions actually public?** Invariant 9
  makes L2 impossible if not. Day-one blocking dependency, absent from the risk register.
  Now **BLOCK-01**, and the first thing P1 does.
- **Invariant 9 versus reality.** The 4DWCM itself tuned ribosome binding +30% and
  degradosome binding −70%; those values have no primary DOI. A literal "cite everything
  or refuse to load" rule makes L2 unreachable. Now: *missing provenance record* ⇒ refuse
  to load, where `Estimated`/`Fitted`/`Inherited` are legal and must carry a
  justification. `ParamTable::fitted_parameters()` generates the disclosure `AGENTS.md`
  requires, mechanically, so it cannot go stale.
- **No LICENSE decision.** LAMMPS is GPL; LM 2.5 and pyLM/jLM have their own terms;
  `INDEX.md` promises a COMBINE archive release. Now **BLOCK-02**.
- **No checkpointing**, on 4–6 day runs, interacting with determinism (a checkpoint must
  capture the PRNG counter state exactly). Now ADR-0006.
- **Compute budget** was an open question. It determines whether L2 is attemptable and
  therefore whether the phase plan is real. Now **BLOCK-03**, blocking entry to P4.

## 13. Smaller corrections

- **T0 was called "Field" but is well-stirred ODE.** A field is spatial; this is pooled.
  Renamed **T0 — Bulk**. The modelling choice is fine (the 4DWCM does the same); the
  label contradicted the project's own premise.
- **RDME bimolecular rates are lattice-spacing dependent** (Isaacson breakdown below the
  reaction radius). At 10 nm this is fine, but changing the spacing silently rescales
  every bimolecular rate. Now an invariant of its own (§4.10).
- **Volume arithmetic.** 200 → 250 nm is (1.25)³ = **+95.3%**, not the "~98%" in
  `BASELINE.md`. Corrected and flagged for verification against the paper.
- **P0's DoD was partly vacuous.** Invariants 1 and 3 asserted against an abstract
  `A ⇌ B` pass trivially — A and B have no atoms. P0 now uses real formulas, and
  `Network::is_abstract()` makes a vacuous balance check report itself.
- **P1's "order of magnitude" flux agreement** is 10× slack propagating into P4.
  Tightened in `docs/PREREGISTRATION.md`.
- **Bead diameter 3.4 nm** is the 10 bp contour length, not DNA's ~2 nm width. Fine as
  coarse-graining, but invariant 6 is then enforced on a fictitious radius. Noted where
  role 5's veto lives.
- **Author list** was inconsistent between `PROMPT.md` and `INDEX.md`. Pinned.

---

## Found while implementing, not in the original review

**A sampling bias in the L0 harness.** After an SSA step, `time` is the instant the new
state took effect. Running until `time >= t` therefore records the state that begins
*after* `t`, not the one spanning it — every sample pulled one jump forward. The first
moment still looked right; the distribution did not. L0b caught it at χ² = 45 on 20
degrees of freedom. Fixed in `ladder::sample_at`, and it is the argument for
distributional benchmarks over "the mean looks about right".

**Single-seed goodness-of-fit tests are a seed-shopping trap.** A correct benchmark fails
a 1%-level test 1% of the time, and the reflex is to try another seed — the same act as
tuning a parameter until a plot looks right. Each ladder level is now judged on eight
fixed seeds combined by Fisher's method. Seed `0x2222` is retained in the suite as the
counter-example: it fails individually while the model is correct. A 40-seed uniformity
check is available behind `--ignored`; last run gave medians 0.58 / 0.54 / 0.41 and
combined p 0.38 / 0.81 / 0.20, showing no bias at any level.

**The naive log estimate was wrong in the project's favour, twice over.** A careless
version of finding 3 gives "~4 PB, 50× worse". The real figure is 215 TB, ~3× worse. The
conclusion survives; the inflated version of it does not, and only one of those is worth
writing down.
