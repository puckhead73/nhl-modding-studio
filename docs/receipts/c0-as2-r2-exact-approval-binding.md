# C0-AS2-R2 — Binding approval to the exact proposal and to current inputs

> **Package:** C0-AS2-R2 — close the approval-reuse defect in `build-mapping-override/1`.
> **Date:** 2026-09-08. **Author role:** developer. Reviewer: Codex (architect).
> **Nothing staged, committed or pushed.** No game-data search, no measurement, no publication, no
> migration installed, no Build engine, no asset editor, no converter. Labelled synthetic fixtures
> only. The removed departure model stays removed; constraint-before-approval ordering is preserved.
> P1–P3 remain undecided.

---

## 1. The defect

R1's approval carried **copies of four fields** — `source_revision`, `source_hash`,
`profile_version`, `mapping_revision` — and resolution compared each copy against the record's own
matching field. That is a freshness check on four fields, not a binding to a proposal. Everything
outside those four could change while the approval kept applying:

| Reuse | Why it survived |
|---|---|
| slot 7 → slot 5, both permitted | the proposed value was not in the binding at all |
| a different setting, both choices valid | same |
| a renamed asset, identical bytes and revision | identity was never compared, only revision and digest |
| a renamed target or mapping, identical revision | same |
| a profile edited in place at the same `profile_version` | only the version number was compared, never the content |
| a changed prior value, resolution method or provenance | none were in the binding |

Two structural problems came with it: resolution compared the record's claims only against copies
**inside the record**, so the current world was never consulted; and `resolve_build_override` took a
bare `BuildOverride`, so nothing stopped a malformed record reaching `ApprovedMapping`.

## 2. The fingerprint

The four copied fields are **gone from `[approval]`**, and reintroducing one is now a parse error —
a test asserts it, because a stale field that looks like a binding while binding nothing is exactly
what caused this. In their place:

```toml
[approval]
approved_by = "…"
approved_at = "2026-09-08"
fingerprint_version = 1
fingerprint = "<64 hex>"
```

`override_fingerprint(record, profile_content_digest)` → SHA-256 over a canonical encoding.
`APPROVAL_FINGERPRINT_VERSION` is stored with the digest, so a construction change is detectable:
an approval carrying another version is **invalidated, never reinterpreted** under today's rules.

### 2.1 Included

| Group | Fields |
|---|---|
| source | `asset_id`, `revision`, `hash_algorithm`, `hash` |
| mapping | `target`, `family`, `mapping_id`, `revision` |
| proposal | the typed `setting` and every field of its operand; `resolution`; `provenance` in order |
| prior | `absent`, `reason`, and the typed prior value when present |
| profile | `family`, `schema`, `profile_version`, and the profile **content digest** |

Identity as well as revision, so a rename at an unchanged revision moves the fingerprint. The
proposed value, so exchanging one permitted value for another moves it. The profile's content, so
an in-place constraint edit moves it.

### 2.2 Deliberately excluded

| Excluded | Why |
|---|---|
| `[approval]` itself | a fingerprint covering the approval could not be stored inside it |
| `note` on the record, the proposal and the approval | free prose; describes rather than decides |
| `synthetic` | a fixture label, not an input to any derivation |
| `record_version` | the record's own bookkeeping; every substantive field it could track is already covered, so bumping it alone must not invalidate an otherwise identical approval |

The exclusions are exactly the fields that cannot change what a Build produces. A test asserts each
one leaves the fingerprint untouched, including replacing `approved_by`, which is the observable
form of "the approval is outside its own fingerprint".

### 2.3 Canonical encoding

Every field is written as `name` US `byte-length` US `value` RS. Length-prefixing makes the encoding
injective — no combination of values can produce the bytes of a different combination, so two
proposals cannot collide by concatenation accident. Lists write their count first. `None` is a
marker byte no present value can spell, so an absent operand and an empty one differ. **Field order
is fixed by the code, never by map iteration**, so output does not depend on hash seeds, locale or
platform.

