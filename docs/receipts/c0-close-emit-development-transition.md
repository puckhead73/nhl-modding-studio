# C0-CLOSE-EMIT — preserve completed work and switch to registry development

**Package:** C0-CLOSE-EMIT · **Date:** 2026-09-15 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **Baseline:** `58d1bc073bbf9e80d5afb4782bc6c132725e5fed`
**Commit made (authorized by this package):** `52a12fd1c0d1e4169e62951382dea6c24e17cd5f` · **Not pushed.**

> **Result.** (A) The file-symlink case is now `#[ignore]`d with its reason and **fails** when
> selected without the privilege (verified: `os error 1314`). (B) The reviewed emitter slice —
> exactly the 20 paths of EMIT1-R3 §5 with that one correction — is committed as `52a12fd1…`
> (parent `58d1bc07…`, tree `5f975f62…` = the tree that was tested). The staged `Cargo.lock` is
> HEAD's lock plus the single `+ "tdb-qa",` line; the live lock's 41 lines of unrelated drift are
> untouched. Root `Cargo.toml` is outside the commit. (C) Roadmap, STATE and CLAUDE.md record the
> user-directed sequence: registry development is the primary track; evidence activation is parked
> until the RC QA transition with its remaining tasks preserved, not completed. (D) A concrete next
> batch is proposed in §6.1: `C0-REG2` (six gen7 adapters from committed evidence) alongside a
> repository-backed portrait/logo/uniform coverage inventory (§6.2), which settles no feature scope.

---

## 1. Task A — the test-accounting correction

**Defect.** `a_file_link_at_the_fixture_itself_is_refused_where_the_platform_can_create_one`
printed `GAP: …` and returned `Ok` when `symlink_file` failed, so cargo counted a pass for a
branch that never executed (R3 receipt §3 reported it as a gap; the test binary did not).

**Change** — [crates/tdb-qa/tests/evidence_fixtures.rs](../../crates/tdb-qa/tests/evidence_fixtures.rs),
the only file that differs from the R3 boundary:

| Before | After |
|---|---|
| `try_file_link(..) -> bool`, `false` swallowed | `file_link(..) -> io::Result<()>` |
| case name `…_where_the_platform_can_create_one` | `a_file_link_at_the_fixture_itself_is_refused` |
| no attribute; early `return` on failure | `#[ignore = "needs file-symlink privilege (Windows: Developer Mode or elevation); run with --ignored"]`; on failure the fixture is restored and the case **panics** naming the OS error |
| — | doc comment states the prerequisite; the junction case's cross-reference updated |

The two junction cases (ancestor junction, junction at the final entry) stay enabled and executed;
they need no privilege (`mklink /J`). No path redesign, platform setup or privilege request.

**Executed counts** (isolated candidate, §3): default run `evidence_fixtures` **16 passed · 0
failed · 1 ignored**; deliberately selected (`--test evidence_fixtures -- --ignored a_file_link`):
**FAILED**, `this case was selected but the file symlink could not be created (missing platform
privilege?): A required privilege is not held by the client. (os error 1314)`, exit 101 — the
required behaviour. Log: `…/scratchpad/close-emit/logs/test-file-link-selected.log`.

## 2. Task B — preflight and staging

**Index empty, HEAD `58d1bc07…`** before staging. Live blob hashes against EMIT1-R3 §5:

