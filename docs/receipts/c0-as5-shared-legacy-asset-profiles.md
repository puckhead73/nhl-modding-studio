# C0-REG5-CLOSE + C0-AS5 — shared Legacy asset profiles — receipt

> **Closed by C0-AS5-CLOSE (section F below, 2026-09-15): committed as `cc913f9`** after a
> validation-order defect was reproduced and corrected. Section D's hashes for `asset.rs` and
> `asset_reuse_tests.rs` are superseded by F.2; the boundary membership is unchanged (10 files).
> 529 -> 532 cases.

**Package:** commit REG5 (CLOSE); implement shared asset-profile reuse for Legacy Recompiled per
Q47 (`asset-output-profile/5`); exercise the actual resolution path; record and verify (AS5 stays
uncommitted).
**Baseline:** `c5e3a2b` → CLOSE commits `039de62`; AS5 is built on `039de62`.
**Date:** 2026-09-15.

**Result.** REG5 is committed as **`039de62`** on a tree byte-identical to the tested one (519,
clippy/fmt clean). AS5 adds `reuses = "<target id>"` to `[[target_profile]]` with a boundary fixed
in the crate — shared definitions inherited verbatim in their source states, destination blocks
never inherited, conflicts refused — and registers `legacyrecomp-recomp` by reference to
`nhllegacy-xbox360` in all three committed families. Ten focused cases run the effective profile
through the existing readiness / resolution APIs; 519 → **529** on the exact isolated candidate,
clippy `-D warnings` and rustfmt clean. Nothing inherited is operative (nothing on Legacy Xbox 360
is), no Recompiled hash or corpus is invented, and Recompiled's database unit stays absent. Index
empty; nothing pushed.

---

## A. C0-REG5-CLOSE — the commit

| Item | Value |
|---|---|
| Pre-checks | HEAD `c5e3a2b`; index empty; all 15 REG5 §D.1 hashes match the working tree — **no drift** |
| Staged | exactly the 15 boundary paths; `git diff --cached --check` clean; +7 525 / −180 |
| Staged tree | `170ff81aa501948fd57e30abbc6250cbb23861f8` |
| Export | `git -c core.autocrlf=false -c core.eol=lf archive <tree>` → 1 226 files, **0** byte mismatches vs staged blobs, 0 extra |
| Verification (fresh target dir, QA vars unset) | test **519 / 0 / 0**; clippy exit 0; fmt exit 0 (`reg4/log-close5-*.txt`) |
| Commit | **`039de62`** — `feat(canonical): register six early-generation adapters`; `HEAD^{tree}` = `170ff81…`; message file; Co-Authored-By line |
| Preservation | index empty afterwards; no manifest/lock change; unrelated entries untouched; nothing pushed |

