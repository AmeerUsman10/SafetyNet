# STATE.md — Single source of truth

**Updated:** 2026-08-03 · **Phase:** P0 · **Status:** **GREEN** · **Next:** P1.S1

---

## Where things stand

**P0 is complete and green.** The scaffold was reviewed (`docs/REVIEW.md`, 13 findings),
the directives were revised against it, and the P0 kernel is built, tested and measured.

```
cargo test --workspace --release     65 tests, all passing, ~5 s
cargo run -p protocell-cli -- dod    P0 DoD: GREEN
```

### What exists

| Crate | Contents |
|---|---|
| `protocell-core` | Philox4x32-10 keyed by `(seed, stream, step, entity)`, verified against the Random123 known-answer vectors. Monotonic `EntityId` with slot recycling. Append-only, index-addressable event log with content digest. |
| `protocell-chem` | Formula parsing and atom/charge balance at load. Thermodynamics with the standard-state factor and the Haldane relation. Provenance types where an unjustified value is unrepresentable. Well-stirred SSA with individuation as an overlay. |
| `protocell-validate` | Dependency-free statistics (log-gamma, incomplete gamma, chi-square, Fisher combination). L0a/b/c plus negative controls. Kill test on ensembles. Replay. The budget arithmetic. |
| `protocell-cli` | `protocell {budget, l0, kill, arms, dod}` |

### P0 Definition of Done — measured, not asserted

| | Result |
|---|---|
| Invariant 1 — atom/charge balance rejects an unbalanced reaction | PASS |
| Invariant 2 — mass conserved over 20,000 steps | PASS |
| Invariant 3 — equilibrium matches the free energy (L0b) | PASS |
| Invariant 4 — kill test on a closed reversible system | PASS |
| Invariant 7 — same seed ⇒ bit-identical trajectory | PASS |
| Invariant 8 — identity conserved; every entity born and buried | PASS |
| Invariant 9 — no parameter loads without a provenance record | PASS |
| Single event's stochastic input recovers in O(1) | PASS |
| L0a / L0b / L0c, 8 seeds each, Fisher-combined | PASS (p = 0.75 / 0.016 / 0.28) |
| Two-arm bit-identity (counted vs individuated) | PASS |

Deep check, run once, opt-in: p-values uniform across 40 seeds per level — medians
0.58 / 0.54 / 0.41, combined p 0.38 / 0.81 / 0.20. No level shows bias.

### The two de-risking numbers

Computed at P0 rather than deferred to P3. `docs/BUDGET.md`.

- **Resident memory for individuation: 6.87 MB** (1.07e5 T2 entities × 64 B). The HPC role's
  10⁷-entity veto threshold was never a Syn3A figure; the real population is three orders of
  magnitude below it. **Memory is not the risk.**
- **Event log: 8.08 MB per cell cycle**, 1.0e-7 of the 4DWCM's 80 TB frame-dump estimate.
  The naive design that logs diffusion hops would be **215 TB — about 3× worse than the dump
  it replaces.** The gap between those is the architecture, and it holds only under
  ADR-0002's two rules.

---

## Next action — P1.S1

**First: resolve BLOCK-01.** Establish whether the 4DWCM's parameter tables and initial
conditions are actually obtainable. Invariant 9 makes L2 unreachable if they are not, and
everything from P4 onward assumes them. This is a day of literature and correspondence, and
it gates a year of work.

Then P1 proper — Syn3A metabolism, well-stirred:

1. `params/` — Syn3A metabolic parameters with provenance records. Expect many `Inherited`
   and `Estimated`; that is legal and labelled.
2. MM kinetics under the Haldane form of invariant 3 (`thermo::check_haldane`).
3. Kill test over the metabolic network. Expect irreversible declarations; each needs a
   `RELAXATIONS.md` entry with a detecting test. A-0001 and A-0002 anticipate the shape.
4. Wegscheider check over every cycle in the network.

**Definition of Done:** kill test passes or every irreversible declaration carries a tested
relaxation; fluxes within the band in `docs/PREREGISTRATION.md`; every parameter loads with
provenance; `fitted_parameters()` output pasted into this file.

Then **P1.5 — the polysome spike**, before the expensive reproduction. Add polysomes to the
published 2022 well-stirred model, occupancy-count representation, ~20 longest genes. Does
median protein scaling move at all? Highest information per GPU-hour in the plan.

