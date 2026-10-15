# C0.5 — Fixture and runtime register

> **Package:** C0-BULK1 checkpoint 5, fulfilling roadmap [C0](../plans/project-completion-roadmap.md)
> item 5. **Date:** 2026-09-07. **Author role:** developer. Reviewer: Codex (architect).
> **Type:** inventory over authorized repository evidence and explicitly documented locations.
> **No fixture was acquired, copied, extracted or committed. No game tree was written. No boot probe
> was run and no runtime was installed or launched. No capability was promoted.**
>
> **Nothing here is qualification.** A registered fixture is an input that exists; it says nothing
> about whether any operation over it has been executed, let alone verified.

---

## 1. Method, and what it does and does not cover

Built from, and only from:

| Source | Used for |
|---|---|
| `anchors/` and the generated [`anchors-manifest.csv`](../audits/data/anchors-manifest.csv) | committed database, save and localization fixtures, with their SHA-256 |
| [C0.2 §3.3, §4.4, §4.5](c0-02-baseline-execution.md) | hardcoded external paths, silent skips, live private data read during the baseline |
| [C0-FX1](c0-fx1-explicit-fixture-tests.md) | the six explicit private-fixture keys and the seven remaining mounted-data gates |
| The roadmap's [§6 runtime inventory](../plans/project-completion-roadmap.md) (user-confirmed 2026-09-05) | installed runtimes |
| A read-only scan of `crates/**/*.rs` performed for this register | current hardcoded absolute-path literals and `#[ignore]` counts |
| `defs/appearance/headmesh-matrix.toml` | conversion-pair inputs |

**Not covered, deliberately:**

- No filesystem outside the repository was inspected to decide whether a fixture exists. Where this
  register says *not found*, it means **not found at the inspected in-repo locations** — never *does
  not exist anywhere*. That distinction is carried in every row.
- No mounted game install was enumerated, hashed or opened.
- No emulator was launched, and no runtime build string was read from any installed runtime.
- Nothing was written anywhere outside this document.

### 1.1 Vocabulary

| Term | Meaning |
|---|---|
| **committed** | the bytes are in this repository and are hashed in `anchors-manifest.csv` |
| **private-explicit** | a named key under `NHL_STUDIO_FIXTURES`; the layout is documented, the bytes are not in the repo |
| **mounted-implicit** | a hardcoded absolute path into a private game install; the code reaches it directly, with no register and no configuration |
| **not found (in-repo)** | searched for inside the repository and absent. Says nothing about any other machine or volume |
| **owner-unassigned** | no accountable person is recorded. Assigning one is an architect action, not a developer inference |

---

## 2. Register A — main-database fixtures (committed)

27 main databases, one per anchor unit; each is hashed in `anchors-manifest.csv` (`role =
main-database`), with its schema metadata (`role = schema-metadata`, 27 files).

| Platform | Titles present | Units | Storage policy | Owner |
|---|---|---|---|---|
| `ps3` | nhl07 nhl08 nhl09 nhl10 nhl11 nhl12 nhl13 nhl14 nhl15 nhllegacy | 10 | **committed** — plain git binaries, EA data policy rule 2 | owner-unassigned |
| `xbox360` | nhl07 nhl08 nhl09 nhl10 nhl11 nhl12 nhl13 nhl14 nhl15 nhllegacy | 10 | committed | owner-unassigned |
| `ps4` | nhl15 nhl16 nhl17 nhl18 nhl19 nhl20 nhl21 | 7 | committed | owner-unassigned |

`anchors/` totals **552 files**: 27 main-database, 27 schema-metadata, 38 aux-database,
25 localization-database, 26 localization-metadata, 243 save-data, 84 save-icon, 82 save-param.

**Two known gaps, both structural, neither a missing file:**

