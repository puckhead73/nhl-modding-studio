# C0-REG2 — six database adapters and cross-game asset registration

**Package:** C0-REG2 · **Date:** 2026-09-15 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **Baseline and HEAD throughout:** `52a12fd1c0d1e4169e62951382dea6c24e17cd5f` ·
**Index before and after:** empty · **Nothing staged, committed or pushed.** `QA_WRITE` / `QA_EVIDENCE_ROOT` unset.

> **Result.** **A — all six targets registered, none blocked as a whole.** Six adapters exist in the
> working tree (`nhllegacy-ps3`, `nhl14-xbox360`, `nhl14-ps3`, `nhl13-xbox360`, `nhl12-xbox360`,
> `nhl11-xbox360`), every physical number read from each target's own committed snapshot, every
> capacity read from each target's own committed ROSTER fixture through the production save codec
> (three container framings: `ZlibV4`, `RawV2`, `RawV1`), and every semantic measured on each
> target's own anchor. Four targets carry 8 properties / 10 fields; **NHL 11 and NHL 12 carry 7 / 9**,
> because their skater block has no field named for an overall rating — recorded as
> `unknown-field-identity` with the search, never as absence. `player.shoots` and `player.team` stay
> blocked on all six for the same reasons as on Legacy / NHL 15. Two twin identities (NHL 14
> PS3 ↔ Xbox 360, Legacy PS3 ↔ Xbox 360: main DB and metadata byte-identical) are asserted on the
> bytes and carry nothing.
>
> **B — logos and jerseys are represented, not populated.** `assets-logos.toml` and
> `assets-jerseys.toml` register both families against the existing `asset-output-profile/3`
> contract on `nhllegacy-xbox360` (the only target with committed records for either) with **every
> block unknown and zero established constraints** — corpora, path families, variants and scoped
> candidates carry the evidence at the scope it actually has. The requirements list maps onto
> existing fields except **two concrete gaps** (a composite kit-id path token; a shared-fallback
> resolution chain with write exclusion) recorded in §B.4 with the fields considered. A 13-row
> cross-game inventory (`docs/receipts/data/c0-reg2-asset-mapping-inventory.csv`) separates
> committed implementation from working-tree experiments — including the fact that an EB
> archive-write capability **is** committed (`EbFile::replace_payload*`) and simply not wired to any
> asset family.
>
> **C — external completed-project ingestion**: the existing corpus / bridge / placement fields
> support the comparison and provenance halves; three model gaps remain (§C).
>
> **D.** Isolated candidate from `52a12fd` + a 16-file overlay, no manifest or lock change:
> **`cargo test -p tdb-canonical --locked --offline` 291 passed · 0 failed · 0 ignored** (225 → 291),
> clippy `-D warnings` clean, fmt clean; focused check `studio_domain::assets::tests::legacy_x360_team_logo_identity_map`
> passed. Registry after this package: **10 / 28 adapters (4 committed + 6 working tree), 3 / 14
> profiles (1 populated + 2 represented).** No cell is game-verified.

---

## 1. Exact files, SHA-256 of the working-tree bytes

*(Update 2026-09-15: boundaries 1 and 2 below were committed as `2ec9fde` and `0251b57`; see
`docs/receipts/c0-reg3-database-asset-design.md` §A for the verification. Boundary 3 remains uncommitted.)*

