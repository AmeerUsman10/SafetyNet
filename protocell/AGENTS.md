# AGENTS.md — Rules of engagement

**Read order for any session:** `PROMPT.md` → `AGENTS.md` → `INDEX.md` → `docs/STATE.md`

---

## The two meanings of "agent" — never conflate them

| | **Simulation agents** | **Engineering agents** |
|---|---|---|
| What | Molecules, complexes, filaments | You, and any Claude Code subagents |
| Governed by | Physics. Stochastic rules, thermodynamic propensities | This file |
| Decide anything? | **Never.** A molecule that decides is a bug | Yes, within veto constraints |
| LLM involved? | **Never. Not once. Not as a fallback** | Yes |

If you find yourself writing a molecule that "wants," "tries," or "chooses" — stop and
re-read `PROMPT.md` §2.

---

## Role protocol

The thirteen roles in `PROMPT.md` §3 are held by you simultaneously. Before any design
decision:

1. Name which roles have standing.
2. State their position explicitly, in their own frame.
3. If any role vetoes — **the design does not get built.** Redesign. Do not proceed and
   note it as a caveat.
4. Log unresolved role conflicts in `STATE.md` under Open Questions. Do not resolve them by
   picking the convenient side.
5. **If the roles deadlock**, use the constraint-relaxation procedure below. A veto system
   with no escape hatch halts the project, and this one demonstrably deadlocks in P1
   (`PROMPT.md` §3.1).

The V&V engineer (role 12) has standing on *everything* and one permanent question:
**what test asserts this?**

## Constraint relaxation

When roles deadlock and no design satisfies every veto, you may proceed only by adding a
numbered entry to `docs/RELAXATIONS.md` containing all five of:

| Field | Why it is required |
|---|---|
| **Invariant relaxed** | Names what is no longer guaranteed |
| **Scope** | Which reactions, phases, or modules. A relaxation without a boundary is a repeal |
| **Justification** | The physical or practical argument |
| **Detecting test** | **The load-bearing field.** What would show that this mattered? An unmonitored relaxation is a silently broken invariant |
| **Retirement condition** | What evidence or work would remove it |

Relaxations are reviewed at every phase boundary. A relaxation whose detecting test has
never been run is not a relaxation, it is a hole.

## Parallelism

Where subagents are available (Claude Code), fan out on independent work only —
literature/parameter mining, per-module implementation behind a fixed interface, test
authoring. **Never fan out on a design decision.** Merge through `STATE.md`, one writer at
a time.

---

## Hard rules

1. No parameter without a provenance record `{value, units, method, uncertainty}` where
   `method` carries non-empty evidence. `Estimated`, `Fitted` and `Inherited` are legal —
   the 4DWCM's own adjusted values are `Inherited`, and refusing them would make L2
   unreachable. What is illegal is an *unjustified* value, or a fitted value dressed as a
   measured one. Missing record ⇒ the model refuses to load.
2. Never set forward and reverse rates independently. Set one plus ΔG°′; derive the other.
   Carry the `(c°)^Δn` standard-state factor even when it is numerically 1.
3. Never enumerate the species network. Pattern-match at runtime.
4. No inertia. Overdamped Langevin only.
5. Same seed ⇒ bit-identical trajectory. If you break determinism, you have broken the
   project. This constrains parallelism: reductions must be fixed-order.
6. `EntityId` unique for life, never reused. Every entity has a birth event and a death
   event. Slots may be recycled; ids may not.
7. Never begin `P(n+1)` before `Pn` Definition of Done is green.
8. Update `docs/STATE.md` before stopping. Always. Even mid-task. Especially mid-task.
9. Report negative results plainly. A failed thesis reported honestly is worth more than a
   tuned plot.
10. Reproduce before you extend. Beating 4DWCM means nothing until you have matched it (L2).
11. **Never attribute a result to individuation without running the counted arm.**
    `PROMPT.md` §1.3.
12. **Never re-roll a seed on a failing statistical test.** Ladder levels are judged on the
    fixed seed set in `validate::ladder::LADDER_SEEDS`, combined by Fisher's method. If a
    level fails, the model is wrong or the test is wrong; the seed is neither.

## Definition of Done (every phase)

- [ ] Invariants in `PROMPT.md` §4 assert and pass
- [ ] Validation level for this phase is green within its **pre-registered** interval
- [ ] Every new parameter carries provenance
- [ ] Tests committed alongside code, not after
- [ ] Every new test has a **negative control** — something it is shown to reject
- [ ] Anything tuned to achieve agreement is named in `STATE.md`
      (`ParamTable::fitted_parameters()` generates this list; do not maintain it by hand)
- [ ] Any relaxation added has its detecting test run
- [ ] `STATE.md` updated with next action

## How to resume

> Read `PROMPT.md`, then `AGENTS.md`, then `INDEX.md`, then `docs/STATE.md`. Check the
> working tree for uncommitted files from the previous session. Run
> `cargo test --workspace --release` and `cargo run -p protocell-cli -- dod` to confirm the
> last phase is still green before adding to it. Continue the in-progress step to its
> Definition of Done, then update `docs/STATE.md` before stopping.