| Gap | Statement | Blocker | Next action |
|---|---|---|---|
| `legacyrecomp-recomp` | **No anchor of any kind.** Legacy Recompiled shares the Xbox-derived Legacy schema and has no independent database in the tree | no independent Recompiled database fixture | acquire or prove equivalence; roadmap §1 already requires independent runtime/save evidence regardless |
| `nhl07-ps3` | Present as a **research copy**, not a shipping target (roadmap §1). Under the R2 evidence contract every write and acceptance cell on it is `not-applicable` with that cited reason | none — this is scope, not a gap | none |

Hashes: per file in `anchors-manifest.csv`. Not restated here; duplicating 552 hashes into prose
creates a second source that can silently diverge.

---

## 3. Register B — save fixtures (committed)

Save-data file counts per unit, measured under `anchors/<platform>/<game>/savefiles/`:

| Unit | Files | Unit | Files | Unit | Files |
|---|---:|---|---:|---|---:|
| ps3/nhl07 | **0** | xbox360/nhl07 | 3 | ps4/nhl15 | 15 |
| ps3/nhl08 | 8 | xbox360/nhl08 | 2 | ps4/nhl16 | 25 |
| ps3/nhl09 | 12 | xbox360/nhl09 | 5 | ps4/nhl17 | 25 |
| ps3/nhl10 | 18 | xbox360/nhl10 | 5 | ps4/nhl18 | 30 |
| ps3/nhl11 | 18 | xbox360/nhl11 | 6 | ps4/nhl19 | 25 |
| ps3/nhl12 | 18 | xbox360/nhl12 | 6 | ps4/nhl20 | 25 |
| ps3/nhl13 | 24 | xbox360/nhl13 | 6 | ps4/nhl21 | 25 |
| ps3/nhl14 | 30 | xbox360/nhl14 | 6 | | |
| ps3/nhl15 | 30 | xbox360/nhl15 | 6 | | |
| ps3/nhllegacy | 30 | xbox360/nhllegacy | 6 | | |

- **26 of 27 anchor units carry save fixtures**; `ps3/nhl07` carries **none**. This is the same
  single-unit gap that made `save-matrix.csv` cover 26 of 27 units (C0.4a §1.2), and it is why the
  PS3 NHL 07 `save-raw-db` claim in the legacy matrix cites a gate with no rows for that unit
  (defect E4).
- PS4 save containers appear as directory trees (`Season0/`, `Profile0/`, `BeAPro0/`, `BeAGM0/`,
  `ThreeOnThree0/`, `RosterCustom0/` with `sce_sys/`); Xbox 360 and PS3 as inner payload files.
- **Mode coverage is not established here.** The directory names suggest Season, Franchise/Be A GM,
  Be A Pro, roster and profile saves, but which required mode (Q03: roster, Franchise, Season,
  settings) each fixture actually is has **not** been verified per title, and this register does not
  assert it. That mapping is C2 work.

| Blocker | Next action | Owner |
|---|---|---|
| `ps3/nhl07` has no save fixture | acquire one, or record the unit as save-fixture-blocked (it is research-only, so it may never need one) | owner-unassigned |
| Per-title save-mode identification is unverified | inventory each fixture's actual mode before any save-mode qualification | owner-unassigned |

---

## 4. Register C — outer console packaging (Q04)

| Item | State |
|---|---:|
| Outer-package fixtures in the repository | **none — 0 files** |
| Xbox 360 STFS | recorded **blocked**; not implemented |
| PS3 / PS4 outer packages | no fixture, no implementation |

The anchors expose **inner** save payloads only. Q04 makes original-console packaging required on
Xbox 360, PS3 and PS4, so under the R2 contract these cells are `applicability = required` and
`unqualified` on every shipping target — never `not-applicable` (R-Q04).

| Blocker | Next action | Owner |
|---|---|---|
| No outer-package fixture of any kind exists in the repository | decide acquisition policy: outer packages are user-supplied console artefacts and may not belong in git under EA data policy rule 2 | architect |

---

## 5. Register D — asset fixtures

### 5.1 Private-explicit — the six `NHL_STUDIO_FIXTURES` keys

