# C0-AS2-R1 — Target Build-mapping overrides (`build-mapping-override/1`)

> **Package:** C0-AS2-R1 — correct the P4 interpretation, remove the unapproved departure model, and
> implement the approved contract. **Date:** 2026-09-08. **Author role:** developer. Reviewer:
> Codex (architect).
> **Nothing staged, committed or pushed.** No asset editor, no conversion engine, no archive write,
> no UI, no Build engine, no migration installed, no live publication. **No portrait measurement and
> no game-data search.** No fixture acquired, no evidence publication changed, no additional target
> or family populated, and **no constraint promoted anywhere**. P1–P3 remain undecided.

> **Sourcing note.** No `C0-AS2` document exists anywhere in this repository, in `docs/plans/`, or in
> the plans directory — searched by name and by content. This package was executed from the
> instruction text as delivered, including the authoritative P4 statement and its worked example,
> which are quoted verbatim in §1. If a written C0-AS2 brief exists elsewhere, §2's design should be
> checked against it.

---

## 1. The correction

The authoritative reading of P4, as delivered:

> An override changes a target Build's requested derivation settings **within** proven target
> constraints. It never permits departure from those constraints. Canonical assets and target
> profiles remain unchanged. Override records, provenance and approval belong in Build mappings.
>
> If a proven target profile permits several output dimensions, a Build may select an allowed
> dimension. It cannot approve a dimension outside that set. If a codec is required, approval cannot
> authorize a different codec.

C0-AS1-R1 read the same question as a **departure** from constraints and built a per-target
permission to depart, in the profile. Both halves were wrong: the direction (departure vs. choice
within) and the location (profile vs. Build mapping). It also had no notion of an approval, of an
approval bound to inputs, or of invalidation — the three things the approved contract is mostly
about.

### 1.1 What was removed

Only this package's own additions, by targeted edit. No `git restore`, no `git reset`, and no other
working-tree change touched.

| Removed | From |
|---|---|
| `OverridePolicy`, `unspecified_disposition`, `[target_profile.override_policy]`, `validate_override_policy` | `crates/tdb-canonical/src/asset.rs` |
| `OVERRIDE_DISPOSITION`, `OVERRIDE_SCOPE`, `OVERRIDE_RESOLUTION`, `OVERRIDE_INVALIDATOR`, `RESOLUTION_CLASS`, `REQUIRED_INVALIDATORS` | same |
| `AssetPopulation::override_disposition`, `AssetPopulation::overridable_blocks` and their counting | same |
| the "Overrides are permitted, never recorded" module-doc section | same |
| `AssetOverridePolicyIncomplete`, `AssetOverridePolicyConflict`, `AssetOverrideOnUndecidedConstraint` and their names and re-exports | `crates/tdb-canonical/src/lib.rs` |
| the ten `Q07a.3a: the per-target override policy` tests and their rule-name entries | `crates/tdb-canonical/src/asset_tests.rs` |
| the permissive and refusing policy tables | `crates/tdb-canonical/tests/fixtures/synthetic-asset-profile.toml` |
| the two `unspecified` policy tables | `defs/adapters/asset-profiles/assets-portraits.toml` |

The accepted C0-AS1 schema work is untouched: fifteen `Constraint<T>` blocks, 26/26 Q07a.1 asset
representation, the v1→v2 transition, and every established/candidate/unknown fact in both committed
profiles. Removal was verified by return to the exact AS1 baseline — **53 rules, 126 tests** — before
any new code was added.

### 1.2 Active guidance corrected

Both receipts that repeat "departure from constraints" now carry an **appended, labelled**
correction; neither is rewritten, and the historical record stands:

* [c0-as1-r1-override-policy.md § *Withdrawn*](c0-as1-r1-override-policy.md) — the whole package
  marked withdrawn, with the error named and the surviving instincts identified.
* [c0-as1-asset-profile-constraints.md § *Correction*](c0-as1-asset-profile-constraints.md) — the
  answer appended to its §8 withdrawn, with a said/says table.

