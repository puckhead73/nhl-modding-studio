# C0-AS3 — Scoped asset evidence and correlated variants

> **Package:** C0-AS3 — represent C0-POR1's findings at the scope that evidence actually has,
> without turning specimen or corpus observations into universal target requirements and without
> flattening distinct variants. **Date:** 2026-09-08. **Author role:** developer. Reviewer: Codex
> (architect).
>
> **Nothing measured.** No game-data read, no archive investigation, no `patch.big` inspection, no
> runtime test, no fixture acquired, no new specimen. Every fact below was already in the preserved
> C0-POR1 / C0-POR1-R1 records and their census; this package changed how it is **represented**, not
> what is known.
>
> **Nothing staged, committed or pushed.** No live publication, no installation, no QA generator run,
> no `capability.csv` touched, no `anchors/` read. No support level and no game acceptance is
> promoted by anything here.

---

## 1. The problem, in one paragraph

C0-POR1 measured three private copies exhaustively and promoted fourteen constraint blocks.
C0-POR1-R1 cut that to three. The review was right, but the schema was complicit: `asset-output-profile/2`
had **one** evidence axis — `measured` / `derived` / `asserted` — which says how well something was
measured and nothing about *which population* the measurement covers or whether that population is
attributable to the target. It held **one** dimension per target, so a target carrying three portrait
forms had to record none of them. And it bundled `{codec, color_space, channels, alpha}` into one
value, so an unmeasurable colour space blocked the codec — which is precisely why C0-POR1 filled it
with a guess.

Every one of R1's eleven corrections had to be enforced by a reviewer reading prose. `/3` makes the
distinctions structural, so the same mistake is refused by the validator rather than caught twice.

---

## 2. `asset-output-profile/3`

### 2.1 Evidence scope, separate from confidence

Two new keys on every block, orthogonal to `confidence`:

| Key | Vocabulary | Meaning |
|---|---|---|
| `scope` | `specimen-observation`, `inspected-corpus-invariant`, `supported-target-constraint` | how far the evidence reaches |
| `force` | `observed-source-format`, `target-required` | what the constraint asserts about **other** values |