```
5ce5846b194a09f13b24500fb2e9b36a3431dc78e167af3b381c55fb278c1ad9  defs/adapters/nhllegacy-ps3.toml                        A  1014 lines
b67f46bf7bf3108d61349d8ff125a632bed6a5f3cbfc3e0cc4b44dbba3e4739b  defs/adapters/nhl14-xbox360.toml                        A  1014
47bb695ffe0d4c78a93545b32fea4b2bfab82ba3ccab956c7e1cdc1cdc9e30f5  defs/adapters/nhl14-ps3.toml                            A  1020
830e7ac4e8da6834435064e2f79640234f713760ad0374a3c79ecc03c4d13597  defs/adapters/nhl13-xbox360.toml                        A  1009
f4d261613de06be4aea6ab9b80302b38d4966484e56b38de64fae7794dd44842  defs/adapters/nhl12-xbox360.toml                        A   946
20f249b8d2ce986afb2fca1ff1329f8610be344ffc380d7c6a0f8e03ea8ca072  defs/adapters/nhl11-xbox360.toml                        A   946
d7ce0a10cc4326e83b87c830652b43c8d08b627b42afac25a8e4f2392464479c  crates/tdb-canonical/src/target_batch_tests.rs          A  1148  (six per-target modules over shared runners)
0b0c7fd07c5d77afeeab3ac230039b790cc922fd190ffdf1b5966b6b7e8e3dac  crates/tdb-canonical/src/target_evidence.rs             M  +26/−13 (framing variants; team-name offset; rated list)
1b23867227bc8df2de2af378711932d8601e1ede698da4c67decb52593ab2afb  crates/tdb-canonical/src/legacy_xbox360_tests.rs        M  +5/−2  (new signatures, same expectations)
4e3f63840b93743440d0bf43a2d71f1832c2f4afd71bbf3e1eba6c29a7b33447  crates/tdb-canonical/src/nhl15_ps3_tests.rs             M  +5/−2  (same)
de8eda283f17ee9776bf70edd74144faa5f9db2b74d258e7a06b54796272c3c2  crates/tdb-canonical/src/snapshot_tests.rs              M  +8/−2  (ten adapters)
8f090a449b294b9a5d248f0f8932ece20b4aa3aafef0ba55bf6cebaf027f4fe4  crates/tdb-canonical/src/tests.rs                       M  +44/−9 (ten adapters; MEASURED_ON_OWN_ANCHOR; 27 sentinel candidates)
002c6a2ecccbafe6e20a18250b6791897fc98a86d4ee75b2c641c70ee67bebc3  crates/tdb-canonical/src/lib.rs                         M  +2    (one `#[cfg(test)] mod`)
59c70e6c7f45f66ba1ed6ddea70c60c58cc458ec67c59867b54d1f56165f9098  defs/adapters/asset-profiles/assets-logos.toml          A   417
c88d1e90f4b06586881de658b2b31793fd370f4ae2d56803993b5850e54a71b2  defs/adapters/asset-profiles/assets-jerseys.toml        A   424
7cc4b0a99bb7c3771d185defe9f77aaa941dc9a14e787d427728457d10964a5f  crates/tdb-canonical/src/asset_tests.rs                 M  +106/−1 (profile count 1 → 3; representation case)
```

Outside the code overlay (documentation): this receipt;
`docs/receipts/data/c0-reg2-asset-mapping-inventory.csv` (A); `docs/plans/project-completion-roadmap.md`
and `docs/STATE-OF-THE-REPO.md` (one passage each, recording the uncommitted state — note that both
of those documents, like `docs/receipts/` and `docs/validation/c0-0[3-6]*`, are **untracked at
`52a12fd`** in this working tree, so they sit outside any commit boundary unless named). No `Cargo.toml`,
root manifest or `Cargo.lock` change. The adapter TOMLs were produced by a throwaway generator that
read every physical number from the target's snapshot and asserted the measured semantics against
it; the generator and the measurement probe lived only in scratch candidates and are not part of the
overlay.

### Proposed commit boundaries

1. **`feat(canonical): register six measured gen7 adapters`** — the 6 adapter TOMLs, `target_batch_tests.rs`,
   `target_evidence.rs`, `legacy_xbox360_tests.rs`, `nhl15_ps3_tests.rs`, `snapshot_tests.rs`, `tests.rs`, `lib.rs` (13 files).
2. **`feat(canonical): represent logo and jersey asset families`** — the 2 profile TOMLs, `asset_tests.rs`, the inventory CSV (4 files).
3. **`docs: record C0-REG2`** — this receipt, roadmap, STATE (3 files), or folded into 1–2 if preferred.

Boundary 1 is self-contained (291 − 1 = 290 cases pass without boundary 2, verified at an intermediate
step); boundary 2 depends on nothing in 1.

## 2. Part A — the six targets

### 2.1 Fixture identities (all measured through `tdb_savedata` / `tdb_core`, pinned by SHA-256 in the test modules)

| Target | Main DB sha256 (metadata sha256) | ROSTER fixture | Container as decoded | Inner TDB |
|---|---|---|---|---|
| `nhllegacy-ps3` | `105d2186…` (`99062a42…`) — **byte-identical to Xbox 360 Legacy** (asserted on the bytes) | `anchors/ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA`, 2 456 120 B, `c079ec4b…` | `PS3RosterFile`, word 4, `ZlibV4`, unc 2 456 076 / comp 722 156, both checksums valid | 2 456 072 B, `469a1346…`, 39 tables |
| `nhl14-xbox360` | `84e1bdff…` (`d1f9318e…`) — **byte-identical to PS3 NHL 14** | `…/xbox360/nhl14/savefiles/ROSTER 20260627054746/ROSTER 20260627054746`, 2 215 156 B, `20f144d7…` | `RosterFile`, word 2 + zlib at 0x30 → `ZlibV4`, 2 215 112 / 676 462 | 2 215 108 B, `700b8e79…`, 38 |
| `nhl14-ps3` | same two files as above | `…/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA`, 2 215 156 B, `e54e9621…` | `PS3RosterFile`, word 2 → `ZlibV4`, 2 215 112 / 677 512 | 2 215 108 B, `c84fadef…`, 38 |
| `nhl13-xbox360` | `64460a15…` (`c5b9994e…`) | `…/xbox360/nhl13/savefiles/ROSTER 20260627051601/…`, 2 180 292 B, `d4bd2f7c…` | `RosterFile`, **word 0 → `RawV2`** (raw body at 0x2C; no compressed-size word) | 2 180 248 B, `0e16b80e…`, 36 |
| `nhl12-xbox360` | `94b31ff2…` (`ddaaff54…`) | `…/xbox360/nhl12/savefiles/ROSTER 20260627052113/…`, 2 350 856 B, `5b361a29…` | `RosterFile`, **word 2 + `DB` at 0x2C → `RawV2`** (same word means `ZlibV4` on NHL 14) | 2 350 812 B, `9b8c75bf…`, 36 |
| `nhl11-xbox360` | `461f8d64…` (`a83aae92…`) | `…/xbox360/nhl11/savefiles/ROSTER1 20260627054323/…`, 1 935 216 B, `8508fc99…` | `RosterFile`, **word 1 → `RawV1`** (32-byte header, one checksum, no size words; **write path experimental** per the codec and `save-matrix.csv`) | 1 935 184 B, `17ba7cd0…`, 32 |

Every snapshot was asserted to be the rendering of its anchor by `snapshot_tests::anchor_snapshot`
(now over ten adapters, 0 discrepancies). NHL 14 and Legacy snapshots are identical to their twins'
modulo `platform`; each adapter cites only its own platform's paths and rendering (asserted per pair).

### 2.2 Capacities — each fixture's own directory (`max_records` / `current_records`)

| Target | ubPc | ZBac | eGlu | dSvy | xmja | mHuy | fMVl | kOtt | Populations = main anchor |
|---|---|---|---|---|---|---|---|---|---|
| `nhllegacy-ps3` | 7988/5632 | 9955/6087 | 10879/5532 | 7331/5066 | 7331/5066 | 957/566 | 957/566 | 252/252 | all 39 tables |
| `nhl14-xbox360` | 6873/5374 | 8803/5827 | 10353/5353 | 6350/4851 | 6350/4851 | 823/523 | 823/523 | 246/246 | all 38 |
| `nhl14-ps3` | 6873/5374 | 8803/5827 | 10353/5353 | 6350/4851 | 6350/4851 | 823/523 | 823/523 | 246/246 | all 38 |
| `nhl13-xbox360` | 6946/5412 | 8907/5874 | 10457/5446 | 6407/4875 | 6407/4875 | 839/537 | 839/537 | 248/248 | all 36 |
| `nhl12-xbox360` | 6898/5395 | 8283/5779 | 7853/5349 | 6324/4821 | 6324/4821 | 874/574 | 874/574 | 243/243 | all 36 |
| `nhl11-xbox360` | 5910/5310 | 7093/5593 | 7728/5228 | 5260/4760 | 5260/4760 | 650/550 | 650/550 | 242/242 | all 32 |

All 48 blocks `measured`, `source = "roster-save-max-records"`, provenance = fixture path + named
case + `save-matrix.csv` row, each note stating *observed directory capacity of that fixture, not a
demonstrated runtime limit*; the NHL 11 notes additionally say the RawV1 container's write path is
experimental and only the read path was used. **Directory vs runtime:** every figure is an allocation
read from one save; nothing shows the game accepts growth to it, and `kOtt` is packed (no headroom)
in every fixture. **Population equality** (save = main anchor for every table the save carries) was
observed by the probe on all six and is asserted by the shared helper as a *fixture observation*; it
was not imposed anywhere it was not observed. The seven roster-bearing figures coincide with the
roster-capacity audit's GEN7 table for every title (the audit measured PS3 saves), which is
corroboration and is not what the adapters cite. For the two twin pairs the fixtures' directories are
identical across platforms, so a dedicated case relabels each adapter's capacity provenance with the
twin's and shows all eight blocks reported — numbers alone cannot tell "read from this fixture" from
"carried"; provenance does.

### 2.3 Semantics — measured on each anchor with labels from EA's metadata names and declared keys

| Fact | Legacy PS3 | NHL 14 X360 | NHL 14 PS3 | NHL 13 X360 | NHL 12 X360 | NHL 11 X360 |
|---|---|---|---|---|---|---|
| Birth-date index origin (stored month / day ranges) | 0..11 / 0..30 → `index-origin-0`; offsets day/month/year **958/975/816** | same; **943/961/800** | same; 943/961/800 | same; **938/954/768** | same; **1128/1136/984** | same; **1126/1134/984** |
| Position / companions (skaters : goalies; codes) | 5066 : 566; {0,1,2,3} / {4} | 4851 : 523 | 4851 : 523 | 4875 : 537 | 4821 : 574 | 4760 : 550 |
| Main team `ubPc.dXSB` (offset 48/32/0/0/0/0) — rostered `pattern−1` / un-rostered 0 / all-ones / `kOtt.vfEq` keys | 5110 / 522 / 0 / 252 | 4989 / 385 / 0 / 246 | 4989 / 385 / 0 / 246 | 5008 / 404 / 0 / 248 | 4995 / 400 / 0 / 243; **`same` = 2** | 4923 / 387 / 0 / 242; **`same` = 1** |
| Handedness `GRkp` (offset; histogram) | 979; {0: 3781, 1: 1851} | 965; {0: 3624, 1: 1750} | 965; same | 958; {0: 3621, 1: 1791} | 1140; {0: 3613, 1: 1782} | 1138; {0: 3580, 1: 1730} |
| `team.name` `kOtt.YmkJ` (offset; records 0 / 1) | 352; `Anaheim Ducks®` / `Winnipeg Jets™` | 352; `Anaheim Ducks™` / `Winnipeg Jets™` | 352; same | **336**; same | **336**; same | **360**; `Anaheim Ducks™` / `Atlanta Thrashers®` |
| Ratings | `eWUF` c_overall @24, `AdEb` c_speed @176; derived offset 36 | same | same, **transform `measured`**: `ratings_write_is_inverse_of_read_ps3_6bit` opens this anchor | **`bPyZ` p_overall @190, `qPWY` p_speed @172** | `qPWY` p_speed @340; **overall omitted** (`unknown-field-identity`) | `qPWY` p_speed @84; **overall omitted** |
| Names (`HdeP` / `QbMR`; byte budget) | 376/576; 25 | 360/560; 25 | same | 328/528; 25 | **328/648; 40** | **328/648; 40** |

*`same` on NHL 11 / 12:* one (two) players are rostered on two adjacent team codes, so a second slot
also satisfies `team == pattern`; each such player still has exactly one `pattern − 1` slot, so the
encoding is unchanged and the count is recorded in the adapter note rather than hidden. *NHL 11 / 12
overall:* the name search found 2 fields containing `overall` in each snapshot, both `pickoverall`
on the draft-pick tables and none on the 36- / 68-field skater block — a discovery note, not proof of
absence, asserted by `no_skater_block_field_is_named_for_an_overall_on_this_snapshot`. *Ratings
transform:* `derived` on five targets (the audit's regime table names each by title and platform);
`measured` on NHL 14 PS3 because the named case opens that anchor by path and asserts the probed
offset — the mappings stay `implemented` because the case picks a skater's first rating cell rather
than a field by id. `team.name` is `automated-tested` / `inverse_verified` on all six (read / edit /
CRC / reopen / no-op serialize / over-budget refusal on a disposable copy, at the target's own field
offset).

### 2.4 Blocked and unknown, per target

| Blocker | Targets | State |
|---|---|---|
| `player.shoots` codes | all six | `blocked`; enum anchored on an uncommitted `NHL Legacy default.db`; histogram recorded as counts only |
| `player.team` parent (`kOtt.vfEq`) | all six | candidate (containment in the key set + name `team`); mapping `blocked` |
| `player.rating_overall` identity | NHL 11, NHL 12 | `[[omission]] unknown-field-identity` with the search recorded |
| Runtime ceilings | all six | directory capacities only |
| RawV1 write path | NHL 11 | experimental in the codec; not exercised here |
| Unmapped fields | 2955 / 2959 / 2959 / 2833 / 2584 / 2505 | unknown |
| Game verification | all six | none |

Nothing blocked one target from the others: all six reached the same shape their evidence allowed.

## 3. Mechanics (task "reuse target_evidence")

* `target_evidence::RosterFraming` gains `version: ContainerVersion` and makes the two size words
  `Option` (RawV2 has one, RawV1 none); `decoded_roster_fixture` asserts the variant the
  discriminator resolves instead of assuming `ZlibV4`. `team_name_round_trip` takes the field's bit
  offset (it was hard-coded to 352, Legacy's / NHL 15's). `assert_discriminator_declared` takes the
  list of rated properties that must carry the position predicate (a title without a stored overall
  maps one). The Legacy and NHL 15 PS3 modules pass their previous values; **their expectations are
  unchanged.**
* `target_batch_tests.rs`: one `Expect` per target (identities, framing, capacities, offsets,
  partition, main-team counts, histogram, team names, rated list, population, candidate ledger,
  mapped fraction) and one runner per fact; a `cases!` macro declares each `#[test]` from the same
  identifier it hands the runner, so an adapter's provenance string
  (`…/target_batch_tests.rs#<module>::<case>`) and the test name cannot drift. 65 new cases; nothing
  is copied between targets, and no thousand-line module was duplicated.