`profile_content_digest` uses the same encoder over a profile's *operative* content: `family`,
`profile_version`, `synthetic`, each `[[canonical_source]]` in order, and per target each of the
fifteen blocks by fixed position with its `state`, `confidence`, `provenance`, `reason` and, when
established, every field of its value; then the declared operations. **Excluded:** `note` and
`derivation` (prose) and the `candidate` channel — a candidate is non-operative by construction, so
it cannot change what a proposal is checked against, and an editorial edit must not invalidate an
approval that moved nothing about the target. Two tests assert both halves of that line.

`sha2 = "0.10"` was added to the crate. It is already vendored for `tdb-qa`, so no new third-party
surface enters the repository; the crate is otherwise still serde+toml and still reads no game data.

## 3. Validation against the current context

`resolve_build_override` no longer takes the record's word for the state of the world:

```rust
resolve_build_override(&ValidatedOverride, &BuildContext, &ValidatedProfile) -> OverrideOutcome
```

* **`BuildContext`** is the *current* source and mapping, supplied by the caller. `context_drift`
  compares the record's declared identity, revision, hash algorithm and digest against it, and a
  difference yields the new `OverrideOutcome::StaleContext(named fields)` — whether or not anyone
  approved the record. It is a plain value: **no filesystem, no Build engine, no I/O**.
* **`ValidatedProfile`** computes the content digest **from the profile it was handed**, never from
  the record. That is what makes an in-place profile edit detectable; the record's declared
  `[profile].content_digest` is compared against it and a mismatch is `RecordProfileMismatch`.

### 3.1 Malformed input cannot reach an approval

`ValidatedOverride::new` and `ValidatedProfile::new` are the only constructors and each runs its
validator, returning `Err(Vec<Finding>)` otherwise. The resolver takes those types, so **a malformed
record or profile cannot be resolved at all** and `ApprovedMapping` is unreachable for one. The
check is at the type boundary rather than inside the resolver so a future caller cannot skip it.

### 3.2 Order, unchanged where it matters

1. profile — family, version **and content digest**
2. target present
3. **constraint** — refuse or block
4. context drift
5. approval — fingerprint comparison only

Steps 3 and 4 return before step 5 exists, so an approval is still never read for a refused value or
past an unknown constraint. Both properties keep their behavioural tests.

## 4. Regression tests

Every mutation case starts from the approved, valid mapping, **re-asserts that baseline**, changes
exactly one thing, and makes two assertions: that the stored fingerprint no longer describes the
record, and that the outcome is not `ApprovedMapping`.

The two are kept apart deliberately. Renaming the target sends the record to a target whose atlas
extents are not both established, so it is stopped by the **constraint** step before any approval is
read. That is the ordering working as intended and must not be mistaken for the binding working —
so the fingerprint assertion is what proves the binding covers the target there.

| Independent change | Case |
|---|---|
| proposed value → another permitted value | `changing_the_proposed_value_to_another_permitted_value_invalidates_the_approval` — slot 7 → slot 5, every revision and digest untouched |
| setting, both choices valid | `changing_the_setting_itself_invalidates_the_approval` — slot 7 and the established `dxt5` are both permitted |
| source asset id, bytes and revision identical | `renaming_the_source_asset_invalidates_the_approval_even_with_identical_bytes` |
| target or mapping id, revision identical | `renaming_the_target_or_the_mapping_invalidates_the_approval_at_an_identical_revision` |
| profile contents, `profile_version` identical | `editing_the_profile_in_place_invalidates_the_approval_at_an_identical_version` — asserts the version is deliberately unmoved and the digest moved |
| prior value, resolution method, provenance | `changing_the_prior_value_the_resolution_or_the_provenance_invalidates_the_approval` — four mutations including prior-value → prior-absence |
| current context, record untouched | `a_changed_current_context_invalidates_the_approval_without_the_record_being_touched` — re-exported source, renamed asset with identical bytes, renamed mapping at the same revision |

Supporting cases: `unchanged_inputs_stay_approved_across_repeated_resolution` (determinism, and the
baseline stays approved); `a_permitted_choice_nobody_approved_is_a_proposal_and_not_a_mapping`;
`an_approval_from_another_fingerprint_construction_is_invalidated_not_reinterpreted`;
`the_committed_fixture_carries_the_digests_the_code_computes` (the fixtures are not merely
self-consistent — a change to either construction fails here rather than silently re-approving
everything); `the_fingerprint_excludes_exactly_the_fields_that_decide_nothing`;
`the_profile_digest_ignores_prose_and_the_non_operative_candidate_channel`;
`a_malformed_record_cannot_be_resolved_at_all`; `a_malformed_profile_cannot_be_resolved_against`;
`a_truncated_digest_is_refused_wherever_one_is_required`.

