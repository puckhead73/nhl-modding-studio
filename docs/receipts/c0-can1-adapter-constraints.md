# C0-CAN1 — Complete required adapter constraint representation

> **Package:** C0-CAN1. **Date:** 2026-09-07. **Author role:** developer. Reviewer: Codex (architect).
> **Nothing staged, committed or pushed.** No migration installed, nothing published, no evidence
> qualification changed. No support status promoted — `game-verified` remains structurally
> unrepresentable in every file this package touches.

## ⚠ Corrected by review (C0-CAN1-R1, 2026-09-07)

Three classes of claim in this receipt, and in the adapters it describes, were **over-stated**. They
are corrected by [C0-CAN1-R1](c0-can1-r1-evidence-strength.md), and further by
[C0-CAN1-R2](c0-can1-r2-target-specific-derivations.md). Read that receipt alongside this one:
this text is kept as the historical record, not as the current state.

| Claim here | Status |
|---|---|
| "NHL 10 mapped `player.rating_overall` at an unknown identity. **The property is target-ABSENT** … 0 hits across 86 tables / 1769 fields" (Checkpoint 3, correction 3) | **WITHDRAWN.** A name search that finds nothing proves only that the search found no named match; it does not establish semantic absence. It is now an `unknown-field-identity` omission with the search retained as a discovery note |
| Five `[mapping.null_sentinel]` blocks declared at confidence `measured` on "no spare code" / declared-range arguments | **WITHDRAWN.** A declared range does not prove that every code in it carries an ordinary value. All five are now `unknown`, with the reasoning preserved as non-operative candidates |
| Four blocks established at confidence `asserted` (the `ubPc.dXSB` index origin and its inferred foreign key, on both targets), plus the operative `read_transform` / `write_transform` names that spelled the proposed origin | **WITHDRAWN as operative.** `asserted` can no longer make a constraint established; these are candidates now, and the transform names are `unresolved` with `qualification = "blocked"` |
| "Question 2 for the architect: is `established` + `asserted` the intended reading of *recorded claim with a citation*?" | **ANSWERED by the review: no.** Enforced by the new `asserted-as-operative` rule |
| Population figures — 43 and 38 established blocks | **SUPERSEDED** by 38 and 34, with candidates counted separately |

Rule and test counts below (41 rules, 69 tests) are likewise superseded by 45 and 78. Everything else
in this receipt stands unchanged: the representations, strict parsing, the version transition, the
string-pool and rating-identity corrections, and the capacity and companion work.

## Result

**Objective met for the database half.** `target-adapter/2` represents every database constraint
Q07a.1 explicitly requires. **C0 item 7 is still not complete**: population is unchanged at 2 of 28
catalog units, and the asset-profile half was out of scope and untouched.

---

## Checkpoint 1 — Representation inventory

Every database constraint Q07a.1 names, mapped to its state **before** this package:

| Q07a.1 constraint | Before | Now |
|---|---|---|
| table identity | `mapping.table`, but a free-text `"unknown"` placeholder | `mapping.table` + a `[[table]]` declaration every reference must resolve to; the placeholder is refused |
| field identity | `mapping.field`, same placeholder; **no composite** (v1 spelled a date as the invented id `"JKLp+Kswi"`) | `mapping.field` **or** `[[mapping.component]]` with per-component storage |
| semantic canonical property | present | unchanged |
| type | **missing** | `[mapping.storage].value.kind` |
| endianness | file-level only, unvalidated | per-field, cross-checked against the adapter's |
| bit/byte offset | **missing** | `[mapping.storage].value.bit_offset` |
| width | partial (`storage_bits`, no offset/signedness) | `[mapping.storage].value.bit_width` |
| signedness | **missing** | `[mapping.storage].value.signedness` — the scalar audit's codec taxonomy |
| range | `storage_range` **and** a declarable `human_range`, i.e. two places to be wrong | storage range declared; human range **derived** |
| enum vocabulary + stored code | **missing** (U-A3) | `[mapping.enumeration]`, validated against the canonical domain, the storage range and the sentinel |
| scale / offset / index origin | free-text transform names only | `[mapping.transform]`, cross-checked against those names |
| null sentinel | **missing** (U-A4) | `[mapping.null_sentinel]` |
| string encoding / pool behaviour | **missing** (U-A4) | `[mapping.string_storage]` |
| update / key / structural status | **missing** | `[mapping.status]` |
| foreign-key relationships | **missing** | `[mapping.foreign_key]` |
| companion / cardinality rules | **missing** (U-A4) | `[[companion_rule]]` + a required `[position_discriminator]` |
| capacity | **missing** (U-A6) | `[[capacity]]` with a load-bearing `source` |
| presence / applicability | omission only, free-text reason | `[mapping.applicability]` + a closed `[[omission]].reason` vocabulary |
| verified inverse write | present | unchanged, plus a checkable inverse for linear offsets |

