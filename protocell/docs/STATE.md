# STATE.md — Single source of truth

**Updated:** 2026-08-03 · **Phase:** P0 complete, P1 started · **Status:** **GREEN** · **Next:** P1.S2

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
| `protocell-chem` | Formula parsing and atom/charge balance at load. Thermodynamics with the standard-state factor and the Haldane relation. Provenance types where an unjustified value is unrepresentable. Well-stirred SSA with individuation as an overlay. **`glycolysis`: the first real P1 slice — 5 reactions, checker-verified balance, independently confirmed against the real Syn3A SBML.** |
| `protocell-validate` | Dependency-free statistics (log-gamma, incomplete gamma, chi-square, Fisher combination). L0a/b/c plus negative controls. Kill test on ensembles. Replay. The budget arithmetic. |
| `protocell-cli` | `protocell {budget, l0, kill, arms, glycolysis, dod}` |

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

## BLOCK-01 — substantially resolved, 2026-08-03

Research subagent + direct clone confirmed: **the real Syn3A parameter data exists and is
obtainable.** `https://github.com/Luthey-Schulten-Lab/Minimal_Cell` is the actual code
release for Thornburg et al. 2022, cloned and inspected directly in this session (not just
indexed). It contains:

- `syn3A.gb` — the real GenBank file, header-verified as `CP016816.2`, matching this
  project's stated genome accession exactly.
- `CME_ODE/model_data/iMB155_NoH2O.xml` — an SBML metabolic model sourced from
  **Breuer et al. 2019, *eLife* 8:e36842, doi:10.7554/eLife.36842** (the real Syn3A
  metabolic reconstruction — this is the primary citation P1 should build against).
- `Nucleotide_Kinetic_Parameters.tsv`, `Central_AA_*.tsv`, `lipid_NoH2O_balanced_model.tsv`,
  `transport_*.tsv` — real kinetic parameter tables in SBtab format, with reversible rate
  laws in convenience-kinetics / Haldane form (`kcrg`/`keq`/`kmc` per reaction) — the same
  structure `thermo::haldane_keq` already implements, for the same physical reason.

**Independent confirmation, not just data availability.** The SBML species declarations are
a bit-for-bit match to the formulas hand-derived for `chem::glycolysis` before this file was
found: G6P/F6P `C6H11O9P`⁻², FBP `C6H10O12P2`⁻⁴, ATP `C10H12N5O13P3`⁻⁴, H⁺ `H`⁺¹. The
glycolysis module's chemistry is independently verified, not merely self-consistent.

**What remains open.** The repo carries **no LICENSE file** at its root (confirmed by
direct inspection). BLOCK-02 is therefore partly answered in the negative: this specific
compiled parameter set is not established as reusable, and its exact fitted values were
deliberately **not** copied into this codebase — see `glycolysis::source_note` and the
module doc for the reasoning. The correct path is to build from the *primary* source
(Breuer et al. 2019, or eQuilibrator) independently, or to contact the authors about reuse
terms, not to copy the downstream compilation.

Network policy in this environment blocked direct verification of the papers' own SI files,
the NCBI record, and the Zenodo archives (403 on cell.com, ncbi.nlm.nih.gov, zenodo.org) —
those remain "should be accessible" rather than "confirmed" and should be re-checked from an
unrestricted environment before being treated as load-bearing.

**Net effect on the phase plan:** L2 is very likely attemptable — real, structured Syn3A
data exists and a path to it is documented — but the parameter values themselves still need
to be sourced from the primary literature rather than harvested wholesale from this repo.

---

## Next action — P1.S2

P1.S1 delivered a first real slice: `chem::glycolysis`, the ATP-consuming half of the EMP
pathway (hexokinase → PGI → PFK → aldolase → TPI), atom/charge balanced under invariant 1
using real BiGG-convention formulas (now independently confirmed against the real Syn3A
SBML above), thermodynamically self-consistent under invariant 3, mass-conserving over
20,000 real SSA steps. ΔG values are honestly `Estimated`, not fabricated as `Measured`.
`cargo run -p protocell-cli -- glycolysis`.

Remaining for P1's Definition of Done:

1. The rest of central carbon metabolism (payoff phase of glycolysis, TCA remnants —
   Syn3A's is truncated — and the pentose phosphate pathway), same pattern: real formulas,
   checker-verified balance, honestly-labelled thermodynamics.
2. **Source real ΔG′/kinetic values from Breuer et al. 2019 directly** (not from the
   Minimal_Cell repo's compiled tables — see BLOCK-01 above) to upgrade the glycolysis
   module's parameters from `Estimated` to `Measured` with a real DOI.
3. Kill test over the assembled metabolic network. Expect irreversible declarations; each
   needs a `RELAXATIONS.md` entry with a detecting test. A-0001 and A-0002 anticipate the
   shape.
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
| Independently derive chemistry, verify against real data after | The `glycolysis` formulas were hand-derived from BiGG convention, *then* checker-verified (invariant 1) and *then* independently confirmed bit-for-bit against the real Syn3A SBML. Confirmed a result rather than assumed one |
| Don't copy unlicensed data even when it would be faster | `Minimal_Cell`'s compiled parameter tables have no LICENSE. Re-derive from Breuer 2019 directly rather than harvest the compilation |

## Open questions and blockers

| Id | Question | Blocks | Status |
|---|---|---|---|
| **BLOCK-01** | Are the 4DWCM parameter tables and initial conditions actually public and obtainable? | L2, therefore P4+ | **Substantially resolved 2026-08-03.** Real data confirmed via direct clone: `Luthey-Schulten-Lab/Minimal_Cell` (genome, SBML metabolic model, kinetic parameter tables), sourced from Breuer et al. 2019. Paper SI/Zenodo/NCBI still unverified (network policy blocked direct fetch); re-check from an unrestricted environment. |
| **BLOCK-02** | Licensing. LAMMPS is GPL; LM 2.5 and pyLM/jLM have their own terms; a COMBINE release implies redistribution | Any hard dependency on LM/LAMMPS | **Partly resolved, negatively.** `Luthey-Schulten-Lab/Minimal_Cell` has no LICENSE file at root (confirmed by direct inspection) — its compiled tables are not established as reusable. Do not copy its parameter values wholesale; rebuild from the primary source (Breuer 2019) or request reuse terms. LAMMPS/LM licensing itself still unresolved. |
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
