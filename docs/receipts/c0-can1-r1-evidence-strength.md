# C0-CAN1-R1 — Correct evidence strength in the example adapters

> **Package:** C0-CAN1-R1, a review correction to [C0-CAN1](c0-can1-adapter-constraints.md).
> **Date:** 2026-09-07. **Author role:** developer. Reviewer: Codex (architect).
> **Nothing staged, committed or pushed.** No migration installed, nothing published, no asset-profile
> expansion, no product decision. `target-adapter/2`, its representations, its strict parsing and its
> v1 transition are preserved.

## ⚠ Corrected further by review (C0-CAN1-R2, 2026-09-07)

R1 kept several facts operative at `derived` on the strength of a citation from another title. The
architect's rule settles it: **derived means a conclusion supported by established premises
applicable to this target; a cross-title analogy with no proven equivalence bridge is a candidate,
even when the layouts or field names match.**
[C0-CAN1-R2](c0-can1-r2-target-specific-derivations.md) applies it. This receipt stays as the
historical record.

| Claim here | Status |
|---|---|
| The handedness enum retained at `derived` ("operator-confirmed on an NHL Legacy anchor; the 1-bit width is measured here") | **WITHDRAWN as operative.** `handedness.toml` claims no scope beyond NHL Legacy `default.db`. Now unknown + candidate on both targets, and `player.shoots` is `unresolved` / `blocked` because an enum-code transform cannot decode through an unproven vocabulary |
| The position discriminators retained at `derived` | **WITHDRAWN as operative.** Same reason. The measured 3-bit width and 0..7 range are preserved inside the candidate |
| The ratings `[mapping.applicability]` target-predicates retained at `derived` | **WITHDRAWN as operative.** They select skaters by position code, and the codes are not established here |
| §4 "Downgraded": NHL 10's companion rule moved to `derived`, justified by this target's own shared-key declarations | **INSUFFICIENT.** A shared primary key establishes that rows can be addressed by a common id; it establishes neither cardinality, nor mandatory presence, nor the skater/goalie condition — which is everything the block asserted. Now unknown + candidate at `asserted` |
| §4 "Retained": NHL 15's companion rule kept at `measured` because the audit's range includes NHL 15 | **STILL TRUE, STILL NOT OPERATIVE.** The rule is derived for NHL 15, but every companion in it is position-conditioned and the discriminator is now a candidate, so it cannot be evaluated. Recorded as a `derived` candidate that promotes with the discriminator |
| §10 question 2 — "should a derived fact whose source audit does not cover the target be operative, or is that a second candidate class?" | **ANSWERED: it is a candidate, and no third class was added.** Enforced by review, and by the new `derivation-unexplained` rule requiring every derived block to state premises, target applicability and limits |
| Population figures (38 / 34 established) | **SUPERSEDED**: 32 and 29 |

Rule and test counts (45, 78) are superseded by 46 and 84. The R1 corrections themselves — the
candidate channel, the sentinel withdrawals, the unresolved transform names, the discovery-note
omission — all stand.

## Result

Every over-claim the review named is corrected. **Nine constraint blocks moved from operative to
non-operative**, one omission moved from a claimed proof of absence to an unresolved identity, and
fifteen further blocks had their confidence corrected to what their citation actually supports. The
format grew one thing to make this expressible: a **candidate** channel that keeps a proposal and its
citation without letting anything act on it.

No supported fact was deleted. The measured and derived facts C0-CAN1 established — bit layouts,
capacities, the rating offset regime, the birth-date origins, string budgets, the companion mechanism
— all still validate, and the human range still derives.

---

## 1. NHL 10 overall rating — absence unproven

**Corrected.** The omission was `reason = "target-absent"`, justified by a metadata name search that
found no field containing `overall` across 86 tables / 1769 fields.

The review is right: that search establishes only that **the search found no named match**. It is not
a proof of semantic absence, and the rule it satisfied (`omission-without-proof`) was satisfied by
citing the search rather than a proof. The argument is especially weak in this schema family — this
very adapter pair turns on `p_*` versus `c_*` name drift between adjacent titles, and fields
elsewhere in the repository are identified by probing stored values precisely because their metadata
names are uninformative.

Now: `reason = "unknown-field-identity"`. The search result is retained in `detail` as an explicit
**DISCOVERY NOTE** — a record of where someone already looked, so the next person does not repeat it
— and the note states what would be needed to move it to `target-absent`: identify the target's
complete rating representation and show the value is not carried anywhere in it.