**Unresolved semantics found and recorded, not guessed:** none blocked the format. Two are registered
as U-A8/U-A9 in the validation record (composite mappings have no single storage block; applicability
does not express cross-field write ordering).

## Checkpoint 2 — Types and validation

**Three states, kept apart.** Every constraint is a `Constraint<T>` declaring `established`
(a fact, with a confidence other than `unknown` and at least one provenance citation),
`unknown` (nothing establishes it — export blocks) or `not-applicable` (the target genuinely has no
such constraint, with a `reason` recording how that was determined). A missing block defaults to
`unknown`, so silence is never a fact. The payload sits in a nested `value` table, so a value present
under a non-established state is structurally visible and refused.

**No placeholders as values.** v1 spelled an unestablished identity as the literal string
`"unknown"`. v2 spells it as an **absent key** and refuses `unknown`/`tbd`/`n/a`/`""` anywhere an
identity belongs.

**Misspellings and bad versions are refused, not ignored.** Every struct is `deny_unknown_fields`, so
`provenence` or `bit_ofset` is a parse error. `target-adapter/1` and `target-adapter/3` are both
refused by `validate_adapter`; v1 does not even parse as v2.

**Version transition, tested.** `upgrade_v1` translates a v1 document conservatively: every new block
comes out `unknown`, and every v1 field with no v2 destination is returned in `Upgrade::notes` with
the reason. It promotes nothing. Both former v1 examples are preserved verbatim at
`crates/tdb-canonical/tests/fixtures/v1/` and the transition runs over both.

**Validated relationships** (only where the meaning is established): range against width and
signedness; field endianness against the adapter's; enum codes against the storage range, the
canonical domain and the sentinel; string byte budget against the storage width; capacity ceiling
against population; structured transform against the `read_transform`/`write_transform` spelling, and
a linear offset's write against its exact inverse; `position-*` companions against a required
discriminator; every table reference against a declaration. Duplicates and conflicts are refused for
tables, mappings, capacities, companion rules, companion tables, enum codes, position codes and
composite roles.

**Rules: 15 → 41.**

## Checkpoint 3 — Evidence-backed examples

Both committed adapters were re-authored against repository evidence only. Sources used, all in-tree:

| Source | What it established |
|---|---|
| `docs/audits/data/schema/xbox360-nhl15.json`, `…-nhl10.json` | every offset, width, range, key/updatable flag, declared FK parent, table name, and both `coverage_declared` totals |
| `docs/audits/roster-capacity.md` | the seven roster-bearing ceilings per title, and that on gen7 only the ROSTER save exposes them |
| `docs/audits/roster-relationships.md` | shared-key 1:1 companions on `wBIz`; the `dXSB → kOtt.vfEq` bridge hop |
| `docs/audits/relational-operation-audit.md` | `companion_presence` as an enforced save-time invariant |
| `docs/audits/scalar-mutation-audit.md` | the codec taxonomy (signedness/sentinel classes) and the inline-string policy |
| `docs/audits/ratings-write-codec.md`, `crates/studio-domain/tests/ratings_write_codec.rs` | the `+36` gen7 regime and its proven inverse |
| `crates/studio-domain/tests/birthdate_encoding.rs` | the birth-date index origins |
| `crates/tdb-core/src/lib.rs` (`read_inline_str`/`set_inline_str`) | NUL-fill, byte budget, over-length rejection |
| `crates/studio-domain/src/capabilities.rs` | the `dXSB` main-team origin, probed at runtime |
| `defs/enums/handedness.toml`, `position.toml` | handedness and position stored codes — **NHL Legacy-anchored, so recorded as `derived`, never `measured`** |