| # | Path | R3 blob | Live blob | |
|---|---|---|---|---|
| 1 | `crates/studio-domain/Cargo.toml` | `af8e2b5d` | `af8e2b5d` | = |
| 2 | `crates/studio-domain/tests/surface_matrix.rs` | `e1586e25` | `e1586e25` | = |
| 3 | `crates/studio-domain/tests/mutation_matrix.rs` | `6e0117af` | `6e0117af` | = |
| 4 | `crates/studio-domain/tests/operation_matrix.rs` | `26e14def` | `26e14def` | = |
| 5 | `crates/studio-domain/tests/save_matrix.rs` | `51e693cf` | `51e693cf` | = |
| 6 | `crates/tdb-qa/src/bin/qa_evidence.rs` | `a145e738` | `a145e738` | = |
| 7 | `crates/tdb-qa/src/evidence/merge.rs` | `110882d7` | `110882d7` | = |
| 8 | `crates/tdb-qa/src/evidence/model.rs` | `1b5ecf96` | `1b5ecf96` | = |
| 9 | `crates/tdb-qa/src/evidence/publish.rs` | `88603971` | `88603971` | = |
| 10 | `crates/tdb-qa/src/evidence/store.rs` | `0a2dd32f` | `0a2dd32f` | = |
| 11 | `crates/tdb-qa/src/evidence/testkit.rs` | `b7271328` | `b7271328` | = |
| 12 | `crates/tdb-qa/tests/evidence_discovery.rs` | `73e8e0d9` | `73e8e0d9` | = |
| 13 | `crates/tdb-qa/tests/evidence_emit.rs` | `241cf72a` | `241cf72a` | = |
| 14 | `crates/tdb-qa/tests/evidence_file_reads.rs` | `57c661c6` | `57c661c6` | = |
| 15 | `crates/tdb-qa/tests/evidence_journal.rs` | `8520616e` | `8520616e` | = |
| 16 | `crates/tdb-qa/tests/evidence_publication.rs` | `c1b83602` | `c1b83602` | = |
| 17 | `crates/tdb-qa/tests/evidence_fixtures.rs` (A) | `97cd09ee` | **`347ccb7d`** | **deliberate — §1 only** |
| 18 | `crates/tdb-qa/tests/evidence_freshness.rs` (A) | `c6d1d077` | `c6d1d077` | = |
| 19 | `defs/qa/scopes.toml` | `1a2bf8a1` | `1a2bf8a1` | = |
| 20 | `Cargo.lock` | `8decfca0` | live `decc4412` (drift) | staged from HEAD's lock, see below |

**Staged lock.** Built from `git show HEAD:Cargo.lock` (`bb02122…`) by inserting ` "tdb-qa",` into
the `studio-domain` dependency list; `diff` against HEAD's lock is exactly `3372a3373 > "tdb-qa",`;
resulting blob `8decfca03723054b7e71e9b62683c44fab5e334d` = R3's recorded blob. Staged with
`git hash-object -w` + `git update-index --cacheinfo`, so the live `Cargo.lock` (41 unrelated
lines, blob `decc4412`) was never touched and is still modified in the working tree.

**Staging.** 19 paths via `git add` by name + the lock as above → `git diff --cached --name-status`:
**18 M · 2 A · 0 D**, 20 files, +3 561 / −325. `git diff --cached --check`: clean. Index tree
`git write-tree` = `5f975f62e0dc4e13fe0e493478de90a0a079afdb`.

## 3. Isolated verification of the staged tree

Candidate = `git archive 5f975f62` extracted to `…/scratchpad/close-emit/candidate` (1 194 files;
files CRLF per the repository's normalisation, blobs identical to the index after CRLF→LF, spot
checked on the lock, `evidence_fixtures.rs`, `store.rs`). Dedicated `CARGO_TARGET_DIR`,
`--locked --offline`, no `QA_*` variable set. SHA-256 manifest before and after.