**P1 not decided.** Nothing here says whether the canonical model should carry a stored overall, a
computed one, or both.

## 2. Null-sentinel claims — range arguments are not measurements

**Audited both adapters for "no spare code" reasoning. Five claims found; all five corrected.**

A declared range says which values the field is *documented* to hold. It does not prove that every
code in that range carries an ordinary value, and it cannot prove that no code is reserved.

| Adapter | Field | Was | Now |
|---|---|---|---|
| nhl15 | `ubPc.GRkp` handedness | `none`, **measured** — "1 bit, two values, no spare code" | unknown + candidate |
| nhl15 | `dSvy.eWUF` overall | `none`, **measured** — "every code 0..63 is a legal rating" | unknown + candidate |
| nhl15 | `ubPc.dXSB` main team | `raw-all-ones` = -1 → `absent`, **measured** | unknown + candidate |
| nhl10 | `ubPc.GRkp` handedness | `none`, **measured** | unknown + candidate |
| nhl10 | `ubPc.dXSB` main team | `none`, **measured** — "range starts at 0" | unknown + candidate |

The `ubPc.dXSB` sentinel deserves its own note, because it was the one claim with a real citation
behind it and it is still wrong. The scalar-mutation audit does describe `neg1-sentinel` as "the
nullable / none encoding" — but that classification is keyed on the declared range alone, and no
evidence shows a stored -1 in *this* field meaning "no main team". Worse, the repository contains a
**competing** reading: `capabilities.rs` documents `0`, not -1, as the free-agent marker under the
origin-1 encoding. Two candidate sentinels cannot both be operative, so neither is; the candidate
records both readings and says which value it holds.

**What was kept.** `[mapping.storage].signedness = "neg1-sentinel"` on nhl15 stays established, at
`derived` rather than `measured`. That block asserts a **decode** fact — the all-ones bit pattern
decodes to the value -1, by the audit's own `range_low == -1` rule applied to a measured range. What
-1 *means* is the separate question, and it is now the unknown one. The adapter says so in words.

## 3. Candidate versus operative — a format change

**`confidence = "asserted"` can no longer make a constraint operative.** New rule
`asserted-as-operative`: an `established` block must be `measured` or `derived`
([`OPERATIVE_CONFIDENCE`]). An asserted claim — a documented runtime expectation, a name-based
inference, a fact confirmed on a neighbouring title — belongs in the new candidate channel:

```toml
[mapping.transform]
state = "unknown"
note = "The main-team index origin is not established for this target."
[mapping.transform.candidate]
confidence = "asserted"
provenance = ["crates/studio-domain/src/capabilities.rs#main_team_offset"]
note = "PROPOSED, NOT OPERATIVE. ... To promote this, measure the origin on this target's anchor and name the case."
[mapping.transform.candidate.value]
kind = "index-origin"
index_origin = 1
```

**Nothing reads a candidate.** `Constraint::established()` — the single accessor every derivation,
default and export decision goes through — cannot see one; the separate `candidate_value()` exists
for reporting to a human. So a proposed origin can never reach a human range, a Clear-to-Defaults
value or a Build mapping. Supporting rules: `candidate-on-decided-constraint` (a proposal for
something already decided is a contradiction) and `candidate-incomplete` (a proposal with no value,
confidence or citation is not a proposal).

**The transform names had the same problem.** `read_transform = "index-origin-1"` is operative text,
so leaving it in place while the origin moved to a candidate would have changed nothing. Both
adapters' `player.team` mappings now read `unresolved` on both sides, `qualification = "blocked"`
with the blocker recorded in the note. New rule `unresolved-transform-conflict` keeps that honest:
unresolved must be unresolved in both directions, and cannot coexist with a verified inverse, an
`automated-tested` qualification, a `default`, or an established `[mapping.transform]`.

`"unresolved"` is deliberately distinct from `"unknown"` on the write side, which still means *this
read has no declared inverse* and is still refused.

**Inferred foreign keys, same rule.** The audit declares the bridge hop `eGlu.dXSB → kOtt.vfEq`.
Extending it to the **bio** table's `dXSB` is a name-based inference about a different table, and a
Build would follow it. Both adapters' `[mapping.foreign_key]` blocks are now unknown + candidate,
each naming what would promote them (value containment from `ubPc.dXSB` into `kOtt.vfEq` on the
anchor, as the staging work did for the `eGlu` hop).

