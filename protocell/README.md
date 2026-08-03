# Protocell

A whole-cell simulator in which every entity that has an identity *keeps* it —
individuated, addressable, persistent agents with position, internal state and history —
for the genetically minimal bacterium **JCVI-syn3A**.

The baseline is the 4D whole-cell model of Thornburg, Maytin, Kwon et al.
(*Cell*, 2026; preprint doi:10.1101/2025.06.10.658899). Its own limitations section states
that every mRNA is indistinguishable from every other mRNA of the same type, and that
following one would mean recording every RDME frame — **>80 TB per trajectory**.

This project asks whether identity can be made cheap enough to afford.

```
Event log for one Syn3A cell cycle ..............   8.08 MB
The same question by dense frame recording ......  80 TB
Ratio ...........................................   1.0e-7
```

Those numbers are computed, not asserted: `cargo run -p protocell-cli -- budget`.

---

## Status

**Phase P0: green.** 65 tests. The kernel, the thermodynamics, the analytic validation
ladder, and the two arithmetic checks that could have ended the project cheaply.

```sh
cargo test --workspace --release          # 65 tests, ~5 s
cargo run -p protocell-cli -- dod         # P0 Definition of Done, end to end
cargo run -p protocell-cli -- budget      # memory and event-log arithmetic
cargo run -p protocell-cli -- l0          # analytic ladder, 8 seeds per level
cargo run -p protocell-cli -- arms        # counted vs individuated, same seed
cargo run -p protocell-cli -- kill        # invariant 4
```

Start with `PROMPT.md`. Then `AGENTS.md`, `INDEX.md`, `docs/STATE.md`.

---

## The three ideas that carry the project

**1. Every random draw is addressed, not sequential.**
`(seed, stream, step, entity_id) → [u32; 4]`, via Philox4x32-10 verified against the
Random123 known-answer vectors. There is no RNG state anywhere. This is what makes the
event log small: anything recomputable is omitted from it and recovered on demand. Log
diffusion hops instead and the same trajectory costs 215 TB — *worse* than the frame dump
it was meant to replace. The architecture is the gap between those two numbers.

**2. Identity is conserved; storage is not.**
`EntityId` is unique for life and never reused, and every entity has a birth event and a
death event. Storage slots *are* recycled, so resident memory tracks the live population
rather than cumulative births. 10,000 births with never more than 11 alive allocate 11
slots — asserted by test.

**3. Nothing is attributed to individuation without a control.**
The same network runs with species `Counted` or `Individuated`. The count trajectories are
**bit-identical** from one seed — asserted, not hoped for — so individuation is a measurable
overhead rather than a confound. Possible only because of idea 1: the individuated arm
spends extra draws on a separate stream, and addressed draws mean spending them shifts
nothing else.

---

## What the review changed

This repository began as a five-file, 399-line directive with no code. `docs/REVIEW.md`
records 13 findings and what each one changed. The three that mattered most:

- **The stated falsification did not test the stated thesis.** L4 (polysomes close the
  protein-doubling gap) tests a biological hypothesis. The engineering thesis — that
  identity is affordable — is tested at L5, the per-mRNA correlation no counted
  representation can answer. Polysomes are reachable from a counted mRNA with an occupancy
  integer, which is the standard TASEP treatment, so L4 alone could never have isolated
  the variable the project is arguing about.
- **The cost claim had no arithmetic behind it,** and its load-bearing constraint was
  unstated. Both fixed above.
- **The project's stated killer risk was deferred to phase P3.** It was an hour of
  arithmetic. Individuation costs 6.87 MB for Syn3A — three orders of magnitude below the
  threshold that was feared. Memory is not the risk; throughput may be, and that is what P3
  now measures.

Two things found while implementing, not in the review: a sampling bias in the L0 harness
that the first moment hid and the distribution caught, and the fact that single-seed
goodness-of-fit tests are a seed-shopping trap. Both are written up in `docs/REVIEW.md`.

---

## Layout

| Path | |
|---|---|
| `PROMPT.md` | Master directive — thesis, roles, invariants, ladder, phases |
| `AGENTS.md` | Rules of engagement |
| `INDEX.md` | Repository map and how to run things |
| `docs/STATE.md` | **Single source of truth for current state** |
| `docs/REVIEW.md` | The 13 findings and what each changed |
| `docs/BASELINE.md` | 4DWCM numbers, prior art, the gap |
| `docs/BUDGET.md` | The arithmetic |
| `docs/PREREGISTRATION.md` | Acceptance intervals, fixed before the runs |
| `docs/DECISIONS.md` | Architecture decision records |
| `docs/RELAXATIONS.md` | Relaxed invariants, each with its detecting test |
| `crates/core` | Identity, PRNG, event log |
| `crates/chem` | Formulas, thermodynamics, provenance, SSA |
| `crates/validate` | Statistics, the ladder, replay, budget |
| `crates/cli` | `protocell` |
