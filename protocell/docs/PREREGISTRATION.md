# PREREGISTRATION.md — Acceptance intervals and costs, fixed before the runs

`PROMPT.md` §7 forbids tuning a parameter until a plot looks right. Deciding what counts as
agreement *after* seeing the numbers is the same act performed on the acceptance criterion
instead of the parameter, and it is harder to spot. So the intervals live here, and they
are set before the run that tests them.

Changing an entry in this file is a legitimate act — better information arrives — but it
must be done in a commit that changes **only** this file, with a reason, and before the run
it governs. A tolerance widened in the same commit that makes a test pass is a tuned
tolerance.

---

## 1. Acceptance intervals

### L0 — analytic (green)

| Benchmark | System | Analytic target | Criterion |
|---|---|---|---|
| L0a | `∅ → X → ∅` | Poisson(k_b/k_d) | Fisher-combined p > 0.001 over the 8 fixed seeds |
| L0b | `A ⇌ B` closed | Binomial(N, k_f/(k_f+k_r)) | as above |
| L0c | `2A ⇌ B` closed | exact chain stationary distribution | as above |

Each level is judged on the eight seeds in `validate::ladder::LADDER_SEEDS`, combined by
Fisher's method. **Not** on one seed: a correct benchmark fails a single-seed 1%-level test
1% of the time, and the reflex when it does is to re-roll — which is seed-shopping. The
seed set is in the source so a reviewer can see it was not chosen after the fact.

Each level also carries a **negative control**: a deliberately wrong hypothesis the same
test must reject. L0b's is a reverse rate 30% off, which must be rejected at combined
p < 1e-6. A goodness-of-fit test never shown to reject anything is decoration.

### L1 — spatial recovers well-stirred

| Observable | Interval |
|---|---|
| Species means, fast-diffusion limit | within 2% of the well-stirred CME, all species |
| Species variances | within 5% |
| Lattice-spacing sweep | bimolecular rates re-derived at each spacing; means invariant within 2% (invariant 10) |

### L2 — reproduce the 4DWCM

Tolerances are set from the spread the 4DWCM reports across replicates where available, and
from a 10% band where it is not. **≥3 replicates**; the criterion applies to the mean.

| Observable | 4DWCM | Accept |
|---|---|---|
| Doubling time | 105 min | 95–115 min |
| ori:ter | 1.28 | 1.20–1.36 |
| Ribosomes at division | 881 | 750–1010 |
| RNAP at division | 176 | 150–202 |
| Degradosomes at division | 192 | 163–221 |
| Ribosome active fraction | ~55% | 45–65% |
| RNAP active fraction | ~70% | 60–80% |
| Degradosome active fraction | ~10% | 5–15% |
| B / C / D periods | 5 / 46 / 54 min | each within ±20% |

### L3 — reproduce experiment

| Observable | Experiment | Accept |
|---|---|---|
| Doubling time | 105 min (Breuer 2019) | 95–115 min |
| ori:ter | 1.21 (sequencing) | 1.13–1.29 |
| Division symmetry | — | daughter volume ratio 0.9–1.1 |

### L4 — the biological hypothesis (both arms)

| Observable | Accept as supporting | Accept as refuting |
|---|---|---|
| Median protein scaling over one cycle | ≥ 1.85 | ≤ 1.70 |
| Genes > 3 kb, mean scaling | ≥ 1.75 (from 1.25–1.5) | ≤ 1.55 |

Between those bands is an inconclusive result and must be reported as one. **Both arms
required.** If the counted (TASEP-occupancy) arm closes the gap as well as the individuated
arm, the correct conclusion is *polysomes explain the deficit and individuation was not
required for it* — and that is a publishable, honest result that the original single-arm
design could not have distinguished from success.

### L5 — the engineering thesis

| Observable | Criterion |
|---|---|
| Per-mRNA lifetime vs. birth-position-to-membrane distance | Spearman ρ with 95% CI excluding 0, over ≥10⁴ mRNA lifetimes pooled across ≥5 replicates |
| Cost of obtaining it | Total log ≤ 1 GB per trajectory (budget: 8 MB predicted, 100× margin) |
| Overhead of individuation | Wall-clock ≤ 1.5× the identity-free control at equal biological time |

**Either sign of the correlation is a result.** A null with a tight CI is also a result. What
would falsify the engineering thesis is not the correlation's sign but the cost columns:
if the log exceeds 1 GB or the overhead exceeds 1.5×, individuation is not affordable at
whole-cell scale and the thesis is wrong.

### L6 — essentiality

Well-stirred surrogate. Accept if predicted essential/non-essential matches the genome-wide
assignment for **≥ 85%** of the 493 genes, with every disagreement listed by name in
`STATE.md`. Disagreements are re-run spatially only where the surrogate is suspected.

### L7 — closure

≥10 generations, no exogenous component except defined SP4 medium, no per-generation
retuning. Accept if doubling time drift over 10 generations is < 10% and no parameter was
changed between generations.

---

## 2. The two-arm protocol

`PROMPT.md` §1.3: no result may be attributed to individuation without a control.

1. Build one network. Mark the species under test `Tier::Counted` in arm A,
   `Tier::Individuated` in arm B. Nothing else differs.
2. Run both from the **same seed**. Assert the observable trajectories are bit-identical
   where both arms can compute them. This is possible only because the PRNG is
   counter-based; see `core::rng::Stream::ENTITY_PICK`.
3. Report three numbers, always together:
   - what arm B can answer that arm A cannot
   - what arm B cost in resident bytes, log bytes and wall clock
   - whether arm A, given a cheaper representation of the same biology (occupancy integers,
     TASEP), reaches the same scientific conclusion
4. If (3c) is yes, say so plainly. That is the outcome the single-arm design would have
   mislabelled as a win for individuation.

Enforced in miniature at P0: `cargo run -p protocell-cli -- arms` asserts bit-identity and
prints the cost table.

---

## 3. Compute costs, so the ladder can be planned rather than discovered

At the 4DWCM's ~250 GPU-hours per cell-cycle replicate:

| Level | Runs | Cost | Affordable? |
|---|---|---|---|
| L0 | CPU seconds | negligible | ✅ done |
| L1 | ~10 CPU-hours | negligible | ✅ |
| L2 | 3 replicates | ~750 GPU-h | Depends on BLOCK-03 |
| L3 | shares L2's runs | ~0 extra | ✅ if L2 is |
| L4 | 2 arms × 5 replicates | ~2,500 GPU-h | Depends on BLOCK-03 |
| L5 | analysis of L4's runs | ~0 extra | ✅ if L4 is |
| L6 (spatial) | 493 knockouts | **~123,000 GPU-h** | ❌ ~8× the entire 4DWCM campaign |
| L6 (surrogate) | 493 well-stirred | ~493 CPU-h | ✅ |
| L7 | 10 generations | ~2,500 GPU-h | Depends on BLOCK-03 |

**Total for L2 + L4 + L5 + L7: ~5,750 GPU-hours.** About a third of the 4DWCM's 15,000-hour
50-cell campaign. Whether that is available is BLOCK-03, and it is unresolved. The phase
plan past P3 is provisional until it is.

The original ladder listed L6 as "the sharpest falsification available" with no cost note
attached. It is the sharpest, and at full spatial fidelity it is also unaffordable by a
factor of about 20 against the whole project's plausible budget. Both facts belong next to
each other.