* `tests.rs`: `COMMITTED_ADAPTERS` → 10; `MEASURED_ON_OWN_ANCHOR` (8) replaces the two-name literals;
  the sentinel ledger extends to 27 candidates (11 + 4·3 + 2·2); the position-predicate expectation
  is computed from which ratings an adapter maps (unchanged for every prior adapter).
* `snapshot_tests.rs`: ten adapters; population count from the list.

## 4. Coverage accounting (adapter counts apart from field counts)

| Target | Mapped fields / total | Mappings / omissions | Established / unknown / candidate blocks |
|---|---:|---:|---:|
| `nhllegacy-ps3` | 10 / 2965 | 8 / 0 | 44 / 30 / 6 |
| `nhl14-xbox360` | 10 / 2969 | 8 / 0 | 44 / 30 / 6 |
| `nhl14-ps3` | 10 / 2969 | 8 / 0 | 44 / 30 / 6 |
| `nhl13-xbox360` | 10 / 2843 | 8 / 0 | 44 / 30 / 6 |
| `nhl12-xbox360` | 9 / 2593 | 7 / 1 | 40 / 26 / 5 |
| `nhl11-xbox360` | 9 / 2514 | 7 / 1 | 40 / 26 / 5 |

| Figure | Before (HEAD `52a12fd`) | After (working tree) |
|---|---:|---:|
| Adapter units | 4 / 28 committed | 4 committed + **6 uncommitted** |
| Registry established / unknown / candidate (all adapters) | 149 / 123 / 32 | **405 / 295 / 66** |
| `tdb-canonical` cases | 225 | **291** (+65 batch: 60 macro-declared + 5 target-specific; +1 asset) |
| Asset profiles | 1 / 14 populated | 1 populated + **2 represented** (0 established constraints on both) |