**Three v1 claims did not survive the check** (detail in the validation record §4.2):

1. `player.first_name` claimed `string-pool`. It is an inline fixed-width NUL-padded field. Corrected.
2. `player.rating_overall` / `rating_speed` carried `"unknown"` identities on both adapters, and
   NHL 15 omitted speed as `unknown-field-identity`. The committed snapshot names `dSvy.eWUF`
   (`c_overall`) and `dSvy.AdEb` (`c_speed`). Identities established; both stay `implemented`, not
   `automated-tested`, because the named case proves the *codec*, not the field.
3. NHL 10 mapped `player.rating_overall` at an unknown identity. **No field in NHL 10's snapshot has
   a name containing `overall`** — 0 hits across 86 tables / 1769 fields, against 15 in NHL 15's. It
   is now an omission with `reason = "target-absent"` **and its proof**. That is the correction the
   constraint layer exists for: v1 could not distinguish a proven absence from an unexamined gap.

**Synthetic fixture.** `crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml` exercises the
representations with no established real fact: pooled strings with `truncate`, two's-complement and
offset-binary signedness, a `literal-code` sentinel, a canonical-value predicate, a metadata-declared
FK, a one-to-many companion, a `main-db-max-records` ceiling and the `not-applicable` state. Every
value is invented and labelled. It carries `synthetic = true`, targets `fixture-synthetic`, lives
outside `defs/adapters/`, and `registry_population` excludes it; a test asserts no file under
`defs/adapters/` carries the flag. **It cannot inflate coverage by one row.**

No private fixture was acquired; `anchors/` and `docs/audits/**` were read only.

## Checkpoint 4 — Verification and coverage

### Representation completeness — a statement about the format

**20 of 20** database constraints Q07a.1 names have a representation
(`tdb_canonical::Q07A1_REPRESENTATION`, asserted by test).

### Registry population — a statement about the registry

| Target | Mappings | Omissions | Established blocks | Unknown blocks |
|---|--:|--:|--:|--:|
| `nhl15-xbox360` | 7 | 0 | 43 | 23 |
| `nhl10-xbox360` | 6 | 1 | 38 | 20 |
| the other **26 catalog units** | 0 | 0 | 0 | — |

Against 69,737 physical fields over 27 targets, still **under 0.02 %** of the semantic layer.
**Supporting a field in the format does not populate it for 28 targets**, and neither this receipt
nor the validation record claims otherwise.

### Checks run