| Check | Result |
|---|---|
| `cargo test -p tdb-qa` | **253 passed · 0 failed · 2 ignored** — ignored: `evidence_fixtures::a_file_link_at_the_fixture_itself_is_refused` (new, §1) and `evidence_migration::real_headmesh_registry_integration_over_the_committed_registry` (pre-existing: registry not committed). Per binary: lib 72, acceptance 32, discovery 13, emit 9, file_reads 9, fixtures 16+1i, freshness 7, journal 31, migration 19+1i, publication 36, inventory 9 |
| selected ignored case | **FAILED** as required (§1) |
| `cargo clippy -p tdb-qa --all-targets -- -D warnings` | clean |
| `cargo fmt -p tdb-qa -- --check` | clean |
| `rustfmt --check` on the four `studio-domain` matrix tests | clean |
| `cargo clippy -p studio-domain --test surface_matrix --test mutation_matrix --test operation_matrix --test save_matrix` | **0 diagnostics in the four overlay files** (exit 0; the only output is the dependency-lib baseline below) |
| `cargo clippy -p studio-domain --all-targets -- -D warnings` | **fails at HEAD baseline — not clean, not repaired.** `-D warnings` stops at `tdb-archive-codec` (12); without deny, cargo reports `tdb-archive-codec` lib 12, `tdb-rx2` lib 53, `tdb-fhm-store` lib 2, **`studio-domain` lib 8** (unchanged from the C0.2/R3 baseline: assets, attributes, discovery, equipment_texture, formula, head_catalog, view), plus `studio-domain` tests `nhl07_encodings` 1, `ratings_scale` 2, `workspace` 1, `read_matrix` 1 warning + **1 deny-by-default error** (`overly_complex_bool_expr`, `read_matrix.rs:305` — HEAD content, not in this slice) |
| retained R2 runs: `qa-evidence build --repo <candidate> --root <R2 evroot>` ×2 | `fixtures: 148 attributed path(s), 148 file(s) hashed, 466003284 byte(s)`; both builds **byte-identical** to each other and **to R2's `out1`** (10 files + `build-inputs.json`) |
| retained R2 runs: `qa-evidence check --root <R2 evroot> --data <R2 data-scratch>` | **`ok: 2175 row(s), no validation failure, no drift`**, exit 0 |
| emitter recapture / matrix execution | **not run** — no capture-path input changed (emitters, `emit.rs`, `scopes.toml`, matrix tests byte-identical to R3) |

**Preservation.** Candidate manifest before = after (1 194 files); R2 evidence root 28 files
unchanged; live `anchors/**` + `docs/audits/**` 621 files, 0 mismatches against the R3 manifest;
`docs/audits/evidence` absent; nothing under `docs/audits/data` touched.

## 4. The commit

```
commit 52a12fd1c0d1e4169e62951382dea6c24e17cd5f
parent 58d1bc073bbf9e80d5afb4782bc6c132725e5fed
tree   5f975f62e0dc4e13fe0e493478de90a0a079afdb   (= the tested candidate tree)
feat(qa): integrate matrix capture and evidence freshness checks
20 files changed, 3561 insertions(+), 325 deletions(-)
```

Message via `git commit -F` (scratch file), ending with the required `Co-Authored-By` line. After
the commit: **index empty**; `git status --porcelain` 269 → **250** lines (the 19 source paths now
match HEAD; `Cargo.lock` remains modified with its unrelated drift, blob `decc4412`); root
`Cargo.toml` still modified and outside the commit; **not pushed**. Documentation (this receipt,
the EMIT1…R3 receipts, roadmap, STATE, CLAUDE.md) is outside the commit as instructed.

## 5. Task C — recorded direction (uncommitted documentation)

| File | Change |
|---|---|
| [docs/plans/project-completion-roadmap.md](../plans/project-completion-roadmap.md) | C0 item 4 row: committed cell adds `52a12fd`; activated cell "parked by direction until C9"; unmet-item 4 bullet records the sequencing decision; "What is committed" paragraph updated; new **Direction update 2026-09-15** block before C1 with the five-step sequence, the parked-task list, the asset-registration requirement (ingestion **and** destination output: canonical identity, storage/archive location, formats and constraints, references/dependencies, conversion, installation, validation), the external completed-mod ingestion requirement (baseline comparison where available, provenance and unresolved files preserved, independent canonical Project, portability limits reported), the deferred-asset-operations rule, and headmesh remaining external; §6 "Next" rewritten |
| [docs/STATE-OF-THE-REPO.md](../STATE-OF-THE-REPO.md) | QA evidence row: counts from this run, `52a12fd` committed, parked-until-C9 status with the open task list |
| [CLAUDE.md](../../CLAUDE.md) | new short section "Current development track (direction update 2026-09-15)" ahead of the architect-workflow section |

