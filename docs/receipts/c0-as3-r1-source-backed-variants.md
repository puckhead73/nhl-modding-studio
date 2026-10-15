# C0-AS3-R1 — source-backed variant selection and scope corrections

**Package:** C0-AS3-R1 · **Date:** 2026-09-08 · **Result:** complete, awaiting review
**Preceded by:** [C0-AS3](c0-as3-scoped-asset-variants.md), [C0-POR1](c0-por1-portrait-measurements.md) + [R1](c0-por1-r1-evidence-scope.md)
**Record amended:** [c0-06 §1, §2.4, §5, §10.2, §10.6](../validation/c0-06-canonical-project-and-adapters.md)

> **Nothing here was measured.** No game data was read, no archive investigated, no `patch.big`
> opened, no runtime launched, no fixture acquired. No constraint on any target was promoted, and
> the two targets that establish nothing still establish nothing. This package corrects four things
> C0-AS3 got wrong about *how* evidence is represented and used, and corrects one stale number.

---

## 1. What was wrong

C0-AS3 built the scope discipline the portrait findings needed, and then made four mistakes with it.

**The resolver believed the record.** `resolve_build_override` selected a variant from
`proposal.variant_selector` — a field in a document somebody wrote. AS3's own test called that field
"the observable facts a Build has about the asset it is planning", which it is not: it is a claim
about them. So a record could reach a different variant's constraints, and a different variant's
approval, by describing its source as that variant. AS3's `a_caller_cannot_select_a_convenient_variant_to_bypass_a_constraint`
tested that a record cannot name a variant *id*, and missed that it could name the variant's
properties instead — which reaches the same place by a longer route.

**Containers were excluded from variants on the strength of one observation.** The portrait targets
happen to share one chain across all their forms. AS3 wrote that into `VARIANT_BLOCK`'s doc comment
as a reason the schema should not be able to say otherwise. A loose file beside an archive member is
an ordinary shape for one family, and no measurement here says it never happens.

**An extraction was treated as evidence of nothing.** AS3 refused *every* established block resting
on an unattributable corpus. That is one step past the C0-POR1-R1 finding. Measuring an extraction
establishes something true about the extraction; what it cannot do is speak for the target, and what
it cannot do at all is recover the placement the extraction discarded. Refusing all three together
discards real measurements of real files.

**Bucket membership sat in the profile as a table of id ranges.** A column reading `p8001_12000 →
8003..12401` is a lookup table whatever the surrounding prose says, and the next reader resolves id
9000 with it. The `bucket_resolution = "unknown"` label a few lines above does not survive contact
with a table that looks like the answer.

---

## 2. Selection is bound to the current source

### The mechanism

`BuildContext` gained an optional `SourceObservation`:

```rust
pub struct SourceObservation {
    pub of: SourceIdentity,     // asset id, revision, hash algorithm, digest
    pub method: String,         // OBSERVATION_METHOD
    pub facts: ObservedFacts,   // path_family, bucket, numeric_id, width, height
}
```

`resolve_variant` now runs `select_variant` over `observation.facts` and over nothing else. Three
gates come first, in this order:

| Gate | Outcome | Why it is a gate and not a warning |
|---|---|---|
| No observation, on a setting whose block is variant-borne | `BlockedByUnobservedSource` | An unobserved source is not a source that matches everything. Which form it is has to be read off it |
| The observation was read from another source | `ObservationNotOfSource(fields)` | Identity, revision, algorithm and digest, exactly as the record's own context is checked. Facts about a previous export describe that export |
| The record's selector states something the source contradicts, **or cannot confirm** | `SelectorExpectationMismatch(fields)` | A record that states a fact is asking to be held to it. Silently dropping an unconfirmable claim would let a record describe its source however it liked as long as nobody looked |

A fact nobody read leaves its clause undecidable, which `select_variant` already reports as
`Ambiguous` — a refusal naming what would have to be decided. Nothing is inferred from the value
being proposed, and there is no vocabulary term for doing so: `OBSERVATION_METHOD` names places a
fact is *read from*.

**The API stays pure.** No filesystem reader was added and none is required. Reading a header is the
caller's job; the result arrives as a plain value.

### Approval

`APPROVAL_FINGERPRINT_VERSION` is **2**. The construction now covers the observed facts, the
observation method, and the identity and digest of the source they were read from. Every version 1
approval is invalidated rather than reinterpreted — which is exactly what the version constant is
for, and is the right answer here: a v1 approval was bound to what a record *claimed* about its
source, and that binding is the one this package removed.