**No fresh measurement was required anywhere.** Every fact already proven by existing evidence was
left operative.

## 4. Full citation audit — retained / downgraded

Every populated block in both adapters, checked against whether its citation supports the **exact
asserted fact** rather than the field's existence or an adjacent target.

### Downgraded

| Adapter | Block | Was | Now | Why the citation did not support it |
|---|---|---|---|---|
| nhl15 | `player.shoots` null_sentinel | established, measured | **unknown + candidate** | range inference |
| nhl15 | `player.rating_overall` null_sentinel | established, measured | **unknown + candidate** | range inference |
| nhl15 | `player.team` null_sentinel | established, measured | **unknown + candidate** | codec class keyed on range; contested by `capabilities.rs` |
| nhl15 | `player.team` transform | established, asserted | **unknown + candidate** | runtime-probed expectation, not a measurement of this target |
| nhl15 | `player.team` foreign_key | established, asserted | **unknown + candidate** | the audit's hop is `eGlu.dXSB`, a different table |
| nhl10 | `player.shoots` null_sentinel | established, measured | **unknown + candidate** | range inference |
| nhl10 | `player.team` null_sentinel | established, measured | **unknown + candidate** | range inference, and depends on the unresolved origin |
| nhl10 | `player.team` transform | established, asserted | **unknown + candidate** | as nhl15 |
| nhl10 | `player.team` foreign_key | established, asserted | **unknown + candidate** | as nhl15 |
| nhl15 | `player.team` storage | measured | established, **derived** | offsets/range measured; the `neg1-sentinel` class is the audit's rule applied to them |
| both | `[mapping.status]` ×13 | measured | established, **derived** | `key`/`updatable` are measured; `structural = false` follows from the audit's enumeration of structural classes, not from the snapshot |
| nhl10 | `[[companion_rule]]` | measured | established, **derived** | **the roster-relationships audit states its own range: nhl11 → legacy. NHL 10 is outside it.** The mechanism is still supported by this target's own snapshot (`wBIz` is the declared key of `ubPc` and of each companion), so the rule is derived, not measured |
| nhl10 | `player.rating_overall` omission | `target-absent` + proof | **`unknown-field-identity` + discovery note** | §1 |

### Retained

| Block | Confidence | The citation asserts exactly this |
|---|---|---|
| all `[mapping.storage]` except nhl15 `player.team` | measured | offset, width, kind, range and endianness are the snapshot's own per-field row |
| birth-date `[[mapping.component]]` storage ×6 | measured | as above, per component |
| `player.birth_date` transform, both targets | measured | `birthdate_encoding.rs` is a named case over the committed anchors, and the two targets disagree |
| ratings `[mapping.transform]` `offset+36` ×3 | measured | the ratings-write-codec audit's regime table names gen7 6-bit → 36 and lists NHL 10 explicitly |
| `[mapping.string_storage]` ×4 | measured | 200/320 bits = 25/40 bytes from the snapshot; NUL-fill, byte budget and over-length rejection from `tdb-core` and the audit's string policy, whose matrix row covers both anchors |
| names `[mapping.transform]` inline-string ×4 | measured | `kind_code = 0` in the snapshot (citation tightened to say so) |
| `[mapping.enumeration]` handedness ×2 | derived | operator-confirmed on an NHL Legacy anchor; the 1-bit width is measured here, the code mapping is not |
| ratings `[mapping.applicability]` ×3 | derived | `dSvy` is the skater companion; the codes come from the Legacy-anchored position enum |
| `[mapping.applicability]` all-records ×9 | measured | the table is a fixed-width record block, so the field is on every record (citation tightened to the record layout) |
| `[[capacity]]` ×14 | derived | the roster-capacity audit's gen7 table, carried to Xbox 360 by its own stated byte-identical-`nhlng.db` argument |
| `kOtt` capacity ×2 | unknown | correctly left unknown rather than read off the packed main DB |
| nhl15 `[[companion_rule]]` | measured | the audit's range (nhl11 → legacy) **includes** NHL 15 |
| `[position_discriminator]` ×2 | derived | 3-bit width measured here; code assignment Legacy-anchored |