No other document in the repository asserted the departure model.

## 2. The approved contract

New module `crates/tdb-canonical/src/build_override.rs`, schema **`build-mapping-override/1`**. It
converts nothing, reads no asset and writes no file — specification and pure validation, like the
rest of the crate.

### 2.1 The record

One record addresses **one setting on one mapping on one target**. Every field the package named is
present and typed:

| Requirement | Field |
|---|---|
| source asset identity, revision, hash | `[source]` — `asset_id`, `revision`, `hash_algorithm` (closed), `hash` |
| target and mapping identity | `[mapping]` — `target`, `family` (closed), `mapping_id`, `revision` |
| typed setting and proposed value | `[proposal.proposed]` — a `SettingChoice`, adjacently tagged: `setting` selects the variant, `operand` carries its typed fields |
| prior value **or explicit absence** | `[proposal.prior]` — `absent` + `reason`, or `value`; never both, never neither |
| provenance | `[proposal].provenance`, non-empty |
| resolution method | `[proposal].resolution`, one of `SETTING_RESOLUTION` |
| profile / adapter identity | `[profile]` — `family`, `schema`, `profile_version` |
| approval bound to exact inputs | `[approval]` — `approved_by`, `approved_at`, and the four bound inputs: `source_revision`, `source_hash`, `profile_version`, `mapping_revision` |

**No arbitrary property bag.** `SettingChoice` is a closed enum of seven typed settings —
`dimensions`, `mipmap-count`, `raster-codec`, `compression-payload`, `tiling`, `atlas-slot`,
`audio-sample-rate` — each with its own `deny_unknown_fields` operand struct. A setting outside the
set, or an operand field outside its struct, is a **parse** error. `deny_unknown_fields` applies to
every table in the document.

### 2.2 Resolution, and why the order matters

`resolve_build_override(&record, &profile) -> OverrideOutcome`, pure, both arguments borrowed:

1. profile identity → `RecordProfileMismatch`
2. target present in the profile → `NoSuchTarget`
3. the constraint that decides the setting:
   * `unknown` → **`BlockedByUnknownConstraint`**
   * `not-applicable` → `RefusedByConstraint` (a decided fact: the target has no such setting)
   * established, proposal outside the permitted set → **`RefusedByConstraint`**
4. **only now** the approval: absent → `Proposed`; bound inputs moved → `ApprovalInvalidated(named)`;
   otherwise → `ApprovedMapping`

The approval is not read until steps 3 has passed, so **there is no code path in which approval can
rescue a violation or reach past an unknown**. Two tests assert it behaviourally rather than
structurally: a refused record resolves identically with and without its approval, and so does a
blocked one.

### 2.3 The permitted set is read off the constraint

Nothing in a record contributes to what a target accepts, and nothing was added to a profile to
widen one. Each setting's permitted set is derived from the established block:

| Setting | Permitted set |
|---|---|
| `atlas-slot` | `0 .. (page_width/cell_width)*(page_height/cell_height)` — a genuine choice among many |
| `dimensions` | the established `width`x`height` for the variant; a non-power-of-two is refused separately where the target requires powers of two |
| `mipmap-count` | the established `count` under `fixed-count`; under any other policy the chain length follows from the surface and the policy, so it is not a Build setting at all |
| `raster-codec`, `compression-payload`, `tiling`, `audio-sample-rate` | the established value |

Most of these are sets of one, because that is what the AS1 schema actually says — it records exact
required values, not ranges. **No target capability was invented to make a choice look larger than
it is**, and the atlas case is a real multi-value set derived from page and cell extents the fixture
already established.

### 2.4 What it cannot express