## 5. Part B — asset registry

### B.1 Requirements → existing contract fields (no new field needed for these)

| Requirement | Where `asset-output-profile/3` already carries it |
|---|---|
| Canonical player / team / item identity | `[identity]` (`key` ∈ player-slug / numeric-id / composite…, `source` = DB field, `ambiguity`); `[[canonical_source]].role` for the family's components |
| Source detection and ingestion | `[[corpus]]` (kind, attribution, identity source, groups, limitations) + `[[canonical_source]].retention` |
| Archive/member or loose-file location | `[discovery]` (`loose-tree` / `archive-member` / `nested-archive-member`, root), `[[path_family]]` + `[[observed_bucket]]`, `[path_rule]` |
| Format / dimensions / compression / mips | `[container].layer[]`, `[raster]` (+ `property.*` unknowns), `[dimensions]`, `[compression]`, `[mipmaps]`, `[tiling]`, `[atlas]`, per `[[variant]]` where forms co-exist |
| Database references and shared assets | `[identity].source`; `[install].updates_references`; `[atlas].sharing` for shared *surfaces* |
| Destination conversion and installation | `[[operation]].requires` (facts, not capability); `[install]` (kind, size policy, repack, backup); `[runtime_lookup]` (source, patch precedence, evidence) |
| Preservation, validation, recovery | `[install].backup`, `[compression].deterministic`, `force = observed-source-format` vs `target-required`, `Corpus.bridge` (`ProvenanceBridge`) |