Layout and per-test mapping: [C0-FX1 §3.3](c0-fx1-explicit-fixture-tests.md). All six are **not
found (in-repo)**; C0-FX1 verified this by searching the whole repository for each filename (0 hits).

| Key | Used by | Cases | Present in repo | Storage policy |
|---|---|---:|---|---|
| `mesh/player_20500.rx2` | `tdb-rx2` `container`, `pack`, `playermesh` | 10 | no | private, never committed |
| `legacy/texlib_67.rx2` | `tdb-rx2` `texlib` | 6 | no | private |
| `legacy/centerlogo_0_cm.rx2` | `tdb-rx2` `texlib` | 1 | no | private |
| `legacy/playerheadssmall_p1512.xpr2` | `tdb-rx2` `xpr` | 5 | no | private |
| `equipment-preview/feprop_glove_0.rx2` | `tdb-rx2` root module | 2 | no | private |
| `equipment-preview/helmet_14_2.rx2` | `tdb-rx2` root module | 2 | no | private |

**26 cases** depend on these keys. All 26 are `#[ignore]`d and **fail loudly** when selected without
a resolvable fixture — they can no longer report success without asserting. **All 26 are recorded
`not run`.** They are registered blockers, not coverage.

> **A returned-early test is not executed coverage.** Before C0-FX1 these same cases reported `ok`
> while asserting nothing. Nothing in this register describes them as covered.

#### 5.1a The two former `texlib_67` sources — kept separate

C0-FX1 §2.2 collapsed two *lookup paths* onto one key:

| Former lookup | Nature |
|---|---|
| `E:\Repositories\nhl-database-studio\Working Assets\Template Texture\NHL Legacy\texlib_67.rx2` | the prior application's working copy — **off limits** under CLAUDE.md hard rule 1 |
| `H:\Emulators\games\Decompiled\Original\rendering\player\texlib_67.rx2` | an operator cache, recorded as no longer mounted |

They share a **logical role**: the NHL Legacy `rendering/player/texlib_67.rx2`, and the
dimension/format assertions on either side agree (hair 256×256 DXT3, head 512×512 DXT1).

**Their physical provenance is unresolved and is recorded as unresolved.** Neither file is available,
so byte identity has **not** been established and is **not** claimed here. The single key
`legacy/texlib_67.rx2` is a *test-configuration* convenience; it is not an assertion that the two
files were the same bytes. If both ever become available they must be hashed and compared before
either is treated as the other.

### 5.2 Mounted-implicit — hardcoded absolute paths still in the source

A read-only scan for `"<drive>:\…"` literals in `crates/**/*.rs`:

| Metric | Value |
|---|---|
| Files containing at least one literal | **11** |
| Lines containing a literal | **48** |
| Distinct path literals | **30** |

| File | Occurrences |
|---|---:|
| `crates/studio-domain/src/assets.rs` | 21 |
| `crates/studio-domain/src/head_catalog.rs` | 8 |
| `crates/studio-domain/src/extract.rs` | 3 |
| `crates/studio-domain/src/head_preview.rs` | 3 |
| `crates/studio-domain/src/kit_preview.rs` | 3 |
| `crates/tdb-archive-big/src/eb.rs` | 2 |
| `crates/tdb-rx2/src/gnf.rs` | 2 |
| `crates/tdb-rx2/src/mcd.rs` | 2 |
| `crates/tdb-rx2/src/texlib.rs` | 2 |
| `crates/studio-domain/src/discovery.rs` | 1 |
| `crates/studio-domain/src/equipment_texture.rs` | 1 |

Roots, and what each stands for:

| Root | Titles reached | Nature |
|---|---|---|
| `X:\Games\Emulation\Roms\XBOX360\…` | NHL 07, 08, 09, 10, 11, 12, 13, 14, 15, Legacy | extracted Xbox 360 game trees (`rendering/`, `data0.big`, `fe/ion/artassets/`) |
| `H:\Emulators\ShadPS4\Games\<CUSA>\…` | CUSA00561, CUSA01909, **CUSA11126**, CUSA15439, CUSA18182 | shadPS4 installs, including `databigs/Extracted/nhl/data/kettle/rendering` extracted trees |
| `X:\rendering\player\…`, `X:\rendering\coachhead\…` | unattributed | bare paths with no title in them — **which title these belong to is unknown** and is not guessed here |

**Comparison with C0.2 §3.3.** C0.2 recorded *33 distinct hardcoded paths across 11 source files*.
This scan finds **30 distinct literals across the same 11 files**. Two of the four roots C0.2 listed
are gone: `E:\Repositories\nhl-database-studio\…` and `H:\Emulators\games\Decompiled\…` were removed
by C0-FX1. The remaining difference is a counting-method difference (C0.2 counted paths, this counts
distinct string literals) and is **not** claimed to be a reduction beyond the C0-FX1 removals.

**One unresolved discrepancy, recorded not fixed:** a hardcoded path names shadPS4 content id
**`CUSA11126`**, while `discovery::ps4_title_ids` maps `nhl19` to **`CUSA11116`**. One digit differs.
Which is correct — or whether both are real regional variants — is **unknown**; nothing here resolves
it, and no code was changed.

### 5.3 The seven remaining mounted-data gates

[C0-FX1 §5.4](c0-fx1-explicit-fixture-tests.md) lists seven `tdb-rx2` cases that still address a
mounted private game tree by absolute path and **return early and report `ok`** when it is absent:

| Case | Root |
|---|---|
| `gnf::tests::nhl16_portrait_bc7_decodes` | `H:\Emulators\ShadPS4\Games\CUSA01909\…` |
| `gnf::tests::parses_real_head_cm_dimensions` | `H:\Emulators\ShadPS4\Games\CUSA00561\…` |
| `mcd::tests::decodes_real_logo_mesh` | `H:\Emulators\ShadPS4\Games\CUSA00561\…` |
| `mcd::tests::decodes_real_player_head` | `H:\Emulators\ShadPS4\Games\CUSA00561\…` |
| `mcd::tests::decodes_real_nhl16_player_head` | `H:\Emulators\ShadPS4\Games\CUSA01909\…` |
| `texlib::tests::nhl07_award_argb8888_is_xbox_tiled` | `X:\Games\Emulation\Roms\XBOX360\NHL 07\…` |
| `texlib::tests::nhl09_nameplate_argb8888_is_linear` | `X:\Games\Emulation\Roms\XBOX360\NHL 09\…` |

Plus the wider set: **27 cases** read live private game data during the C0.2 baseline (C0.2 §4.5),
across `studio-domain` and `tdb-archive-big` as well as `tdb-rx2`.

| Blocker | Next action | Owner |
|---|---|---|
| These cases pass or skip depending on what is mounted, silently | the conversion is mechanical and the `tdb-rx2` fixture module already supports it. It needs **an architect decision on whether a mounted game install belongs behind the same fixture root** as private files — C0-FX1 §5.4 referred exactly this question here | **architect** |

**No policy change is proposed or made by this register**, per the package's limits.

### 5.4 Extracted trees

Referenced by the source as a distinct fixture class, not as loose files:

| Tree | Referenced shape |
|---|---|
| shadPS4 extracted art | `<root>/Games/<CUSA>/databigs/Extracted/nhl/data/kettle/rendering`, `…/nhl/ui/assets/portraits/` |
| Xbox 360 extracted game | `<root>/<title>/rendering/…`, `data0.big`, `fe/ion/artassets/…` |
| PS4 `.big` containers | `databigs/inxp_00.big` (CUSA15439), `databigs/overlays_20.big` (CUSA18182) |

Each depends on an **extraction step the register cannot verify**: whether the tree is present, and
whether it was produced by Studio or by another tool, is unknown per machine and is not asserted.