(The first staging attempt staged nothing — a CRLF list-file artefact caught by the tree check,
`6904cda…` = HEAD's tree — and was redone before anything else ran.)

---

## B. `asset-output-profile/5` — shared definitions by reference

### B.1 Contract inspected first

`TargetProfile` is the unit every consumer takes (`block_states`, `operation_constraint_readiness`,
`select_variant`, `resolve_read` / `resolve_write`, `CreationSupport::of`, `shared_steps`,
`ownership_sets`, `asset_registry_population`, the build-override resolver). A `ProvenanceBridge`
ties a corpus to another corpus **on the same target**; a `Corpus` is attributable to one target.
Nothing could point one target profile at another, so registering Recompiled meant duplicating
every source block — which Q47 rules out. The representation chosen therefore stays inside the
family document, keeps the existing registry identity `legacyrecomp-recomp`, and produces an
ordinary `TargetProfile` for every existing consumer.

### B.2 Representation

* `TargetProfile.reuses: Option<String>` — the only new key. A `/5` construct: `/3` and `/4`
  documents that use it get an `UnsupportedSchema` finding naming `/5`; `/4` documents without it
  validate exactly as before (their `/4` constructs now name `/4` "or newer" in the `/3` refusal).
  `ASSET_PROFILE_SCHEMAS_NATIVE` = [`/5`, `/4`, `/3`]; the `/1` and `/2` transitions land on `/5`.
* `asset_reuse.rs`: [`SHARED_BLOCKS`], [`DESTINATION_BLOCKS`], [`SHARED_TABLES`],
  [`LOCAL_EVIDENCE_TABLES`], `ReuseError`, `shared_declarations`, `reuse_source`,
  `resolve_target(profile, id) -> ResolvedTarget { profile, reused_from, inherited }`.
* Validator: a reusing target is checked structurally (`AssetReuseUnresolved`,
  `AssetReuseSelfReference`, `AssetReuseChain`, `AssetReuseConflict`), then its **effective**
  profile is validated as a whole (subject suffixed `(resolved via reuses)`).
* `AssetPopulation.reused_from` — a reusing target's row is counted on its effective profile and
  reported apart as registered by reference.

### B.3 The inheritance boundary (fixed in the crate, not by the document)

| Side | Content | Rule |
|---|---|---|
| **Shared — inherited verbatim** | blocks `identity`, `container`, `path_rule`, `raster`, `dimensions`, `mipmaps`, `compression`, `tiling`, `atlas`, `mesh_layout`, `uv`, `rig`, `audio`; tables `path_family`, `variant`, `resolution_chain`, `operation`; the source's `corpus` records | copied with their **states unchanged** — a source unknown stays unknown, a candidate stays a candidate, a specimen-scoped block stays non-operative. Declaring any of these locally on a reusing target is a **conflict** (validator finding, resolver `Err`); nothing is merged and nothing wins silently |
| **Evidence identity — merged by id** | `corpus` | the reusing target may declare its own corpora (a Recompiled installation inspection, when one exists) for its destination blocks; a local id equal to a source id is refused (`CorpusIdCollision`), never shadowed |
| **Destination — never inherited** | `discovery`, `install`, `runtime_lookup` | always the reusing target's own: declared there or unknown. `CreationSupport` reads the destination's `install`, so reuse creates no install or reference-update capability |
| **Never carried** | qualification, automated-case attribution, evidence cells, runtime evidence, database equivalence | not profile content; not inherited; the database registry is untouched (`legacyrecomp-recomp` has no adapter) |

References must name a different target profile in the same document; a source may not itself
reuse (no chains ⇒ no cycles). Every `ASSET_BLOCK` is on exactly one side (asserted by a test).

### B.4 The Recompiled registration (Q47, user-attributed)

In each of `assets-portraits.toml` (`/3`→`/5`, profile_version 3→4), `assets-logos.toml` (`/3`→`/5`,
1→2) and `assets-jerseys.toml` (`/4`→`/5`, 2→3): one appended `[[target_profile]]
target = "legacyrecomp-recomp" reuses = "nhllegacy-xbox360"` with a note attributing the reuse to
the user's decision ("not a measurement: no Recompiled file was hashed or inspected for it, no
corpus is declared here, and no provenance bridge is claimed"), and three **unknown** destination
blocks (`discovery`, `install`, `runtime_lookup`) each stating that it is not inherited and why.
Nothing else is written: the definitions live once, on `nhllegacy-xbox360`. Absent source support
is not manufactured — where the source is a candidate (jersey composite id, thumbnail chain) or
unknown (everything else), Recompiled is too.

---

## C. Tests — the effective profile through the existing APIs (`asset_reuse_tests.rs`, 10 cases)

| Case | What it runs |
|---|---|
| `every_block_is_either_shared_or_destination_and_never_both` | the boundary partitions `ASSET_BLOCK`; `PLACEMENT_BLOCK` minus `path_rule` is destination |
| `a_valid_reference_inherits_the_shared_definitions_without_duplicating_them` | `resolve_target`: effective families / chains / operations / corpora / identity equal the source's; `inherited` lists what came across; the document spells the share once |
| `a_missing_reference_is_a_finding_and_a_resolver_error` | `AssetReuseUnresolved`; `UnresolvedReference`; `UnknownTarget`; `NotAReuse` |
| `a_self_reference_is_refused` | `AssetReuseSelfReference`; `SelfReference` |
| `chains_and_cycles_are_refused` | A→B→A (both ends reported, neither resolves) and A→B→C (the end refused, the middle resolves) |
| `a_local_shared_declaration_on_a_reusing_target_is_a_conflict_not_a_merge` | local `path_family` + `container` → one `AssetReuseConflict` naming both; a local *candidate* is a declaration too; a shadowing corpus id is refused; a distinct local corpus merges beside the source's |
| `shared_candidates_remain_non_operative_through_reuse` | inherited identity candidate: `operation_constraint_readiness(identify)` blocked on `identity` on reuser and source alike; shared `block_states` equal |
| `destination_blocks_are_never_inherited_and_the_write_exclusion_survives_reuse` | source has a loose-file install: `CreationSupport` true there, `install: false` on the reuser; `install` op ready on source, `BlockedByUnknownConstraints(["install"])` on reuser; `resolve_read` falls back to the shared default on both; `resolve_write` → `Create` on source, `CreationUnsupported { missing: ["install", "references"] }` on reuser; existing owned file still `Existing`; a token that aliases the shared default → `AliasesSharedDestination` on **both**, exists or not; only the reuser's **own** corpus + install lifts creation, and the exclusion still holds then |
| `older_schemas_do_not_silently_accept_reuse` | `/3` and `/4` documents with `reuses` → `UnsupportedSchema` naming `/5`; `/4` without it clean; native list |
| `legacy_recompiled_reuses_the_legacy_xbox360_definitions_in_every_committed_family` | the three committed profiles: reference, note attribution, no local corpus, destination unknown; effective families / chains / variants / operations / corpora equal the source's; shared block states equal and none operative; every operation blocked, the installing ones on Recompiled's own destination blocks; `CreationSupport.install` false; no chain writable; population rows `reused_from = nhllegacy-xbox360`, 0 operative |

Existing tests updated for the fourth row / `/5`: the portrait population and synthetic-exclusion
counts (3 → 4, the reference row asserted apart), the logo/jersey representation test (two
targets, reference row apart), the runtime-lookup sweep (readiness on the effective profile), the
committed-profile schema checks, and the rule-name registry (+4 rules). `/3`-in-place and `/4`
meaning are unchanged (`a_v3_document_may_not_use_v4_constructs`, the `/4` synthetic fixture).

---

## D. Validation and boundary (AS5, uncommitted)

**Candidate:** `git -c core.autocrlf=false -c core.eol=lf archive 039de62` (1 226 files) + the
10-file boundary → 1 228 files; fresh `CARGO_TARGET_DIR`; `QA_WRITE` / `QA_EVIDENCE_ROOT` unset; no
manifest or lock change.

| Check | Result | Log |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **529 passed · 0 failed · 0 ignored** | `reg4/log-as5-test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 | `log-as5-clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | `log-as5-fmt.txt` |
| non-boundary files vs `039de62` blobs, byte-exact | 0 changed · 0 missing · 0 extra | inline |

Failures met on the way, none fixed by weakening: the exclusion fixture's owned template could not
alias the shared default (template changed to the aliasing shape used by the existing exclusion
tests); four existing asset tests counted three targets / one target per profile (updated to
assert the reference row apart); two clippy lints.

### D.1 Boundary (SHA-256 of working-tree bytes; A = new, M = modified vs `039de62`)

```
afd66005ee153326d7b66514bd626641db270b13ddec90fd8e3a6d5f15c99f6c  crates/tdb-canonical/src/asset_reuse.rs            A   293 lines
f88b43dc13ff055171a78d69adfd6a959b3e90eb30d719876cde3695e1b19438  crates/tdb-canonical/src/asset_reuse_tests.rs      A   753 lines
02354969a95efa75991aa74742ebaf57ec46b64e184a801f58c996f06f13fcf6  crates/tdb-canonical/src/asset.rs                  M  +75/−6
de95435466f49706d9aa9d8d669010ca282c7171b8097b53ed5da3fb92ddc456  crates/tdb-canonical/src/lib.rs                    M  +34/−6
e490dc89dd1f8dc1e428ca1708840434d9836a17c065f3e5edf9ea0d270a057f  crates/tdb-canonical/src/asset_tests.rs            M  +46/−12
3e9faab410dbb4f74ff6be9cce51f2188f9c0ff3144efc354e3c85ddb3800f12  crates/tdb-canonical/src/asset_resolution_tests.rs M  +21/−9
ee16e68523317d80d708427ce82362ae6914fa3791c0dbdc437e6263e09102e2  defs/adapters/asset-profiles/assets-portraits.toml M  +39/−3
d83508c7c83f633dc9858ce996fcbf4d9edcbad9ebdc5397a6460c701bac6598  defs/adapters/asset-profiles/assets-logos.toml     M  +37/−2
f5eccf75b9ef0736bf2eaa201f1c3e12d6efa0181c8eeceaf80a2613ee3d7c77  defs/adapters/asset-profiles/assets-jerseys.toml   M  +37/−2
a773cafba78c9c1951f1668db00ac20faa2dba0b4058dab020546579264540f1  docs/receipts/data/c0-reg2-asset-mapping-inventory.csv  M  +3/−0  (three Recompiled by-reference rows)
```

Outside the boundary (documentation): this receipt; `docs/plans/completion-design-questions.md`
(Q47 marked implemented); `docs/plans/project-completion-roadmap.md` and `docs/STATE-OF-THE-REPO.md`.
Live tree: HEAD `039de62`, index empty, nothing pushed. Proposed boundary: one commit,
`feat(canonical): asset-output-profile/5 shared definitions by reference for Legacy Recompiled`
(10 files; expected 529).

---

## E. Limitations, kept explicit

* **Registration, not support.** Recompiled's three by-reference rows inherit nothing operative
  (nothing is operative on Legacy Xbox 360 either) and declare destination blocks unknown; every
  operation is blocked; no installer, archive writer, importer or headmesh-repository inspection.
* **User-attributed, not measured.** The identity of the asset files is Q47's direction; no
  Recompiled file, corpus, hash or bridge exists in the repository. When a Recompiled installation
  is inspected, its corpus and destination blocks go on the reusing profile itself.
* **Database side untouched.** File equivalence establishes no database equivalence and no
  reference-update capability; `legacyrecomp-recomp` has no database adapter (FR4).
* **NHL 07 team-name join — canonical execution requirement, retained.** `nhl07-xbox360` maps
  `team.name` to `stockteamtable.fullname`; its case measures the `kOtt.(table, teamid) →
  stockteamtable.index` join on the anchor, but the contract has no join-path block and the
  direct-field round trip does **not** implement the join. Executing it (as the engine's
  `team_name_source` thin shape does) is an open requirement for any canonical read of a team name
  on NHL 07, recorded in the roadmap; this package does not claim it.
* Database registration remains partial (24 attributed + 3 representation-only of 28); QA
  activation stays parked.

---

## F. C0-AS5-CLOSE — validation after composition, and the commit

### F.1 The defect and the correction

**Reproduced** (`an_inherited_established_container_satisfies_a_local_archive_installation`,
failing before the fix with `asset-constraint-dependency-unmet` on the reuser): a synthetic source
with an **established container** and a reuser with **its own attributable corpus** and an
**established `archive-replace-in-place` install** citing it. `validate_asset_profile` validated
the raw reusing declaration as a whole before resolving the shared definitions, so
`validate_install` saw no container even though the effective profile supplies one (and the
missing-container case reported the finding twice).

**Corrected sequencing** for reusers (non-reusers unchanged):

1. *Declaration level* (`validate_reuser_declaration`): reference structure, local conflicts and
   evidence ownership (`reuse_source`); the target's own corpora; its destination blocks
   (`discovery`, `install`, `runtime_lookup`) held to the evidence envelope **against its own
   corpora only** and to their vocabularies (`validate_install_value`: kinds, policies, internal
   bounds).
2. *Resolution* of the shared definitions (`resolve_target`), source provenance attached.
3. *Cross-field* on the **effective** profile (`validate_target`): the archive-install -> container
   dependency, class applicability, chains, geometry, operations.

`validate_install` is split into `validate_install_value` (value-only) and the dependency check;
the dependency now runs where the container can be present. Destination evidence is not weakened:
a destination block citing an inherited **source** corpus is `asset-corpus-undeclared` at step 1 --
an inherited corpus speaks for the shared definitions, never for the destination.

**Regressions added (3; every other AS5 case unchanged and green):**

| Case | Asserts |
|---|---|
| `an_inherited_established_container_satisfies_a_local_archive_installation` | clean validation; effective container operative with `corpus = "copy"` (source provenance kept), install operative on `destination-copy`; `install` op `ConstraintsEstablished`; **`CreationSupport { install: false, container: false, references: true }`** -- describing an archive installation adds no creation capability; `resolve_write` -> `CreationUnsupported { missing: ["install", "container"] }` when the owned file is missing, `Existing` when present |
| `a_missing_inherited_container_still_fails_a_local_archive_installation` | source container unknown -> exactly one `AssetConstraintDependencyUnmet` on the reuser (effective profile); a source *candidate* container fails too |
| `a_source_corpus_cannot_substitute_for_destination_evidence` | local install citing the source's `copy` -> `AssetCorpusUndeclared` (and no false dependency finding); uncited install -> findings; a non-reusing target with its own container + corpus clean, without the container the same dependency finding as always |

No installer or archive-creation capability was added; `CreationSupport`'s restrictions are
asserted unchanged.

### F.2 Commit

| Item | Value |
|---|---|
| Boundary | the same 10 files (no expansion); `asset.rs` and `asset_reuse_tests.rs` re-hashed: `a5eb23e12b06223af09532cc909e8327bd64546098ebe1fcb34abd0668ad1a54` (`asset.rs`, +140/-17 vs `039de62`), `201538c81ce2240538e0c5e9dd32565e807df62510fb9581398b8bdcf0543487` (`asset_reuse_tests.rs`, 944 lines); the other eight as in D.1 |
| Staged | exactly the 10 paths; `git diff --cached --check` clean; +1 594 / -51 |
| Staged tree | `c39331904c7c5c936e59fb082f4f8e491fd0c622` |
| Export | `git -c core.autocrlf=false -c core.eol=lf archive <tree>` -> 1 228 files, **0** byte mismatches vs staged blobs, 0 extra; 10 paths differ from `039de62` (2 A, 8 M) |
| Verification (fresh target dir, QA vars unset) | test **532 / 0 / 0**; clippy exit 0; fmt exit 0 (`reg4/log-closeas5-*.txt`) |
| Commit | **`cc913f9`** -- `feat(canonical): share Legacy asset profiles with Recompiled`; parent `039de62`; tree `c393319...` (the tested tree); message file; Co-Authored-By line |
| Preservation | index empty afterwards; no manifest/lock change; unrelated modified/untracked entries untouched; nothing pushed |

Documentation (this receipt, roadmap, STATE, decision log) stays outside the source commit.
Next development focus: asset-family population, not further expansion of the reuse contract.
