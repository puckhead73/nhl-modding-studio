# C0-AS3-R2 — observation authority boundary

**Package:** C0-AS3-R2 · **Date:** 2026-09-08 · **Result:** complete, awaiting review
**Preceded by:** [C0-AS3](c0-as3-scoped-asset-variants.md) + [R1](c0-as3-r1-source-backed-variants.md)
**Record amended:** [c0-06 §1 header, §10.7](../validation/c0-06-canonical-project-and-adapters.md)

> **No game data was read, nothing was measured, no profile was promoted and no schema block or
> validation rule was added.** This package closes one gap R1 left open, records two decisions, and
> corrects a documentation over-statement. R1's source identity checks, correlated variants, scope
> handling and fingerprint v2 are preserved unchanged.

---

## 1. The gap

C0-AS3-R1's entire argument was that variant selection must not run on what an override record
*claims* about its source. Selection was moved onto `SourceObservation` — facts read off the current
source, carrying the identity and digest of the source they were read from — and the record's own
selector was demoted to an expectation that gets compared and can be refused.

Then `OBSERVATION_METHOD` admitted `operator-entered`, at full standing. A hand-entered width and a
width read out of a header are both `Option<u32>` in the same struct, and the resolver could not tell
them apart. That is the door R1 closed, reopened one step further along: instead of the record
claiming its source is the 512x128 form, a person types 512 and 128 into the context and the resolver
treats it as a reading.

R1's own receipt flagged it as a question. This package answers it: **the term stays, and its
authority goes.**

---

## 2. The boundary

```rust
pub const OBSERVATION_METHOD: &[&str] =
    &["header-read", "path-read", "index-read", "operator-entered"];

/// The methods whose facts may DECIDE a variant.
pub const OPERATIVE_OBSERVATION_METHOD: &[&str] = &["header-read", "path-read", "index-read"];
```

`resolve_variant` gained one gate, immediately after the source-identity check and **before** the
facts are used for anything:

```rust
if !OPERATIVE_OBSERVATION_METHOD.contains(&observation.method.as_str()) {
    return Err(OverrideOutcome::BlockedByUnverifiedObservation(...));
}
```

Three things follow, and each is a property of where the gate sits rather than a rule someone has to
remember:

**It is blocked, not discarded.** The observation is still representable, still carries its facts,
its method and the source identity it claims to describe, and still goes into the fingerprint. What
somebody believed about the source is worth keeping; it is simply not authority. The outcome names
the method, so the report says *why* it is blocked rather than reporting a generic unknown.

**No approval reaches past it.** The gate is at step 3 of `resolve_build_override`; step 5 is where
an approval is first read. Approving a hand-entered observation is possible — the fingerprint
recomputes fine — and it changes nothing.

**The expectation comparison is not even attempted.** Comparing the record's selector against
hand-entered facts would be theatre: two claims agreeing with each other is not a source, and
reporting the agreement would read like a confirmation. The gate runs first.

### What this is not

**Not a rule about who ran the tool.** A person who opens a file and has the parser read its header
has produced a `header-read`, and it is operative. The method names *where the fact came from*, not
who initiated the inspection. Recording such a read as `operator-entered` would be mislabelling it,
and nothing here asks for or rewards that. The test asserts every operative method still reaches
`ApprovedMapping`.

**Not a verification workflow.** No UI, no flow, no route by which a hand-entered fact could be
promoted to a verified one. The outcome says the observation is unverified, and stops. Defining what
verification would mean is a separate question and is not opened here.

---

## 3. The test

`a_hand_entered_observation_is_kept_and_is_not_authority`, built exactly as the package specifies:

1. Start from a **genuinely approved** synthetic mapping — a `header-read` observation of a 512x128
   banner, a record that agrees with it, an approval bound to both. Asserted `ApprovedMapping` first,
   so the mutation below is the only thing that moved.
2. Change **only** `method`, to `operator-entered`. Same source, same digest, same facts, same
   record.
3. Recompute the approval so it genuinely matches — asserted against `override_fingerprint`, because
   the method is inside the v2 construction and a stale approval would prove nothing about authority.
4. Assert `BlockedByUnverifiedObservation`, naming the method.
5. Remove the approval entirely and assert the same outcome, which is what makes "approval cannot
   elevate it" structural rather than a policy check.
6. Loop over `OPERATIVE_OBSERVATION_METHOD` and assert each still reaches `ApprovedMapping`.
7. Assert the vocabulary relationship itself: `operator-entered` is in `OBSERVATION_METHOD`, is not
   in `OPERATIVE_OBSERVATION_METHOD`, and the operative set is a subset of the full one.

**Retained coverage**, unchanged and still passing:

| Case | Test |
|---|---|
| Positive: verified observations select, and one target answers differently per form | `one_target_supports_several_variants_and_a_choice_is_checked_against_the_selected_one` |
| Missing facts, wrong source identity, partial reads | `a_missing_or_misattributed_observation_blocks_rather_than_letting_the_record_decide` |
| Contradictory expectations, with a matching approval | `a_source_matching_one_variant_cannot_obtain_another_variants_approved_mapping` |
| Ambiguous selection, and no-match reported as such | `an_ambiguous_source_is_refused_and_a_matching_none_is_reported_as_such` |
| Both halves of the approval binding | `a_caller_cannot_select_a_convenient_variant_to_bypass_a_constraint` |

---

## 4. Provenance bridges, stated accurately

R1 wrote that a bridge "is **evidence**, not a label". That reads as a stronger guarantee than the
code gives, and the representation and its structural checks are unchanged — what changed is that the
limit is now written down, on `ProvenanceBridge` and on `validate_bridge`.

What validation checks, all structural: a `method` in `BRIDGE_METHOD`; an `original_corpus` that is a
declared and genuinely attributable corpus on the same target; a non-empty `provenance`; no
self-bridge; no bridge onto a corpus that already claims attribution.

What it does **not** do:

* open, read or verify anything the citation points at — a manifest that says what it should and a
  manifest that says nothing are the same string here;
* check that the extracted files are the members that manifest names, or that a byte-identity claim
  holds;
* establish **runtime lookup precedence**. `[runtime_lookup]` is a separate block and a bridge does
  not touch it. Where a file sat inside an archive and which copy the runtime prefers are different
  questions, and the first answers nothing about the second. The bridge test now asserts this
  directly: a valid bridge reaches `[discovery]` and leaves `[runtime_lookup]` unknown.

So a valid bridge means a tie has been **declared** in the form the schema requires. Populating a
real one requires separately reviewed evidence linking that specific source corpus to that specific
extracted material. **No real bridge is authorized by this package, and none exists in the
repository.**

---

## 5. Decisions recorded

**Fingerprint v2 invalidation — approved by Codex.** A change to the fingerprint construction
invalidates an approval carrying another version, and never reinterprets it under today's rules.
There are no approvals outside the fixtures, so the cost is zero. Recorded in c0-06 §10.7.

**Operator entry is non-operative.** Recorded on `OPERATIVE_OBSERVATION_METHOD`, in c0-06 §10.7, and
as a labelled correction in the R1 receipt.

**R1's receipt is corrected in place.** Its question 1 said `operator-entered` was admitted "with the
same standing as a header read". That sentence is struck through and followed by a labelled
**CORRECTION** block giving the answer and pointing here. Questions 2 and 3 carry **ANSWERED** blocks
for the fingerprint decision and the bridge clarification. The original text is left visible: the
statement described what R1 shipped, and the record should show what was shipped and what was then
corrected.

---

## 6. Files changed

| File | Change |
|---|---|
| `crates/tdb-canonical/src/build_override.rs` | `OPERATIVE_OBSERVATION_METHOD`; `OverrideOutcome::BlockedByUnverifiedObservation`; the gate in `resolve_variant`; resolver and vocabulary docs |
| `crates/tdb-canonical/src/lib.rs` | re-export |
| `crates/tdb-canonical/src/build_override_tests.rs` | `current_observing_by` helper; `a_hand_entered_observation_is_kept_and_is_not_authority` |
| `crates/tdb-canonical/src/asset.rs` | `ProvenanceBridge` and `validate_bridge` docs: declaration vs evidence, and no runtime lookup claim |
| `crates/tdb-canonical/src/asset_tests.rs` | bridge test asserts `[runtime_lookup]` stays unknown |
| `docs/validation/c0-06-…md` | seventh amendment: header, new §10.7 |
| `docs/receipts/c0-as3-r1-source-backed-variants.md` | labelled correction to question 1; answers to questions 2 and 3 |

**Not changed:** no schema block, no validation rule (still **64**), no vocabulary term removed, no
profile, no constraint, no population figure. `OBSERVATION_METHOD` is unchanged.

## 7. Checks

```
cargo test -p tdb-canonical          191 passed; 0 failed   (190 -> 191)
cargo test -p tdb-qa                 215 passed; 0 failed
cargo clippy -p tdb-canonical --all-targets -- -D warnings    clean
cargo fmt --all -- --check           clean (working tree)
```

`cargo fmt` clean refers to the **working tree**; the repo-wide `fmt`/`clippy` baseline defect at
HEAD recorded in CLAUDE.md is untouched.

## 8. What is not claimed

No build provenance, no installation compatibility, no game acceptance, no converter. No target
reaches `supported-target-constraint`, so nothing can be **refused** a value on the strength of a
real target. `patch.big` remains unopened and `[runtime_lookup]` unknown on every target. No bridge
is populated anywhere, and a populated one would still say nothing about what the runtime loads.

**Nothing staged, committed or pushed. Stopping for review.**
