# C0-AS6 — asset-family coverage and evidence population — receipt

> **Closed by C0-AS6-CLOSE (section H below, 2026-09-16): committed as `9e02acf`.** Codex review
> passed after P2/P3 were corrected in place (§E.2); the nine-path boundary of §E.3 was committed
> byte-for-byte, on `cc913f9`, with an empty index before and after. Not pushed.

**Package:** reconcile the 14-family inventory into asset-bearing families and workflow categories;
populate the asset registry from evidence already committed, keeping specimen and target scope
apart; register Recompiled by reference where new Legacy Xbox 360 definitions appear; make the
missing measurements actionable; verify on an isolated candidate.
**Baseline:** `cc913f9` (C0-AS5-CLOSE). **Date:** 2026-09-16. **Status:** committed as `9e02acf`
(section H); nothing pushed; QA activation parked; no schema version added.

**Result in one paragraph.** The "11 missing asset families" figure was a category error: seven of
the fourteen matrix families are database, save and onboarding workflows that have no raster, mesh
or audio to profile, and the crate now says so — a family-category table, a refusal rule for a
profile filed under a workflow family, and a coverage report over the **seven asset-bearing
families**. Against those seven the committed registry stands at **1 populated (portraits), 3
represented (logos, jerseys, and — new — equipment), 3 absent (heads, arenas, audio)**. The new
facts are all **database-side and measured on committed anchors**: the portrait id field on all
three declared portrait targets (now identity *candidates*, not established), the eight equipment id
fields on the Legacy anchor, and every asset-referenced field on all 27 anchors (a 513-row
drift-guarded record). Nothing became operative; no operation is unblocked; the file half of every
identity remains unobserved in any committed record, and the receipt says which specimen would
change that.

---

## A. The family inventory, reconciled

### A.1 Fourteen families, two kinds

`tdb_qa::catalog::FAMILIES` is the QA matrix's single coverage axis; `asset-output-profile`
documents borrow it as their `family` key. Seven of its entries are asset families; seven are not.

| Family | Category | Roadmap deliverable that owns it | Registry treatment |
|---|---|---|---|
| `db-presentation` | database | C1 (database parity: presentation, field disposition) | **not profile-bearing**; database adapters (`defs/adapters/*.toml`) are its registry object |
| `db-operations` | database | C1 (database operations, relational ops) | same |
| `save-raw-db` | save | C2 (save/container write paths) | **not profile-bearing**; `[[capacity]]` on the adapters reads the ROSTER save; no asset profile |
| `save-inner-container` | save | C2 | same |
| `save-outer-package` | save | C2 (+ FR3 outer-package acquisition) | same |
| `onboarding-install` | onboarding | C1 (discovery/metadata, `studio-domain::discovery`) and C6 for localization-adjacent install resolution | **not profile-bearing** |
| `onboarding-localization` | onboarding | C6 (localization) | same |
| `assets-portraits` | asset | C4 (portraits) on C3 shared writes | profile committed — **populated** (7 operative blocks, all on `nhllegacy-ps3`) |
| `assets-logos` | asset | C4 (logos) on C3 | profile committed — **represented** |
| `assets-jerseys` | asset | C4 (uniforms/nameplates) on C3 | profile committed — **represented** |
| `assets-equipment` | asset | C5 (Hockey Shop) on C3 | profile **new in this package** — represented |
| `assets-heads` | asset | C5 (mesh runtime, canonical head library; conversion is the **external workstream**) | absent — see A.3 |
| `assets-arenas` | asset | **stretch** (roadmap §2 and C5 text: arena art excluded from the initial release gate) | absent by decision — see A.3 |
| `assets-audio` | asset | C7 (audio production tools) | absent — see A.3 |

Implemented as `tdb_canonical::FAMILY_CATEGORY` (+ `family_category`, `asset_bearing_families`,
`ASSET_BEARING_CATEGORY`), with `Rule::AssetFamilyNotAssetBearing` (`asset-family-not-asset-bearing`)
refusing a profile filed under a workflow family, and `asset_family_coverage` reporting
`FamilyCoverage { status: Absent | Represented | Populated, declared_targets, by_reference_targets,
operative_constraints, specimen_observations, candidate_constraints }` over the seven. A test
asserts the categories partition the matrix axis exactly, another that each workflow family is
refused and never counted, a third that the committed registry reads 1 / 3 / 3.

### A.2 Corrected wording