### B.2 The two profiles

Both: schema `/3`, one target `nhllegacy-xbox360`, two corpora — `xbox360-shipping-archives`
(observed-installed-copy, **attributable**, 13 archive headers from `archives.csv`, build incomplete,
member names never committed) and `xbox360-compiled-tree` (derived, **not attributable**, `inspected_files = 1`:
the one specimen / one write the capability CSV names). All sixteen blocks unknown; no operation's
requirements met (asserted by `the_logo_and_jersey_profiles_represent_their_families_and_establish_nothing`).

* **Logos:** path families `fe-crest` (`fe/ion/artassets/teamlogos/t{team-id}.big`, `{team-id}` = `kOtt.lOnr`
  art id, not the key) and `rink-texlib` (`rendering/logo/texlib_{team-id}_0_0.rx2`); variants
  `fe-crest` (container candidate BIGF→XPR2, dimensions candidate 256×256 — both `asserted` from source
  prose) and `rink-texlib` (container RW4 and raster DXT5 candidates at **specimen-observation** scope on
  `texlib_10_0_0`). Identity: DB side measured (identities.csv regenerates from the anchor — focused
  check passed), file binding asserted → block unknown with provenance, no candidate (a candidate must
  carry a value and `ambiguity` would have to be invented). Discovery candidate `archive-member` under
  `cacherender.big` at specimen scope (a summary line). Compression candidate `lzx` (archived members;
  Studio does not decode `chunklzx`). Install / runtime lookup: unknown, no value — and the note records
  that `EbFile::replace_payload*` is committed and unwired.
