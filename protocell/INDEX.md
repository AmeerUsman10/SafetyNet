# INDEX.md — Repository map

## Documents

| Path | Purpose |
|---|---|
| `PROMPT.md` | Master directive. Thesis, agent ontology, the 13 roles, invariants, validation ladder, phases |
| `AGENTS.md` | Rules of engagement. Read second |
| `INDEX.md` | This file |
| `docs/STATE.md` | **Single source of truth for current state.** Read last, write first |
| `docs/REVIEW.md` | Review of the original scaffold and what each finding changed |
| `docs/BASELINE.md` | 4DWCM reference numbers, prior art, and the gap being closed |
| `docs/BUDGET.md` | The memory and event-log arithmetic. Computed, not asserted |
| `docs/PREREGISTRATION.md` | Acceptance intervals and costs, fixed **before** the runs |
| `docs/DECISIONS.md` | Architecture decision records |
| `docs/RELAXATIONS.md` | Register of relaxed invariants, each with its detecting test |

## Code

| Path | Purpose | Phase |
|---|---|---|
| `crates/core/` | ECS entity table, `EntityId` allocation, Philox PRNG, event log | P0 ✅ |
| `crates/chem/` | Formulas, balance checking, thermodynamics, provenance, well-stirred SSA | P0–P1 ✅ |
| `crates/validate/` | Statistics, the L0 ladder, kill test, replay, budget arithmetic | P0 ✅ |
| `crates/cli/` | `protocell {budget,l0,kill,arms,dod}` | P0 ✅ |
| `spatial/` | RDME lattice, site types, diffusion | P2 — planned |
| `dogma/` | Chromosome BD, RNAP, ribosome, degradosome | P4–P5 — planned |
| `membrane/` | Growth, FtsZ filaments, division | P6 — planned |
| `params/` | Syn3A parameter tables. Load fails on a missing provenance record | P1 — planned |

Directories marked *planned* do not exist yet. They are named here so the seams are
decided before the code arrives, not after.

## Running it

```sh
cargo test --workspace --release          # 65 tests, ~5 s
cargo run -p protocell-cli -- dod         # P0 Definition of Done, end to end
cargo run -p protocell-cli -- budget      # the two de-risking calculations
cargo run -p protocell-cli -- l0          # analytic ladder, 8 seeds per level
cargo run -p protocell-cli -- arms        # counted vs individuated, same seed
cargo run -p protocell-cli -- kill        # invariant 4

# The deep p-value uniformity check (~30 s), opt-in:
cargo test --release -p protocell-validate -- --ignored --nocapture
```

## External baseline

- Thornburg, Maytin, Kwon, … Luthey-Schulten, *Cell* (2026). doi:10.1101/2025.06.10.658899 (preprint)
- Thornburg et al., *Cell* **185**, 345 (2022) — well-stirred predecessor
- Lattice Microbes 2.5 / pyLM / jLM · LAMMPS · odeCELL (LSODA) · FreeDTS
- Syn3A genome: GenBank **CP016816.2** · Sequencing: NCBI SRA **PRJNA1257452**
- Kappa (KaSim) / BioNetGen (NFsim) — rule-based agent semantics. Adopt, don't reinvent
- **Particle-based reaction–diffusion** — Smoldyn, MCell, ReaDDy, eGFRD, Spatiocyte.
  These already preserve particle identity; see `docs/BASELINE.md` §3 for why that matters
  to how this project's contribution should be stated
- **TASEP / ribosome-flow models** — the established formalism for polysome traffic.
  Adopt at P5 rather than reinventing
- **Spatial rule-based** — MCell-R, SpringSaLaD, Simmune. The closest prior art to the
  actual research risk, and none of it is GPU or whole-cell

## Licensing

Undecided, and blocking. LAMMPS is GPL; LM 2.5 and pyLM/jLM carry their own terms; a
COMBINE archive release implies redistribution. Tracked as BLOCK-02 in `docs/STATE.md`.
Settle it before taking a hard dependency, not after.