**Parked QA tasks (preserved, none complete):** storage/retention policy for evidence trees (U3,
layer-A retention, FR2/FR3 storage); migration installation (`mig-c0-bulk1`) and first publication;
consumer transition of `game-qa-matrix.csv`, `matrix_guard` and the emitter CSVs under
R-TRANSITION; `conversion.class`/recipe-id vocabulary and variant freeze (U2, D8, D9; the 2
unmapped `uniforms` claims, D7); the `qa-evidence check` CI gate (and Windows CI, C0.2 R6). All
historical receipts preserved unchanged.

## 6. Task D — proposed next development batch

### 6.1 `C0-REG2` — six gen7 database adapters from committed evidence

**Units (all have a committed schema snapshot, main anchor + `*-meta.xml`, an audit document, and
a committed ROSTER save fixture the production save codec already decodes for this framing):**

| Unit | Snapshot | ROSTER fixture in `anchors/` | Twin already committed | Why in this batch |
|---|---|---|---|---|
| `nhllegacy-ps3` | `ps3-nhllegacy.json` | `BLUS315400200 - ROSTER` | `nhllegacy-xbox360` (`1ede5ba`) | the REG1b PS3-twin mechanic verbatim; byte-identity of DB/metadata to be asserted, not assumed |
| `nhl14-xbox360` | `xbox360-nhl14.json` | `ROSTER 20260627054746` | — | first non-anchor-pair title of the `c_*`/`p_*` boundary; the snapshot decides which ratings set it carries |
| `nhl14-ps3` | `ps3-nhl14.json` | `BLUS311790200 - ROSTER` | pairs with the row above | PS3-twin mechanic |
| `nhl13-xbox360` | `xbox360-nhl13.json` | `ROSTER 20260627051601` | — | contiguous era coverage NHL 11–14 |
| `nhl12-xbox360` | `xbox360-nhl12.json` | `ROSTER 20260627052113` | — | same |
| `nhl11-xbox360` | `xbox360-nhl11.json` | `ROSTER1 20260627054323` | — | closes the gap to the committed `nhl10-xbox360` |

**Mechanics reused (no new machinery):** `target_evidence` parameterised test module; the
anchor-snapshot cross-check with target/snapshot identity guard; capacities read as the ROSTER
fixture's table directory through `tdb-savedata`; birth-date origin, main-team encoding, companion
partition and position discriminator measured on the target's own anchor; `team.name` round trip
automated; `registry_population` reporting established/unknown/candidate apart. Expected shape per
unit: ~8 canonical properties / ~10 physical fields / 8 capacity blocks, as REG1a/REG1b.

**Bounds:** evidence already committed only — no new fixtures, no mounted trees, no
`NHL_STUDIO_FIXTURES`; `player.shoots` / `player.team` stay blocked unless the snapshot itself
supplies what Legacy/NHL 15 lacked; unknowns explicit; adapter presence ≠ target completion; no
capability promotion. One package, one commit after review (≈6 adapter TOMLs + 6 test modules +
`lib.rs`/`tests.rs`/`snapshot_tests.rs` edits). Follow-ons, not in this batch: `nhl10-ps3`…
`nhl13-ps3` (REG3, same mechanic); PS4 ×7 (needs the different capacity mechanic — the PS4 main DB
carries the ceiling directly; the LZ4 save codec is an open unknown); NHL 07–09 (gen7 ceilings
deferred; NHL 07 gen6 schema); Legacy Recompiled (no independent anchor, FR4).