*Was:* "3 of 14 asset families have a committed profile and 11 have none" (roadmap item 7 and the
direction update; `no_committed_asset_profile_is_a_synthetic_fixture` said "eleven do not").
*Is:* **4 of 7 asset-bearing families have a committed profile (1 populated, 3 represented) and
3 have none; the other 7 matrix families are database, save and onboarding workflows owned by
C1/C2/C6 and are not asset-profile scope.** No scope is dropped: every one of the fourteen keeps
its QA-matrix row; only the *registry object* differs by category.

### A.3 The three absent asset families — inventoried, not profiled

Deliberately no profile file for any of these: a profile consisting only of unknowns with no
committed record behind any block is proliferation, not coverage.

| Family | Concrete committed evidence | Gaps | Studio integration need |
|---|---|---|---|
| `assets-heads` | DB side measured on all 27 anchors (`ubPc.pPaD`, 14–16 bits, pool ids ≥ 60 000 on NHL 14+, see B.4); HEAD source: `head_catalog.rs`, `head_census.rs`, `head_preview.rs`, `head_submesh.rs`, `orphan_heads.rs`, `tdb-headmesh`, `blender-job`, `defs/appearance/orphan-heads.toml`; the bespoke rule `pPaD == lOnr` (HEAD `assets.rs`) | no committed census of head files with digests on any target; no committed decode record; mesh blocks (`mesh_layout`, `uv`, `rig`) never populated on any real target | **external conversion workstream** (C5 direction update): Studio must later ingest the canonical head library by slug, bind a head id to `ubPc.pPaD` per target (the DB half is now measured), and install mesh + textures — the profile that would carry that must wait for the workstream's own format facts. No conversion work and no inspection of the separate repository was done here |
| `assets-arenas` | roadmap §2 / C5: arena art and presentation editing is **stretch**; arena *database* fields stay under C1 | no file evidence, no code path, no profile — by decision | none required for the initial release gate; `assets-arenas` is used only as the synthetic fixture's family (`synthetic-asset-profile.toml`, excluded from population) |
| `assets-audio` | `archives.csv`: `audioboot.big`, `audioboot2.big`, `cacheaudio.big` (Legacy Xbox 360, headers + entry counts, "audio"); `AUDIO_CODEC` / `AUDIO_BANK` / `LOOP_METADATA` vocabularies exist in the contract | no bank layout, codec, slot or cue evidence in any committed record; `tdb-archive-codec` decodes no audio | C7 items 1–2 (audit + decode/encode proof) precede any profile |

---

## B. Population from committed evidence

Scope discipline throughout: **committed source is cited at HEAD `cc913f9`** (the working tree
carries ~300 unrelated modified files, none of which supplies a claim here); **audit CSVs** are
specimen-level unless they say otherwise; **anchors** are measured by named `tdb-canonical` cases
that run in CI. Nothing measured on a database becomes a file fact.

### B.1 Newly established (measured) facts — database side only

| # | Fact | Where measured | Carried as |
|---|---|---|---|
| 1 | The two NHL Legacy main anchors (`anchors/ps3/nhllegacy/…/nhlng.db`, `anchors/xbox360/nhllegacy/…/nhlng.db`) are byte-identical: SHA-256 `105d2186da3d1f0f3d3ea8a350ead42ac9d5d802ca9fd37652c39bb4e1f1c230` | `legacy_equipment_id_fields_are_measured_on_the_anchor`, `legacy_portrait_id_field_is_measured_on_the_anchor` | REG2's twin observation re-asserted; lets one DB measurement speak for both platforms' DB side |
| 2 | Legacy `ubPc.lOnr` (`artid`, 16 bits): 5 632 rows, 5 450 distinct values in 0..12814; 180 rows store 0; no all-ones; ids 2, 7, 12 each on two rows; against the census ranges 1 313 / 1 201 / 2 585 distinct ids fall in `p0_4000` / `p4001_8000` / `p8001_12000`, none in `silhouettes`, and **350 distinct ids (12403..12814) lie above the highest id observed in either Gen7 copy** | same case, PS3 anchor bytes | identity **candidates** on `nhllegacy-ps3` and `nhllegacy-xbox360` (`confidence = measured`, `scope = specimen-observation`) |
| 3 | NHL 21 PS4 `ubPc.lOnr`: 7 247 rows, 7 244 distinct in 1..21813, no zeros; ids 5, 12, 16 each on two rows; the census's 10 421 files exceed the distinct ids by 3 177 | same case, `anchors/ps4/nhl21/databases/nhl_game.db` (SHA-256 `90ee1ec4…8e9f9`) | identity **candidate** on `nhl21-ps4` |
| 4 | Legacy equipment id fields: `xmja` (5 066 rows) `wXFx` stick 7 b / 19 distinct (35..112), `NmUR` gloves 7 b / 14 (27..83), `YUZp` helmetbrand 8 b / 10 (1..25), `AOOe` skatebrand 7 b / 8 (39..48); `fMVl` (566 rows) `Kmxz` blocker 7 b / 29 (11..82), `OTJR` trapper 7 b / 26 (11..82), `mIUA` pads 7 b / 29 (11..82), `JOlt` maskid 9 b / 173 (0..290, 36 zeros); no all-ones anywhere; the HEAD `SLOT_SPECS.max_id` constants 127/255/511 equal `2^bits − 1` on Legacy; every stored skater id is among the ids the committed enum catalogues name (74 / 53 / 20 / 31 entries) | `legacy_equipment_id_fields_are_measured_on_the_anchor` | the equipment profile's `[identity]` **note** on both Legacy targets (no candidate value — see B.3) |
| 5 | The 19 asset-referenced fields on all 27 anchors — presence, metadata name, bit width, rows, distinct values, min/max, zero rows | `asset_reference_fields_are_measured_on_every_anchor_and_recorded` → `docs/receipts/data/c0-as6-asset-db-references.csv` (513 rows), regenerated from the anchors on every run and compared byte-for-byte after CRLF normalisation | a record; establishes no block. Selected findings in B.4 |