| Check | Command | Result |
|---|---|---|
| Scoped crate tests | `cargo test -p tdb-canonical` | **Passed — 69/69** (was 16) |
| Strict clippy, scoped | `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | **Passed**, exit 0 |
| Formatting, scoped | `cargo fmt -p tdb-canonical -- --check` | **Passed**, exit 0 |
| Every committed `defs/` file validates clean | `the_committed_*` tests | **Passed** |
| Repo-wide suite / emitters | — | **Not run.** No cross-workspace issue warranted it: no crate outside `crates/tdb-canonical` references `tdb_canonical`, so the breaking v1→v2 type change has no other consumer. Repo-wide `cargo fmt --all -- --check` remains a known open baseline defect (C0.2) and was deliberately not touched |

Negative tests, one or more per enforceable rule: misspelled keys at three nesting depths and an
invented top-level key; placeholder identities; established blocks without value / confidence /
provenance; values and confidences smuggled under `unknown`; `not-applicable` without a reason;
out-of-vocabulary terms; undeclared and duplicate tables; inverted, oversized and sign-contradicting
ranges; contradicting endianness; duplicate and out-of-range enum codes; enum values outside the
canonical domain; constraint/property kind mismatches; four transform-declaration conflicts;
sentinel/encoding, sentinel/regime and sentinel/enum collisions; string budget conflicts both ways;
incomplete and undefined-property predicates; three composite-identity conflicts; capacity below
population and a bad capacity source; duplicate capacities, companion rules and companion tables; a
missing position discriminator; overlapping and empty position codes; a default on a key field; an
unproven `target-absent` omission; and unsupported schema ids and canonical versions.

---

## Files changed

| Path | Change |
|---|---|
| `crates/tdb-canonical/src/lib.rs` | Rewritten. Vocabularies, 41 rules, three validators, the Q07a.1 representation map, `human_range`/`scalar_editable` derivations, `representation_completeness` and `registry_population` |
| `crates/tdb-canonical/src/model.rs` | **New.** All document types, every one `deny_unknown_fields` |
| `crates/tdb-canonical/src/v1.rs` | **New.** `target-adapter/1` types and the reporting `upgrade_v1` transition |
| `crates/tdb-canonical/src/tests.rs` | **New.** 69 tests, split out of `lib.rs` |
| `crates/tdb-canonical/tests/fixtures/v1/nhl15-xbox360.toml`, `nhl10-xbox360.toml` | **New.** The former v1 files, preserved verbatim as transition fixtures |
| `crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml` | **New.** The labelled synthetic fixture |
| `defs/adapters/nhl15-xbox360.toml` | Re-authored to `target-adapter/2`; 7 mappings, 8 table declarations, 8 capacities, companion rule, position discriminator |
| `defs/adapters/nhl10-xbox360.toml` | Re-authored to `target-adapter/2`; 6 mappings, 1 proven omission, same adapter-level blocks |
| `docs/validation/c0-06-canonical-project-and-adapters.md` | Amended: §1 transition, §2 rules, §4 divergences/corrections/fixture, §5 coverage and U-A3/4/6 closed + U-A8/9 opened, §8, new §9 |
| `docs/plans/project-completion-roadmap.md` | C0 item-7 status row only |
| `docs/receipts/c0-can1-adapter-constraints.md` | **New.** This receipt |

Unchanged on purpose: `defs/canonical/project-schema.toml` (still `canonical-project/1` v1, so both
adapters' `canonical_schema_version = 1` stays valid) and
`defs/adapters/asset-profiles/assets-portraits.toml`.

## Preservation checks

| Check | Result |
|---|---|
| HEAD | `a2bbdc4f…`, unchanged |
| Index | **empty** — nothing staged |
| Pre-existing dirty/untracked work | **preserved.** `git status --short` is byte-identical before and after (242 entries; the paths this package touched already sat under existing untracked directory entries) |
| `anchors/`, `docs/audits/**` | read only; not modified |
| Other crates | none modified; no crate outside `crates/tdb-canonical` references it |
| `Cargo.toml` / `Cargo.lock` | **not modified** by this package |

Package-specific hashes (SHA-256):

| File | Before | After |
|---|---|---|
| `crates/tdb-canonical/src/lib.rs` | `018ab746…` | see below |
| `defs/adapters/nhl10-xbox360.toml` | `b5395fe5…` | see below |
| `defs/adapters/nhl15-xbox360.toml` | `c7d51275…` | see below |
| `defs/canonical/project-schema.toml` | `702b8e4f…` | **unchanged** |
| `docs/validation/c0-06-…md` | `17a7409e…` | see below |
| `docs/plans/project-completion-roadmap.md` | `5efe9613…` | see below |
| `Cargo.toml` | `5272dcbd…` | **unchanged** |
| `Cargo.lock` | `4a334c65…` | **unchanged** |

### Appendix — package-specific SHA-256, after

```
d72c2d153e0ba7a9b8b3ee5af1f762061b8ef73263b52c1161b8e8c9bf2406c4  crates/tdb-canonical/src/lib.rs
02060ce12068490d5cdb9e210d1111b23c6f7fa8bfc5ae9e0d5c1dfebcb868ea  crates/tdb-canonical/src/model.rs
53c29f33807d59da64de957f7a555bea3b28f1cd2a1157931c9ce577cb0cbaa0  crates/tdb-canonical/src/v1.rs
60e02efccff30f37428543b78a34c976207469d413c8d60700a8b5b4e84c3ed0  crates/tdb-canonical/src/tests.rs
ab4473ae68a8b5fbd226875deb9069c42ba7eff0caf0699d00099142651f3928  defs/adapters/nhl10-xbox360.toml
474372d37cd3625cc57962a9fe4e8bc7b679a192fd8b6e22ae904611aabc561c  defs/adapters/nhl15-xbox360.toml
b5395fe50441bdfb7e10370e1b17de217804a7db733030fe4545dea9a8ee05e7  crates/tdb-canonical/tests/fixtures/v1/nhl10-xbox360.toml
c7d51275e4de71bc5d95a492c30b8b52f891a35632f7d9fe640b630fcf47c780  crates/tdb-canonical/tests/fixtures/v1/nhl15-xbox360.toml
315c724bbd38f8cda62c522f4d32a6f813de262754781c70315d617d220e7349  crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml
80432b492dd65ed5a731302270b7906e684bdc21fdf326718624f77f7c276964  docs/validation/c0-06-canonical-project-and-adapters.md
2c7770f83edca291cce1c96974085e5529cac0d0ed697ea9a79cadffba7e5765  docs/plans/project-completion-roadmap.md
702b8e4fc4c7dc9130dfde137266b6b808fe6018b9eeb4f296ec14f0cacd12ac  defs/canonical/project-schema.toml   (unchanged)
5272dcbddaaeb834e1defbb659f090becc6eca5167f63c512c9481b849459e82  Cargo.toml                           (unchanged)
4a334c6570f19cd2d6e5b76c9bba83bbce8e503196bdc5bd7d44e7fa57f4aa2b  Cargo.lock                           (unchanged)
```

The two v1 transition fixtures hash **byte-identically to the pre-package adapters**
(`b5395fe5…`, `c7d51275…`), which is the proof that the historical documents were preserved verbatim
rather than re-typed.

---

## Unresolved / out of scope

**Boundaries respected — none of P1–P4 was decided.** `[[mapping.component]]` was added because a
canonical *date* spans three target fields, which is a target-encoding fact, not an answer to P1's
question about what the canonical model carries. No extension behaviour (P2), uniforms mapping (P3),
asset override mechanism (P4), variant catalog or permanent evidence store was touched, and no
evidence qualification or publication behaviour was changed.

### Questions for the architect

1. **Asset profiles (relates to P4, but is a separate question).** `asset-output-profile/1` still has
   free-text fields with no state/confidence envelope, so an asset fact cannot distinguish unknown
   from not-applicable the way a database fact now can. Should a follow-up package give asset
   profiles the same constraint envelope? This package deliberately did not, because item 7's asset
   half was outside its objective.
2. **Confidence `asserted` on an established block.** Two facts (`ubPc.dXSB` index origin on both
   targets) are recorded as `established` at confidence `asserted`, citing a documented runtime probe
   rather than a measurement. Is `established` + `asserted` the intended reading of "recorded claim
   with a citation", or should such a fact be `unknown` until measured? The rule set accepts it today
   and the note on each block says exactly what the citation covers.
3. **Population sequencing.** 26 units have no adapter. Is populating them a C0 obligation or does it
   move to C1 with the database-semantics work?

**Stop for Codex review.** No commit, push, staging or publication is authorized or performed.
