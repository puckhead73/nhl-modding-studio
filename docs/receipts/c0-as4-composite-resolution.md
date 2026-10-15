# C0-REG3-CLOSE + C0-AS4 — receipt

> **Superseded in part by C0-AS4-R1 (section G) and closed by C0-AS4-CLOSE (section H),
> 2026-09-15.** Boundary B is committed (`6ba9af4`) and boundary C is committed (`c3f4181`). Two
> defects in the write resolver were corrected in R1 and one remaining case-alias defect in CLOSE;
> section C.2's description of `resolve_write` / `CreationSupport`, section C.7's manifest and
> G.2's casing rule are superseded by G.2–G.4 and H.1–H.2. Everything else in A–F stands.

**Package:** commit the accepted REG3 slice; quarantine the NHL 10 Xbox 360 ratings conflict;
implement `asset-output-profile/4` (composite identities, ordered resolution chains with shared-file
write exclusion); measure the Legacy `xjgw` composite on the committed anchor and update the jersey
profile; correct the Project-import design.
**Baseline:** `0251b57cf7311098f372534b755ef23f6268c061`, branch `feat/phase5-asset-tooling`.
**Date:** 2026-09-15.

**Result.** REG3 committed as **`26d5e44`** (tree = tested tree, 341 cases, clippy/fmt clean, index
empty). Two uncommitted boundaries remain for review, each verified in its own isolated candidate on
`26d5e44`: **B** quarantines `nhl10-xbox360`'s speed rating (344 cases); **C** implements `/4` and
re-represents the jersey family under it (357 cases; B + C together 360). The `xjgw` composite is
**not unique** on the Legacy anchor — 703 rows render 698 ids, three tuples collide — which the
profile now records as measured, not assumed. Nothing pushed; no archive installation, no
production conversion pipeline claimed.

---

## A. REG3 committed

| Item | Value |
|---|---|
| Commit | **`26d5e441279a0e097d45c351fa66e64d4797e65b`** — `feat(canonical): register NHL10 through NHL13 PS3 adapters` |
| Staged tree (`git write-tree`, exported with `git archive` and tested) | `5c9f4b3392ed25b81c8f381240b5cb10b8e6fdb7` = `git rev-parse 26d5e44^{tree}` |
| Files | exactly the 10 REG3 paths; every SHA-256 matched the REG3 receipt §B.7 before staging (`diff` of the two hash lists empty); staged individually; `git diff --cached --check` clean; message file |
| Isolated checks on the exported tree | `cargo test -p tdb-canonical --locked --offline` **341 passed / 0 failed** (`as4/log-reg3-test.txt`); clippy `-D warnings` exit 0; `cargo fmt -- --check` exit 0, empty |
| After | index empty (0 lines); live tree 308 unrelated modified/untracked entries preserved; no path under `crates/tdb-canonical` or `defs/adapters` left modified; no manifest or lock change |

---

## B. NHL 10 Xbox 360 ratings quarantine (uncommitted, boundary B)