### 6.2 Repository-backed inventory — portraits, logos, uniforms (ingestion and output)

Sources: `crates/studio-domain/src/assets.rs` (6 459 lines), `studio-api/src/registry.rs`
(`import_texture`, `import_texture_bytes`), `tdb-rx2` (`xpr`, `gnf`, `texlib`, `dds`),
`tdb-archive-big` (`eb.rs`: decode + entry encode only), `defs/adapters/asset-profiles/`,
`docs/audits/data/asset-capability-matrix.csv`, `docs/audits/data/assets/nhllegacy/xbox360/*`,
C0-05 register FR14/FR17, C0.2 §5.4. **Levels are implemented / automated-tested / blocked /
missing-fixture as defined in CLAUDE.md; no cell is game-verified.** This inventory does not settle
remaining feature scope (C4, Q09–Q12 own that).

| Axis | Portraits (`assets-portraits`) | Logos (`assets-logos`) | Uniforms (`assets-jerseys`) |
|---|---|---|---|
| **Canonical identity** | universal slug `lastname_firstname_dd_mm_yyyy` (`tdb-core::dev_key`, tested); `ubPc.lOnr` → `p{id}.big` bucket map (tested `portrait_bucket_covers_shipped_range_only`) | `kOtt.lOnr` art id → FE crest `t{artid}.big` / rink texlib; Legacy X360 identity map committed (`identities.csv`, 252 teams) | `xjgw` kit rows `{style}_{art}_{variant}`; team-id-token filename keying (RW) / team-name keying (PS4); tested synthetically (`outfit_output_name…`, `ps4_outfit_naming…`) |
| **Storage / archive location (discover)** | X360/PS3 `fe/ion/artassets/playerheads[small]/p{bucket}/p{id}.big`; PS4 NHL 15 `assets/portraits/p{id}.png`; PS4 16+ `kettle_assets/playerportraits/p{id}.gnf` — **extracted trees only**; in-archive members (PS4 EB `inxp_00.big`, X360 Legacy EB) **not located** (FR14 open); PS3 `patch.big` shadowing unexamined (FR17) | X360/PS3 FE `teamlogos/t{artid}.big` + `rendering/logo/texlib_*.rx2` fallback; **PS4 logo path: none found in code** | RW `rendering/{jersey,pant,sock}/` texlibs; FE `jerseys/jersey_{id}.big` thumbnails with `custom*.big` fallback chain; PS4 kettle `characters/player/*.gnf` |
| **Formats / constraints (identify, decode)** | BIGF→RefPack→XPR2 (X360) / DDS (PS3) largest-raster rule; PNG; tiled BC7 GNF — all implemented; profile `assets-portraits` **3/28 target profiles, 7 established constraints on `nhllegacy-ps3` only, all `observed-source-format`**, 0 `supported-target-constraint` | RW4 texlib DXT5; BIGF crest — implemented; **no profile** | RW4 texlib bundles (body + `font` raster, `name_*` marker sheets); GNF — implemented; **no profile**; D7 (`uniforms` rows ownership) undecided |
| **Ingestion (source → target bytes)** | `decode_source_image` (PNG/JPEG…) → `replace_portrait_texture` (XPR2 same-size/resized; PS3 DDS full mip chain) → `import_texture` with safe-join + one-time `.bak`; **source library index, import/match review, persistent mappings: not implemented** (C4 text) | `import_texture` → `texlib::replace_*` | same texlib path; nameplate editor via `import_texture_bytes`; `clone_kit_textures` (Add New Jersey file half; NHL 08 whole-file RefPack cannot be string-rewritten) |
| **Destination output / installation** | **extracted-file overwrite only** (`capability.csv` install = `external`); GNF write is **base-mip patch** (`replace_gnf_texture`) — mip-chain regeneration open (C4); PS4 NHL 15 **PNG is decode-only** (no write container); **no archive-backed write** on any platform (C3a not started; `tdb-archive-big` has no writer beyond entry `encode`; Legacy X360 EB chunklzx decode TBD) | extracted `.rx2` overwrite only | extracted `.rx2` / `.gnf` overwrite only; bulk PNG export (`export_team_outfits`) |
| **Validation** | 4-aligned DXT dims; container magic dispatch (GNF/BIGF/RW4); Q10 spec-preservation (mip count/layout, tiling, sRGB, encoded size) **not a shared validator** | same | same |
| **Automated evidence** | synthetic: `ps3_dds_portrait_write_roundtrip_synthetic`, `ps4_gnf_bc7_portrait_write_roundtrip_synthetic`, bucket/generic tests; **mounted-tree skips** (`X:\Games\…\NHL Legacy`, `H:\Emulators\ShadPS4\…`): `legacy_x360_extracted_portrait_write_roundtrip`, `ps4_gnf_bc7_portrait_write_roundtrip_real`, `ps4_portrait_png_decodes` — silent early returns (C0.2 §5.4 class) | mounted-tree skips: `legacy_x360_team_logo_identity_map`, `legacy_x360_team_logo_uses_fe_crest` | synthetic naming/label/filter tests; mounted-tree skips: `legacy_x360_list_team_outfits_surfaces_font_raster`, `legacy_x360_outfit_and_equipment_decode_from_extracted_tree` |
| **Recorded evidence** | `asset-capability-matrix.csv` + `assets/nhllegacy/xbox360/capability.csv`: one target (`nhllegacy/xbox360`, 2026-08-03), legacy CSV format, predates the evidence system; consumer transition parked (§5) | same one target | same one target |
| **External completed-mod ingestion** | **not implemented** — no baseline comparison, no provenance record, no unresolved-file preservation, no Project creation from a mod, no portability report | same | same |

