# C0-CAN1-R2 — Target-specific derivation boundaries

> **Package:** C0-CAN1-R2, a review correction to [C0-CAN1](c0-can1-adapter-constraints.md) and
> [C0-CAN1-R1](c0-can1-r1-evidence-strength.md). **Date:** 2026-09-07. **Author role:** developer.
> Reviewer: Codex (architect).
> **Nothing staged, committed or pushed.** No third confidence class, no UI decision, no
> asset-profile expansion, no additional target populated, no private fixture acquired, no change to
> evidence qualification. The candidate channel, `target-adapter/2` and every supported constraint
> are preserved. **Population remains a C0 obligation.**

## The rule applied

> **Derived** means a conclusion supported by established premises **applicable to this target**.
> A cross-title analogy with no proven equivalence bridge is a **candidate**, even when the layouts
> or field names match.

Applying it needed one operational test, because every citation in these adapters is a document that
either does or does not say who it speaks for:

- a source that **states its own applicability and names this target** supplies an established
  premise here — the ratings-write-codec audit scopes its 6-bit/offset-36 row to
  `NHL 10 → Legacy / 11-21` by name; the anchors manifest states `PS3 = Xbox360 databases are
  byte-identical` as **verified for all of NHL 10-15 + Legacy**;
- a source **anchored on one title that claims no wider scope** supplies nothing here, however well
  the layouts match — `defs/enums/handedness.toml` states a 22-player audit of NHL Legacy
  `default.db`; `position.toml` states a five-player audit of the same file. Neither claims to speak
  for another title, and a shared field id plus a shared bit width is a resemblance, not a bridge.

**Nine more blocks moved to candidate. Nothing was measured afresh**, and every genuinely
target-supported derivation stayed operative.

---

## 1. Cross-title claims corrected

| Adapter | Block | Was | Now | Why |
|---|---|---|---|---|
| both | `player.shoots` `[mapping.enumeration]` | established, derived | **unknown + candidate** | `handedness.toml`'s stated anchor is NHL Legacy `default.db`; it claims no scope beyond that title |
| both | `player.shoots` `[mapping.transform]` (`enum-code`) | established, derived | **unknown + candidate** | an enum-code transform decodes through a vocabulary that is not established here |
| both | `[position_discriminator]` | established, derived | **unknown + candidate** | the 3-bit width and 0..7 range are measured here; the code **assignment** (0..3 skater, 4 goalie) is Legacy-anchored |
| nhl15 ×2, nhl10 ×1 | ratings `[mapping.applicability]` (`target-predicate` on `ubPc.vjla`) | established, derived | **unknown + candidate** | the predicate selects skaters by position **code**, and the codes are not established here |
| nhl15 | `[[companion_rule]]` | established, measured | **unknown + candidate** | see §2 — a dependency, not an evidence gap |
| nhl10 | `[[companion_rule]]` | established, derived | **unknown + candidate** | see §2 — an evidence gap, and a larger one than R1 recorded |

Two operative surfaces had to move with them. `player.shoots` on both adapters now reads
`unresolved` on both transform sides with `qualification = "blocked"`: leaving
`read_transform = "enum-code"` in place while the codes became a candidate would have changed
nothing, and decoding handedness from an unproven two-code vocabulary is a coin flip with no error.

**Independently supported physical facts were preserved separately, and are untouched:**
`ubPc.GRkp` is still measured at 1 bit, declared range 0..1, offset 981 (nhl15) / 1412 (nhl10), with
its `[mapping.status]` intact; `ubPc.vjla` is still recorded as a 3-bit field with range 0..7 inside
the discriminator's candidate. What was withdrawn is only what the codes **mean**.

## 2. The two companion rules — same state, different reasons

The architect's instruction was explicit: *shared key declarations alone do not establish companion
cardinality, mandatory presence or skater/goalie conditions; an audit excluding NHL 10 cannot
establish those semantics for NHL 10.* Both rules are candidates now, and each records **which**
problem it has, because they are not the same problem.