* **Jerseys:** DB half measured in the snapshot (`xjgw` 703 rows: `mCXg` team key, `GDQL` teamartid,
  `cVLR` style, `FccT` variant; `kOtt.Rrgs` font etc.); canonical roles jersey-body / pant / sock /
  nameplate-font / thumbnail; families `outfit-texlib`, `sock-loose`, `fe-thumbnail`; three variants.
  `{variant}` stands in for the composite kit id and every family note says so (gap 1). The
  `fe-thumbnail` family, the `custom*.big` fallback chain, `clone_kit_textures` and the `xjgw` clone are
  **working-tree only** (not at `52a12fd`) and are labelled so; no candidate cites them.

Observed-source-format vs target-required: nothing in either file reaches `supported-target-constraint`,
so `target-required` is unused; extracted-file placement is never promoted into `[discovery]`,
`[install]` or `[runtime_lookup]`.

### B.3 Cross-game inventory

`docs/receipts/data/c0-reg2-asset-mapping-inventory.csv` — 13 rows × (family, target, canonical
identity, ingestion path, destination representation, references/dependencies, evidence source,
implementation state, committed-or-working-tree, blocker). Notes that matter:

* **Committed archive write exists and is unwired.** `tdb_archive_big::eb::EbFile` (`encode`,
  `replace_payload_in_place`, `replace_payload` with 16-byte reflow) is at HEAD, verified against a PS4
  `data0.big` (Phase 5.4), with the per-entry `+12` checksum unidentified and preserved verbatim.
  "No production Studio archive integration" is true; "no archive-write experiment" is false. The
  working tree adds variant-parameterised synthetic tests and `eb_verify_append.rs`.