**No additional targets were populated.**

## 5. Regression tests

| Requirement | Test |
|---|---|
| unresolved identity does not become target-absent | `an_unfound_field_is_an_unresolved_identity_not_a_proven_absence` — asserts the reason, that the search survives as a discovery note, and that **no** committed adapter claims `target-absent` |
| an unsupported no-sentinel claim is not treated as measured | `an_unsupported_no_sentinel_claim_is_not_operative` — no mapping in either adapter has an established sentinel; all five candidates are `asserted`, cited, and say "NOT OPERATIVE" |
| candidate origins do not drive derivation | `a_candidate_index_origin_does_not_drive_derivation` — `player.team` has established storage **and** a candidate origin, and `human_range` is still `None`; same for a synthetic candidate storage block, where `scalar_editable` is `None` too |
| supported existing constraints still validate | `supported_existing_constraints_still_validate` — human range still `[36, 99]`, the offset regime and verified inverse intact, 7 capacities, the companion rule and the discriminator still established, birth-date still `automated-tested` on both targets |
| migration does not promote candidates | `migration_does_not_promote_candidate_claims` — no upgraded mapping carries any candidate, and v1 transform names survive verbatim (the transition never invents `unresolved` either) |

Plus negative tests for each new rule: `an_asserted_constraint_cannot_be_operative` (which also
asserts the same claim **is** accepted in the candidate channel),
`a_candidate_on_a_decided_constraint_is_refused` (established and not-applicable),
`a_candidate_without_a_value_confidence_or_citation_is_refused`, and
`an_unresolved_transform_cannot_be_qualified_verified_or_defaulted` (one-sided, over-claiming, and
contradicting an established transform block).

## 6. Checks