### 5.5 Audio

| Item | State |
|---|---|
| Audio fixtures in the repository | **none** |
| Audio codecs/banks implementation | **unimplemented** (roadmap §2) |
| Registered need | format discovery, codecs, bank editing, preview/export/replace/install |

**Missing, and legitimately so** — there is nothing yet to fixture. Recorded as a blocker with no
next action beyond C7 starting.

### 5.6 Conversion-pair inputs (headmesh)

`defs/appearance/headmesh-matrix.toml` declares **14 pairs across 8 distinct destinations**
(measured; see [c0-bulk1-04 §Corrections](../receipts/c0-bulk1-04-migration.md)). Sources are
`nhl24` (7 pairs), `nhllegacy-xbox360` (2), and one each from `cc5`, `nhl10-xbox360`,
`nhl15-ps4`, `nhl15-xbox360`, `nhl16-ps4`.

`nhl24` is a **conversion source only** — it is not in the catalog, has no anchor, and is never a
coverage target. Its corpus is **not found (in-repo)** and is not registered as a target fixture.

### 5.7 Portrait corpora inspected by C0-POR1 (supplied-explicit, private, uncommitted)

Added 2026-09-08 by [C0-POR1](../receipts/c0-por1-portrait-measurements.md); **corrected the same day
by [C0-POR1-R1](../receipts/c0-por1-r1-evidence-scope.md)**, which withdrew this section's original
provenance labelling. R1 governs where the two differ.

Three user-supplied read-only locations. Container and texture headers were parsed for **every** file
in the five folders; pixel-level decoding covers only a handful of specimens. **No game bytes entered
the repository** — the recorded outputs carry counts, header field values, digests and at most one
example relative path per observed group.

These are a **new class for this register**: neither `private-explicit` (they are not
`NHL_STUDIO_FIXTURES` keys) nor `mounted-implicit` (no code reaches them — they were supplied
explicitly for one package). They are recorded so the inspection can be reproduced or challenged,
**not** so anything may depend on them silently. **No test reads them.**

| Key | Location | Title / id (measured) | Corpus kind | Files | Owner |
|---|---|---|---|---:|---|
| `por1/ps3-nhllegacy` | `H:\Emulators\RPCS3\dev_hdd0\game\BLUS31540\USRDIR\fe\ion\artassets` | NHL Legacy, `BLUS31540`, `APP_VER 01.01` (PARAM.SFO) | **observed installed copy** | 18 663 (`playerheads` 9331 + `playerheadssmall` 9332) | owner-unassigned |
| `por1/x360-nhllegacy` | `H:\Emulators\games\XBOX\NHL Legacy - Vanilla\_compiled\fe\ion\artassets` | NHL Legacy, XEX title id `454109EC`, media id `3CF2A23F` | **derived tree** (extraction or rebuild) | 18 458 (`playerheads` 9228 + `playerheadssmall` 9230) | owner-unassigned |
| `por1/ps4-nhl21` | `H:\Emulators\ShadPS4\Games\CUSA18182\databigs\Extracted\nhl\ui\kettle_assets\playerportraits` | NHL 21, `CUSA18182`, `EP0006-CUSA18182_00-NHLICEHOCKEY2021` (param.sfo) | **extracted tree** | 10 421 | owner-unassigned |

Identity was read from each copy's own container (`PARAM.SFO`, `sce_sys/param.sfo`, the XEX2
`EXECUTION_INFO` header), not from folder names — the Xbox 360 directory is *named* "NHL Legacy -
Vanilla", which is user-supplied text and is **not** evidence that the tree is vanilla. Two of the
three title ids corroborate against this repository's own record at
`crates/studio-domain/src/discovery.rs:48`. **Build identification is incomplete on all three** — a
title id plus an SFO version field is not a build.

**"Observed installed copy" is deliberately weaker than "shipping layout".** The PS3 corpus is that
title's own directory layout, which is why it is the only one supporting any established constraint;
it is **not** established to be pristine or complete.