---

## Decisions made

| Decision | Rationale |
|---|---|
| Target Syn3A, not a generic cell | A generic cell is unfalsifiable. Syn3A has genome, proteomics, cryo-ET, essentiality, sequencing, and a published whole-cell model to check against |
| Kappa/BNGL agent semantics | Already the rigorous formalization of "every thing gets an agent". Network-free evaluation defeats combinatorial explosion — but only well-stirred; see Risks |
| ECS, structure-of-arrays | The software realization of per-entity agency |
| Event-sourced lineage, not frame dumps | Verified: 8 MB vs 80 TB. Holds only under ADR-0002 |
| **Rust for the kernel** | ADR-0001. Resolves the scaffold's open question. Determinism is a language-level concern |
| **Thesis split in two** | The stated falsification tested the biology, not the engineering. L5 tests the thesis; L4 tests the hypothesis. `docs/REVIEW.md` finding 1 |
| **Two-arm rule** | No result attributed to individuation without a counted control. Enforced at P0 |
| **Identity in the kernel, not P3** | ADR-0004. P3 was circular as written |
| **Ladder judged on 8 fixed seeds, Fisher-combined** | A single-seed test flakes at 1% and the reflex is to re-roll. That is seed-shopping |
| Reproduce (L2) before extend (L4) | Non-negotiable |

## Open questions and blockers

| Id | Question | Blocks | Status |
|---|---|---|---|
| **BLOCK-01** | Are the 4DWCM parameter tables and initial conditions actually public and obtainable? | L2, therefore P4+ | **Unresolved. Do this first.** |
| **BLOCK-02** | Licensing. LAMMPS is GPL; LM 2.5 and pyLM/jLM have their own terms; a COMBINE release implies redistribution | Any hard dependency on LM/LAMMPS | Unresolved |
| **BLOCK-03** | What compute budget actually exists? ~5,750 GPU-hours are needed for L2+L4+L5+L7 | Entry to P4 | Unresolved. The phase plan past P3 is provisional until it is |
| Q-01 | Reuse LM directly, or reimplement RDME? | P2 | Resolve by inspection at P2, not by guessing now |
| Q-02 | Can network-free rule matching run against a 10 nm lattice at 10⁵ entities on a GPU? | P2, P4 | Unknown. This is the real research risk |

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| **Network-free matching + RDME lattice + GPU is unsolved in the literature** | **Highest, and newly named** | MCell-R is the closest prior art and is neither GPU nor whole-cell. Prototype at P2 before committing to P4 |
| Reproducing L2 takes longer than expected | **High** — it is most of the project's cost and gates the thesis test | P1.5 spike tests the biological hypothesis cheaply *before* L2. Do not skip to P5 |
| Compute budget insufficient for the ladder | High | BLOCK-03. L6 already moved to a well-stirred surrogate; L2/L4/L7 have no cheaper form |
| Individuation costs more throughput than it buys | Medium — *no longer memory* | Memory measured at 6.87 MB and dismissed. P3 measures wall clock against an identity-free control |
| Determinism costs GPU throughput | Medium | ADR-0005. 10–30% expected, unpaid so far |
| Demo trap — visualization before validation | High | No renderer before P4 is green |
| Silent thermodynamic violation | High | Invariant 3 with the standard-state factor + kill test, asserted. `k_r` is not settable |
| Accumulating unmonitored relaxations | Medium | `RELAXATIONS.md` requires a detecting test per entry; reviewed at every phase boundary |

## Tuned parameters

**None.** No parameter in the codebase carries `Method::Fitted`. Asserted by
`budget::tests::no_budget_input_was_fitted_to_produce_a_favourable_answer`.

This section is generated from `ParamTable::fitted_parameters()` and must be regenerated,
not hand-maintained, at every phase boundary. P1 will populate it — the 4DWCM's ribosome
+30% / degradosome −70% adjustments are `Inherited` with a note, and anything this project
tunes itself will be `Fitted` and will appear here.

## How to resume

> Read `PROMPT.md`, then `AGENTS.md`, then `INDEX.md`, then this file. Check the working
> tree for uncommitted files. Run `cargo test --workspace --release` and
> `cargo run -p protocell-cli -- dod` to confirm P0 is still green before building on it.
> Continue P1.S1 to its Definition of Done, then update this file before stopping.