The profile digest gained `[corpus.bridge]`, per-variant containers, and the census reference behind
each observed bucket.

### The test that matters

`a_source_matching_one_variant_cannot_obtain_another_variants_approved_mapping` builds a 512x256
source, a record claiming it is the 512x128 form, and an approval whose fingerprint is **asserted to
be current** before resolution runs. It is refused at step 3, naming
`proposal.variant_selector.height: record 128, source 256`. The same attempt through the path family
gets the same answer, and removing the claim entirely does not help — the source is still the form
whose codec is unknown.

`a_caller_cannot_select_a_convenient_variant_to_bypass_a_constraint` now tests both halves of the
binding: editing the record's expectation, and leaving the record alone while the **source** becomes
the other form.

---

## 3. Containers by variant

`VARIANT_OPTIONAL_BLOCK = ["container"]`. Unlike the correlated four, declaring it in a variant does
**not** force the target-level block to `unknown`:

* the variant decides it if the variant's block is decided;
* otherwise the shared target-level block does;
* deciding it at **both** levels is `asset-variant-block-conflict`.

That is a rule, not a precedence puzzle: there is no case where two decided values compete, because
validation refuses that profile before resolution runs. `block_states` resolves it, the digest covers
it, and `resolve_variant` treats a container-borne setting as variant-borne only when a variant
actually decides one.

**The real profile is unchanged.** `nhllegacy-ps3`'s one chain stays at target level, stated once.
The per-variant case is proved on `fixturefour-synthetic`, added for it: a `loose` form that is a
bare `dds`, and a `packed` form that is a `bigf` archive member containing one — two forms of one
family, two chains, each stated where it holds.

---

## 4. Observation separated from attribution

`OPERATIVE_SCOPE` is unchanged, and it now does the work AS3 was doing with a refusal:

* `AssetConstraint::established()` — is the block decided? Unchanged, and what a bounds check wants.
* `AssetConstraint::operative()` — **new**. Decided *and* scoped to a population that speaks for the
  target. The accessor every Build decision goes through: `permitted_choice` reads it,
  `block_states` reports it, and an operation's requirement is not met without it.

So a `specimen-observation` may now be **established** on an unattributable corpus — a true,
checkable statement about named files — and it decides nothing. Proposing another value against one
is `BlockedByUnprovenConstraint` with the scope named, which is a different report from "nobody
knows" and a reader acts differently on it.

Two things such a corpus still cannot do:

| Refused | Rule | Why |
|---|---|---|
| A `PLACEMENT_BLOCK` (`discovery`, `path_rule`, `install`, `runtime_lookup`) at any scope | `asset-corpus-attribution-unsupported`, or `asset-evidence-scope-insufficient` at specimen scope | Extracting is the step that discarded where the file sat. No more careful reading of the output recovers it |
| Any scope wider than the specimens | `asset-corpus-attribution-unsupported` | A fact about a corpus is a fact about the target only once the corpus is attributable |

`[corpus.bridge]` is the one route past the first: a `BRIDGE_METHOD` naming a record that **survived
the extraction** (a member manifest, or byte-identity against a member still sitting in an
attributable corpus), the id of the attributable corpus it bridges to, and a citation. It is checked
as evidence — `asset-provenance-bridge-invalid` refuses an unknown method, an uncited bridge, a
bridge to itself, a bridge to another unattributable corpus, and one declared on a corpus that
already claims attribution and therefore bridges nothing. **No real corpus in this repository
declares one**, and none was given one here.

**No real promotion was made.** `nhllegacy-xbox360` and `nhl21-ps4` still establish nothing; their
13 and 6 measurements remain non-operative candidates. Re-scoping them is authoring against
evidence, which this package was not asked to do.

---

## 5. Bucket evidence

`ObservedBucket` lost `observed_id_low` / `observed_id_high` and gained `census`, `census_digest` and
`scope`. The membership itself already lived in
`docs/receipts/data/c0-por1-portrait-specimens.json` under
`path_and_id_observations.observed_bucket_membership`, where it is recorded beside the withdrawn
claim about the bucket rule — so nothing was moved, copied or rewritten: the profile now points at
it by digest instead of restating it.