**Retained unchanged in substance:** `violating_a_known_constraint_is_refused_even_with_a_valid_approval_supplied`
(now with a *freshly recomputed* approval, so it cannot pass merely because the fingerprint was
stale), `removing_the_approval_changes_nothing_about_a_refusal`,
`an_unknown_constraint_blocks_and_names_itself`, `no_approval_can_reach_past_an_unknown_constraint`,
and the whole malformed-input, prior-value and support-claim group.

| Metric | R1 | R2 |
|---|---|---|
| `tdb-canonical` tests | 151 | **162** |
| Rules | 56 | 56 |
| Q07a.1 asset representation | 26/26 | 26/26 |
| Established constraints, real profiles | 0 | **0** |

## 5. Scope, restated

* The **seven settings are initial coverage only**, not a closed statement of what a Build may
  request. `install`, `path_rule` and `identity` remain excluded — they read as placement policy
  rather than derivation settings.
* `<build>/overrides/<id>.toml` in the module docs is **illustrative**. No Build layout is settled
  and nothing in this crate reads a directory.
* **No database override implementation** in this correction. `target-adapter/2` still has none.
* No portrait measurement, no game-data search.

## 6. Files changed

| File | Change |
|---|---|
| `crates/tdb-canonical/src/build_override.rs` | `Canon`, `override_fingerprint`, `profile_content_digest`, `APPROVAL_FINGERPRINT_VERSION`, `BuildContext`, `context_drift`, `ValidatedOverride`, `ValidatedProfile`, `OverrideOutcome::StaleContext`, the rewritten `Approval` and resolver, `[profile].content_digest`, module docs |
| `crates/tdb-canonical/src/build_override_tests.rs` | the mutation harness and eleven new cases; existing cases adapted to the validated-input API |
| `crates/tdb-canonical/tests/fixtures/build-overrides/*.toml` | `content_digest` on all four; `fingerprint_version`/`fingerprint` replacing the four copied fields on the two approved ones |
| `crates/tdb-canonical/Cargo.toml` | `sha2 = "0.10"`, with the reason recorded in the file |
| `crates/tdb-canonical/src/lib.rs` | re-exports |
| `docs/receipts/c0-as2-r1-build-mapping-overrides.md` | appended labelled correction to the exact-binding claim; nothing rewritten |
| `docs/receipts/c0-as2-r2-exact-approval-binding.md` | this record |

Nothing else in the working tree was touched, and no historical evidence was rewritten.

## 7. Checks

| Check | Result |
|---|---|
| `cargo test -p tdb-canonical` | **162/162**, 0 failed |
| `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean (repo-wide) |
| HEAD | `a2bbdc4`, unchanged |
| Index | **empty**; nothing staged, committed or pushed |
| `anchors/`, `docs/audits/**` | not read, not written |
| QA generators | not run |
| Game data | not searched, not read, not measured |

## 8. Questions for the architect

1. **Should `context_drift` distinguish an expected re-export from a stale record?** Today any
   difference is `StaleContext`. A Build that has just re-exported an asset will hit this on every
   record for it, and re-approval is the only exit. That may be exactly right, or it may want a
   "source moved, everything else intact" outcome that a UI can offer to re-approve in bulk.
2. **Is excluding `record_version` from the fingerprint correct?** The reasoning is in §2.2; the
   alternative is that bumping it should invalidate, which would make the record's own bookkeeping a
   binding input.
3. **Should the profile digest cover the candidate channel after all?** It is non-operative by
   construction, so excluding it is defensible — but a reviewer may prefer that *any* profile byte
   change invalidates.
4. Carried forward: P1–P3 undecided; the shipped-portrait read remains authorized but needs one
   explicit game-data path; 26 units have no adapter; 13 families have no profile; ownership
   unassigned; population still a C0 obligation.

**Stop for Codex review.**