* **Working-tree experiments, not implementation:** `jersey_thumbnail` / `custom*.big` chain,
  `clone_kit_textures`, the `xjgw` row clone, `mcd_write.rs`, `tdb-rx2/examples/`, and edits across
  `xpr.rs` / `texlib.rs` / `gnf.rs` / `bc7.rs` / `refpack.rs` / `lzx.rs` (2 873 + / 765 − across 23 files).
* **Install everywhere = extracted or loose file** via `import_texture` (safe-join, one-time `.bak`).
* **Mounted-tree cases skip silently** (C0.2 §5.4 class): five portrait/logo/uniform cases; none is
  evidence here. Synthetic cases cover DDS / GNF portrait round trips and outfit naming.
* **No PS4 logo path exists in code;** PS4 outfits are team-name-keyed GNFs, not `xjgw` tokens.

### B.4 Contract gaps — concrete, with the fields considered

| # | Fact that cannot be expressed | Fields considered | Real example |
|---|---|---|---|
| 1 | A path token for a **composite id from several DB fields** (`{style}_{teamartid}_{variant}`), or a `style` token | `PATH_TOKEN` = player-slug / numeric-id / team-id / variant; `[identity].key = composite` names the key but no token spells it in a template | `rendering/jersey/texlib_0_10_0.rx2` (NHL Legacy X360) — declared as `texlib_{variant}.rx2` with `{variant}` documented as the whole kit id |
| 2 | A **fallback resolution chain** whose shared members a per-asset replace must not write | `[runtime_lookup].patch_precedence` (base vs patch only); `[[path_family]]` (one template each, no ordering, no shared flag); `[atlas].sharing` (surfaces, not files); `[install]` (no exclusion) | `jersey_{id}.big` → `custom.big` → `customhome.big` → `customaway.big` (working-tree source) |
| — | *Observation, not a gap:* a candidate must carry a value (`candidate-incomplete`), so evidence that supports part of a value (identity: DB half measured, file half asserted, `ambiguity` undecidable) can only be recorded as an unknown block with provenance and prose. `PropertyEvidence` solves this for the three `OPTIONAL_BLOCK_PROPERTY` sub-fields only. | `[identity.candidate]` without `value` was refused by the validator during this package | logos / jerseys `[identity]` |

No extension was implemented; both profiles fit the current contract with the workaround stated in
place. The atlas-inside-a-role case (the font sheet is a glyph atlas within one canonical role) is
noted in the jersey profile and is not claimed as a gap: `[atlas]` describes the family and the role
description carries the rest.

## 6. Part C — external completed-project ingestion (record only)

**Wording clarification (C0-COMMIT-REG2 review).** The table below lists *reusable provenance
primitives* — fields that already exist in `asset-output-profile/3` and could be cited by an ingestion
record. It does **not** describe an implemented baseline-diff or import workflow, and no mechanism
exists today that retains opaque (unresolved) files: there is no importer, no per-file comparison, no
Project import record and no retention store. "Supported by" therefore reads "has a vocabulary that a
future record could reuse", nothing stronger. The Project-side import record is designed (not
implemented) in `docs/receipts/c0-reg3-database-asset-design.md` §C.3.

What existing registry fields provide as primitives:

| Need | Supported by |
|---|---|
| A complete modified installation or changed-files package as an input | `[[corpus]]` with `identity_source` read from the copy's own headers (SFO / XEX / EB header), `groups` (e.g. `current` / `backup-labelled`, as the PS3 portrait corpus already distinguishes third-party writes), `limitations`, `inspected_files` |
| Comparison with a matching vanilla baseline | `ProvenanceBridge.method = byte-identical-to-archive-member` against an attributable corpus is exactly the per-file identity check; `Corpus.attribution` keeps the modded copy from speaking for the target |
| Conversion into an independent canonical Project | `[[canonical_source]]` roles + `retention = unmodified` (lift the bytes as they are); the database half's `target-adapter/2` mappings for the DB side |
| Source provenance and unresolved-file preservation | `[[corpus]].provenance` / `limitations`; `PLACEMENT_BLOCK` discipline forbids inferring where an unresolved file sat |
| Source-target reproduction and cross-target portability | `force = observed-source-format` (reproduce what was observed, refuse nothing), `[[operation]].requires` per target (what facts a re-target needs), `select_variant` (which form a file is) |