**Gaps that are independent of any external project path and can be worked now:** (1) 13/14
asset families have no profile; `assets-logos` and `assets-jerseys` profiles can be *represented*
from the Legacy X360 records already committed (identities, archives, capability CSVs) with every
target-level block `unknown` — representation, not population, and reported as such; (2) the
ingestion/output contract for the three families is not written down anywhere as a registry object
— the roadmap's asset-registration requirement (§5) has no schema field for storage/archive location,
installation or validation yet, so `asset-output-profile/3` needs a reviewed extension before the
"destination output" half can be registered; (3) the mounted-tree asset tests can move behind
`NHL_STUDIO_FIXTURES` with fail-on-absence as C0-FX1 did for `tdb-rx2` (baseline hardening, step 3
of the sequence); (4) an external-mod ingestion design has no code and no decision record — it
needs a package of its own after the contract extension in (2). **Not claimed:** any PS4 logo
path, any PS3 logo/uniform test, any in-archive write, or any cross-title asset compatibility.

### 6.3 Recommended batch

Run **`C0-REG2` (§6.1)** as the next package, and in the same package (or a paired `C0-ASR1`) add
the **contract extension** for asset registration (ingestion + destination-output fields on the
profile schema, evidence-scoped like the existing constraints) with `assets-logos` and
`assets-jerseys` represented from committed Legacy X360 records. The database half reuses REG1b's
mechanics six-fold; the asset half produces the registry object the direction requires before any
population. Neither depends on an external project path, a mounted tree or a private fixture.

## 7. Limits observed

No push; no staging beyond the 20 named paths; no `QA_WRITE`; no live evidence migration or
publication; `docs/audits/data` and `docs/audits/evidence` untouched; no CI change; no game
installation; no headmesh work; no other repository inspected; no registry expansion performed
(only proposed); baseline lint failures reported, not repaired; the R2/R3 retained evidence roots
and all three candidates unchanged. Scratch: `…/scratchpad/close-emit/{candidate,target,out1,out2,logs,commit-msg.txt,Cargo.lock.{head,staged}}`.

**Stop for Codex review.**