| It cannot | Enforced by |
|---|---|
| authorise a value outside a proven constraint | the outcome enum has no "approved despite" variant; constraint before approval |
| bypass an unknown constraint | `BlockedByUnknownConstraint`, reached before any approval read |
| change a canonical asset | no key addresses one; both arguments are borrowed; a test injects `rewrite_canonical` and `canonical_value` and asserts parse errors |
| change a target profile | same; a test asserts the profile is identical before and after resolving four records, and identical to the file on disk |
| reach another target's mapping | the record names one `target` + one `mapping_id`; a test resolves the same record against a second target and gets a different answer, and against an absent one and gets `NoSuchTarget` |
| claim converter availability or game acceptance | no key and no variant; a test asserts `converter`, `supported`, `qualification` and `game_verified` are all parse errors |

### 2.5 Three new rules

`override-record-incomplete`, `override-prior-conflict`, `override-approval-unbound`. Document
well-formedness only — they say nothing about whether a proposal is permitted, which is
`resolve_build_override`'s question and needs the profile.

## 3. Tests

25 cases in `build_override_tests.rs`, all against **labelled synthetic fixtures**. Every fixture is
`synthetic = true`, names a `fixture*-synthetic` target, and resolves against
`synthetic-asset-profile.toml`, whose targets are not catalog units — a test enumerates the fixture
directory and asserts all four.

| The package asked to prove | Case |
|---|---|
| a permitted setting choice can validate | `a_permitted_setting_choice_validates_and_resolves_to_an_approved_mapping`; `the_permitted_set_comes_from_the_constraint_and_the_record_cannot_widen_it` walks all 16 slots and then slot 16 |
| violating a known constraint is rejected even with approval supplied | `violating_a_known_constraint_is_refused_even_with_a_valid_approval_supplied` (a fully bound, valid approval on a `bc7` proposal against a required `dxt5`); `removing_the_approval_changes_nothing_about_a_refusal` |
| unknown constraints cannot be bypassed | `an_unknown_constraint_blocks_and_names_itself`; `no_approval_can_reach_past_an_unknown_constraint` |
| proposals are distinct from approved mappings | `a_permitted_choice_nobody_approved_is_a_proposal_and_not_a_mapping` |
| relevant source/mapping/profile changes invalidate approval | `a_changed_source_revision_or_digest_invalidates_the_approval`, `a_changed_mapping_revision_invalidates_the_approval`, `an_approval_bound_to_a_stale_profile_version_is_invalidated`, `a_profile_that_moved_on_is_a_mismatch_before_it_is_anything_else` |
| target mappings remain independent | `one_targets_record_never_reaches_another_targets_mapping` |
| canonical assets and profiles remain unchanged | `resolving_an_override_changes_neither_the_profile_nor_the_record`; `no_override_record_can_name_a_canonical_source_field_to_change` |
| malformed schemas and unknown fields are rejected | `an_unknown_field_anywhere_is_a_parse_error_not_a_silent_drop` (7 injection points), `an_unknown_setting_or_a_mistyped_operand_is_a_parse_error`, `a_wrong_schema_or_a_zero_revision_is_reported_rather_than_assumed`, `a_record_bound_to_nothing_is_incomplete`, the two prior-value cases, `a_support_or_acceptance_claim_has_no_spelling_in_an_override_record` |

Plus `a_setting_the_target_does_not_have_is_refused_rather_than_permitted` (not-applicable refuses
where unknown blocks — different facts, different answers) and
`a_pinned_constraint_permits_a_set_of_one_and_refuses_everything_else`.

| Metric | AS1 baseline | After removal | After C0-AS2-R1 |
|---|---|---|---|
| Rules | 53 | 53 | **56** |
| `tdb-canonical` tests | 126 | 126 | **151** |
| Q07a.1 asset representation | 26/26 | 26/26 | 26/26 |
| Established constraints, real profiles | 0 | 0 | **0** |

## 4. Files changed