Remaining ingestion-specific model gaps (no importer, no external path accessed):

1. `CORPUS_KIND` has no term for a **modified installation** or a **changed-files package**; the
   nearest is `observed-installed-copy` with `groups`, which says nothing about intent.
2. No per-file **comparison outcome** vocabulary (identical-to-baseline / modified / added /
   unresolved) and no place to record the baseline's identity per file; the bridge is one method for a
   whole corpus.
3. The Project-level provenance record (source target, declared file set, unresolved files retained
   as opaque bytes, portability report per candidate target) is outside the profile schema and belongs
   to `defs/canonical/project-schema.toml` — not represented there today.

## 7. Verification in isolation (Part D)

`…/scratchpad/reg2/candidate` = `git archive 52a12fd` (1 194 files) + the 16 overlay files (§1) =
1 203 files; `Cargo.lock` and every manifest are HEAD's. Build output in `…/reg2/target-final`.
`QA_WRITE` / `QA_EVIDENCE_ROOT` verified unset. Live tree: HEAD `52a12fd`, index empty before and
after; `git status --porcelain` differs only by the files named in §1 and the documentation.

| Command | Result | Log |
|---|---|---|
| `cargo test -p tdb-canonical --locked --offline` | **291 passed · 0 failed · 0 ignored** | `log-test.txt` |
| `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | exit 0 | `log-clippy.txt` |
| `cargo fmt -p tdb-canonical -- --check` | exit 0, empty | `log-fmt.txt` |
| `cargo test -p studio-domain --lib legacy_x360_team_logo_identity_map` (focused: the identities map the logo profile cites regenerates byte-identically from the anchor) | 1 passed | `log-identity.txt` |

Target-bound snapshot checks cover all ten adapters (`snapshot_tests::COMMITTED_ADAPTERS`), the
per-adapter expectation tables in `tests.rs` name all ten, and the four prior adapters' expectations
are unchanged. No shared production code changed (`tdb-canonical`'s non-test sources gained one `mod`
line), so no further focused suite applies; `tdb-savedata` was used, not modified.

## 8. Unresolved questions

1. Whether Codex wants boundary 1 and 2 as one commit or two.
2. Gap 1 / gap 2 (§B.4): extend `PATH_TOKEN` (a `style` or composite token) and add a resolution-chain
   representation, or keep the documented stand-in until a population package needs them.
3. The `candidate-incomplete` rule vs partially supported values (§B.4, last row): a schema question,
   not urgent.

## 9. Next development batch (proposal)

* **C0-REG3** — `nhl10-ps3`, `nhl11-ps3`, `nhl12-ps3`, `nhl13-ps3` with the same mechanic (their
  ROSTER fixtures are committed; PS3 NHL 11–13 containers need the RawV1/RawV2 framings now
  supported). Then NHL 08 / 09 (RawV3, gen7 ceilings deferred) and the PS4 seven (main-DB ceiling
  mechanic; LZ4 save codec open).
* **Asset side, no new corpus needed:** locate the logo / kit members in the Legacy X360 shipping
  archives from the committed `EB\0\3` reader (FR14) so `[discovery]` can move off a summary line —
  reading only; then decide gaps 1–2.
* Baseline hardening step 3 (mounted-tree asset cases behind `NHL_STUDIO_FIXTURES` with
  fail-on-absence) remains queued and was not started here.

## 10. Limits honoured

No staging, commit or push; no live evidence generation or publication; no game installation; no
headmesh work; no broad hardening; no deferred-scope expansion; no external repository or private
project path touched; no mounted game tree read (the probe and the cases read `anchors/` and
`docs/audits/data/schema/` only). Stopping for Codex review.