`asset-evidence-scope-missing` refuses an established block that states neither.
`asset-evidence-scope-insufficient` refuses two things: a `specimen-observation` promoted to an
established block (C0-POR1's `[tiling]`, now a schema refusal rather than a review finding), and a
`target-required` force resting on anything weaker than a supported target constraint — because
reading the bytes one copy carries establishes what the source **is**, never what the target
**rejects**.

**The consequence for a Build is the point.** `resolve_build_override` now distinguishes:

* `RefusedByConstraint` — the target is known to require something else;
* `BlockedByUnprovenConstraint` — the constraint is established as the observed source format only.
  Reproducing the observed value is authorised; departing from it is **blocked**, because nothing
  shows the target would reject it *or* accept it.

The second outcome is what "an observed source format may inform source-preserving planning, but does
not prove alternative formats are forbidden or supported" looks like in code. Under `/2` there was
only a refusal, which quietly promoted "every file we inspected is RefPack" into "the game rejects
anything else".

### 2.2 Corpus identity and attribution

`[[target_profile.corpus]]` records what a body of inspected files **is**: its kind, whether it is
attributable to the target, whether the build behind it was identified, how its own identity was read
(a container header, an SFO, a XEX — never a folder label), how many files were inspected, which
neutral groups the inspection distinguished, and what it cannot support.

Every established block names the corpus it rests on. Three rules enforce the rest:

* `asset-corpus-attribution-unsupported` — a `derived-tree` or `extracted-tree` cannot claim to be
  attributable, and an established block cannot rest on an unattributable corpus. This is R1's
  central finding, now mechanical;
* `asset-corpus-undeclared` — an established block that names no corpus, or names one the target does
  not declare;
* `asset-corpus-incomplete` — no inspected files, no citation, an unidentified build with no stated
  limitation, or a duplicate id.

Build-identification limits are preserved rather than implied: all three portrait corpora carry
`build_identification = "incomplete"` with the limitations spelled out, including that the PS3 copy
carries third-party writes and that `patch.big` was never opened.

### 2.3 Migrating POR1's container/compression findings to their actual scope

`container` and `compression` stay established on `nhllegacy-ps3` — they are genuine
inspected-corpus invariants holding across the backup-labelled files too — and now say explicitly
that they are **observed source formats** over a corpus whose build is unidentified. Everything on
the Xbox 360 and PS4 targets stays a candidate, and the schema now refuses to let it be anything
else.

### 2.4 `atlas = none` → unknown, with the measured fact preserved separately

`AtlasValue.kind` is optional. `sharing` (`per-asset-surface` / `shared-surface`) is required and is
exactly what counting one texture payload per container measures. C0-POR1 read `kind = "none"` off
that count; whether the family is an atlas, a pool or neither is a further question about page
geometry and slot addressing that the count does not answer.

So `[atlas]` is established on `sharing`, `kind` is unknown with `[atlas.property.kind]` recording
why, and `replace` — which requires `atlas.kind` — stays blocked. `resolve_build_override` refuses an
`atlas-slot` proposal outright while the kind is unknown: a slot is a coordinate in a scheme, and
without the kind there is no scheme.

---

## 3. Correlated variants

`[[target_profile.variant]]` carries the four correlated blocks — `raster` (codec **and** colour
format), `dimensions`, `mipmaps`, `tiling` — together, selected by structured clauses over
`path-family`, `bucket`, `numeric-id-range` or `stored-dimensions`.

**There is no per-target list of allowed sizes and no per-target list of allowed codecs**, so an
unobserved pairing is not spellable. `asset-variant-block-conflict` refuses a target that decides the
geometry at target level *and* declares variants, so a caller can never read a value no variant
carries. The container chain is deliberately **not** per-variant: one chain holds every portrait form,
and repeating a corpus-wide invariant three times invites three copies to drift.

### 3.1 Selection is deterministic, and refuses rather than guesses

A clause a query cannot decide is **undecidable**, not a wildcard. A variant matches only when every
clause matches; one mismatch rules it out; any undecidable clause leaves the whole variant live.

| Definite matches | Undecidable | Result |
|---|---|---|
| 1 | 0 | `Selected` |
| 0 | 0 | `NoMatch` — explicit, never a fallback |
| anything else | | `Ambiguous` — a **refusal** naming them |

Declaration order never enters the decision, nothing is preferred for being more specific or better
populated, and a test asserts that reversing the variant list changes no answer.

**This matters on the real profile.** `playerheads` holds 512x512 and 512x256, and nothing observed
separates them by path — so a query giving only the path family is **refused, naming both**. Supplying
the stored size resolves it, which is exactly the source-preserving case: a Build planning a
replacement knows the size of the asset it is replacing.

`asset-variant-selector-incomplete` refuses a variant nothing can select and a clause missing its
operands; `asset-variant-selector-contradiction` refuses a duplicated dimension, a selector naming
dimensions the variant does not store, and two variants nothing can tell apart.

### 3.2 Impossible cross-variant combinations

Because dimensions and codec sit inside one variant, a size the variant's codec cannot store is
caught at authoring time by the existing block-alignment rule — now variant-local. A test covers both
that and a selector naming a size the variant's own `[dimensions]` contradicts.

### 3.3 Neither backup filenames nor mip counts prove provenance

Nothing in the profile or the schema treats them as evidence. The PS3 corpus declares its groups as
`current` / `backup-labelled` with a limitation recording that the label is a filename and nothing
more, and `[mipmaps]` stays unknown on every variant with the circularity spelled out.

---

## 4. Partially known properties

`RasterValue.color_space` and `.alpha` are optional; `AtlasValue.kind` is optional. Absence means
unknown, and `[<block>.property.<name>]` carries that unknown's own reason, scope, citation and
non-operative proposal. `asset-property-evidence-conflict` refuses a table for a property that is
known, for one the block cannot leave unknown, or with no reason.

`[[operation]].requires` accepts `<block>.<property>`, and readiness reports the missing requirement
in the form it was declared. So on `nhllegacy-ps3`:

* `preview` requires `raster.codec` — the unmeasurable colour space does not block it;
* `replace` requires `raster.color_space` and `atlas.kind` — and is blocked by both, by name.

Under `/2` the unknown colour space made the whole `raster` block unknown and blocked both, which is
what made filling it in tempting.

`mipmaps.filter` and `tiling.alignment_bytes` are **not** in this set. They were always optional
refinements whose absence blocks nothing; treating them as gaps would have made "requires `tiling`"
unsatisfiable everywhere for no evidential reason.

---

## 5. Observed paths, without an invented lookup rule

`[[target_profile.path_family]]` — several per target — records template, tokens, case, and
`bucket_resolution`. `[[observed_bucket]]` records which ids were **seen** in which directory, citing
the corpus it was seen in.

The split is enforced: `asset-path-family-unresolved` refuses `[path_rule]` established through a
family whose `bucket_resolution` is `unknown`, refuses `template-only` on a family that observes
buckets its template cannot produce, and refuses `derived-directory` outright — because `PATH_TOKEN`
has no token for a directory computed from an id, so labelling a rule would not record one.

C0-POR1 wrote that the id→bucket rule "is not the literal folder-name ranges" and treated that as a
finding; R1 withdrew it because no candidate rule was ever formulated or tested. Nothing here
reinstates it: the `p8001_12000` observation is recorded as membership, in the field for membership,
with a note saying so.

**`[runtime_lookup]` is the new block for the question observed placement was standing in for**: what
the game loads, and what wins when a base and a patch both carry the asset. It is unknown on all three
targets and blocks `discover` and `install` everywhere.

---

## 6. Approval safety

`profile_content_digest` now covers the corpora and their attribution, the path families and their
resolution state, every variant with its selectors and its constraints, each block's `scope`, `force`
and `corpus`, and which sub-properties are unknown. `override_fingerprint` covers the record's
**variant selector**. Tests assert each of those moves the digest, and that the exclusions still hold:
prose and the non-operative candidate channel decide nothing and must not invalidate an approval.

**A caller cannot select a convenient variant to bypass a constraint.** A record carries the
observable *facts* — path family, bucket, numeric id, stored size — and never a variant id. There is
no key for one, and `deny_unknown_fields` makes adding one a **parse** error, which a test asserts.
The profile selects, deterministically; an ambiguous selector is refused. And because the selector is
inside the fingerprint, swapping it for one that lands on a different variant invalidates the
approval rather than carrying it across.

The ordering C0-AS2-R2 established is unchanged and still structural: profile → target → **variant,
then constraint** → context → approval. Steps 3 and 4 return before step 5 exists, so no approval is
read for a refused, blocked or unproven value.

---

## 7. The conservative transition

`upgrade_asset_profile_v2` translates `/2` into `/3`. **Every** block — `established` and
`not-applicable` alike — comes out `unknown`, with its value, confidence and provenance preserved in
the candidate channel and an upgrade note recording the demotion.

Nothing is inferred, because nothing can be: a `/2` "measured" says a measurement happened, not what
population it covered; a `/2` document has no corpus records at all; and attribution is exactly what
decided which C0-POR1 promotions survived review.

`not-applicable` is demoted **deliberately**. A decided absence is operative — it *satisfies* an
operation's requirement — so carrying one across unexamined would let a `/2`-era judgement keep
driving `/3` readiness without ever citing a corpus. Its reason is preserved as prose.

Corpora, path families and variants are never produced: all three are new evidence, not restatements,
and each is reported per target as something the transition cannot supply.

The `/2` document is preserved verbatim at
`crates/tdb-canonical/tests/fixtures/asset-profiles/v2/assets-portraits.toml`, and the transition is
tested against it. Both older schemas now name **their own** transition in the refusal, because
pointing at the wrong one would send an author through a translation that cannot read their document.

---

## 8. Population, from existing evidence only

`defs/adapters/asset-profiles/assets-portraits.toml`, `profile_version = 3`. Counts are
`asset_registry_population`; a test pins them.

| Target | Established | Unknown | Candidates | Variants | Established in variants | Unknown sub-properties | Path families (unresolved) | Corpora (unattributable) |
|---|--:|--:|--:|--:|--:|--:|--:|--:|
| `nhllegacy-ps3` | **3** | 13 | 3 | 3 | **4** | 7 | 2 (2) | 1 (0) |
| `nhllegacy-xbox360` | 0 | 16 | 13 | 3 | 0 | 0 | 2 (2) | 1 (1) |
| `nhl21-ps4` | 0 | 16 | 6 | 1 | 0 | 0 | 1 (1) | 1 (1) |

**Seven established constraints, and the delta from R1's three is entirely representational.**
`container`, `compression` and `atlas` are R1's three, rescoped. The four new ones are `raster` in
each PS3 variant and `dimensions` in one of them — facts R1 already recorded and `/2` could not hold:

* **`raster` per variant.** `DXT5` on all 18 663 files of the PS3 copy, both groups. `/2` could not
  establish it because the same value carried `color_space`, which is not in the `DDS` header at all.
* **`dimensions` on `playerheadssmall-256x128` only.** That variant is selected by **path family**,
  an observable criterion independent of the property being reported, and 256x128 holds across every
  file in that folder including the mip-chain ones. The two `playerheads` variants are selected by
  their **stored size**, so establishing that size there would report the definition as the discovery
  — the circularity R1 caught in `[mipmaps]` — and both stay unknown.

**Every established constraint is `observed-source-format` over an `inspected-corpus-invariant`.**
Nothing reaches `supported-target-constraint`, and a test asserts it: no load, acceptance or
shipping-manifest test was performed on any target, so nothing in the committed registry authorises
refusing a Build anything.

**Every (target, variant, operation) triple remains blocked**, and readiness names the specific
requirement.

The **only** place a `target-required` constraint exists is the labelled synthetic fixture, which
exists so the refusal path has a test without any real profile claiming one. The fixture is excluded
from population.

---

## 9. Tests

`tdb-canonical` **162 → 183**, all passing. (This line originally read "126 → 183"; 126 was the
C0-AS1-era figure rather than the state this package started from. Corrected by C0-AS3-R1.) New coverage, by the package's list:

| Requirement | Cases |
|---|---|
| Multiple variants on one target | `the_committed_profile_carries_correlated_variants_and_no_invented_combination`, `one_target_supports_several_variants_and_a_choice_is_checked_against_the_selected_one` — the same proposal permitted on one variant, blocked on a second and refused on a third |
| Overlapping / no-match selectors | `variant_selection_is_deterministic_and_never_falls_back`, `an_ambiguous_selector_is_refused_and_a_matching_none_is_reported_as_such` — including that declaration order changes no answer and that an empty selector refuses rather than guesses |
| Impossible cross-variant combinations | `an_impossible_cross_variant_combination_is_not_spellable`, `a_target_with_variants_may_not_also_decide_the_geometry_at_target_level`, `a_variant_must_be_selectable_and_its_selector_must_be_coherent` |
| Partial unknown raster fields | `a_raster_block_may_be_established_with_its_unmeasurable_fields_absent`, `an_atlas_may_record_what_a_payload_count_measures_without_naming_a_kind`, `an_unknown_sub_property_blocks_only_the_operations_that_need_it` |
| Observed paths with unknown lookup | `observed_bucket_membership_is_never_an_id_to_directory_rule`, `a_target_may_declare_several_path_families_and_path_rule_names_the_ones_it_uses`, `an_unknown_runtime_lookup_blocks_discovery_and_install_on_every_committed_target` |
| Corpus evidence cannot authorize universal overrides | `corpus_evidence_cannot_authorize_a_universal_constraint_it_does_not_cover`, `an_established_block_must_say_how_far_its_evidence_reaches_and_what_it_asserts`, `an_observed_source_format_authorises_reproducing_it_and_nothing_else` |
| Fingerprint invalidation | `a_scope_force_or_selector_change_in_the_profile_invalidates_an_approval`, `a_caller_cannot_select_a_convenient_variant_to_bypass_a_constraint` |
| Conservative migration | `the_v2_asset_transition_demotes_every_constraint_and_promotes_none`, `a_v2_decided_absence_is_demoted_rather_than_carried_across` |
| Unchanged database behavior | `the_database_half_of_the_registry_is_untouched_by_the_asset_schema_revision` — a committed `target-adapter/2` document still parses and validates unchanged, and still refuses an asset evidence key as unknown |

The updated tripwires — `the_committed_profile_establishes_only_what_its_evidence_scope_supports` and
`no_operation_on_the_committed_profile_is_ready_and_every_gap_is_named` — pin the **explicit** expected
set rather than a total, assert every established block's scope, force and corpus attribution, and
assert that no operation is ready on any variant.

---

## 10. Checks run

| Check | Result |
|---|---|
| `cargo test -p tdb-canonical` | **183 passed, 0 failed** |
| `validate_asset_profile` on the committed profile | **zero findings** |
| `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | clean (strict) |
| `cargo fmt --all -- --check` | clean, repo-wide |
| `cargo test -p tdb-qa` | **215 passed, 0 failed** — the family vocabulary and evidence ledger are unaffected |
| HEAD | `a2bbdc4`, unchanged. Index **empty**; nothing staged, committed or pushed |
| `anchors/`, `docs/audits/**` | not read, not written |
| QA generators | not run |
| Game data | not searched, not read, not measured |

**Scoped, and the scope stated.** `cargo test --all` was **not** run: it carries the known C0.2
baseline failures. `tdb-canonical` is a leaf crate — no crate in the workspace depends on it, checked
against every `Cargo.toml` — and the sole reader of `defs/adapters/asset-profiles/`, so the targeted
runs are sufficient **for the reverse-dependency question**. They are not coverage of the findings
themselves, which are documentary.

**fmt/clippy baseline.** The clean results describe the **working tree**. CLAUDE.md records that both
fail **at HEAD**, because the C0-INT1 repairs live only in the tree. Nothing here repaired or depended
on that; the repo-wide `cargo fmt --all` run touched only files this package edited.

---

## 11. Files changed

| File | Change |
|---|---|
| `crates/tdb-canonical/src/asset.rs` | `asset-output-profile/3`: `AssetConstraint`, `AssetCandidate`, `PropertyEvidence`, `Corpus`, `PathFamily`, `ObservedBucket`, `AssetVariant`, `SelectorClause`, `RuntimeLookupValue`; eight new vocabularies; optional raster/atlas sub-properties; `select_variant`; property-narrowed requirements; the new validators; population fields |
| `crates/tdb-canonical/src/asset_v2.rs` | **new** — the `/2` document shape and the conservative `upgrade_asset_profile_v2` |
| `crates/tdb-canonical/src/build_override.rs` | `VariantSelector` on the proposal, variant resolution before the constraint check, `deny` at the strength `force` supports, `BlockedByUnprovenConstraint` / `NoMatchingVariant` / `RefusedByAmbiguousVariant`, and the widened profile digest and fingerprint |
| `crates/tdb-canonical/src/lib.rs` | schema constants (`/3`, previous `/2`, legacy `/1`), ten rules with their diagnosable names, module and re-exports |
| `crates/tdb-canonical/src/asset_tests.rs` | tripwires re-pinned; ~15 new cases |
| `crates/tdb-canonical/src/build_override_tests.rs` | five new cases; existing cases adapted |
| `defs/adapters/asset-profiles/assets-portraits.toml` | re-authored at `/3` — corpora, path families with observed buckets, variants, scoped constraints, property evidence, `runtime_lookup` |
| `crates/tdb-canonical/tests/fixtures/synthetic-asset-profile.toml` | `/3`; a third target exercising variants, partial values and the one `target-required` constraint in the repository |
| `crates/tdb-canonical/tests/fixtures/asset-profiles/v2/assets-portraits.toml` | **new** — the C0-POR1-R1 `/2` document, preserved verbatim as the migration fixture |
| `crates/tdb-canonical/tests/fixtures/build-overrides/*.toml` | schema, profile version and recomputed digests/fingerprints |
| `docs/validation/c0-06-canonical-project-and-adapters.md` | fifth amendment: §1, §5, §10.1, §10.2, new §10.5, U-A7/U-A10 updated, U-A12/U-A13 added |
| `docs/receipts/c0-as3-scoped-asset-variants.md` | this record |

Unrelated working-tree changes were not touched, and no historical evidence was rewritten: C0-POR1
and C0-POR1-R1 keep their text and their corrections log.

---

## 12. What this package does **not** claim

* **No build provenance.** Build identification is `incomplete` on all three portrait corpora, and
  the schema requires that to be accompanied by stated limitations.
* **No installation compatibility.** `[install]` is unknown on every target. Nothing was written,
  repacked, replaced or installed.
* **No game acceptance.** No runtime was launched. `game-verified` remains unspellable, and no
  operation on a real profile has its constraints met.
* **No new measurement.** Every fact populated here was already in the C0-POR1 / R1 records.

---

## 13. Unknowns and next actions

| # | Unknown | Next action |
|---|---|---|
| 1 | `patch.big` (PS3 `USRDIR`) never opened — blocks `[runtime_lookup]`, and through it `discover` and `install` | Examine it; determine whether it shadows loose portraits. **Explicitly out of scope for this package** |
| 2 | Xbox 360 / PS4 corpora not attributable to their shipped titles | Locate portrait members inside the shipping archives and compare |
| 3 | No PS4 pixel decode; BC7 and `tiling_index 13` unexercised | Decode one GNF specimen and check against a known image |
| 4 | Colour space unmeasured on Gen7, non-uniform on PS4 (0 on 10 246 files, 9 on 175) | Determine whether the 175 are a distinct asset class |
| 5 | `dimensions` circular on both `playerheads` variants | Find an observable discriminator other than the stored size, or accept that only a source-preserving Build can resolve them |
| 6 | The id→bucket function is unidentified, and `PATH_TOKEN` could not express it if it were (**U-A12**) | Establish it from the game's own lookup; then decide the token |
| 7 | No target reaches `supported-target-constraint`, so no real constraint can refuse a Build (**U-A13**) | A load or acceptance test — a separate, unauthorized step |
| 8 | `ubPc.lOnr` binding unexamined on all three targets | Establish with a citation naming these targets |
| 9 | RefPack determinism untested — `deterministic = false` everywhere | Test whether the encoder reproduces EA's bytes |
| 10 | The roadmap's C0 item 7 table still reads *"0 established constraints"* | An architect edit. It has been stale since C0-POR1 and is more stale now |

---

## 14. Questions for the architect

1. **Is the four-block variant set right?** `raster`, `dimensions`, `mipmaps` and `tiling` are
   per-variant; `container` is not, because one chain holds every portrait form and repeating it
   invites drift. A family whose forms genuinely differ in container would need it moved.
2. **Should `[[observed_bucket]]` live in the profile at all?** It is observation rather than
   constraint, and the alternative is to keep it only in the census manifest. It is here because
   without it the two path families read as if their templates resolved.
3. **Is demoting `not-applicable` across the `/2`→`/3` transition the right call?** It is the
   conservative one and §7 argues it, but it does discard a decided absence that a `/2` author may
   have established carefully.
4. Carried forward: P1–P3 undecided; ownership unassigned; 26 units have no adapter; 13 families have
   no profile.

**Stop for Codex review. Nothing staged, committed or pushed.**