| Check | Command | Result |
|---|---|---|
| Scoped crate tests | `cargo test -p tdb-canonical` | **Passed — 78/78** (was 69) |
| Strict clippy, scoped | `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | **Passed**, exit 0 |
| Repo-wide formatting | `cargo fmt --all -- --check` | **Passed**, exit 0, zero diffs. Note: CLAUDE.md records this as failing **at HEAD** (C0.2 defect B1–B3); it passes here because the working tree carries the uncommitted C0-INT1 repairs. This package neither fixed nor relied on that — it simply reports what the command returned |

Rules 41 → **45**. Tests 69 → **78**.

## 7. Population, restated

| Target | Mappings | Omissions | Established | Unknown | of which candidates |
|---|--:|--:|--:|--:|--:|
| `nhl15-xbox360` | 7 | 0 | **38** (was 43) | **28** (was 23) | 5 |
| `nhl10-xbox360` | 6 | 1 | **34** (was 38) | **24** (was 20) | 4 |
| the other **26 catalog units** | 0 | 0 | 0 | — | — |

`registry_population` now reports `candidate_constraints` as a **separate** field, and a test asserts
candidates never exceed unknown blocks. A proposal is not coverage, and counting it beside
established blocks would restate the exact over-claim this package removes.

Representation completeness is unchanged at **20 of 20** — no representation was withdrawn.

## 8. Files changed

| Path | Change |
|---|---|
| `crates/tdb-canonical/src/model.rs` | `Candidate<T>`; `Constraint::candidate` and `candidate_value()` |
| `crates/tdb-canonical/src/lib.rs` | `OPERATIVE_CONFIDENCE`, `CANDIDATE_CONFIDENCE`, `UNRESOLVED_TRANSFORM`; 4 new rules; `check_candidate`, `validate_unresolved_transform`, `transform_unresolved`; `Population::candidate_constraints` |
| `crates/tdb-canonical/src/tests.rs` | 2 tests rewritten, 9 added |
| `crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml` | established blocks re-labelled `derived`; a candidate mapping added to exercise the new channel |
| `defs/adapters/nhl15-xbox360.toml` | §1–§4 corrections |
| `defs/adapters/nhl10-xbox360.toml` | §1–§4 corrections, header rewritten |
| `docs/validation/c0-06-canonical-project-and-adapters.md` | amended for R1 |
| `docs/receipts/c0-can1-adapter-constraints.md` | corrections labelled in place |
| `docs/plans/project-completion-roadmap.md` | C0 item-7 status row |
| `docs/receipts/c0-can1-r1-evidence-strength.md` | **New.** This receipt |

Unchanged: `defs/canonical/project-schema.toml`, `crates/tdb-canonical/tests/fixtures/v1/**` (the
preserved v1 documents), `defs/adapters/asset-profiles/**`, `Cargo.toml`, `Cargo.lock`.

## 9. Preservation

| Check | Result |
|---|---|
| HEAD | `a2bbdc4f…`, unchanged |
| Index | **empty** |
| Pre-existing dirty/untracked work | **preserved** — `git status --short` byte-identical before and after (242 entries) |
| `anchors/`, `docs/audits/**` | read only; not modified |
| Prior evidence records | untouched; the two prior receipts are amended by label, never rewritten |
| Other crates | none modified |

## Appendix — package-specific SHA-256, after C0-CAN1-R1

```
ad561614625776cd7945df4d0becf62815c5f4fd681841095d65a47d95c1ac4e  crates/tdb-canonical/src/lib.rs
893212cfb776f3f5b92e1f62c33f49e6126037a6f17095d6840f580ca84ae675  crates/tdb-canonical/src/model.rs
53c29f33807d59da64de957f7a555bea3b28f1cd2a1157931c9ce577cb0cbaa0  crates/tdb-canonical/src/v1.rs        (unchanged by R1)
e2aa6241221ef39e9717d92a689f243d9957aea23da8a760e0a50ec1acb2be7d  crates/tdb-canonical/src/tests.rs
2f2f90d4d4976de4708bf9caa24f3a27448e59db032df03b580f23e8cabc19b0  defs/adapters/nhl10-xbox360.toml
040b31d336e968152a8a49060aae5ff9b17989111aa7ecb427855e3002426966  defs/adapters/nhl15-xbox360.toml
13182c055e5cf72de9dda447ab7792c79f3c86ba7b184d3fa00f9d984dfcbea9  crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml
b5395fe50441bdfb7e10370e1b17de217804a7db733030fe4545dea9a8ee05e7  crates/tdb-canonical/tests/fixtures/v1/nhl10-xbox360.toml   (unchanged)
c7d51275e4de71bc5d95a492c30b8b52f891a35632f7d9fe640b630fcf47c780  crates/tdb-canonical/tests/fixtures/v1/nhl15-xbox360.toml   (unchanged)
dc19ed74de24c837ce302cad6a08211c8787364ba840b84d3a71ef871a64b94e  docs/validation/c0-06-canonical-project-and-adapters.md
8eeba6770cb3cee4657eb71cdef4d0c1f1e11e37b86a875c414f9da00d7099a5  docs/receipts/c0-can1-adapter-constraints.md
7f7a58d6d6d3c985ffd8b11d6fc32e99f6a181d1d6c3ee22f84c6780ed552c2c  docs/receipts/c0-can1-r1-evidence-strength.md
e89831821e091b5ff5b97ba935b6de5ea929729faff7843bca828e87d9d17163  docs/plans/project-completion-roadmap.md
702b8e4fc4c7dc9130dfde137266b6b808fe6018b9eeb4f296ec14f0cacd12ac  defs/canonical/project-schema.toml   (unchanged)
5272dcbddaaeb834e1defbb659f090becc6eca5167f63c512c9481b849459e82  Cargo.toml                           (unchanged)
4a334c6570f19cd2d6e5b76c9bba83bbce8e503196bdc5bd7d44e7fa57f4aa2b  Cargo.lock                           (unchanged)
```

The two v1 transition fixtures still hash byte-identically to the pre-C0-CAN1 adapters, so the
preserved historical documents are untouched by this correction as well.

## 10. Questions for the architect

1. **Does the candidate channel belong in the reconciliation UI?** A candidate is exactly the
   material a user would need to resolve a blocked mapping ("we think the origin is 1, here is why,
   here is what would prove it"). Surfacing it is a product decision and was not made here.
2. **`derived` still carries weight.** Several retained facts are `derived` from an audit whose stated
   anchor range excludes the target (the NHL 10 companion rule) or from an enum confirmed on another
   title (handedness, position). The rule set permits `derived` as operative. Should a derived fact
   whose source audit does not cover the target be operative, or is that a second candidate class?
   The NHL 10 companion rule is the live example.
3. Carried forward from C0-CAN1, unchanged: asset profiles have no constraint envelope; 26 units have
   no adapter; is population a C0 obligation or C1?

**Stop for Codex review.**
