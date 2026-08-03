# RELAXATIONS.md — Register of relaxed invariants

`PROMPT.md` §4 says invariant failure is a crash, not a log line. That is right, and a veto
system with no escape hatch is still a halt — `PROMPT.md` §3.1 works through the deadlock
that arrives in P1.

This file is the escape hatch, and it is deliberately uncomfortable to use. Every entry
carries five fields, and the fourth is the load-bearing one: **a relaxation whose detecting
test has never been run is not a relaxation, it is a hole.**

Reviewed at every phase boundary. Entries are never deleted, only retired with a date and
the evidence that retired them.

---

## R-0001 — Irreversible reactions in the L0a benchmark

| Field | |
|---|---|
| **Invariant relaxed** | 4 (kill test: monotone relaxation to equilibrium) |
| **Scope** | The two reactions of the L0a immigration–death benchmark only. No biological network. |
| **Justification** | `∅ → X → ∅` *is* the definition of the immigration–death process. Its stationary distribution — Poisson(k_b/k_d) — is the analytic target being tested. Making it reversible would test a different process. |
| **Detecting test** | `ladder::kill_test` reports `irreversible` non-empty and refuses to pass. `ladder::tests::kill_test_names_irreversible_reactions_instead_of_passing_quietly` asserts the failure path works. The relaxation cannot leak into a biological network without the kill test naming it. |
| **Retirement** | Never — L0a is a solver benchmark, not a model of anything. It is recorded so the pattern is visible when P1 tries to use it as precedent. |

**Status:** active, benign, tested.

---

## R-0002 — Abstract species in solver fixtures

| Field | |
|---|---|
| **Invariant relaxed** | 1 (atom and charge balance), vacuously |
| **Scope** | `Formula::abstract_species()`, used in fixtures for the SSA absorbing-state test and the kill test's failure path. |
| **Justification** | Testing that the SSA halts on an absorbing state requires a sink reaction, and giving it real atoms would require inventing chemistry to satisfy a test about control flow. |
| **Detecting test** | `Network::is_abstract()` reports when every species in a network is abstract, so a balance check that proves nothing says so. `network::tests::abstract_networks_admit_they_are_abstract`. |
| **Retirement** | At P1. No network containing an abstract species may be used for any scientific claim; the L0b/L0c benchmarks already use real formulas (G6P/F6P conformers) so that invariant 1 is exercised rather than bypassed. |

**Status:** active, benign, tested.

---

## Anticipated — not yet needed, recorded so the shape is agreed in advance

These are the relaxations P1 is expected to require. Writing them down before the pressure
arrives is cheaper than negotiating them under it.

### A-0001 — Reactions with no published ΔG°′

The deadlock in `PROMPT.md` §3.1. Expected shape: adopt the 4DWCM's rate pair as
`Inherited`, derive nothing, and mark the reaction as thermodynamically unconstrained.
Detecting test: a Wegscheider check over every cycle containing the reaction, which will
fail loudly if the unconstrained pair is inconsistent with its neighbours. That converts an
unknown into a *bounded* unknown, which is the most that can honestly be claimed.

### A-0002 — Michaelis–Menten steps declared irreversible in the source model

The 2022 and 4DWCM metabolic models contain them. Expected shape: scope the kill test to
the reversible subnetwork, and assert separately that no irreversible step forms a cycle
capable of net ATP regeneration — which is the specific perpetual-motion failure invariant 4
exists to catch. `thermo::check_wegscheider` is the machinery.