| File | Change |
|---|---|
| `crates/tdb-canonical/src/build_override.rs` | **new** — the schema, validation and resolution |
| `crates/tdb-canonical/src/build_override_tests.rs` | **new** — 25 cases |
| `crates/tdb-canonical/tests/fixtures/build-overrides/*.toml` | **new** — four labelled synthetic records: approved, refused-with-approval, blocked-by-unknown, proposal-only |
| `crates/tdb-canonical/src/lib.rs` | removals per §1.1; `BUILD_OVERRIDE_SCHEMA`, the module, three rules, re-exports |
| `crates/tdb-canonical/src/asset.rs` | removals per §1.1 only |
| `crates/tdb-canonical/src/asset_tests.rs` | removals per §1.1 only |
| `crates/tdb-canonical/tests/fixtures/synthetic-asset-profile.toml` | removals per §1.1 only |
| `defs/adapters/asset-profiles/assets-portraits.toml` | removals per §1.1 only — the file is byte-equivalent to its C0-AS1 state |
| `docs/receipts/c0-as1-r1-override-policy.md`, `docs/receipts/c0-as1-asset-profile-constraints.md` | appended labelled corrections; nothing rewritten |
| `docs/receipts/c0-as2-r1-build-mapping-overrides.md` | this record |

## 5. Checks

| Check | Result |
|---|---|
| `cargo test -p tdb-canonical` | **151/151**, 0 failed (25 in `build_override`) |
| `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean (repo-wide) |
| HEAD | `a2bbdc4`, unchanged |
| Index | **empty**; nothing staged, committed or pushed |
| `anchors/`, `docs/audits/**` | not read, not written |
| QA generators | not run |
| Game data | not searched, not read, not measured |

`tdb-canonical` is referenced by no other crate, so no other code compiles against the removed
`AssetPopulation` fields.

## 6. Questions for the architect

1. **Is the setting vocabulary the right seven?** They are the AS1 blocks a Build could plausibly
   request. `install`, `path_rule` and `identity` were left out deliberately — they look like
   placement policy rather than derivation settings, but that is a judgement, not a decision.
2. **Should the database half get the same record?** `target-adapter/2` has no override record
   either, and Q07a.3a was written about invalid *database* values first. Not widened into here.
3. **Where do override records live on disk?** The module documents `<build>/overrides/<id>.toml`
   as an expectation. No Build layout is settled, and nothing reads a directory.
4. **The shipped-portrait read remains authorized but unscheduled** — it needs one explicit
   game-data path, and was explicitly out of scope for this package.
5. Carried forward: P1–P3 undecided; 26 units have no adapter; 13 families have no profile;
   ownership unassigned; population still a C0 obligation.

**Stop for Codex review.**

---

## Correction — appended 2026-09-08 by package C0-AS2-R2

**R1's exact-binding claim was wrong.** §2.1 called the approval "bound to exact inputs" and §2.2
described drift as "bound inputs moved"; §3's invalidation row claimed the regression cases proved
it. They proved less than that.

The approval carried **copies of four fields** — `source_revision`, `source_hash`,
`profile_version`, `mapping_revision` — and resolution compared each copy with the record's own
matching field. That detects a moved revision or digest and nothing else. It does **not** detect:

| Reused approval | Why R1 missed it |
|---|---|
| a different permitted proposed value (slot 7 → slot 5) | the proposal was not part of the binding at all |
| a different setting, both choices valid | same |
| a renamed source asset with identical bytes and revision | only the revision and digest were compared, never the identity |
| a renamed target or mapping at an identical revision | same |
| a profile edited in place at an identical `profile_version` | only the version number was compared, never the content |
| a changed prior value, resolution method or provenance | none were part of the binding |

Two further R1 statements are corrected by the same package. Resolution compared the record's claims
only with copies **inside the record**, so it never checked the record against the current world;
and nothing stopped a malformed record reaching `ApprovedMapping`, since `resolve_build_override`
took a bare `BuildOverride`.

Everything else in R1 stands: the removal of the departure model, constraint-before-approval
ordering, the closed typed setting vocabulary, the derived permitted set, and the scope statements.

See [c0-as2-r2-exact-approval-binding.md](c0-as2-r2-exact-approval-binding.md).