`defs/adapters/nhl10-xbox360.toml` `player.rating_speed`: `offset+36` / `derived` (C0-CAN1-R2) →
**`unresolved` / `blocked`**, `confidence = measured`, `transform` **unknown** with a `derived`
candidate `linear-offset 50` (premises: the measured maximum on this file + the probe's own rule),
`inverse_verified = false`, no `automated_case`. The note names both sources and the disagreement;
the candidate note states that the probe's value is held only because it is the one measured against
these bytes, not because it is preferred. Storage, status and the (already candidate) applicability
are unchanged. Neither 36 nor 50 is chosen; the production codec and the audit are untouched.

Focused coverage — `crates/tdb-canonical/src/nhl10_ratings_conflict_tests.rs` (3 cases):

| Case | Asserts |
|---|---|
| `the_xbox360_anchor_stores_the_same_six_bit_maximum_and_the_adapter_records_the_same_conflict` | on the Xbox 360 file by path and hash: 24 six-bit `dSvy` fields, 21 six-bit `mHuy` fields, largest stored value **49**, `p_speed` 2..47; the bytes equal the PS3 anchor's (observation); the adapter is blocked/unknown with candidate 50 citing this case and both sources; validates clean |
| `both_nhl10_adapters_hold_the_same_quarantine_on_the_speed_rating` | `nhl10-xbox360` and `nhl10-ps3` agree on field, storage, blocked state, candidate value and cited sources; neither carries an operative offset |
| `the_cited_sources_still_disagree_as_recorded` | the audit row (`NHL 10 → Legacy / 11–21 (PS3/X360) 6-bit \| 0..63 \| **36**`) and the probe (`> 49` → `return 36;`, else `50`) still say what they are cited for — a change to either surfaces here |

`tests.rs::a_target_supported_derivation_stays_usable` now exempts both NHL 10 adapters with the
same stated reason (previously `nhl10-ps3` only).

**C1 finding (owner: Codex for acceptance, Claude for implementation).** *The NHL 10 six-bit rating
offset is contested between two committed sources and cannot be settled from the repository.*
Smallest independent witness: **one observation of one displayed rating on the actual NHL 10 title**
— any single 6-bit skater or goalie rating (Speed is the mapped one) as the game shows it on the
player card, for one player identifiable by `ubPc.wBIz` on the committed anchor, on a build whose
identity is recorded. The two hypotheses differ by exactly 14 for every player (raw + 36 vs raw + 50;
the best stored Speed, raw 47, displays as 83 or 97), so one reading decides. Implementation on
acceptance: a named case that pins the observation (player key, field, raw, displayed value, build
id, date), promotion of the transform on both NHL 10 adapters at `measured` citing it, and a
correction of whichever source is wrong — the audit's regime row or the probe's threshold — in the
same package. Not acceptable as the witness: the stored distribution, "realism", or agreement with
the existing probe.

Isolated candidate B (`26d5e44` + 4 files, lib.rs = HEAD + one `mod` line): **344 passed / 0
failed**, clippy `-D warnings` exit 0, fmt clean (`as4/log-cb-*.txt`).

---

## C. `asset-output-profile/4` (uncommitted, boundary C)

### C.1 Schema and compatibility

* `ASSET_PROFILE_SCHEMA = "asset-output-profile/4"`; `ASSET_PROFILE_SCHEMA_V3 = "/3"`;
  `ASSET_PROFILE_SCHEMAS_NATIVE = [/4, /3]`. A `/3` document **parses into the same types and is
  validated in place under exactly its `/3` rules** — `/4` adds only optional structures, so no
  `/3` field changes meaning — plus one rule: a document that claims `/3` while using a `/4`
  construct (`[[resolution_chain]]`, `[identity…composite]`, a `composite-id` token) gets an
  `UnsupportedSchema` finding naming the construct. No transition function is needed and none is
  written; `/1` and `/2` still transition to the current schema string. `build-mapping-override/1`
  records accept either native schema string (the four committed fixtures say `/3`).
* Committed profiles: `assets-portraits.toml` and `assets-logos.toml` **stay `/3`** and validate
  clean; `assets-jerseys.toml` moves to `/4` (`profile_version = 2`).

### C.2 Additions (types in `crates/tdb-canonical/src/asset_resolution.rs`, 605 lines)

| Construct | Shape | Validation (`Rule::AssetCompositeInvalid` / `Rule::AssetResolutionChainInvalid`, new) |
|---|---|---|
| `[identity.value.composite]` (also under `.candidate.value`) | `format = "{style}_{art}_{variant}"`, `[[part]] { name, table, field, render, null_code? }`; `render ∈ decimal \| zero-padded:<1..16> \| lower-hex \| slug` | key must be `composite` iff a composite is present; ≥ 1 part; part names `[a-z][a-z0-9-]*`, unique; no two parts on one table.field; render parses; every `{placeholder}` names a part, each part is used, none used twice, braces balanced, format substitutes something; checked wherever written (established or candidate) |
| `PATH_TOKEN += "composite-id"` | a template may substitute the rendered composite | a family using it needs a composite declared (established or candidate); an **established** `[path_rule]` through it needs an **established** composite identity (same dependency rule as `player-slug`) |
| `[[target_profile.resolution_chain]]` | `{ id, purpose ∈ read \| write \| read-write, declared: AssetConstraint<{ [[step]] { family, ownership ∈ owned \| shared } }> }` — the full evidence envelope (state/scope/force/corpus/candidate) | ids unique; purpose/ownership vocab; every step a declared family, none twice; ≥ 1 step; a write chain has ≥ 1 owned step; a family is not `shared` in one chain and `owned` in another; with chains declared, no two families share a template (an alias); `[[operation]].requires` may name `resolution_chain:<id>` — must exist, and a writing operation (`import/replace/rebuild/install`) may not require a `read` chain |
| Readiness | `resolution_chain:<id>` is met only by an **operative** chain (established at an operative scope); unknown/candidate/specimen chains block; `runtime_lookup` remains a separate requirement | `operation_constraint_readiness`, `AssetPopulation.{declared,operative}_resolution_chains` |

Pure functions (no I/O; reusable by ingestion and Build generation): `render_part` /
`render_composite` → `Result<String, CompositeBlocker::{MissingPart, NullPart, UnrenderablePart,
InvalidDeclaration}>` — a missing or null part yields **no** id, a 0 with no declared `null_code` is
a value; `collisions(rows)` groups rows rendering one id; `render_path(family, tokens)` →
`Result<String, PathBlocker::{MissingToken, UnsafeTokenValue, UnsafePath}>` restating the committed
`safe_join` / `safe_join_rel` convention on the string (no empty/`.`/`..`/absolute/drive/control
characters; token values may not carry separators); `resolve_read` follows an **operative** chain's
declared order and reports whether the hit was shared; `resolve_write` returns **only the first
`owned` step** — `Existing` or `Create` — and never a shared path, with `Create` allowed only when
`CreationSupport::of(target)` (install operative, container decided, references producible) is
complete, else `WriteBlocker::CreationUnsupported { missing }`; `ownership_sets`.

### C.3 Composite measurement — Legacy Xbox 360 `xjgw` on the committed anchor

Named case `asset_resolution_tests::legacy_x360_xjgw_composite_tuples_are_measured_not_assumed_unique`
(renders every row through `render_composite` / `render_path` themselves):

| Fact | Measured |
|---|---|
| Table | `xjgw` (`stockteamjerseys`), 703 rows, 8 bytes/record |
| Part fields (EA names) | `cVLR` **style** 1 bit, stored 0..1 (290 / 413); `GDQL` **teamartid** 15 bits, 0..421; `FccT` **variant** 5 bits, 0..20 (173 rows 0) |
| All-ones patterns | none on the two fields wider than one bit (the 1-bit style's all-ones is its value 1) |
| Null / sentinel cases | no part has an established null code; the **9 rows with `GDQL = 0` are the 9 rows of team key 0**, whose `kOtt.lOnr` is 0 — a value |
| `teamartid` vs team | equals the owning team's `kOtt.lOnr` on **all 703 rows** |
| Distinct `{style}_{art}_{variant}` | **698 of 703** — three shared tuples: `1_19_3` (rows 166, 695; teams 19, 216), `1_21_4` (rows 181, 696; teams 21, 217), `1_255_0` (rows 699–702; teams 220, 220, 221, 221) |
| Path collisions | 3 under `rendering/jersey/texlib_{composite-id}.rx2` and 3 under `fe/ion/artassets/jerseys/jersey_{composite-id}.big` |
| Not measured | that any rendered id names a real file (no member listing; `chunklzx` undecoded) |

### C.4 Jersey profile (`assets-jerseys.toml`, `/4`)

* `[identity]` stays **unknown**; gains a **candidate** with a value: `key = composite`, `ambiguity =
  reject` (the contract's own rule, now a live case), the three parts, `confidence = measured`
  (the tuple measurement) at `scope = specimen-observation` on the shipping-archive corpus (the DB
  bytes only), citing the case; the note carries the collision facts and what would promote it.
* `outfit-texlib` and `fe-thumbnail` substitute `{composite-id}`; three literal shared families
  `fe-thumbnail-custom / -customhome / -customaway` added; `sock-loose` unchanged (its token shape
  is not established).
* `[[resolution_chain]] kit-thumbnail`, `read-write`, **unknown with an `asserted` candidate** whose
  only provenance is labelled `WORKING TREE ONLY, NOT AT BASELINE: …#jersey_thumbnail`; the note
  states the working-tree comment's claim about the game's menu is *not* carried. `install` now also
  requires `resolution_chain:kit-thumbnail` and stays blocked on it and on `runtime_lookup`.
* Header, `[path_rule]`, `[install]`, `[runtime_lookup]` notes rewritten: gaps 1 and 2 are spelled,
  not established; no archive installation implemented; chain order ≠ runtime behaviour.
* Every block still unknown; population 0 established; 6 families all unresolved; 1 declared /
  0 operative chains (asserted by tests).

### C.5 Tests (`asset_resolution_tests.rs`, 16 cases) and adjustments

Compatibility (three committed profiles under their own schemas; a `/3` document using `/4`
constructs refused with three named findings and clean under `/4`); rendering (each rendering kind;
missing/null/overflow/text-vs-code/empty-slug/unknown-render blockers; 0 as a value; `parse_render`
bounds); collisions; path safety (token values, `..`, absolute, drive, control characters, literal
templates); read order vs write exclusion (read finds the shared default, the write to the same state
is `CreationUnsupported`, never `custom.big`; owned present → `Existing`); creation support (`Create`
only with install + container + references; references missing → blocked); candidate chain enables
nothing and `runtime_lookup` stays required even beside an operative chain; read-only chain refuses
writes; all-shared write chain refused; shared→owned alias across chains and duplicate templates
refused; chain declaration negatives (undeclared family, purpose, ownership, undeclared requirement,
duplicate step, duplicate id, empty chain); composite declaration negatives (unknown placeholder,
unused part, duplicate part, bad render, wrong key, bad name, placeholder twice, key without table,
token without composite, `composite-id` dependency on the path rule); the Legacy measurement; the
jersey chain's status. `asset_tests`: the portrait contract test now asserts the profile is `/3`
validated in place; the REG2 representation test's jersey row is 6 families / 3 variants / 3
candidates with an identity candidate.

### C.6 Verification (isolated, `26d5e44` + enumerated overlays; external `CARGO_TARGET_DIR`; QA variables unset)

| Candidate | Overlay | `cargo test -p tdb-canonical` | clippy `-D warnings` | fmt |
|---|---|---|---|---|
| B | 4 files (lib.rs = HEAD + `mod nhl10_ratings_conflict_tests;`) | **344 / 0** | exit 0 | clean |
| C | 7 files (lib.rs = live minus that line) | **357 / 0** | exit 0 | clean |
| B + C | 10 files (lib.rs = live) | **360 / 0** | exit 0 | clean |

One artefact: the first combined run reported 357 with the three quarantine cases absent from the
log although the module was wired — a stale cargo fingerprint from the shared external target dir;
`touch lib.rs` and a re-run gave 360 with the three present. Recorded, not hidden.

### C.7 Exact uncommitted files (SHA-256 of working-tree bytes)

```
--- boundary B ---
6229c42744fe9692654c3742042a85c63e8c8e08114ab656e41ef936d4d52928  defs/adapters/nhl10-xbox360.toml                       M  +30/−14
00f6626d1cf7ffd13d0840cc50c44f16f0d57d1753aed766a31403b3a8105ec7  crates/tdb-canonical/src/nhl10_ratings_conflict_tests.rs A   164
1a5ce7b6c6cdb29693d36f7acb292fc86692ddf4d26614fea8b945a1a0ccee21  crates/tdb-canonical/src/tests.rs                       M  +4/−3
bb75804c5700f8b72269439be73af06062c2963199c40a776e5167d5ae04e6f1  crates/tdb-canonical/src/lib.rs  (B-only variant: HEAD + one mod line; scratch `as4/lib-b.rs`)
--- boundary C ---
5bea53237821506771f27de542aa0b2fe5ea3e9f6029fb47e742adc578ce802c  crates/tdb-canonical/src/asset.rs                       M  +446/−5
1574aba30f10314cdc44a7ac8176f3647350497e3fafe5bd92b5767c1940fdc1  crates/tdb-canonical/src/asset_resolution.rs            A   605
563e26747a78df5ecb8ca182a27a6e5878852c72d13650834f4ad8fd1b6e3005  crates/tdb-canonical/src/asset_resolution_tests.rs      A  1064
f64c53192570e1a99357d55eb919a5662a06020da1dacc8eb711b547c61aefeb  crates/tdb-canonical/src/asset_tests.rs                 M  +19/−9
44e3a69867e0e780becbf94f6017b03aaf23014ef39069478ba4be03c0d042fa  crates/tdb-canonical/src/build_override.rs              M  +4/−4
2ea9c23dd66a9dc9dba23d5544177241aeee5a9ad2fe7630bcfeef8c9fd5543e  defs/adapters/asset-profiles/assets-jerseys.toml        M  +186/−51
b8eea6bb1abb1df3bdb8988b7934224a7d6e005bf81f2bed30101b45b56eab25  crates/tdb-canonical/src/lib.rs  (C-only variant: live minus the B mod line; scratch `as4/lib-c.rs`)
--- live ---
99f95a5c182753d4bb564a1ade811e6bba197a9995e19dd0574de5dda2422761  crates/tdb-canonical/src/lib.rs  (B + C; +24/−1 vs HEAD)
```

**Commit boundaries.** `lib.rs` is the one file both boundaries touch (B: one `mod` line; C: schema
constants, two `Rule` variants, `pub mod asset_resolution`, one test `mod`). Either order works if
the second commit's `lib.rs` is the live file; committing one boundary alone means staging the
matching scratch variant (or hand-removing the other's lines) — say which. Proposed:

1. `fix(canonical): quarantine the NHL 10 six-bit rating regime` — B's 4 paths.
2. `feat(canonical): asset-output-profile/4 composite identities and resolution chains` — C's 7 paths.

No manifest, lock, roadmap, STATE or other receipt is in either boundary. This receipt is untracked
like the others.

---

## D. Project-import record — corrected design (design only; no importer)

Separate from asset profiles; a Developer-Mode Project artefact. Corrections to REG3 §C.3 are
marked **(corr.)**.

**Inputs and identities.**
* `[source]`: kind `complete-folder | changed-files-package`, declared origin (as provided, never
  verified), build identity read from the copy's own headers where present, and a **byte manifest**
  of every input file (path, size, SHA-256). **(corr.)** A changed-files package is *not* assumed to
  contain only changed files: its members are byte-compared like any other, and identical ones are
  reported as identical.
* `[baseline]`: the matching vanilla baseline (target id, attributable corpus id, manifest) or
  `none` with the reason. **(corr.)** Two identity layers are kept apart: **file-level** identity
  (path + bytes in the source tree) and **archive-member** identity (archive + member path/index +
  member bytes, through the existing `ProvenanceBridge`), because a loose file and an archive member
  with the same bytes are two facts, and a source may carry either or both.

**Per-file comparison** — `[[file]]` with `path`, `sha256`, `size`, `member` (optional archive
coordinate), and **(corr.)** `comparison ∈`:

| Outcome | Meaning |
|---|---|
| `identical` | bytes equal the baseline's file (or member) at the same identity |
| `modified` | baseline has the identity, bytes differ |
| `added` | baseline has no such identity |
| `unavailable-baseline` | no baseline, or the baseline lacks the corpus/member listing needed to compare this file (e.g. an archive Studio cannot open) — *not* "modified" |
| `unresolved` | compared or not, the file maps to no declared family/identity (reason: family unknown, composite unconstructible per C.2, container undecoded, outside every discovery root) |

`mapping ∈ mapped(family, target, canonical id, variant) | unresolved`; `retention ∈ retained-opaque
| lifted(role, retention)`; `reason` free text for `unresolved` / `unavailable-baseline`.

**Missing baseline members** — **(corr.)** a separate section `[[baseline_only]]`: identities the
baseline has and the input does not. They are recorded, never inferred: they become `deletion`
intents only when (a) `[source].kind = complete-folder` **and** the input's declared completeness
covers that identity's discovery root, or (b) an explicit deletion manifest supplied with the input
names them. Otherwise they are `not-supplied`, and a Project built from the record carries no
deletion.

**Canonical Project** — `[canonical]`: entities created from mapped files and DB rows through the
target adapter, with the property qualification recorded per property (blocked properties imported as
retained raw values, never as canonical values); `[unresolved]`: the retained-opaque set as a
sub-manifest, byte-for-byte, with reasons, so a later re-target can re-run mapping. **(corr.)**
Retained opaque files are **never installed on another target automatically**: they have no family,
no identity and no destination representation there; a Build on any target other than the source
lists them as *carried, not installed*, and installing them anywhere requires a reviewed mapping first.

**Portability** — `[[portability]]` per candidate target: for each mapped entity, the family's
`OperationReadiness` there (blocked reasons verbatim), and the opaque set as carried-not-installed.

**Restoration of the source target** — **(corr.)** provenance primitives (corpus, bridge, manifest)
identify what was there; they do **not** reproduce it. A `[restoration]` section is only
`reproducible` when it lists: the reviewed destination path (or archive member) for every file
(including the C.2 write rule — owned step only, shared files excluded), the collision handling for
every path more than one entity renders (the measured `xjgw` collisions are the first real case:
reject, or a reviewed manual assignment recorded here), and the dependency order (database rows
before the files that reference them, archives after every member they must contain, references
updated in the same transaction per `InstallValue.updates_references`). Absent any of those it is
`not-reproducible` with the missing item named.

Nothing above is implemented; no schema file is written for it; no importer exists.

---

## E. Blockers and next batch

* NHL 10 rating regime — C1 finding above; both NHL 10 adapters blocked until the witness exists.
* `xjgw` composite: measured non-unique; identity stays a candidate until a committed member listing
  or a bridged extracted-tree census binds a rendered id to a file; ambiguity policy `reject` means
  the five colliding rows cannot be bound without manual resolution.
* Thumbnail chain: working-tree provenance only; promotion needs an attributable observation of the
  four files and per-shared-step multiplicity.
* `runtime_lookup` on every family: unknown; no chain changes that.
* Archive placement / `chunklzx` (FR14) still gates every Legacy Xbox 360 archive-member fact.

Recommended next: commit B and C after review (order per §C.7); C0-REG4 (`nhl15-xbox360` and the
NHL 07–09 Xbox 360 measurements); the C1 witness package for NHL 10; a logo-profile `/4` pass only
if a chain or composite is actually needed there (the crest/texlib pair is a variant question, not a
fallback — nothing to add today).

## F. Limits honoured

Evidence infrastructure parked (no generators, no `capability.csv`, no live publication). No pushes,
outside repositories, headmesh work, game installation, mounted-archive inspection or broad
hardening. No archive installation and no production conversion pipeline claimed. `/3` documents
unchanged in meaning. The working-tree thumbnail code stays working-tree only and establishes
nothing.

---

## G. C0-AS4-R1 — NHL 10 quarantine committed; asset resolution corrected (2026-09-15)

### G.1 Boundary B committed

| Item | Value |
|---|---|
| Commit | **`6ba9af4be0a297d88f29bbd3ae3787fbc9e5f8f4`** — `fix(canonical): quarantine the NHL 10 six-bit rating regime` |
| Staged tree (exported and tested) | `d7994fc2783ff6ab80034540dcf14e6329e5e038` = `git rev-parse 6ba9af4^{tree}` |
| Files | `defs/adapters/nhl10-xbox360.toml`, `crates/tdb-canonical/src/nhl10_ratings_conflict_tests.rs`, `crates/tdb-canonical/src/tests.rs` (staged from the tree), `crates/tdb-canonical/src/lib.rs` (staged as the **B-only variant** `as4/lib-b.rs` via `git hash-object -w` + `update-index --cacheinfo`; the exported blob equals that variant line for line — the variant file carried CRLF, the blob is LF-normalised like HEAD). All four SHA-256s matched §C.7 before staging; the staged `lib.rs` diff is exactly `+mod nhl10_ratings_conflict_tests;` (2 lines); `git diff --cached --check` clean |
| Isolated checks (exported tree, **fresh** `CARGO_TARGET_DIR = as4/target-b`, QA vars unset) | `cargo test -p tdb-canonical --locked --offline` **344 passed / 0 failed** (the three quarantine cases present); clippy `-D warnings` exit 0; fmt clean |
| Preservation | index empty after commit; the live combined `lib.rs` (`99f95a5c…`) untouched — it now differs from HEAD by C's 22 lines only; all boundary-C files and the 316 unrelated live entries preserved; nothing pushed |

### G.2 Defect 1 — shared-path exclusion compared templates, not destinations

**Reproduced** (`repro_owned_template_renders_onto_the_shared_destination`, before the fix): the
synthetic `/4` fixture with the owned template changed to `fe/jerseys/{composite-id}.big` (shared
family `fe/jerseys/custom.big`) validates clean — the templates differ as text — and
`resolve_write(…, tokens = {composite-id: "custom"}, exists = true)` returned
**`Ok(Existing { path: "fe/jerseys/custom.big" })`**: a write destination that *is* the shared
file, through the owned step.

**Correction** (`asset_resolution.rs`): before returning `Existing` or `Create`, `resolve_write`
calls `check_not_shared`, which renders **every `shared` step of every chain on the target**
(established or candidate — `shared_steps(target)`) with the same tokens and compares rendered
paths under `paths_alias`: `normalize_path` folds `\`/`/`, empty and `.` segments. *(R1 as
written treated a case-only difference as `Same` when both families declared a casing and
`CaseUndetermined` only when one was `preserve`; that was wrong — declared casing is a rendering
rule, not a statement about the destination's case sensitivity — and CLOSE, §H.1, makes every
case-only difference `CaseUndetermined`.)* Outcomes: `Distinct` → continue;
`Same` → **`WriteBlocker::AliasesSharedDestination { path, shared_family, chain }`**;
`CaseUndetermined`, or a shared family that needs a token the caller did not supply →
**`WriteBlocker::SharedDestinationUndetermined { …, reason }`** (refused, never guessed). Static
validation (duplicate templates, shared/owned relabelling) is kept as it was. Reads are unchanged:
a read may still fall back to a shared file.

Regressions: `an_owned_step_that_renders_onto_a_shared_destination_is_refused` (both `exists`
states; the read still finds it; distinct tokens still resolve),
`a_shared_destination_declared_through_another_chain_is_excluded_too` (a candidate, read-only
second chain's shared step excludes `fe/jerseys/league.big` from a write through the first chain;
`shared_steps` lists both), `normalisation_aliases_are_refused_and_undecidable_case_is_not_guessed`
(`\` vs `/` and `./` aliases refused; `Custom` under two lowering families is `Same`; under a
case-preserving owned family it is `SharedDestinationUndetermined` (and, since §H.1, so is
lower-vs-upper); a shared family needing
`{team-id}` is undetermined without the token and distinct with it).

### G.3 Defect 2 — creation inferred support from described formats

**Reproduced** (`repro_creation_with_reference_updates_is_allowed_on_identity_alone`, before the
fix): with `[install]` established (`loose-file`, `updates_references = true`), `[container]`
not-applicable and `[identity]` established, `resolve_write` on a missing owned file returned
**`Ok(Create)`** — `CreationSupport.references` took an operative identity as sufficient although
nothing in the schema represents a reference-update plan or capability.

**Correction** (`CreationSupport::of`), with the same distinction applied to the other two
predicates — *describing a format is not the capability to create in it*:

| Prerequisite | Before | After (C0-AS4-R1) |
|---|---|---|
| `install` | `[install]` operative | `[install]` operative **and** `kind = "loose-file"` — an archive install kind describes storage; inserting a new archive member is not represented |
| `container` | `[container]` operative **or** not-applicable | `[container]` **not-applicable only** — an established container describes the wrapping an existing file has; producing a new wrapped payload is not represented |
| `references` | `!updates_references`, or identity operative | `!updates_references` **only** — no reference executor exists and none is added; identity says what a reference is, not that anything writes it |

So creation is authorised for exactly one shape — a loose file, no container, no reference to
update — and every other combination returns `CreationUnsupported { missing }` naming the
prerequisites. The documented meaning of the results (on `CreationSupport`): `Create` = "this
loose path may be written new", nothing about bytes, game acceptance or references; `Existing` =
"this owned path exists and is not a shared destination", nothing about contents. This is logical
resolution — not an installer, not race-proof.

Regressions: `creation_requiring_reference_updates_is_blocked_even_with_established_identity`
(identity + install + container established, `updates_references = true` → `missing =
["references"]`; the existing file is still a replace destination),
`described_archive_or_container_formats_do_not_imply_creation_support` (loose-file case still
`Create`; `archive-replace-in-place` with its required established container → `["install",
"container"]`; loose install with an established `bigf` container → `["container"]`). The earlier
positive/negative creation cases still pass unchanged. The jersey profile's chain note now states
the corrected creation rule.

### G.4 Corrected boundary C — exact manifest (SHA-256 of working-tree bytes)

```
5bea53237821506771f27de542aa0b2fe5ea3e9f6029fb47e742adc578ce802c  crates/tdb-canonical/src/asset.rs                   M  +446/−5   (unchanged since C0-AS4)
263a741da2fb7d9bcbe256a61153bf92a1b228be495716b10c402701e4fa15bc  crates/tdb-canonical/src/asset_resolution.rs        A   764      (R1: exclusion, CreationSupport, docs)
883ac0e28b81aecc14ab33fff059fa9a551b7357f4f5fd42c9de757bee7ef24f  crates/tdb-canonical/src/asset_resolution_tests.rs  A  1405      (R1: +5 regressions)
f64c53192570e1a99357d55eb919a5662a06020da1dacc8eb711b547c61aefeb  crates/tdb-canonical/src/asset_tests.rs             M  +19/−9    (unchanged)
44e3a69867e0e780becbf94f6017b03aaf23014ef39069478ba4be03c0d042fa  crates/tdb-canonical/src/build_override.rs          M  +4/−4     (unchanged)
99f95a5c182753d4bb564a1ade811e6bba197a9995e19dd0574de5dda2422761  crates/tdb-canonical/src/lib.rs                     M  +22/−1 vs 6ba9af4 (the live file; B's line is now in HEAD)
99a3c43c5718a7b96dab878cfa6c28206cc9b5349836333c307426651994e6a4  defs/adapters/asset-profiles/assets-jerseys.toml    M  +189/−51  (R1: chain note)
```

Seven files; no manifest, lock or unrelated file. The two reproduction tests were replaced by the
regressions above (they are not kept as failing tests). Proposed commit:
`feat(canonical): asset-output-profile/4 composite identities and resolution chains` — the live
`lib.rs` stages as-is now that B is in HEAD.

### G.5 Verification

Candidate `as4/cc-r1` = `git archive 6ba9af4` + the seven files (manifest 1209 → 1211, the two
new sources the only additions; every overlay file byte-compared to the live tree before and after
the build — unchanged), **fresh** `CARGO_TARGET_DIR = as4/target-c-r1`, QA variables unset:

| Check | Result |
|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **365 passed / 0 failed** (344 at HEAD + 21 `asset_resolution_tests`, the five R1 regressions among them) |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty |

### G.6 Preservation checks

`/3` documents (`assets-portraits`, `assets-logos`) still validate in place and unchanged;
`/4` composite declarations and the jersey thumbnail chain stay non-operative candidates;
`runtime_lookup` stays independently required (`a_candidate_chain_enables_nothing_and_runtime_lookup_stays_required`
unchanged); the measured Legacy `xjgw` facts (698 of 703, `1_19_3` / `1_21_4` / `1_255_0`, art =
`lOnr`, nine zero art ids on team 0) unchanged and re-asserted; the §D import design unchanged; no
importer, archive installer, runtime promotion or headmesh work; evidence infrastructure parked;
index empty; C uncommitted; nothing pushed.

---

## H. C0-AS4-CLOSE — case-alias correction; boundary C committed (2026-09-15)

### H.1 Remaining defect — declared casing taken as destination case sensitivity

R1's `paths_alias(a, a_case, b, b_case)` returned `Distinct` for a case-only difference whenever
both families declared a casing (e.g. an owned `lower` family and a shared `upper` family), on the
reasoning that each had "already normalised". That is a statement about how each family *spells*
its output, not about whether the destination treats `fe/jerseys/custom.big` and
`FE/JERSEYS/CUSTOM.BIG` as one file — which no profile field states.

**Correction** (`asset_resolution.rs`): `paths_alias(a, b)` — the casing parameters are gone.
Exact normalised equality → `Same`; a difference beyond case → `Distinct`; **every** case-only
difference → `CaseUndetermined`, which `check_not_shared` turns into
`WriteBlocker::SharedDestinationUndetermined` before `Existing` or `Create`. No file-system policy
subsystem was added; the reason text now says the destination's case sensitivity is not stated by
any profile field. API comments on `PathAlias`, `paths_alias`, `normalize_path`,
`WriteBlocker::SharedDestinationUndetermined` and `check_not_shared` corrected; the jersey
profile's chain note corrected likewise.

**Regression** `a_case_only_difference_between_lower_and_upper_families_is_refused_not_declared_distinct`:
a validated profile with owned `fe/jerseys/{composite-id}.big` (`case = lower`) and shared
`FE/JERSEYS/CUSTOM.BIG` (`case = upper`); token `Custom` renders `fe/jerseys/custom.big` against
`FE/JERSEYS/CUSTOM.BIG`; `resolve_write` refuses it as `SharedDestinationUndetermined` for both an
existing and a missing destination; the distinct token `1_019_3` still resolves (`Existing`, or
the named creation blocker when missing — creation support is absent in that fixture). The
existing `normalisation_aliases…` case was updated to the parameterless API and asserts
`CaseUndetermined` for both `a/B` vs `a/b` and `A/B` vs `a/b`.

### H.2 Boundary C committed

| Item | Value |
|---|---|
| Commit | **`c3f4181b11fc28a93aa3c3f01e70bc246e51c867`** — `feat(canonical): asset-output-profile/4 composite identities and resolution chains` |
| Parent | `6ba9af4be0a297d88f29bbd3ae3787fbc9e5f8f4` |
| Tree | `e75785d874ebc02f1ca44fe4a30f17cea8a1b2af` = the staged `write-tree` exported and tested; re-read after the build, unchanged |
| Membership (7 files, +2 903 / −70; no manifest, lock or documentation) | `crates/tdb-canonical/src/asset.rs` `5bea5323…` (as §G.4) · `asset_resolution.rs` **`365b8d70…`** (CLOSE: casing rule) · `asset_resolution_tests.rs` **`5f7bd0a8…`** (CLOSE: +1 regression, updated aliases case) · `asset_tests.rs` `f64c5319…` (as §G.4) · `build_override.rs` `44e3a698…` (as §G.4) · `lib.rs` `99f95a5c…` (the live file, staged as-is; as §G.4) · `defs/adapters/asset-profiles/assets-jerseys.toml` **`d9ee7370…`** (CLOSE: chain note) — the three changed hashes are exactly the CLOSE correction; the other four match §G.4 |
| Staging | only those seven paths; `git diff --cached --check` clean; `git diff --cached --stat -- Cargo.toml Cargo.lock docs` empty |
| Isolated checks (exported staged tree, **fresh** `CARGO_TARGET_DIR = as4/target-c-close`, `QA_WRITE` / `QA_EVIDENCE_ROOT` unset) | `cargo test -p tdb-canonical --locked --offline` **366 passed / 0 failed** (344 at `6ba9af4` + 22 `asset_resolution_tests`); `cargo clippy … -- -D warnings` exit 0; `cargo fmt … -- --check` exit 0, empty; SHA-256 of every candidate file identical before and after the build; `git write-tree` unchanged after the build |
| After | index empty; live tree 309 unrelated modified/untracked entries preserved (no path under `crates/tdb-canonical` or `defs/adapters` modified); nothing pushed |

### H.3 Preservation

Unchanged and re-asserted by the committed tests: `/3` documents (`assets-portraits`,
`assets-logos`) validate in place; the jersey composite identity and thumbnail chain are
non-operative candidates; `runtime_lookup` is required independently of any chain; the measured
Legacy collisions (703 → 698; `1_19_3`, `1_21_4`, `1_255_0`); creation restricted to a loose file
with no container and no reference update; the NHL 10 quarantine; the corrected §D import design
(no importer). Deferred QA activation and the external headmesh workstream remain parked.

### H.4 Documentation reconciled (outside the commits; roadmap and STATE are untracked files)

`docs/plans/project-completion-roadmap.md` (item 7 row, C0 item 3, sequence item 1) and
`docs/STATE-OF-THE-REPO.md` (registry cell) now state: **14 of 28 database units** have a
committed partial adapter (`ef0fdd7`, `1ede5ba`, `58d1bc0`, `2ec9fde`, `26d5e44`, `6ba9af4`) —
12 measured on their own anchor and ROSTER fixture, 2 structural; all partial; `player.shoots` /
`player.team` blocked everywhere; NHL 10 rating regime quarantined on both platforms; none
game-verified — and **3 of 14 asset families** have a committed profile, of which only
`assets-portraits` has any established block; `assets-logos` and `assets-jerseys` (`0251b57`,
`c3f4181`) are **represented, not populated**: every block unknown, the composite and chain
candidates. Represented ≠ populated ≠ supported is stated in both documents.

### H.5 Remaining and next

Open: the NHL 10 C1 witness; a committed member listing or bridged census to bind a rendered
jersey id to a file (FR14 / `chunklzx`); runtime lookup evidence for every family; `nhl15-xbox360`
and `nhl10-xbox360` re-measurement on their fixtures; NHL 07–09 Xbox 360 snapshots. Recommended
next: C0-REG4 (the remaining gen7 pairs), then the C1 witness package.