Validation refuses a reference with no usable digest (a reference to whatever a path holds today is
not a reference to an observation) and refuses `scope = "supported-target-constraint"` on membership
(a directory holding an id is never the target requiring it to be there). `bucket_resolution` is
untouched and still `unknown` on both Gen7 families, so `[path_rule]` remains unestablishable.

---

## 6. Migration

The `/2` → `/3` transition still demotes **everything**, `not-applicable` included. What changed is
that the demoted block now carries, in the document rather than only in the transition report:

* what `/2` asserted, in words (`asserted this constraint NOT-APPLICABLE`);
* why it was demoted — missing scope and corpus — with an explicit *"not because the assertion was
  found to be wrong"*;
* the `/2` reason verbatim, or a statement that none was recorded, which is itself part of why it
  could not be carried across.

Demoting is not overruling, and a later author re-deciding the block should not have to rediscover
why somebody wrote it down.

---

## 7. Population, reported so it cannot be misread

`AssetPopulation` gained `operative_constraints`, `specimen_observations`,
`target_required_constraints` and `operative_variant_constraints`.

| Target | Established | Operative | Specimen | **Target-required** | Unknown | Candidates | Variants | Est. in variants |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| `nhllegacy-ps3` | 3 | 3 | 0 | **0** | 13 | 3 | 3 | 4 |
| `nhllegacy-xbox360` | 0 | 0 | 0 | **0** | 16 | 13 | 3 | 0 |
| `nhl21-ps4` | 0 | 0 | 0 | **0** | 16 | 6 | 1 | 0 |

Seven established constraints, all `observed-source-format` over one inspected corpus, and **zero
target requirements** — pinned by `the_population_report_separates_observed_facts_from_target_requirements`.
The only `target-required` constraint in the repository is on the labelled synthetic fixture, which
`asset_registry_population` excludes, so exercising the refusal path cannot inflate a real count.

---

## 8. Tests

`tdb-canonical` **183 → 190**. Every category the package named:

| Asked for | Test |
|---|---|
| A source matching A cannot obtain an approved B mapping, even with a matching fingerprint | `a_source_matching_one_variant_cannot_obtain_another_variants_approved_mapping` |
| Changed facts, absent facts, ambiguity, unchanged valid inputs | `a_missing_or_misattributed_observation_blocks_rather_than_letting_the_record_decide`, `an_ambiguous_source_is_refused_and_a_matching_none_is_reported_as_such`, `one_target_supports_several_variants_and_a_choice_is_checked_against_the_selected_one` |
| Selector context and provenance inside approval invalidation | `a_caller_cannot_select_a_convenient_variant_to_bypass_a_constraint` (both halves) |
| Two variants with different containers; contradictory declaration refused | `two_variants_may_carry_different_containers_and_never_two_answers_to_one_question` |
| Scoped specimen measurement admissible; placement and wider scopes not | `an_unattributable_corpus_supports_a_scoped_specimen_measurement_and_nothing_wider` |
| Bridges checked as evidence, not accepted as labels | `a_provenance_bridge_is_checked_as_evidence_and_not_accepted_as_a_label` |
| Established ≠ operative | `an_established_specimen_observation_satisfies_no_operation_and_decides_no_build`, and the rewritten half of `an_established_block_must_say_how_far_its_evidence_reaches_and_what_it_asserts` |
| Census reference, and membership that is never a target requirement | `observed_bucket_membership_is_never_an_id_to_directory_rule` |
| Conservative migration preserving the original assertion | `a_v2_decided_absence_is_demoted_rather_than_carried_across` (extended) |
| Population separated | `the_population_report_separates_observed_facts_from_target_requirements` |
| Unchanged database behaviour | `the_database_half_of_the_registry_is_untouched_by_the_asset_schema_revision` |

## 9. Checks

```
cargo test -p tdb-canonical          190 passed; 0 failed
cargo test -p tdb-qa                 215 passed; 0 failed
cargo clippy -p tdb-canonical --all-targets -- -D warnings    clean
cargo fmt --all -- --check           clean (working tree)
```

`cargo fmt` clean refers to the **working tree**; the repo-wide `fmt`/`clippy` baseline defect at
HEAD recorded in CLAUDE.md is untouched and was not repaired opportunistically.

## 10. Files changed

