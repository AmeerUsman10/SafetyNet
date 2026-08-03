# DECISIONS.md — Architecture decision records

One entry per decision that would be expensive to reverse. Format: context, decision,
consequences — including the ones we would rather not have.

---

## ADR-0001 — Rust for the kernel

**Status:** accepted, 2026-08-03. Resolves the open question the scaffold left for P0.S1.

**Context.** Candidates were Rust (ECS ergonomics, determinism, no GC) and C++/CUDA
(proximity to Lattice Microbes and LAMMPS).

**Decision.** Rust for the kernel. CUDA via FFI at P2 when the RDME lattice arrives.

**Why.** Invariant 7 — bit-identical determinism — is the hardest constraint in the project,
and it is a language-level concern: no GC, no unspecified evaluation order, no undefined
behaviour silently reordering floating-point work, and `BTreeMap` where a `HashMap` would
have made iteration order part of the trajectory. Proximity to LM and LAMMPS matters less
than it appears, because both are separate processes coupled at the file and socket level,
not linked libraries.

**Consequences.**
- The GPU path is FFI, not native. Accepted; the kernels that need CUDA are P2+.
- No numeric ecosystem to lean on, so `validate::stats` implements log-gamma, the
  regularised incomplete gamma and chi-square goodness-of-fit directly. That turned out to
  be a feature — a goodness-of-fit test is not a place to inherit someone else's
  convergence criteria unread — but it is real work that a Python prototype would not have
  needed.
- Rust 1.94.1 verified working in this environment; CUDA toolchain is not present, which
  would have made the C++/CUDA choice unbuildable here today.

---

## ADR-0002 — Counter-based PRNG, addressed rather than sequential

**Status:** accepted. **This is the load-bearing decision of the project.**

**Context.** The cost claim is "event log instead of an 80 TB frame dump". `docs/BUDGET.md`
shows the naive version of that is 215 TB — *worse* than what it replaces. The log is only
small if reconstructible outcomes are omitted from it, and they can only be omitted if they
can be recomputed on demand.

**Decision.** Philox4x32-10, keyed by address:

```
(seed, stream, step, entity_id) -> [u32; 4]
```

No sequential state anywhere. `Rng` holds the seed and nothing else.

**Consequences.**
- **Two rules bind every system built on top.** (1) No state-dependent RNG consumption: the
  number of draws at an address must not depend on the branch taken. (2) Distinct purposes
  get distinct streams, never sequential draws from one stream. Both are stated at the top
  of `core/src/rng.rs` because both are invisible until replay diverges.
- A single event's *stochastic* input is O(1) recoverable. Its *state* input is not — that
  needs the log prefix, O(events). `PROMPT.md` invariant 7 says this precisely rather than
  claiming full O(1) replay, which would be false.
- It makes the two-arm comparison possible at all. The individuated arm spends extra draws
  on `Stream::ENTITY_PICK`; because draws are addressed, spending them shifts nothing else,
  so both arms produce bit-identical trajectories from one seed. With a sequential generator
  they would diverge on the first reaction and `PROMPT.md` §1.3 could not be enforced.
- Correctness is verified against the Random123 known-answer vectors, not just self-checked.
  If Philox were subtly wrong, "same seed ⇒ same trajectory" would still hold and every
  statistical test would fail mysteriously.

---

## ADR-0003 — Structure-of-arrays entity table, ids never reused, slots recycled

**Status:** accepted.

**Context.** Invariant 8 requires `EntityId` unique for life. Read literally alongside "no
entity vanishes silently", it suggests retaining every entity ever created.

**Decision.** Monotonic id counter that never rewinds; storage slots return to a free list on
death. `BTreeMap` for the live index, not `HashMap`.

**Consequences.**
- Resident memory tracks the **live** population, not cumulative births. For mRNA under
  turnover that is the difference between a bounded table and one that grows all cycle.
  Asserted by a test: 10,000 births with never more than 11 alive allocates 11 slots.
- History lives in the log, not in RAM. Consistent with ADR-0002.
- `BTreeMap` costs some lookup speed against a hash map. Accepted: iteration order is part
  of the trajectory, and `HashMap`'s order is not reproducible across runs.

---

## ADR-0004 — Identity is a kernel property, not phase P3

**Status:** accepted. Supersedes the original phase plan.

**Context.** The scaffold put individuation at P3, before the central dogma at P4. But P3's
Definition of Done was "a named mRNA is traceable end-to-end", and mRNA does not exist until
P4. With the rule against starting P(n+1) early, P3 would have required a stub transcription
system that P4 then discards.

**Decision.** Identity lives in `core::entity` from P0. P3 keeps the *measurement*: what
individuation costs in resident bytes and wall clock.

**Consequences.** P3 becomes a go/no-go gate on a number rather than a construction phase,
which is what the risk register wanted from it in the first place. The measurement is also
better targeted: the original P3 DoD measured log size (finding 3's easy quantity) while the
risk was throughput.

---

## ADR-0005 — Determinism constrains parallelism, and it costs

**Status:** accepted, with a cost we have not yet paid.

**Context.** Invariant 7 wants bit-identical trajectories. Role 9 wants GPU kernels.
`atomicAdd` reduction order on a GPU is not deterministic, and floating-point addition is not
associative.

**Decision.** Determinism wins. Reductions must be fixed-order; where that is too slow, use
fixed-point accumulation, which is associative and therefore order-free.

**Consequences.**
- Expect to give up roughly 10–30% throughput at P2+, on a project whose stated premise is
  "assume you have less than 250 GPU-hours". This is a real, unpaid cost and it belongs in
  the risk register rather than surfacing during P4 optimisation.
- Today's consequence is visible in `chem::ssa::refresh_propensities`: propensities are
  summed in fixed reaction order, because summing them in a different order changes the
  total and therefore the selected channel. The comment there is a warning to whoever
  parallelises it.

---

## ADR-0006 — Checkpoints must capture the RNG address, not an RNG state

**Status:** accepted, unimplemented (P2).

**Context.** Runs are 4–6 days. Restart-from-checkpoint is mandatory, and it interacts with
determinism. The scaffold did not mention checkpointing at all.

**Decision.** A checkpoint stores `(seed, step, counts, entity table, log offset)`. There is
no PRNG state to serialise — that is the point of ADR-0002. Restart must reproduce the
uninterrupted trajectory bit-for-bit, and a test must assert that by comparing log digests
across a checkpoint boundary.

**Consequences.** Checkpointing becomes nearly free and cannot silently desynchronise the
generator, which is the usual failure mode. The test is the deliverable, not the feature.

---

## ADR-0007 — Irreversibility is a declaration, not a default

**Status:** accepted.

**Context.** Invariant 4 (the kill test) requires monotone relaxation to equilibrium. Any
strictly irreversible reaction runs to completion instead, pinning entropy production at
infinity. Whole-cell models are full of steps declared irreversible for convenience, so
invariant 4 as originally written was unreachable for any realistic P1 model.

**Decision.** `Network::add_irreversible` requires a non-empty justification and records it.
The kill test **names** the irreversible reactions rather than failing opaquely. Proceeding
past one requires a numbered `RELAXATIONS.md` entry with a detecting test.

**Consequences.** The invariant survives contact with real biochemistry without being quietly
abandoned. The register makes the accumulated debt visible at every phase boundary, which is
the thing a silently-weakened invariant never does.