#### Reconciled counts

| Count | Value |
|---|---:|
| Files inventoried | **47 542** |
| Accepted by the C0-POR1 inspection parser | **47 541** |
| Excluded | **1** — `playerheadssmall/p0_4000/p1112_glump.options`, an EA build command line, not an archive |
| Texture payloads found | **47 541** (exactly one per accepted file) |
| Gen7 `BIGF` archives accepted | **37 120** |

Per-folder, per-extension, per-subfolder, per-group and per-structural-tuple counts are generated
into [`c0-por1-census.json`](../receipts/data/c0-por1-census.json), which is the single source for
them. **"Accepted" means the inspection parser found the fields it looks for at the offsets it
expects — not semantic validation.**

#### Group vocabulary (corrected by R1)

C0-POR1 labelled files "vanilla" and "third-party-modified". **Neither was established**, and the
mipmap version was circular. Neutral terms are used instead: **current** (`.big` / `.gnf`),
**backup-labelled** (`.bak` — the label is the filename's; nothing establishes who wrote either file),
**other** (the one `.options` artefact), and **base-level-only** / **mip-chain** headers.

#### Six registrable conditions

| # | Condition | Consequence |
|---|---|---|
| a | **`por1/ps3-nhllegacy` carries third-party writes** — 206 backup-labelled files and 206 current files with mip-chain headers, against 18 251 current files with base-level-only headers | The copy must not be treated as a clean reference. Note the two 206s are *suggestive*, not connected by evidence: no filter here separates EA-authored from third-party-authored files |
| b | **`patch.big` at PS3 `USRDIR` root was never opened** | Nothing rules out that it shadows or supersedes the loose portrait files. This alone blocks any `discovery` claim on PS3 |
| c | **Two of the three corpora are derived**, so nothing measured on them is attributable to their shipped titles | **Zero** established constraints on `nhllegacy-xbox360` and `nhl21-ps4`, however exhaustively they were measured |
| d | **The PS4 outer `.big` packaging was not examined** | An extracted tree does not establish it. Related to the existing §5.4 row for `CUSA18182`. PS4 payload compression is *unobservable* here, not measured as absent |
| e | **No PS4 pixel data was decoded.** BC7 is a table lookup from `data_format 41`; `tiling_index 13` was never exercised | Every PS4 geometry/format value is an observed header-field interpretation |
| f | **Colour space is unmeasured on Gen7 and non-uniform on PS4** (`num_format` 0 on 10 246 files, 9 on 175) | `raster` cannot be established on any portrait target while the schema bundles colour space into it |

#### Reproducibility

The inspection tooling is preserved, non-private, at
[`docs/receipts/data/c0-por1-tools/`](../receipts/data/c0-por1-tools/) with a README giving
invocation, hashes and limits. The three roots are hardcoded there so the run reproduces on that
machine; that is a record, **not** a fixture declaration.

#### What this registration does not do

It does not qualify any target, promote any capability, make these locations a dependency of any
test, or change fixture-acquisition or mounted-install policy. The inspection used code written for
the package rather than this repository's parsers — with one exception, the Xbox 360 tiling formula,
which was **transcribed from `crates/tdb-rx2/src/texlib.rs`** and is therefore not independent of it.
So this is **not** a qualification of `tdb-rx2`, `tdb-archive-big` or `tdb-archive-codec`.

## 6. Register E — crate-local fixtures and generated data

| Item | Location | State |
|---|---|---|
| EA formula defaults | `crates/tdb-formula/assets/default_ea_formulas.json` | **committed**, in-repo, no external dependency |
| FHM corpus | `corpus/` — untracked, ~800 MB | **present on this machine** (contains `ea_attribute_maxima.json`, `ea_formula_profiles/`, `ea_formula_state.json`). Excluded from the C0.2 snapshot by architect decision 2, so the two corpus-dependent `#[ignore]`d tests could not have run there |
| Nhl19BigStudio tester | `Nhl19BigStudio-Tester/` | **present**; the authorized C3a reference material (CLAUDE.md rule 1 recorded exception). Executable SHA-256 recorded in the roadmap §2 |
| Blender 4.5 | resolved from Settings or `<managed_root>/blender/4.5/blender.exe` | **unknown** — not on `PATH`, and neither resolution location was checked (C0.2 §6). No default test needs it |

`#[ignore]`d cases outside `tdb-rx2`, all fixture- or corpus-dependent and all **not run**:

| Crate / file | Count |
|---|---:|
| `studio-domain/src/head_census.rs` | 1 |
| `studio-domain/tests/ea_match_key.rs` | 1 |
| `studio-domain/tests/formula_real.rs` | 1 |
| `studio-domain/tests/import_corpus_e2e.rs` | 2 |
| `studio-domain/tests/import_history_cost.rs` | 2 |
| `studio-domain/tests/roster_builder_e2e.rs` | 1 |
| `tdb-archive-big/src/eb.rs` (`chunklzx_decodes_against_extracted_oracle`) | 1 |

---

## 7. Register F — runtimes

User-confirmed 2026-09-05 (roadmap §6). **Not re-verified here**: no runtime was launched, no build
string was read, no install directory was enumerated.

| Runtime | Titles installed | Shipping targets covered | Exact build | Deployment path | Acceptance of Studio output |
|---|---|---:|---|---|---|
| Xenia | NHL 07–15, NHL Legacy | 10 | **unknown** | unknown | **none recorded** |
| RPCS3 | NHL 08–15, NHL Legacy | 9 | **unknown** | unknown | **none recorded** |
| shadPS4 | NHL 15–21 | 7 | **unknown** | unknown | **none recorded** |
| Legacy Recompiled | Legacy Recompiled | 1 | **unknown** | unknown | **none recorded** |
| | | **27 of 27** | | | |

The user also confirms every listed installation boots into playable gameplay and creates saves.

**Baseline playability promotes nothing.** Exact runtime builds, deployment paths and acceptance of
Studio-edited output are all **unknown** and are required evidence for C2 and C9. Under the R2
contract, a `game-acceptance` record missing `target.runtime_build` is `withheld` with
`reason = unknown-identity` — so **no game acceptance can qualify anywhere today**, for the register
reason recorded here rather than as an accident.

| Blocker | Next action | Owner |
|---|---|---|
| No runtime build identity is recorded for any of the four runtimes | record the exact build string of each before any manual campaign starts; the acceptance record schema already requires it | owner-unassigned |
| No deployment path is recorded per runtime | document per runtime how an edited artefact is installed, before C2 write qualification | owner-unassigned |

---

## 8. Ownership

**Every row in this register is `owner-unassigned`.** No accountable implementer or manual tester is
recorded anywhere in the repository, and assigning one is an architect action. This is the single
largest outstanding item against C0's exit criterion *"every required feature has an owner and a
test route or an explicit unresolved fixture/runtime blocker"*: the **test routes and blockers are
now enumerated; the owners are not**.

---

## 9. Consolidated blockers

| # | Blocker | Class | Owner | Next action |
|---|---|---|---|---|
| **FR1** | 6 private-explicit fixtures absent; **26 cases not run** | fixture | owner-unassigned | acquire an authorized fixture set, or accept the surfaces as unproven |
| **FR2** | 7 `tdb-rx2` cases (and 20 more elsewhere) silently depend on mounted game trees | policy | **architect** | decide whether a mounted install belongs behind the fixture root |
| **FR3** | No outer-package fixture exists anywhere in the repo (Q04 required) | fixture + policy | architect | decide acquisition and storage policy |
| **FR4** | `legacyrecomp-recomp` has no anchor of any kind | fixture | owner-unassigned | acquire, or record permanently blocked |
| **FR5** | `ps3/nhl07` has no save fixture | fixture | owner-unassigned | likely never needed — research-only unit |
| **FR6** | No runtime build identity or deployment path recorded for any runtime | runtime | owner-unassigned | record before any manual campaign |
| **FR7** | Per-title save-mode identification unverified against Q03's required modes | evidence | owner-unassigned | inventory each fixture's actual mode |
| **FR8** | `texlib_67.rx2`: one logical role, two former physical sources, **byte identity unproven** | provenance | owner-unassigned | hash and compare if both become available; never assume |
| **FR9** | `CUSA11126` (hardcoded) vs `CUSA11116` (`ps4_title_ids`) for NHL 19 | correctness | owner-unassigned | determine which is right; do not "fix" either by guessing |
| **FR10** | Audio: no fixtures, no implementation | scope | owner-unassigned | C7 |
| **FR11** | Blender 4.5 presence unknown at both resolution locations | runtime | owner-unassigned | check the two documented locations when Blender work starts |
| **FR12** | Every row is owner-unassigned | ownership | **architect** | assign owners |
| **FR13** | `por1/ps3-nhllegacy` carries third-party writes (206 backup-labelled + 206 mip-chain current files); provenance of individual files is unknown | fixture provenance | owner-unassigned | register as non-pristine; obtain a copy with known provenance. Do **not** treat `.bak` or `mipMapCount = 0` as a provenance filter |
| **FR14** | The Xbox 360 and PS4 corpora are derived trees, not attributable to their shipped titles | evidence | owner-unassigned | locate the portrait members inside the shipping archives and compare, before any target-scoped claim on either |
| **FR15** | No PS4 portrait pixel data has been decoded; BC7 and `tiling_index 13` are unexercised | evidence | owner-unassigned | decode one GNF specimen and check it against a known image |
| **FR16** | Portrait colour space is unmeasured on Gen7 and **non-uniform on PS4** (10 246 vs 175) | evidence + schema | **architect** | decide whether the 175 are a distinct asset class, and whether `RasterValue` should stop bundling colour space |
| **FR17** | `patch.big` at PS3 `USRDIR` root was never opened; it may shadow the loose portrait files | evidence | owner-unassigned | examine it — it blocks `discovery` on the one corpus that could otherwise support it |
| **FR18** | Two `asset-output-profile/2` gaps block portrait population: `dimensions` cannot express co-existing per-target variants, and `path_rule` cannot express one target's multiple path families | schema | **architect** | required semantics are specified in [C0-POR1-R1 §6.4](../receipts/c0-por1-r1-evidence-scope.md); no schema change was made |

---

## 10. What this register does **not** establish

- It does not qualify, promote or demote any target, family or operation.
- It does not claim any fixture is *absent from the machine* — only that it was not found at the
  inspected in-repo locations.
- It does not establish that any registered fixture has ever been exercised. 26 + 7 + 8 named cases
  are recorded **not run**, and a case that returns early is never described here as coverage.
- It does not prove the two former `texlib_67` sources are the same bytes.
- It does not establish where a shipped portrait lives on Xbox 360 or PS4, nor what installing one
  requires on any target. Two of the three C0-POR1 trees are derived, and folder placement is not
  proof of required install behaviour.
- It does not assert that the PS3 and Xbox 360 portrait sets are equivalent, nor that either tree is
  authentic or unmodified. C0-POR1 compared 50 sampled ids (49 identical, 1 differing) and used the
  agreement as evidence that the derived Xbox 360 tree carries authentic content; **C0-POR1-R1
  withdrew that**. The two decode chains share three stages, so the comparison exercises only the
  Xbox 360-specific ones, and agreement between two copies says nothing about whether either was
  modified.
- It does not establish that any portrait corpus here is the set its title ships, or that any
  observed format property is a *requirement* of any target. No load, acceptance or shipping-manifest
  test was performed.
- It does not change fixture-acquisition policy, boot-probe policy or mounted-install policy. FR2 and
  FR3 are referred to the architect unchanged.