**NHL 15 — blocked on a dependency, not on evidence.** The roster-relationships audit verifies this
exact rule over `nhl11 → legacy`, a range that *includes* NHL 15, and the engine enforces it at save
time as the `companion_presence` predicate. The rule is derived. It is still not operative, because
every companion in it is conditioned on `position-skater` or `position-goalie` and nothing
establishes how this target tells those apart. A rule that cannot be evaluated cannot be acted on:
applying it would mean guessing which rows are goalies and then demanding or tearing down companion
records on the strength of the guess. Its candidate is `derived` and says it promotes the moment the
discriminator does — no further companion evidence needed.

**NHL 10 — the rule itself is unproven here.** R1 downgraded this to `derived` on the argument that
NHL 10's own snapshot shows `wBIz` is the declared key of `ubPc` and of each companion table. That
argument does not survive. A shared primary key establishes that rows **can be addressed by a common
id**. It does not establish the **cardinality** (one-to-one rather than zero-or-one or one-to-many),
it does not establish **mandatory presence** (that a usable skater must have both companions), and it
says nothing whatever about the **skater/goalie condition**. Those three things are exactly what the
block asserted, and a key declaration is evidence for none of them. With the audit's stated range
excluding NHL 10, the rule is `asserted` here. Its candidate says precisely what is measured on this
target (the key declarations, 13-bit against NHL 15's 14-bit) and what is not.

## 3. Validation was propagated, not weakened

The dependency is enforced by a rule that already existed. When `[position_discriminator]` became a
candidate, `position-discriminator-missing` immediately refused **NHL 15's** companion rule — the one
whose companion evidence is strongest. That refusal is correct, and the data was changed to match it.
No rule was relaxed to keep an example passing.

One rule was **added**, for item 3: **`derivation-unexplained`**. An established block at
`confidence = "derived"` must record a `derivation` — the premises, why each is applicable to *this*
target, and what the argument does not establish. A confidence label and a citation string are not an
argument, and the format no longer accepts them as one. This is a single optional key, not a
provenance-framework redesign.

## 4. Retained derivations, with their premises

Thirty-one derived blocks across the two adapters now carry a written argument. The distinct ones:

| Block | Premises | Applicability to this target | Not established |
|---|---|---|---|
| ratings `[mapping.transform]` (offset 36) ×3 | (1) measured here: field is 6-bit, range 0..63; (2) measured here: title is gen7 — NHL 07 is the only gen6 title and ships a different DB; (3) the ratings-write-codec audit's regime table | the audit **names** `NHL 10 → Legacy / 11-21 (PS3/X360)` in the 6-bit row, and the offset is resolved by a per-database probe (`sixbit_rating_offset`), not a per-title constant | the named case ran the gen7 regime on **nhl14/ps3**, not on either of these targets — which is why this is derived and not measured |
| `[[capacity]]` ×14 | (1) measured per title: `max_records` read from a real PS3 vanilla ROSTER save that parsed CRC-clean, with a GEN7 row for this title by name; (2) the equivalence bridge | `PS3 = Xbox360 databases are byte-identical` per game — same tables, record counts and record lengths — stated in the anchors manifest as **verified for all of NHL 10-15 + Legacy**; the platforms diverge only in the save container | no Xbox 360 ROSTER save was decoded; the capacity audit says so itself |
| `[mapping.status]` ×13 | (1) measured here: the snapshot's own `key` and `updatable` flags; (2) the scalar mutation audit's enumeration of structural field classes | that audit's matrix carries a row for **this** title and platform | no named case exercises these specific fields |
| nhl15 `player.team` `[mapping.storage]` | (1) measured here: offset 48, width 8, range -1..254; (2) the codec taxonomy's `range_low == -1` rule | the classification is mechanical over any `FieldMeta` and the matrix runs over all 27 anchors, this one included | what a decoded -1 **means** — `[mapping.null_sentinel]` stays unknown for exactly that reason. This block is a decode fact only |

Measured blocks carry no derivation, and are not asked to: they are not conclusions.

## 5. Tests

| Requirement | Test |
|---|---|
| cross-title-only claims become candidates | `a_cross_title_only_enum_is_a_candidate_not_a_derivation` — no established enumeration on either target; the Legacy proposal survives with its citation; the transform is unresolved and blocked; **and the measured 1-bit layout and status are asserted still present** |
| dependent constraints stay non-operative | `an_unproven_discriminator_does_not_drive_dependent_constraints` — discriminator unknown with its proposal intact; no companion rule established on either target; every established applicability is `all-records`, never a position predicate |
| the two reasons stay distinguishable | `the_two_companion_rules_record_different_reasons_for_being_candidates` — NHL 15's candidate is `derived` and says "blocked on a dependency"; NHL 10's is `asserted` and says the audit "excludes this title" |
| target-supported derivations remain usable | `a_target_supported_derivation_stays_usable` — offset 36 and `human_range` `[36, 99]` on both targets, 7 established capacities each, status blocks intact |
| premises are recorded, not just labelled | `every_derived_block_records_its_premises` — every derived block states PREMISES, CONCLUSION, NOT ESTABLISHED and its target applicability; capacity derivations must name their equivalence bridge |
| the rule bites | `a_derived_constraint_must_record_its_premises` — a derived block with a citation but no argument is refused; a measured block needs none |

Representation completeness is asserted **separately** from populated facts by the pre-existing
`representation_completeness_is_reported_apart_from_registry_population`, and is unchanged at
**20 of 20**. Removing analogies did not remove a representation.

## 6. Checks

| Check | Command | Result |
|---|---|---|
| Scoped crate tests | `cargo test -p tdb-canonical` | **Passed — 84/84** (was 78) |
| Strict clippy, scoped | `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | **Passed**, exit 0 |
| Repo-wide formatting | `cargo fmt --all -- --check` | **Passed**, exit 0, zero diffs (working tree carries the uncommitted C0-INT1 repairs; CLAUDE.md records this as failing at HEAD) |

Rules 45 → **46**. Tests 78 → **84**.

## 7. Counts

| Target | Mappings | Omissions | Established | Unknown | of which candidates |
|---|--:|--:|--:|--:|--:|
| `nhl15-xbox360` | 7 | 0 | **32** (R1: 38, C0-CAN1: 43) | **34** (R1: 28) | **11** (R1: 5) |
| `nhl10-xbox360` | 6 | 1 | **29** (R1: 34, C0-CAN1: 38) | **29** (R1: 24) | **9** (R1: 4) |
| the other **26 catalog units** | 0 | 0 | 0 | — | — |

Of the 61 established blocks, **31 are derived** (each now carrying its argument) and **30 are
measured**. Representation completeness: **20 / 20**, unchanged.

The trend across C0-CAN1 → R1 → R2 is the honest one: 81 → 72 → 61 established. Each round removed
claims the evidence did not carry, and none removed a claim it did.

## 8. Files changed

| Path | Change |
|---|---|
| `crates/tdb-canonical/src/model.rs` | `Constraint::derivation` |
| `crates/tdb-canonical/src/lib.rs` | `Rule::DerivationUnexplained` and its check |
| `crates/tdb-canonical/src/tests.rs` | 6 tests added, 1 updated |
| `crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml` | derivations added to its derived blocks |
| `defs/adapters/nhl15-xbox360.toml` | §1–§4 corrections |
| `defs/adapters/nhl10-xbox360.toml` | §1–§4 corrections |
| `docs/validation/c0-06-canonical-project-and-adapters.md` | amended for R2 |
| `docs/receipts/c0-can1-adapter-constraints.md`, `c0-can1-r1-evidence-strength.md` | corrections labelled in place |
| `docs/plans/project-completion-roadmap.md` | C0 item-7 status row |
| `docs/receipts/c0-can1-r2-target-specific-derivations.md` | **New.** This receipt |

Unchanged: `defs/canonical/project-schema.toml`, `crates/tdb-canonical/tests/fixtures/v1/**`,
`crates/tdb-canonical/src/v1.rs`, `defs/adapters/asset-profiles/**`, `Cargo.toml`, `Cargo.lock`.

## 9. Preservation

| Check | Result |
|---|---|
| HEAD | `a2bbdc4f…`, unchanged |
| Index | **empty** |
| Pre-existing dirty/untracked work | **preserved** — `git status --short` byte-identical before and after (242 entries) |
| `anchors/`, `docs/audits/**` | read only |
| Historical evidence | the two prior receipts are amended by label, never rewritten; the v1 transition fixtures still hash to their original bytes |
| Other crates | none modified |

## Appendix — package-specific SHA-256, after C0-CAN1-R2

```
1c4d6af54c5536d9f4ad241db3a04a6ad81811237a4105ebb3b9c1611072c4c4  crates/tdb-canonical/src/lib.rs
394494d64a796be9330b19b9cd28ae0af9303b37edffffb346f55ee62f0f48d3  crates/tdb-canonical/src/model.rs
53c29f33807d59da64de957f7a555bea3b28f1cd2a1157931c9ce577cb0cbaa0  crates/tdb-canonical/src/v1.rs        (unchanged since C0-CAN1)
a26949344413ed785819ede14c30bded45924c1f01f6062a25202554e05efeaf  crates/tdb-canonical/src/tests.rs
d8073312d444fdbee1f9bfa9315284209dc3360fd7e47e5a9b2aa1a1ca44607a  defs/adapters/nhl10-xbox360.toml
6bfabff9a0b63d1ae24c75cccf17dc03396501152fd28d4cd5a9532ab3337528  defs/adapters/nhl15-xbox360.toml
fced8e983df7879901ea61a25c22abd600fa8f088c88f8a3a9b6c6131a0a6d4c  crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml
b5395fe50441bdfb7e10370e1b17de217804a7db733030fe4545dea9a8ee05e7  crates/tdb-canonical/tests/fixtures/v1/nhl10-xbox360.toml   (unchanged)
c7d51275e4de71bc5d95a492c30b8b52f891a35632f7d9fe640b630fcf47c780  crates/tdb-canonical/tests/fixtures/v1/nhl15-xbox360.toml   (unchanged)
d4cb63ffaffe454ad50bbe6f6567ffd82630bb5d7ebf9716f5c8fc145d9e40cb  docs/validation/c0-06-canonical-project-and-adapters.md
18182b992d1ed5099cd9f5476bcf76a40db740f8d035fa29eceb9a11e77b435a  docs/receipts/c0-can1-adapter-constraints.md
eba4e905dc93cc2cbfd5e9a70e3c01de8bdf9c205071a2f3ccefa1751457c77b  docs/receipts/c0-can1-r1-evidence-strength.md
519433df8fb077eb2930453fa82a7b6eb37b100af62aec54dded6c68182fb4db  docs/receipts/c0-can1-r2-target-specific-derivations.md
6102da3e252e767ea0761f5f321e7efd6ae3d187a888ddf41ad3e1c2b30a15a4  docs/plans/project-completion-roadmap.md
702b8e4fc4c7dc9130dfde137266b6b808fe6018b9eeb4f296ec14f0cacd12ac  defs/canonical/project-schema.toml   (unchanged)
5272dcbddaaeb834e1defbb659f090becc6eca5167f63c512c9481b849459e82  Cargo.toml                           (unchanged)
4a334c6570f19cd2d6e5b76c9bba83bbce8e503196bdc5bd7d44e7fa57f4aa2b  Cargo.lock                           (unchanged)
```

The v1 transition fixtures still hash to the bytes of the pre-C0-CAN1 adapters, and `v1.rs` is
untouched: the version transition and the preserved historical documents are unaffected by this
correction.

## 10. Questions for the architect

1. **The dependency is now the critical path.** Both targets' companion rules, both position
   applicability sets and both handedness mappings are blocked on one unproven fact: the position
   code assignment. A single named case probing `ubPc.vjla` against known players on one anchor
   would promote roughly a dozen blocks across both adapters. That is a fixture measurement, which
   this package was told not to perform — should it be scheduled?
2. **Does `player.shoots` still earn its place as an example?** With the codes unproven it maps a
   field it cannot decode. It is honest and it demonstrates the blocked state well, but it is no
   longer a demonstration of enum representation working end to end. The synthetic fixture covers
   the mechanics either way.
3. Carried forward unchanged: asset profiles have no constraint envelope; 26 units have no adapter,
   and population remains a C0 obligation.

**Stop for Codex review.**

---

## Correction — appended 2026-09-08 by package C0-AS1

> **Labelled correction, appended not rewritten.** It corrects three claims made in **§10 question 1**
> of this receipt (and echoed in [c0-06 §9.3 row 7](../validation/c0-06-canonical-project-and-adapters.md)).
> **No adapter fact is changed, no constraint state moves, and no measurement was performed.** The
> corrected text is the *scope estimate* attached to a proposed future fixture measurement, not any
> block in `defs/adapters/**`.

§10 question 1 wrote that *"a single named case probing `ubPc.vjla` against known players on one
anchor would promote roughly a dozen blocks across both adapters."* Each of the three things that
sentence assumes is wrong, and each is wrong in the same direction — it over-states what one
measurement would settle.

| # | The claim | The correction |
|---|---|---|
| C1 | probing the position field would promote **handedness** among the ~dozen blocks | **Position evidence does not establish handedness.** `player.shoots` is `ubPc.GRkp`, a different field with a different vocabulary; its codes became candidates because `defs/enums/handedness.toml` is anchored on NHL Legacy `default.db` and claims no wider scope. Measuring `ubPc.vjla` says nothing about what `GRkp`'s two codes mean. The two blocks were moved to candidate for the *same reason*, not by a *shared premise*, and only §1's table ever said otherwise by proximity |
| C2 | NHL 10's companion rule would promote once the discriminator did | **NHL 10 companion semantics need separate evidence.** §2 of this receipt states it correctly and question 1 contradicts it: NHL 10's rule is `asserted`, and its problem is not the discriminator. The roster-relationships audit's stated range (`nhl11 → legacy`) excludes the title, so cardinality, mandatory presence and the skater/goalie condition are unproven on NHL 10 *independently*. Only **NHL 15's** rule is blocked on the discriminator alone, and only NHL 15's promotes with it |
| C3 | one anchor's probe would promote blocks **across both adapters** | **One anchor does not establish both targets without a proven equivalence.** The bridge this package relied on for capacity — the anchors manifest's `PS3 = Xbox360 databases are byte-identical`, verified for NHL 10–15 + Legacy — is a **per-game, cross-platform** statement. It carries a fact from `nhl10-ps3` to `nhl10-xbox360`; it says nothing between NHL 10 and NHL 15, which is precisely the axis this adapter pair exists to demonstrate diverges. A probe on one title's anchor establishes that title. Establishing the other needs its own probe, or an equivalence bridge that has not been shown |

**What survives.** A named case probing `ubPc.vjla` against known players remains worth scheduling,
and it is still the right first measurement. Its honest scope is narrower than question 1 stated: on
**one** target it would promote that target's `[position_discriminator]`, that target's ratings
`[mapping.applicability]` predicates, and — for NHL 15 only — that target's `[[companion_rule]]`.
Handedness on either target and the companion rule on NHL 10 remain blocked on evidence this
measurement does not produce.

**Consequence for the record.** [c0-06 §9.3 row 7](../validation/c0-06-canonical-project-and-adapters.md)
carries the same over-statement and is amended in place by C0-AS1 with a pointer here.

*— C0-AS1, [receipt](c0-as1-asset-profile-constraints.md)*