### B.2 Representation-only additions (nothing measured on a file)

| Addition | Source of the representation | Confidence recorded |
|---|---|---|
| `assets-equipment.toml` — `nhllegacy-xbox360`: 2 corpora (the same shipping-archive and derived-tree corpora as logos/jerseys), 8 path families (the texlib-keyed stick / skate / helmet, the single-component per-map glove / goalie-stick / goalie-pad shapes, the goalie-helmet paint shape, the stick thumbnail — the helmet mapped shape and the blocker / trapper shapes carry two independent components beside the id and are prose, see B.5), 3 variants, discovery candidate (`cacherender.big`, archive-member) and compression candidate (`lzx`) | HEAD `equipment_slots.rs#SLOT_SPECS`, `equipment_texture.rs`, `equipment_piece.rs`, `studio-api/hockey_shop.rs`; `capability.csv#equipment`; `archives.csv#cacherender.big` | asserted |
| `assets-equipment.toml` — `nhllegacy-ps3`: 1 corpus (`ps3-operator-copy`, 180 files by the record's count, attribution resting on the record), 2 path families (`rendering/playerstick/texlib_{numeric-id}.rpsgl`, `fe/ion/artassets/createplayer/sticks/stick{numeric-id}.big`), 2 variants, discovery candidate (loose-tree) | `defs/enums/stick-model.toml` — the operator's 2026-05-18 record: 106 thumbnails, 74 texlibs, and the on-ice test in which `xmja.wXFx = N` rendered `texlib_N.rpsgl` | asserted (prose, no digests, copy not pristine). **The only in-game asset-binding observation in the repository**, carried in the `runtime_lookup` note with no value |
| `assets-equipment.toml` — `legacyrecomp-recomp`: `reuses = "nhllegacy-xbox360"` | Q47 (C0-AS5) | by reference; destination blocks unknown and not inherited |
| `assets-portraits.toml` `profile_version` 4 → 5: identity candidates on three targets (B.1 #2–3), header history and coverage lines | this package | measured DB half; the file half explicitly unobserved |

### B.3 Per-family / per-target status on the six axes

Levels: *implemented* = code at HEAD; *automated-tested* = a named case executed; *measured* = a
named case read the bytes; *asserted* = source prose or an operator record; *unknown* = no
committed evidence. **No cell is game-verified.**

| Family / target | Canonical identity & ingestion | Paths, containers, payload | Destination encoding & replacement | DB references | Installation & runtime | Missing evidence vs missing implementation |
|---|---|---|---|---|---|---|
| portraits / `nhllegacy-ps3` | slug (`tdb-core::dev_key`, tested) → `ubPc.lOnr` **measured** (candidate); ingestion `decode_source_image → replace_portrait_texture (DDS DXT5 full chain) → import_texture` implemented, synthetic-tested | BIGF (6 members) → RefPack → DDS **established** (`observed-source-format`); 3 forms as variants; bucket rule unknown | DXT5 established per variant; `playerheadssmall` 256×128 established; `replace` still blocked (dimensions circular on the two `playerheads` forms, atlas kind, colour space) | `ubPc.lOnr`; `ubPc.pPaD` (bespoke iff equal) | unknown (`patch.big` never opened, FR17); install = loose write in code only | **evidence**: per-id census + `patch.big`; **implementation**: no Q10 validator, no archive path |
| portraits / `nhllegacy-xbox360` | same DB half **measured** (candidate); XPR2 same-size/resized replace implemented | BIGF → RefPack → XPR2 — candidates (derived tree) | DXT5 / base-only / tiled — candidates | same | unknown; in-archive member never located (FR14) | **evidence**: attributable corpus or provenance bridge; member listing |
| portraits / `nhl21-ps4` | same DB half **measured** (candidate) | GNF (outer `.big` layer unexamined) — candidates | BC7 base-only descriptor interpretation — candidates; GNF write is a base-mip patch (C4 gap) | same | unknown | **evidence**: id list of the tree; **implementation**: mip-chain regeneration |
| portraits / `legacyrecomp-recomp` | by reference | inherited | inherited | inherited | own, unknown (FR4) | **evidence**: any Recompiled installation |
| logos / `nhllegacy-xbox360` | `kOtt.lOnr` measured on the anchor (identities.csv, CI case) → `t{artid}.big` / `texlib_{artid}_0_0.rx2` asserted | crest BIGF→XPR2 (asserted), texlib RW4 (one specimen) | DXT5 one specimen | `kOtt.lOnr` (all anchors measured, B.4: absent on the gen6 title) | unknown | **evidence**: `cacherender.big` member listing, a crest file opened |
| jerseys / `nhllegacy-xbox360` | `/4` composite `{style}_{art}_{variant}` measured (703 rows → 698 ids, 3 collisions) | RW4 texlib bundles + loose socks + FE thumbnails — asserted; thumbnail chain from working tree | unknown | `xjgw.*` measured; `kOtt` style fields **single-valued 0 on every anchor that has them** (B.4) | unknown | **evidence**: `nocacherender.big` member listing; D7 decision |
| equipment / `nhllegacy-xbox360` | eight fields **measured** (B.1 #4); grammar asserted; **no candidate value (per-slot gap, B.5)** | 8 path families asserted (3 grammars unspellable, in prose); RW4 texlib candidate; loose-map container unknown | unknown ("loose RX2 rasters", capability row) | `xmja`/`fMVl` fields; `hHbv` zone colours (preview dependency) | unknown; Hockey Shop store outside the install; id clone = byte-for-byte file copies | **evidence**: any equipment file opened from an attributable corpus; **implementation**: per-title `max_id` (constants are Legacy widths) |
| equipment / `nhllegacy-ps3` | same DB half; stick binding **asserted by an on-ice record** | `.rpsgl` texlib (no Studio codec), FE `.big` thumbnails — asserted counts | unknown | `xmja.wXFx` + cascade `NISO/jmjk/MMuu/FCNk` | **the record's on-ice test** (prose) — `runtime_lookup` note only | **evidence**: the same test with copy identity, hashed list, capture; **implementation**: `.rpsgl` decode |
| equipment / `legacyrecomp-recomp` | by reference | inherited | inherited | inherited | own, unknown | as portraits |
| heads / arenas / audio | see A.3 | — | — | `ubPc.pPaD` measured on all anchors (heads) | — | heads: external workstream; arenas: stretch; audio: C7 audit first |

### B.4 What the cross-anchor record shows (`c0-as6-asset-db-references.csv`)

Measured on the anchors, recorded as data; none of it is a claim about a file.

* **Jersey composite id is not universal.** `xjgw` (`stockteamjerseys`) exists on **NHL 11–21 only**;
  NHL 07–10 (both platforms) have no such table, so the `/4` composite identity cannot apply there
  and any kit identity on those titles needs its own measurement. NHL 21 PS4 widens `FccT` to
  7 bits (values to 41) and `cVLR` to 2 bits from NHL 18.
* **Team logo art id is absent on the gen6 title** (`kOtt.lOnr` missing on NHL 07; its team table is
  the thin two-level `exhibitionteams`).
* **The `kOtt` style references carry no information on any vanilla anchor**: `Rrgs` / `Kacu` /
  `tftz` / `nfzw` store the single value 0 on every anchor that has them (NHL 08–16) and are absent
  from NHL 17+ PS4. The jersey profile lists them as references; they are not usable as identity
  parts.
* **Equipment field widths move by title**: on NHL 08 the stick is 8 bits and the other present
  slots 4–5 (with hundreds of zero rows — an
  absence code there is plausible and unestablished), 7–9 on NHL 10–16 (Legacy = the HEAD
  `SLOT_SPECS` constants), 8 on NHL 17–18 PS4, 9 on NHL 19–20, 10 on NHL 21. `fMVl.JOlt` (mask paint)
  is absent on NHL 08/09. The code's `max_id` constants are therefore **Legacy facts carried as
  code**, an implementation gap for any other target.
* **Portrait and head ids exist on every anchor**: `ubPc.lOnr` 14 bits through NHL 14, 16 from
  NHL 15; `ubPc.pPaD` reaches the 60 000+ pool range from NHL 14 (`59999` on NHL 14, `60182` on
  NHL 15, `60210` on Legacy / NHL 16+). NHL 16 PS4's roster stores the same portrait id set as
  Legacy (5 450 distinct, 0..12814).
* **Two pairs of anchors are one set of bytes**: `nhl07-ps3` = `nhl07-xbox360` (research copy) and
  `nhl19-ps4` = `nhl20-ps4` (attribution unresolved, C0-REG4-R1); their rows are flagged in
  `unit_note` and are one witness each, not two.

### B.5 Schema gaps met, and why no `/6`

* **Per-slot identity.** Equipment keys eight slots to eight fields (nine grammars); `[identity]`
  is one per target with one `source`. Recorded in both Legacy targets' notes as a gap; **not
  extended**, because no slot's file half exists in a committed record, so nothing an extension
  could spell would become operative. Reopen when a slot's file binding is measured.
* **Non-id path tokens.** `PATH_TOKEN` has `{variant}` as the only stand-in beside the id, and a
  token substitutes one value everywhere it appears. Grammars with ONE further component (the map
  suffix on `glove_{id}_{map}`, `goaliestick_{id}_{map}`, `goaliepad_{id}_{map}`; the style on
  `goaliehelmet_{style}_{id}_cm`) are declared with `{variant}` carrying that component. Grammars
  with TWO independent components beside the id — `helmet_0_{id}[_{team}]_{map}`,
  `blocker_{model}_{id}_{map}`, `trapper_{model}_{id}_{map}` — **cannot be spelled** (the first
  draft reused `{variant}` twice; `render_path` would never produce `helmet_0_15_am` or
  `blocker_1_80_nm` — review finding P2) and are kept in prose on the nearest declared family until
  the schema has independently addressable tokens. Same disposition as the portrait profile's
  missing computed-directory token.
* **Zone-mask rasters.** Glove/helmet `_tm` maps are channel *flags* over up to nine zones, not
  colour; `[raster]` has no term for that semantic. Noted on the `glove-map` variant.

---

## C. Legacy Recompiled reuse

`legacyrecomp-recomp` is registered by reference (`reuses = "nhllegacy-xbox360"`) in the new
equipment profile — the only new Legacy Xbox 360 asset definition this package adds. Destination
blocks (`discovery`, `install`, `runtime_lookup`) are declared unknown on the reusing target and
are not inherited; `resolve_target` is exercised by the equipment case (8 families, 3 variants
resolve; the declared target carries no families or corpora of its own). No database equivalence
is implied; Recompiled's database unit stays absent (FR4). The portraits, logos and jerseys
by-reference rows are unchanged; their inventory rows now record that AS5 is committed
(`cc913f9`).

---

## D. Missing measurements, made actionable

Every item names a location already recorded in this repository. **No mounted installation was
scanned and no corpus acquired in this package.** Ordered by how much real editing each unlocks per
unit of work.

| # | Specimen / files | Recorded location | Observation it would resolve | Unlocks |
|---|---|---|---|---|
| D1 | **Legacy Xbox 360 `cacherender.big` and `nocacherender.big` member tables** (names, offsets, sizes — no payload decode needed; the EB v0x400 16-byte-entry reader `tdb_archive_big::eb` is committed) | the title's shipping archives beside the `_compiled` tree: `H:\Emulators\games\XBOX\NHL Legacy - Vanilla\` (C0-POR1 census root) and the mounted-tree cases' `X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy` | which archive holds logos / kits / equipment, and whether rendered ids (`texlib_10_0_0`, `texlib_{style}_{art}_{variant}`, `texlib_{wXFx}`) name real members → promotes `[discovery]` from a summary line to a member-table reading and gives every identity candidate its file half on an **attributable** corpus | discover + identify on three families at once; the FR14 item |
| D2 | **PS3 Legacy `USRDIR/rendering/{logo,jersey,pant,sock,playerstick,glove,helmet,skate,goalie*}` census** with header reads and digests (as POR1 did for portraits) | `H:\Emulators\RPCS3\dev_hdd0\game\BLUS31540\USRDIR` (C0-POR1 location `ps3-nhllegacy`, PARAM.SFO identity recorded) | PS3 container/codec/dimensions for logos, kits and equipment — currently **no PS3 record at all** for those families; joins the operator's stick record to a hashed list | preview/export on the one attributable corpus we have |
| D3 | **`patch.big` at PS3 Legacy USRDIR root** — open it, list members, note any `fe/ion/artassets` or `rendering` paths | same copy | `[runtime_lookup]` patch precedence for every PS3 Legacy family (blocks `discover`/`install` everywhere today) | the FR17 item |
| D4 | **Per-id portrait census** of the PS3 copy (ids, not ranges) joined to the anchor | same copy, `fe/ion/artassets/playerheads[small]` | whether the 350 anchor ids above 12401 have files anywhere, and whether the 5 099 in-range ids all resolve → could promote `[identity]` on `nhllegacy-ps3` (the first identity anywhere) | identify on portraits |
| D5 | **Stick on-ice test, re-recorded**: copy identity (PARAM.SFO), hashed `playerstick/` listing, DB before/after digests, a capture per id (or a file-open trace) | the operator's PS3 copy (`defs/enums/stick-model.toml`) | turns the only in-game binding observation into evidence with a scope; first `runtime_lookup` candidate with a value on any target | install on one slot |
| D6 | **NHL 19 PS4 `inxp_00.big` / kettle `.big` member listing** through the committed PS4 EB extractor, portrait and kit members located | `H:\Emulators\ShadPS4\Games\CUSA00561\databigs` (HEAD test constant) and the authorized `Nhl19BigStudio-Tester/` fixtures | outer container layer for PS4 portraits (`GNF` is known not to be the whole chain), archive-member discovery | C3a's first target |
| D7 | **One extracted Legacy Xbox 360 equipment file per slot** opened and recorded (container, raster, dimensions) — the mounted-tree case `legacy_x360_outfit_and_equipment_decode_from_extracted_tree` already does this and skips silently | `X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\_compiled\rendering\…` | specimen-level format facts for eight slots (candidates, derived tree) — and moving that case behind `NHL_STUDIO_FIXTURES` with fail-on-absence (C0-FX1 pattern) | preview on equipment |

**Smallest useful batch: D1 + D4 + D3** — one afternoon on two already-identified copies, no new
acquisition. D1 supplies the file half of the logo, kit and equipment identities on an attributable
corpus and settles discovery for three families; D4 can establish the first identity block in the
registry; D3 is the single blocker shared by every PS3 Legacy operation. Real destination output
(an archive write) additionally needs C3a's write planner, which no measurement substitutes for.

---

## E. Verification and boundary

### E.1 Checks (working tree, then the isolated candidate — see E.3)

| Check | Result |
|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **539 passed · 0 failed · 0 ignored** (532 at `cc913f9` + 7 in `asset_coverage_tests`) |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 |
| `cargo fmt -p tdb-canonical -- --check` | exit 0 |
| existing cases updated deliberately | `no_committed_asset_profile_is_a_synthetic_fixture` (3 → 4 profiles, wording corrected); `the_committed_profile_establishes_only_what_its_evidence_scope_supports` (identity candidate now expected — measured, specimen scope, citing the AS6 case — with the block still unknown); `every_rule_has_a_stable_diagnosable_name` (+1 rule) |
| no manifest / lock change | `Cargo.toml` / `Cargo.lock` untouched by this package (the test reads the census ranges as transcribed constants rather than adding a JSON dependency) |

### E.2 Failures met on the way, none fixed by weakening

* **Review findings (Codex, after the first sign-off), both corrected in place:** P2 — three
  equipment templates reused `{variant}` for independent components; removed as templates
  (11 → 8 path families) and kept in prose, with a new assertion that no declared template uses a
  token twice. P3 — the NHL 08 width summary said "4–5 bits" for all equipment fields; the record
  and test measured `xmja.wXFx` at 8 bits there, so every summary now reads "8 bits for sticks, 4–5
  for the other present slots". The CSV and the measuring cases were right and are unchanged.

* The 350 above-range portrait ids start at **12403**, not 12402 as first written; test and
  profile note corrected to the measured value.
* `clippy::type_complexity` on the equipment expectation tuple → a named `type`.

### E.3 Boundary (SHA-256 of the LF-normalised bytes git would store; A = new, M = modified vs `cc913f9`; hashes as of the P2/P3 corrections — three files changed from the first sign-off, the other six are unchanged)

```
fb49d8439aec2ceefbb959df04c25952317c721a678f5035145066a68a852b0f  crates/tdb-canonical/src/asset.rs                  M  +154/-0   FAMILY_CATEGORY, family_category, asset_bearing_families, the refusal, FamilyCoverage + asset_family_coverage
f197716cc6a4044ccd0b92824a4f8f6b02cb979fbada48f4fe2e4edecd59dae8  crates/tdb-canonical/src/lib.rs                    M  +18/-11   Rule::AssetFamilyNotAssetBearing, re-exports, module
a37b241a536c232d5006d163024b2ad99c4b6ff3e65ab4fa363eddc2741a9f23  crates/tdb-canonical/src/tests.rs                  M  +1/-0     rule-name coverage
beae2ec074955b98a7aaa0ea5eece48e0769bf43d4a2e4ef3b59bc47c05e22ac  crates/tdb-canonical/src/asset_tests.rs            M  +30/-4    the two deliberately updated cases
220d1626542d2602d7162b361770f9202fb68a6ae47d8c4bb6cf6f38a3f6ae49  crates/tdb-canonical/src/asset_coverage_tests.rs   A  1008 lines, 7 cases
70163ad06f4ff5d3ec15b80c63a862850b31d3b80832d083b9f16bce3f386f49  defs/adapters/asset-profiles/assets-portraits.toml M  +99/-11   profile_version 5; identity candidates x3; history
c194a2a325bc60042bc70a19fb8c7718259e83334800618246fc9093d25ad707  defs/adapters/asset-profiles/assets-equipment.toml A  791 lines
5da1f1714801018f44560d8161980954de9c9d5f5680e1db8d942008fa8ebaf8  docs/receipts/data/c0-as6-asset-db-references.csv  A  513 rows + header (read by a test)
558577b2ea0dcad806d0c03ea6324d8b7ce8cdba07bcf8ed403759be5fc5fec0  docs/receipts/data/c0-reg2-asset-mapping-inventory.csv M  +14/-10 (4 new rows; 10 rows' committed-state cell)
```

Outside the boundary (documentation only, not read by any test): this receipt;
`docs/plans/project-completion-roadmap.md` (item 7 and the direction update — the "11 families"
wording corrected in four places); `docs/STATE-OF-THE-REPO.md`; `docs/validation/c0-06-canonical-project-and-adapters.md`
(§5.2 asset row, U-A2, new §11). `Cargo.toml` / `Cargo.lock` are **not** in the boundary: the
working tree carries unrelated modifications to both from other work, and this package neither
needs nor touches them (the candidate runs on `cc913f9`'s manifest and lock, `--locked`).

Proposed commit, when authorized: one commit over the nine boundary paths,
`feat(canonical): reconcile asset-family coverage; represent equipment; measure asset id fields on the anchors`
(expected 539 canonical cases). The census the portrait profile pins by digest
(`docs/receipts/data/c0-por1-portrait-specimens.json`, SHA-256 `23964c81…3eb2`, matches the file)
remains uncommitted with the rest of the POR1 receipts; the new test transcribes its four ranges
rather than reading it, so the boundary does not force that decision.

---

## F. Isolated verification

**Candidate:** `git -c core.autocrlf=false -c core.eol=lf archive cc913f9` (1 228 files) + the nine
boundary files above, CRLF-normalised to LF → 1 231 files; re-run in full after the P2/P3 corrections; fresh `CARGO_TARGET_DIR`; `QA_WRITE`,
`QA_EVIDENCE_ROOT` and `NHL_STUDIO_FIXTURES` unset; `cc913f9`'s `Cargo.toml` / `Cargo.lock`.

| Check | Result | Log |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **539 passed · 0 failed · 0 ignored** | `as6/logs/test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 | `as6/logs/clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | `as6/logs/fmt.txt` |
| non-boundary files vs the `cc913f9` archive, byte-exact | 1 228 compared: **0 changed · 0 missing · 0 extra**; all 6 modified boundary files differ from the archive, all 3 new ones are absent from it | `as6/compare.py` |

**Focused existing cases relevant to the new claims (working tree, `--locked --offline`):**
`studio_domain::equipment_slots::tests` (10 passed — the filename grammar the equipment profile
transcribes, exercised synthetically) and `studio_domain::assets::tests::legacy_x360_team_logo_identity_map`
(1 passed — the anchor-generated `identities.csv` the logo profile rests on). Both run against the
working-tree `studio-domain`, which carries unrelated modifications; the profile's source citations
are to HEAD `cc913f9`, and the `SLOT_SPECS` ceilings were transcribed from `git show cc913f9:…`
(127 / 127 / 127 / 255 / 127 / 127 / 127 / 127 / 511 across the nine slots), not from the working
tree.

---

## G. Limits observed, unresolved questions, next opportunity

**Limits.** No mounted installation scanned; no corpus acquired; no file of any asset family
opened; no archive member located; no headmesh conversion work and the separate repository not
inspected; no QA generator run and `docs/audits/data/**` untouched; no schema version; no
manifest/lock change; nothing committed or pushed; index empty. Scratch:
`…/scratchpad/as6/{candidate,ref,target,logs,boundary.txt,compare.py}`.

**Unresolved questions for the architect.**
1. **Per-slot identity (equipment).** One `[identity]` per target cannot carry eight slot sources.
   Extend the contract (a `[[identity.slot]]` shape, or one profile per slot family) only when a
   slot's file half is measured — recommended trigger: D1 or D7.
2. **Committing the POR1 census.** The portrait profile pins
   `docs/receipts/data/c0-por1-portrait-specimens.json` by digest; the file is uncommitted. Commit
   it with the receipts body, or accept a profile that cites an uncommitted record.
3. **D7 (jersey rows ownership)** still undecided; the cross-anchor record adds that `xjgw` does
   not exist before NHL 11, so whatever D7 decides applies to NHL 11–21 only.
4. **`SLOT_SPECS.max_id` as constants** is a C1/C5 implementation finding (Legacy widths on every
   title); recorded, not fixed here.

**Next implementation opportunity.** The measurement batch **D1 + D4 + D3** (§D): member tables of
the two Legacy Xbox 360 rendering archives through the committed EB reader, a per-id PS3 portrait
census joined to the anchor, and `patch.big` opened. It needs no new acquisition, uses two copies
whose identity is already recorded, and is the first work that can move a block on any of logos,
jerseys, equipment or portraits from candidate to established at a scope that speaks for the
target. Archive-backed destination output beyond that is C3a and needs the write planner, not a
measurement.

**Stop for Codex review.**

---

## H. C0-AS6-CLOSE — review closure and the commit

Codex review passed: P2 (unspellable two-component templates) and P3 (NHL 08 width summary)
closed in place, no remaining findings; 539 / 0 / 0, clippy `-D warnings` and fmt independently
reproduced on `cc913f9` plus the nine-file boundary, all nine LF hashes matched.

| Item | Value |
|---|---|
| Pre-checks | HEAD `cc913f9`; index empty; all nine §E.3 LF-normalised SHA-256 hashes match the working tree — **no drift** |
| Staged | exactly the nine boundary paths (6 M, 3 A); every staged blob's SHA-256 equals its §E.3 hash; `git diff --cached --check` clean; +2 629 / −36 |
| Staged tree | `657f3d2bce97f6daea4af2afcb52e4a5b1b1d1d8` (`git write-tree` before the commit = `HEAD^{tree}` after it) |
| Verification relied on | the isolated run of section F on these exact bytes (not repeated: bytes and baseline matched) |
| Commit | **`9e02acfe7e50e0737970fccf3b7d4321bef00f4b`** — `feat(canonical): reconcile asset-family coverage; represent equipment; measure asset id fields on the anchors`; parent `cc913f9`; message file; Co-Authored-By line |
| Preservation | index empty afterwards; `Cargo.toml` / `Cargo.lock` and every other working-tree change untouched; POR1 census not committed; nothing pushed |

**Remaining blockers, unchanged:** the file half of every asset identity is unobserved (D1 + D4 +
D3 is the batch that changes that); per-slot identity is a recorded schema gap; the POR1 census
the portrait profile pins by digest stays an uncommitted pre-existing evidence dependency; D7
open; `SLOT_SPECS.max_id` constants are a C1/C5 implementation finding. Nothing is game-verified.
