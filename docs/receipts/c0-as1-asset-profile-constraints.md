# C0-AS1 — Evidence-qualified asset output profiles

> **Package:** C0-AS1, the **asset** half of roadmap C0 item 7 (Q07a.1). **Date:** 2026-09-08.
> **Author role:** developer. Reviewer: Codex (architect).
> **Nothing staged, committed or pushed.** No asset editor, no conversion engine, no archive write,
> no UI, no Build engine. No fixture measured or acquired, no evidence publication changed, no
> additional target or family populated. P1–P4 undecided, P4 (asset overrides) explicitly so. The
> reviewed C0-CAN1-R2 implementation and every supported database constraint are preserved.

---

## 0. First deliverable — the correction to CAN1-R2's receipt

Appended, labelled, in place, changing no adapter fact and performing no measurement:
[c0-can1-r2 § *Correction — appended 2026-09-08 by package C0-AS1*](c0-can1-r2-target-specific-derivations.md#correction--appended-2026-09-08-by-package-c0-as1).

It corrects the **scope estimate** in R2's §10 question 1 — *"a single named case probing
`ubPc.vjla` … would promote roughly a dozen blocks across both adapters"* — on three counts:

| # | Correction |
|---|---|
| C1 | **Position evidence does not establish handedness.** `player.shoots` is `ubPc.GRkp`, a different field with a different Legacy-anchored vocabulary. Measuring the position field says nothing about what `GRkp`'s codes mean. The two became candidates for the same *reason*, not from a shared *premise* |
| C2 | **NHL 10 companion semantics need separate evidence.** R2 §2 says this correctly and question 1 contradicted it: NHL 10's rule is `asserted` because the roster-relationships audit's stated range excludes the title, so cardinality, mandatory presence and the skater/goalie condition are unproven there **independently** of the discriminator. Only NHL 15's rule promotes with it |
| C3 | **One anchor does not establish both targets without a proven equivalence.** The anchors manifest's bridge (`PS3 = Xbox360 databases are byte-identical`) is **per-game, cross-platform**. It carries a fact from `nhl10-ps3` to `nhl10-xbox360` and nothing at all between NHL 10 and NHL 15 — the very axis this adapter pair exists to show diverges |

The honest scope survives: a probe on **one** target promotes that target's `[position_discriminator]`,
that target's ratings `[mapping.applicability]` predicates, and — NHL 15 only — its
`[[companion_rule]]`. [c0-06 §9.3 row 7](../validation/c0-06-canonical-project-and-adapters.md)
carried the same over-statement and is amended with a pointer.

---

## 1. Checkpoint 1 — representation inventory

The roadmap's asset sentence enumerates the requirements; `asset-output-profile/1` represented them
as seven free-text strings plus one target-wide evidence label. Mapping requirement → v1 → v2:

| Q07a.1 asset requirement | `asset-output-profile/1` | `asset-output-profile/2` |
|---|---|---|
| discovery rules | **absent** | `[discovery]` (scope, root, recursion) |
| identity rules | **absent** | `[identity]` (key, source, ambiguity policy) |
| archives | `container_chain` prose | `[container].value.layer[].format` |
| indexes | **absent** | `[container].value.layer[].index` |
| paths | **absent** | `[path_rule].value.template` |
| slugs | **absent** | `[path_rule].value.tokens = "player-slug"` |
| IDs | **absent** | `[path_rule].value.tokens = "numeric-id"/"team-id"` |
| raster formats | `raster_codec` prose | `[raster].value.codec` |
| dimensions | `dimensions` prose | `[dimensions].value.width`/`height`/`variant` |
| channels | **absent** | `[raster].value.channels` + `alpha` |
| mipmaps | `mipmaps` prose | `[mipmaps].value.policy`/`count`/`filter` |
| compression | conflated into `container_chain` | `[compression].value.payload` + `deterministic` |
| swizzle/tiling | `swizzle` prose | `[tiling].value.kind` + `alignment_bytes` |
| atlases/pools | `atlas` prose | `[atlas].value.kind` + page/cell/slot |
| mesh vertex layouts | **absent** | `[mesh_layout].value.vertex_attribute[]` + stride/index/winding/coords/unit |
| UV conventions | **absent** | `[uv].value.origin`/`wrap`/`channel_count` |
| rigs/bones | **absent** | `[rig].value.bone_count` |
| weights | **absent** | `[rig].value.max_weights_per_vertex` + quantization |
| palettes | **absent** | `[rig].value.palette_size` |
| LODs | **absent** | `[mesh_layout].value.lod_levels` |
| audio banks | **absent** | `[audio].value.bank` |
| audio codecs | **absent** | `[audio].value.codec` |
| audio sample constraints | **absent** | `[audio].value.sample_rate_hz`/`channels`/`bit_depth`/`max_duration_ms` |
| repack rules | conflated into `install_rule` | `[install].value.repack` |
| install rules | `install_rule` prose | `[install].value.kind`/`size_policy`/`backup`/`updates_references` |
| provenance / confidence / qualification | **one label for the whole target** | per block: state + confidence + provenance + derivation + candidate |

**16 of 26 had no representation at all; 9 were undifferentiated prose; 1 was a target-wide label.**
`tdb_canonical::Q07A1_ASSET_REPRESENTATION` is the machine-readable form, asserted against a parsed
document by a test.

**Representation completeness is not target knowledge.** §4 states the population separately, and
it is zero.

## 2. Checkpoint 2 — schema and validation

`asset-output-profile/2`: fifteen `Constraint<T>` blocks per target, reusing the *same*
`Constraint`/`Candidate` types as `target-adapter/2` — not a parallel copy — so every principle the
architect accepted for the database half holds here by construction:

- `unknown` ≠ `not-applicable`; an absence must record how it was determined;
- an established block is `measured` or `derived`, never `asserted` (`asserted-as-operative`);
- a candidate is legal only under `unknown`, must carry value + confidence + citation, and
  `established()` cannot see it, so it drives no derivation and satisfies no operation requirement;
- a `derived` block must record premises, their applicability to **this** target, and what the
  argument does not establish (`derivation-unexplained`);
- cross-title *and* cross-family similarity are analogies, not derivations — §4 applies this to the
  one place the repository tempted it.

**Typed structures, not descriptive strings.** Twenty-eight closed vocabularies replace the free
text: asset class, interchange, retention, discovery scope, identity key, ambiguity policy,
container format, index kind, path token/case, raster codec, colour space, channel, alpha mode, mip
policy/filter, payload codec, tiling kind, atlas kind, vertex semantic, component type, index
format, winding, coordinate system, mesh unit, UV origin/wrap, weight quantization, bind pose, audio
bank/codec, loop metadata, install kind, size policy, repack rule, backup rule and operation. Every
struct is `deny_unknown_fields`: a misspelled key is a **parse error**.

### 2.1 Rules added (46 → 53)

| Rule | What it prevents |
|---|---|
| `unknown-asset-family` | registering a profile against a coverage axis the QA ledger does not have |
| `duplicate-target-profile` | two profiles for one target in a family |
| `duplicate-source-role` | two canonical sources sharing a role |
| `asset-bounds-inconsistent` | zero extents; a `power_of_two` claim that is not; block-compressed dimensions that are not whole blocks; a mip count a surface cannot have; an atlas cell outside its page; a vertex attribute past its stride; a duplicate attribute; a palette larger than its rig; >8 weights; a channel set contradicting its alpha mode or codec; audio channels/rate/depth out of range; an install kind contradicting its size policy or repacking a loose file |
| `asset-operands-incomplete` | an atlas with no cell, a pool with no slot, `fixed-count` with no count, an empty container chain, a path template and token list disagreeing **in either direction**, an operation requiring nothing, a profile with no canonical source |
| `asset-constraint-dependency-unmet` | an archive install rule with no established `[container]`; a slug path with no `player-slug` identity; more UV sets than the vertex layout carries; more skin weights than it stores |
| `duplicate-operation-declaration` | two requirement sets for one operation |

Reused verbatim on asset blocks: `established-without-value`, `established-without-confidence`,
`value-without-established-state`, `not-applicable-without-reason`, `missing-provenance`,
`asserted-as-operative`, `candidate-on-decided-constraint`, `candidate-incomplete`,
`derivation-unexplained`, `placeholder-as-value`, `unknown-vocabulary-term`,
`constraint-kind-mismatch`, `unsupported-schema`, `unsupported-version`, `malformed-target`,
`adapter-claims-game-verified`.

**References, contradictions, bounds, versions and unknown keys** are all covered: internal
references (path tokens ↔ identity, install ↔ container, uv/rig ↔ mesh layout, operation ↔ block)
resolve or are reported; contradictions between two established blocks are refused; invalid bounds
are refused; the previous schema id is refused **in place** with the transition named; and unknown
keys never parse.

### 2.2 Format capability kept apart from converter capability — structurally

A v2 target profile has **no key** in which a support level, an implementation status or an
operation result could be written. v1's per-target `qualification` and `confidence` are simply gone,
and because every struct is `deny_unknown_fields`, writing one is a **parse error** — asserted by
`a_converter_status_key_cannot_even_be_written_in_a_v2_profile`, which also refuses `supported`.

What a profile *may* say is which format constraints an operation would need:

```toml
[[target_profile.operation]]
operation = "replace"
requires = ["container", "raster", "dimensions", "tiling", "mipmaps", "compression", "atlas"]
```

`operation_constraint_readiness` reports the unknown ones **by name**. Its two variants are
`ConstraintsEstablished` and `BlockedByUnknownConstraints`; neither mentions Studio, and
`readiness_never_states_that_studio_can_perform_an_operation` asserts the type never renders as a
capability claim — so adding a "supported" variant later has to break a test deliberately. A
`not-applicable` block **satisfies** a requirement (a decided absence is actionable); an `unknown`
never does; a candidate never does.

**A valid profile therefore cannot imply Studio can perform its operations**, and the committed one
demonstrates it: it validates clean while every one of its operations is blocked.

### 2.3 Transition from `asset-output-profile/1`

`upgrade_asset_profile_v1` is conservative in the strongest sense available: **every** v2 block
comes out `unknown` with **no candidate manufactured** — not even `swizzle = "xbox-tiled"`, which
resembles a v2 vocabulary term and is the single most likely thing to be waved through. All ten
populated v1 fields are returned as notes naming the v2 block they would belong to; `qualification`
is reported as having no v2 destination *at all*, because an implementation level describes Studio,
not the target format. Only `[canonical_source]` carries across, and only when both its `kind` and
`format` are expressible — otherwise it is dropped and the resulting profile is correctly refused
for declaring no source. The v1 bytes are preserved verbatim at
`crates/tdb-canonical/tests/fixtures/asset-profiles/v1/assets-portraits.toml`
(sha256 `a436865c011cac57b94d44e0beafd1bbdeb818f24d757ae99db313f06d263a8e`, identical to the
pre-C0-AS1 committed file).

## 3. Checkpoint 3 — examples and tests

**The one real profile was re-authored from the same repository evidence, and every fact in it went
down.** `defs/adapters/asset-profiles/assets-portraits.toml` cites exactly one source:
`docs/audits/data/assets/nhllegacy/xbox360/capability.csv`. That CSV is an **operations** record —
its note says Studio's ported pipeline wrote a `p{id}.big` that re-decoded with its gradient intact.
That is evidence about a round trip Studio performed on an already-extracted tree; it is not a
measurement of a shipped EA portrait and states no format requirement. Under the R1 rule it is an
`asserted` source, so the five facts derivable from it are **candidates**:

| Block | Candidate | Why it is not established |
|---|---|---|
| `[container]` | `bigf`(name-table) → `xpr2`(none) | decomposed from the prose `BIGF->refpack->XPR2/RX2`; the decomposition is an interpretation, and the CSV's `XPR2/RX2` is an alternation this profile cannot resolve |
| `[path_rule]` | `p{numeric-id}.big` | the artefact the pipeline *produced*, not a path measured in an installed tree; says nothing about its directory |
| `[raster]` | `dxt5` | the CSV says `DXT` with no variant. DXT5 comes from the **logos** row of the same CSV — a neighbouring family is an analogy, not a bridge |
| `[compression]` | `refpack` | from the same chain; whether Studio's encoder reproduces EA's bytes is unrecorded, so `deterministic = false` rather than assumed |
| `[tiling]` | `xbox360-tiled` | v1's `xbox-tiled` string, never checked against a vocabulary; the alignment is unrecorded and omitted rather than guessed |

The other ten blocks are plain `unknown`, each with a note saying what is missing. `nhl21-ps4`
records **no candidate at all**: no source to cite, and a proposal with no citation is not a
proposal. Nothing was fabricated for any other target or family, and no family was qualified from a
tested variant.

**Synthetic fixture.** `crates/tdb-canonical/tests/fixtures/synthetic-asset-profile.toml`
(`synthetic = true`, targets `fixtureone-synthetic` / `fixturetwo-synthetic`, outside
`defs/adapters/`) exercises what no real evidence can: mesh vertex layouts, UV conventions, rigs and
palettes, audio bank/sample constraints, an atlas **and** a pool, a `fixed-count` mip policy, tiling
alignment, `not-applicable` with its reason, and a candidate on an asset block.
`asset_registry_population` excludes synthetic profiles, and a test asserts no file under
`defs/adapters/asset-profiles/` carries the flag — exercising a representation cannot inflate
coverage by a row.

**Tests: 84 → 126** (43 added, 1 replaced). Each new representation has a positive case and each new
rule a negative one; the notable ones:

| Requirement | Test |
|---|---|
| the committed profile promotes nothing it cannot cite | `the_committed_profile_promotes_nothing_it_cannot_cite` — 0 established on both targets; the chain survives as a candidate; PS4 has none |
| a valid profile implies no capability | `every_operation_on_the_committed_profile_is_blocked_by_named_unknowns`, `readiness_never_states_that_studio_can_perform_an_operation` |
| converter status is unwritable | `a_converter_status_key_cannot_even_be_written_in_a_v2_profile` |
| unknown ≠ not-applicable ≠ candidate | `a_decided_absence_satisfies_a_requirement_but_an_unknown_never_does` |
| class applicability | `a_block_belonging_to_an_undeclared_asset_class_is_refused` |
| every dependency rule | `a_slug_path_cannot_be_established_while_the_identity_it_names_is_not`, `an_archive_install_rule_needs_an_established_container`, `a_uv_convention_cannot_claim_more_sets_than_the_vertex_carries`, `a_palette_cannot_exceed_its_rig_and_weights_cannot_exceed_the_vertex` |
| every bounds rule | `dimensions_must_be_real_and_must_fit_the_codec_that_stores_them`, `a_mip_policy_owns_its_level_count`, `an_atlas_or_pool_must_carry_exactly_the_operands_its_kind_defines`, `a_vertex_attribute_must_fit_the_stride_and_be_declared_once`, `audio_sample_constraints_must_be_realisable_and_owned_by_the_codec`, `an_install_kind_and_its_size_policy_cannot_contradict_each_other`, `a_raster_channel_set_must_agree_with_its_alpha_mode_and_its_codec`, `a_container_chain_must_be_a_chain_and_only_its_innermost_layer_may_lack_an_index`, `a_path_template_and_its_token_list_must_agree_in_both_directions`, `a_tiling_alignment_must_be_a_power_of_two` |
| the evidence envelope carries over | `the_shared_evidence_envelope_applies_to_asset_blocks`, `an_asserted_asset_claim_cannot_be_operative_and_a_derived_one_must_argue`, `an_asset_candidate_obeys_the_same_rules_as_a_database_one` |
| version transition | `a_v1_asset_profile_is_not_validated_in_place`, `the_v1_asset_transition_promotes_no_prose_into_a_fact`, `an_unexpressible_v1_canonical_source_is_dropped_and_the_profile_then_refused` |
| synthetic exclusion | `the_synthetic_fixture_is_excluded_from_registry_population`, `no_committed_asset_profile_is_a_synthetic_fixture` |
| the family vocabulary cannot drift | `the_family_vocabulary_matches_the_qa_matrix` — asserts `ASSET_FAMILY == tdb_qa::catalog::FAMILIES` |

**Every database constraint and its tests are preserved**: `tests.rs` is unchanged except for the
one asset test that moved to `asset_tests.rs`, and all 83 database cases still pass.

## 4. Checkpoint 4 — coverage

### Representation

**26 / 26** asset constraints represented (`asset_representation_completeness`). Database:
**20 / 20**, unchanged. Neither number says anything about a target.

### Population — real, established, unknown and candidate

| Family | Target | Established | Unknown | of which candidates | Not-applicable |
|---|---|--:|--:|--:|--:|
| `assets-portraits` | `nhllegacy-xbox360` | **0** | 15 | 5 | 0 |
| `assets-portraits` | `nhl21-ps4` | **0** | 15 | 0 | 0 |
| the other **13 families** | — | 0 | — | — | — |

Adapters are untouched: `nhl15-xbox360` 32 established / 34 unknown / 11 candidates,
`nhl10-xbox360` 29 / 29 / 9.

### Evidence sources used

| Source | Used for | Strength |
|---|---|---|
| `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` | the five Legacy portrait candidates | **asserted** — an operations record of a Studio round trip on an extracted tree, stating no scope over the shipped format |
| the roadmap's Q07a.1 asset sentence, Q10, Q11, Q14, C3/C3a/C4/C5/C7 | which constraints must be representable, and several vocabulary shapes (fit-in-slot vs relocate, ambiguity policy, retention, loop-metadata preservation) | requirements, not target facts — they shaped the **format**, never a value |
| `tdb_qa::catalog::FAMILIES` | the family vocabulary | the ledger's own axis, asserted equal by a test |

No anchor, audit, private fixture or game tree was read.

### Unresolved functionality decisions, recorded and not invented

| # | Decision | Effect here |
|---|---|---|
| **P4** | do asset profiles need per-target **overrides** parallel to Q07a.3a? | not decided. v2 represents a target's output **constraints** and has no override syntax, so it neither presumes nor forecloses one. Independent work continued around it |
| **U-A10** | may one profile carry several **variants** of an asset (menu/detail vs HUD)? | `[dimensions].variant` names the variant, but a target with several needs several profiles today. No accepted decision requires the richer shape |
| **U-A11** | what **tolerance** does each operation carry (C3 item 5's visual tolerance, C4's rasterization tolerance)? | not representable, because no accepted decision fixes the values |
| **P3** | is `assets-jerseys` the family the legacy `uniforms` rows belong to? | unchanged; it decides which family a future profile would be written under |

### Remaining gaps

1. **13 of 14 families have no profile, and the one that does declares nothing established.** This
   is the whole remaining half of item 7 and it is population, not format.
2. The asset critical path is a **read of one shipped portrait** on one target — dimensions, pixel
   codec, mip layout, tiling, install location — which would promote most of one target profile at
   once. That is a fixture measurement this package was scoped out of.
3. The database critical path is unchanged and its scope is now stated correctly (§0).
4. `legacyrecomp-recomp` still has no anchor (U-A5); ownership is still unassigned (item 6).

## 5. Checks

| Check | Command | Result |
|---|---|---|
| Scoped crate tests | `cargo test -p tdb-canonical` | **Passed — 126/126** (was 84) |
| Strict clippy, scoped | `cargo clippy -p tdb-canonical --all-targets -- -D warnings` | **Passed**, exit 0 |
| Repo-wide formatting | `cargo fmt --all -- --check` | **Passed**, exit 0, zero diffs (the working tree carries the uncommitted C0-INT1 repairs; CLAUDE.md records this as failing at HEAD) |

No full workspace suite and no QA emitter run: nothing outside `tdb-canonical` changed, and the QA
generators are explicitly not to be run as a side effect.

Rules 46 → **53**. Tests 84 → **126**.

## 6. Files changed

| Path | Change |
|---|---|
| `crates/tdb-canonical/src/asset.rs` | **New.** `asset-output-profile/2` types, vocabularies, `Q07A1_ASSET_REPRESENTATION`, validator, readiness, population |
| `crates/tdb-canonical/src/asset_v1.rs` | **New.** v1 document + `upgrade_asset_profile_v1` |
| `crates/tdb-canonical/src/asset_tests.rs` | **New.** 43 tests |
| `crates/tdb-canonical/src/lib.rs` | schema constants, 7 rules, module wiring, re-exports; the v1 asset validator removed (v1 is no longer validated in place) |
| `crates/tdb-canonical/src/model.rs` | the v1 asset types moved out to `asset_v1.rs`; **no change to any database type** |
| `crates/tdb-canonical/src/tests.rs` | one asset test moved to `asset_tests.rs`; the 83 database cases untouched |
| `crates/tdb-canonical/Cargo.toml` | `[dev-dependencies] tdb-qa` — **test-only**, for the family-vocabulary agreement assertion. The shipping crate is still serde + toml |
| `crates/tdb-canonical/tests/fixtures/synthetic-asset-profile.toml` | **New.** Labelled synthetic fixture |
| `crates/tdb-canonical/tests/fixtures/asset-profiles/v1/assets-portraits.toml` | **New.** The v1 bytes, preserved verbatim |
| `defs/adapters/asset-profiles/assets-portraits.toml` | re-authored as v2; every fact now unknown or candidate |
| `docs/validation/c0-06-canonical-project-and-adapters.md` | amended: §1, new §2.4, §4.5, §5, §7, §8, §9.3, new §10 |
| `docs/plans/project-completion-roadmap.md` | C0 item-7 status row + exit-criteria paragraph |
| `docs/receipts/c0-can1-r2-target-specific-derivations.md` | labelled correction appended |
| `docs/receipts/c0-as1-asset-profile-constraints.md` | **New.** This receipt |
| `Cargo.lock` | the `tdb-qa` dev-dependency edge on `tdb-canonical` |

Unchanged: `defs/canonical/project-schema.toml`, `defs/adapters/nhl10-xbox360.toml`,
`defs/adapters/nhl15-xbox360.toml`, `crates/tdb-canonical/src/v1.rs`,
`crates/tdb-canonical/tests/fixtures/synthetic-adapter.toml`,
`crates/tdb-canonical/tests/fixtures/v1/**`, the workspace `Cargo.toml`, every other crate.

## 7. Preservation

| Check | Result |
|---|---|
| HEAD | `a2bbdc4`, unchanged |
| Index | **empty** |
| Pre-existing dirty/untracked work | preserved; only the paths in §6 were written |
| `anchors/`, `docs/audits/**` | read only — the capability CSV was read, never written |
| Historical evidence | the CAN1-R2 receipt is amended **by appended label**, never rewritten; the v1 asset profile's bytes are preserved as a fixture at their original hash; the adapter v1 fixtures are untouched |
| QA generators | not run |

## 8. Questions for the architect

1. **P4 is now the blocking product decision for the asset half.** The format represents a target's
   output constraints and deliberately has no override syntax. If asset derivations are always
   recomputed from the canonical source, v2 is complete as written; if they need Q07a.3a-style
   per-target overrides, that is a further block (and a reconciliation-UI shape), not a value.
2. **Should a shipped-portrait read be scheduled?** One target's dimensions, pixel codec, mip layout,
   tiling and install location would take that profile from zero established constraints to most of
   one. Every asset operation in the registry is blocked until something like it happens.
3. **Does `assets-portraits` still earn its place as the worked example?** It now demonstrates the
   blocked state honestly and demonstrates nothing else; the synthetic fixture carries the
   mechanics. The alternative is to keep it exactly as is and treat the zero as the finding.
4. Carried forward: 26 units have no adapter, 13 families have no profile, ownership is unassigned
   (item 6), and population remains a C0 obligation.

**Stop for Codex review.**

---

## Answer — appended 2026-09-08 by package C0-AS1-R1

**§8 question 1 (P4) is answered: yes, mirror Q07a.3a.** Nothing above is rewritten and no fact in
this record changes — §8's statement that v2 "deliberately has no override syntax" describes the
revision this receipt reports, and remains accurate for it.

`asset-output-profile/2` now carries a per-target `[target_profile.override_policy]`: the
**permission** half of Q07a.3a (may a Build depart from this target's constraints, from which of
them, by which resolution methods, and what makes a departure stale). The override **records**
stay Build data and have no spelling in a profile. Population is unchanged at zero established
constraints, and both portrait targets record `unspecified` — an override departs from an
established requirement, and neither target has one.

See [c0-as1-r1-override-policy.md](c0-as1-r1-override-policy.md).

**§8 question 2 (the shipped-portrait read) is authorized but not yet scheduled**: it needs one
explicit game-data path, which the Phase 5 path-intake gate forbids searching for.

---

## Correction — appended 2026-09-08 by package C0-AS2-R1

**The answer appended immediately above is withdrawn. It misread P4, and the implementation it
announced has been removed.** Nothing before it is rewritten; both sections stay as the historical
record of what was believed and built on 2026-09-08.

The withdrawn text describes an override as a **departure from** a target's constraints, and
describes the profile as carrying a permission to depart. That is wrong in both halves:

| The withdrawn text said | The authoritative P4 says |
|---|---|
| an override is a departure from an established constraint | an override changes a Build's **requested derivation settings within** proven constraints, and never permits departure from them |
| a profile carries a permission (`override_policy`) | profiles are **unchanged**; the record, its provenance and its approval live in the target **Build mapping** |
| a target with no established constraint could permit nothing | a setting whose constraint is unknown **blocks approval**; that is a property of resolution, not of a permission block |

Every statement about `override_policy`, `overrides-allowed`, `overridable_blocks` or the three
`asset-override-*` rules describes code that no longer exists. §8 question 1 is answered instead by
[c0-as2-r1-build-mapping-overrides.md](c0-as2-r1-build-mapping-overrides.md).

§8 question 2 is unchanged and still open: the shipped-portrait read is authorized, needs one
explicit game-data path, and is **not** part of C0-AS2-R1.
