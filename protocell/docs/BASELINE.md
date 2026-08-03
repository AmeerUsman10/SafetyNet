# BASELINE.md — What already exists, and the gap

## 1. 4DWCM (Thornburg/Maytin/Kwon/…/Luthey-Schulten, *Cell* 2026)

**Architecture.** RDME (Lattice Microbes 2.5) as parent method, 10 nm cubic lattice,
**50 µs** timesteps. Hybrid interrupt every **12.5 ms** biological time, handing off to:
- **Global CME** (Gillespie direct) — transcription, tRNA charging
- **ODE** (LSODA/BDF, odeCELL) — glycolysis, nucleotide synthesis, lipid synthesis, transporters
- **Brownian dynamics** (LAMMPS, separate GPU) — chromosomes, replication, SMC looping, topoisomerase

**Geometry.** Cytoplasm sphere r = 20 lattice cubes (200 nm), grown to 250 nm, then
constant volume as surface area grows through division. Site types by ascending priority:
extracellular, cytoplasm, outer cytoplasm, ribosomes, DNA, ribo-centers, membrane.

> **Correction.** The original scaffold recorded the 200 → 250 nm growth as "~98% volume
> increase". For a sphere it is (1.25)³ = **+95.3%**. Either the scaffold rounded loosely
> or the modelled geometry is not a pure sphere over that interval. **Verify against the
> paper before this number is used for anything.** Flagged because a project that hard-fails
> on units should not carry an unchecked 3% discrepancy in its own reference file.

**Chromosome.** 543 kbp as 54,338 beads at 10 bp/bead, 3.4 nm diameter, 45 nm persistence
length, FENE stretching. SMC anchors re-placed every 4 s, hinges translocate every 0.4 s in
~20-bead steps. Topoisomerase = periodic soft-potential window allowing strand crossing.

> **Note on the bead diameter.** 3.4 nm is the contour length of 10 bp of B-DNA
> (0.34 nm/bp), not the ~2 nm width of the double helix. Excluded volume (invariant 6) is
> therefore enforced on a coarse-graining radius, not a physical one. Defensible, standard,
> and worth stating wherever DNA/ribosome exclusion distances matter.

**Initial conditions.** 500 ribosomes (uniform random, no two centres co-sited); RNAP and
degradosomes start unassembled at 0 and assemble in <1 s; 200 tRNA per isoform across 29
isoforms; mRNA sampled Poisson at 2× the 2022 model's means.

**Results.** Doubling 105 min (exp: 105). ori:ter 1.28 (seq: 1.21). B ≈ 5 min, C ≈ 46 min,
D ≈ 54 min. At division: ~881 ribosomes, ~176 RNAP, ~192 degradosomes. Active fractions
~55% / ~70% / ~10%. Partitioning approaches binomial, unbiased.

**Cost.** ~250 GPU-hours per replicate; 4–6 days on two A100s; ~15,000 A100-hours for 50 cells.

---

## 2. The gap — stated in their own Limitations section

1. **Every mRNA is indistinguishable from other mRNA of the same type.** Individual fate
   cannot be tracked without recording every RDME frame — estimated **>80 TB per trajectory.**
2. **No polysomes.** Each ribosome independent in RDME; one ribosome per mRNA at a time.
   Measured polysome fractions: up to 70% (*E. coli*), 20–40% (Syn3A).
3. **No polycistronic transcription.** Every gene transcribed independently.
4. **Protein doubling falls short.** Median scaling < 2.0; genes > 3 kb reach only 1.25–1.5×.
   They attribute this directly to the absence of polysomes.
5. **Gene–membrane proximity vs. mRNA half-life: no significant correlation found** —
   because bulk properties were all that was measurable.
6. **No FtsZ polymerization kinetics.** Division morphology imposed geometrically as two
   overlapping spheres. Helfrich/FreeDTS alone gave shapes too elongated.
7. **Chromosome partitioning requires a fictitious ~12 pN repulsive force.** Their own
   preliminary work suggests a better SMC model could replace it, but it is currently too
   expensive.
8. **No assembly reactions for all complexes.**

**Which of these are actually downstream of item 1** is analysed in `PROMPT.md` §1.2. The
short version: items 5, 6 and 7 are; item 3 is not; item 2 only partly, and item 4 follows
item 2 rather than item 1. The original scaffold claimed all of 2–5 were downstream, which
would have led to claiming credit for results a cheaper representation could produce.

---

## 3. Prior art the scaffold omitted

The original scaffold framed identity as unexplored territory. It is not. This section
exists so the contribution can be stated in a form that survives a referee.

### 3.1 Particle-based reaction–diffusion — identity is already solved, at a price

**Smoldyn, MCell, ReaDDy, eGFRD, Spatiocyte** all track individual particles with position
and identity *by construction*. Each particle is an object; following one is trivial;
per-particle histories are native.

They are not used at whole-cell scale because they cost more per unit biological time than
RDME. That is the trade-off the 4DWCM made deliberately: RDME for speed, accepting the loss
of identity as the price.

**So the honest framing is not "identity is unexplored". It is:**

> Identity-preserving methods exist and do not reach whole-cell scale. RDME reaches
> whole-cell scale and discards identity. The contribution is making identity affordable
> *at* that scale — an event-sourced overlay on a lattice method, rather than a switch to a
> particle method.

That is a narrower claim than the scaffold made, and a defensible one. It also means these
codebases are prior art with directly reusable machinery, not a blank page.

### 3.2 TASEP — polysomes without individuation

Ribosome traffic on a transcript is standardly modelled as a **totally asymmetric simple
exclusion process**. A counted mRNA species carrying a ribosome-occupancy integer
(`mRNA_k`, k = 0…L/80) gives polysomes with a bounded state space and no combinatorial
explosion.

This is why L4 needs two arms. Adopt TASEP at P5 for the counted arm; the individuated arm
gets per-ribosome positions on the transcript, and the comparison is the experiment.

### 3.3 Spatial rule-based — the real research risk

`INDEX.md` says of Kappa/BNGL: adopt, don't reinvent. Correct, with a limit. NFsim's
network-free matching is **well-stirred**. The spatial rule-based field is much thinner —
**MCell-R, SpringSaLaD, Simmune** — none of it GPU, none at whole-cell scale.

Network-free pattern matching against a 10 nm RDME lattice at 10⁵–10⁶ entities on a GPU is,
as far as the literature goes, unsolved. That is the genuine research risk in this project.
It was not in the original risk register. It is now.

---

## 4. Sensitive parameters — handle with a documented sweep or not at all

- **RNAP–promoter binding rate.** Global, and the whole cell state is sensitive to it.
- **mRNA:ribosome vs. mRNA:degradosome binding ratio.** They shifted ribosome binding +30%
  and degradosome binding −70% to approach protein doubling. Both changes < 1 order of
  magnitude, but the model is knife-edge sensitive here. These are `Inherited` parameters
  with an adjustment note, not `Measured` ones — the provenance schema must be able to say
  so, which is why "cite everything or refuse to load" had to be restated.
- **DnaA on/off rates.** 100 mM⁻¹s⁻¹ / 0.55 s⁻¹ gave *no replication initiation within
  60 min*; 140 / 0.42 initiated in nearly every cell within 15 min. Both pairs from the same
  single-molecule FRET study. A <50% change flips the phenotype.

If you tune any of these, say so in `STATE.md`. Agreement obtained by tuning a knife-edge is
not agreement.