| File | Change |
|---|---|
| `crates/tdb-canonical/src/asset.rs` | `operative()`/`is_operative()`; `PLACEMENT_BLOCK`, `VARIANT_OPTIONAL_BLOCK`, `BRIDGE_METHOD`; `ProvenanceBridge` and `CorpusStanding`; `ObservedBucket` reshaped; per-variant `container`; specimen scope admitted; `BlockState.operative`/`.target_required`; population split |
| `crates/tdb-canonical/src/build_override.rs` | `SourceObservation`/`ObservedFacts`/`OBSERVATION_METHOD`; `BuildContext.observation` + constructors; three new outcomes; `resolve_variant` rewritten; `observation_drift`, `selector_expectation_drift`, `observed_query`, `not_operative`; fingerprint v2; digest additions |
| `crates/tdb-canonical/src/asset_v2.rs` | `not-applicable` demotion preserves the assertion and reason in the document |
| `crates/tdb-canonical/src/lib.rs` | `Rule::AssetProvenanceBridgeInvalid` (63 → 64); re-exports |
| `crates/tdb-canonical/src/asset_tests.rs`, `build_override_tests.rs` | rewritten variant cases, seven new tests, helper rework |
| `crates/tdb-canonical/tests/fixtures/synthetic-asset-profile.toml` | `fixturefour-synthetic`; bucket references |
| `crates/tdb-canonical/tests/fixtures/build-overrides/*.toml` | new profile digest, new fingerprints, `fingerprint_version = 2` |
| `defs/adapters/asset-profiles/assets-portraits.toml` | 16 observed buckets now reference the census by digest; two notes |
| `docs/validation/c0-06-…md` | sixth amendment: header, §2.4, §5, §10.2, new §10.6 |
| `docs/plans/project-completion-roadmap.md` | C0 item 7 row corrected (authorized here) |
| `docs/receipts/c0-as3-scoped-asset-variants.md` | the "126 → 183" baseline corrected to 162, with a note saying so |

## 11. What is not claimed

No build provenance, no installation compatibility, no game acceptance, no converter. No target
reaches `supported-target-constraint`, so no Build can be **refused** a value on the strength of a
real target — the refusal path is exercised only against the labelled synthetic fixture. A
`specimen-observation` is now representable and no real profile carries one. `patch.big` remains
unopened and `[runtime_lookup]` unknown everywhere, so `discover` and `install` stay blocked on every
target.

## 12. Questions for the architect

1. **`OBSERVATION_METHOD` includes `operator-entered`.** ~~That admits a human-supplied fact into
   selection, with the same standing as a header read.~~ It is honest about how a Build would
   actually work when a header cannot be parsed, and it is also the one term that lets a claim back
   in through the door this package closed. Should it stay?

   > **CORRECTION (C0-AS3-R2, 2026-09-08).** The struck sentence described what R1 shipped, and what
   > it shipped was wrong. `operator-entered` did carry the same standing as a header read, and
   > should not have: R1's whole point was moving selection off claims about the source and onto
   > readings of it, and a typed-in fact is a claim. **Answer: the term stays and its authority
   > goes.** `OPERATIVE_OBSERVATION_METHOD` excludes it, variant resolution that would rely on one
   > returns `BlockedByUnverifiedObservation`, and because that gate is at step 3 no approval
   > reaches past it. The observation itself is still representable with its facts and provenance —
   > it is kept, not believed. See [c0-as3-r2](c0-as3-r2-observation-authority.md).

2. **Fingerprint version 2 invalidates every existing approval.** There are none outside the
   fixtures, so the cost is zero today. Confirming the principle — construction change means
   invalidate, never re-read — is worth doing while it is free.

   > **ANSWERED (C0-AS3-R2, 2026-09-08).** Codex approves invalidating all v1 approvals. The
   > principle stands: a fingerprint construction change invalidates, and never reinterprets, an
   > approval carrying another version.
3. **The bridge is representable and unused.** It exists so the rule can be stated and tested rather
   than left as prose. If you would rather the schema not carry an unexercised route to a placement
   fact, it can be removed and the refusal made unconditional.

   > **ANSWERED (C0-AS3-R2, 2026-09-08).** The representation and its structural checks stay. What
   > was under-stated is now written down: validating a bridge checks the **declaration**, not the
   > evidence — nothing opens the cited record, tests the tie, or says anything about runtime lookup
   > precedence. Populating a real bridge needs separately reviewed evidence linking that specific
   > source corpus to that specific extracted material, and no real bridge is authorized.

**Nothing staged, committed or pushed. Stopping for review.**
