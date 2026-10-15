# Handoff 19 (1 Oct) — Legacy Recompiled arena read route: venue packages, jumbotron, ice surface and banners as archive members

Research lane, 1 October 2026, published 01:45 (system clock). After-receipt-8 prompt, second package. Claim
`research-q18-recomp-arena-read`, written before work began.
- Read-only: archive directories, every member's 8-byte head, a 24-member in-memory byte sample, and names-only reads of two small
  `gamedata` archives.
- Nothing launched, extracted to disk or written. No application edit.
- **Read route only.** This says nothing about replacing arena art on Xbox 360 or Recompiled; that board writer stays withheld
  (`board_replace.rs:47`).

Machine-readable:
- [`data/r147-recomp-arena-members.json`](data/r147-recomp-arena-members.json) (`3fac4659…`);
- [`data/r147b-recomp-gamedata-small-archives.json`](data/r147b-recomp-gamedata-small-archives.json) (`f1fabf81…`).

Probes, one run each:
- `local/runs/overnight-research-2026-09-28/probes/r147_recomp_arena_members.py` (`e05da0c2…`);
- `r147b_gamedata_small_archives.py` (`d3e778fd…`).

## Plain-language summary

- Everything the arena card and the venue-art card read is inside Recompiled's archives: 67 venue packages, 66 jumbotron files,
  474 ice-surface files and 318 banner files. Almost all of it is in `cacherender.big`; two ice files are in `renderboot.big`.
- Each file appears once. The file names are exactly the Xbox 360 Legacy names, and every file sampled matches the Xbox 360 Legacy
  copy once unpacked.
- Today Studio shows Recompiled arenas as **"not measured"**, which is honest. It does not wrongly call the files missing.
- The two small archives the developer could not list (receipt 67) hold no art: one is a 15-entry settings-like file, the other holds
  904 face-animation files. That could let the developer change "unknown" to a firm answer. The developer decides.

## 1. Fully compared versus sampled (reuse first)

| Class | Already measured before this run | Measured now (R147) |
|---|---|---|
| Venue packages `rendering/stadium/*` (67) | **full**: R137 compared all 67 X360 Legacy loose packages with their `cacherender.big` members after unwrapping, and all were equal (09 K10). R128: Recompiled `cacherender.big` has the same SHA-256 as X360 Legacy's | wrapper of every member; no duplicates; name set equals the X360 loose set; 6 sampled byte-equal |
| Jumbotron, ice surface, banners | R128 archive identity (`cacherender.big`, `renderboot.big`) only. **No** loose-versus-member byte comparison for these directories before now | wrapper of every member; no duplicates; name sets equal; **6 per directory** sampled byte-equal (18) |
| Any of these, on a later or modified install | — | **not covered**: history for this install (18 K21) |

## 2. Measured members (registered install `H:\Emulators\games\XBOX\NHL Legacy Decomp Test\game`)

| Directory (consumer) | Members | Archive | Wrapper | Duplicates (within / across archives) | Names = X360 Legacy loose | Largest stored |
|---|---|---|---|---|---|---|
| `rendering/stadium/` (venue art; arena card venue family; named ice bases) | 67: 64 `stadium_<art>_container_0` + `ice_nhl_container_0`, `ice_intl_container_0`, `ice_nhl_full_lm_container_0` | `cacherender.big` | chunklzx (67) | 0 / 0 | yes, 67 = 67 | 5 907 996 B |
| `rendering/jumbotron/` (`texlib_<arenaartid>`) | 66 | `cacherender.big` | chunklzx | 0 / 0 | yes, 66 | 91 662 B |
| `rendering/icesurface/` (centre-ice logos, bounce maps, ice layers) | 474: 231 `centerlogo_<team>_cm`, 37 `icesurface_<n>_bm`, 203 `ice_cline / lines / lightmap / hotspot_*`, plus `ice_lightmap_lm`, `ice_normal_nm`, `ice_spec_sm` | `cacherender.big` 472; **`renderboot.big` 2** (`ice_normal_nm`, `ice_spec_sm`) | chunklzx | 0 / 0 | yes, 474 | 1 147 210 B |
| `rendering/banner/` (`banner_<team>_cm`) | 318: 218 `banner_<team>_cm`, plus `header_N_N_cm` 38, `year_N_cm` 32, `banner_N_N_dm` / `_tm` 9 + 9, `banner_N_nm` / `_sm` / `banner_N` 3 each, `poles_cm` / `_nm` / `_sm` | `cacherender.big` | chunklzx | 0 / 0 | yes, 318 | 118 878 B |

- Every arena member is chunklzx, so a "stored" fallback must never apply here (18 K21).
- The stadium samples unwrap to 10.5–12.7 MB. The whole-member read the shared decoder does is in the same size class as the loose
  read `venue_art::package_view` already does (`std::fs::read`). No streaming question arises.

**Byte sample** (unwrapped SHA-256, first 16 hex; every one equals the X360 Legacy loose file):

| Member | Stored → unwrapped | SHA-256 |
|---|---|---|
| `stadium/ice_nhl_full_lm_container_0.rx2` | 11 510 → 117 776 | `27f0aa4ab91685b9` |
| `stadium/ice_nhl_container_0.rx2` | 11 554 → 117 776 | `9a10d17c89850543` |
| `stadium/ice_intl_container_0.rx2` | 11 634 → 116 864 | `d39e9931b5183f3a` |
| `stadium/stadium_26_container_0.rx2` | 3 616 636 → 10 546 176 | `188feff4f5b40fe8` |
| `stadium/stadium_14_container_0.rx2` | 3 785 088 → 10 896 784 | `ddbbcd6bbf35998d` |
| `stadium/stadium_2_container_0.rx2` | 4 067 314 → 12 656 432 | `11644f8343f53221` |
| `jumbotron/texlib_507.rx2`, `_321`, `_331`, `_197`, `_603`, `_17` | 13 548 – 60 356 → 45 056 – 135 168 | `7e2d642b…`, `ecf3577d…`, `b3341403…`, `d7790655…`, `ed6ff206…`, `d5331f3b…` |
| `icesurface/ice_hotspot_0_0_cm`, `_1_0_cm`, `_1_99_cm`, `_0_39_cm`, `centerlogo_326_cm`, `centerlogo_411_cm` | 456 – 114 736 → 36 864 – 1 445 888 | `d10788cf…`, `90211a65…`, `297f7d51…`, `ae1482d3…`, `f3155ca9…`, `ebb02903…` |
| `banner/year_999_cm`, `banner_143_cm`, `poles_cm`, `banner_17_cm`, `header_0_13_cm`, `banner_91_cm` | 536 – 38 416 → 36 864 – 397 312 | `d0135f6c…`, `ddcdd1b8…`, `f1e577b2…`, `745c33f4…`, `a3165bdb…`, `1fa2f4d2…` |

Full hashes are in the JSON.

## 3. Current consumers (source reading, working tree of 1 Oct; not run)

| Consumer | What it does on a Recompiled project today |
|---|---|
| `arena_asset_contract` (`arena_assets.rs:118`) | `legacyrecomp` matches no title arm, so the result is **`Unmeasured`** ("nothing is joined"). Correct and honest; no false "missing" |
| `venue_art` (`venue_art.rs:115`) | `MEASURED` (`:22`) lacks `("legacyrecomp", Recomp)`, so the card reads "not measured; nothing is parsed" |
| `arena_venue_raster` (`registry.rs:3595`) | same `MEASURED` gate, so it is refused |
| `AssetRoots::resolve` / `art_root` (`assets.rs:228`, `:680`) | the rendering root is `game\rendering`, which does not exist on this install |

**Edge case for a regression (source reading):** an `nhllegacy` / Xbox project with the **Recompiled** root attached.
- The Gen7 contract applies, and `list_dir` of the non-existent `game\rendering\stadium` returns `Dir::Absent`.
- Every arena file then shows **"missing"**: "the `stadium/` directory is absent: its files are missing" (`arena_assets.rs:525`).
- A platform-mismatch note is added (`registry.rs` `arena_assets_with_defs`). The status is still the wrong class: the files ship,
  as archive members.
- Not reproduced.

## 4. Contract (developer; read route only)

**Title and roots.**
- `legacyrecomp` (alias `nhllegacy-recompiled`) with `Platform::Recomp`.
- The two root forms and their deterministic resolution follow 18 K21: `<install>` with `game\*.big`, or `<install>\game` with
  `*.big`. Refuse ambiguity.
- Admit the arena contract as Gen7 with `rx2` and `jumbotron_shipped: true`, the same grammar as `nhllegacy` (R62 measured Legacy;
  the archives are identical, R128).
- Add `("legacyrecomp", Recomp)` to the venue-art measured set **only together with** the member route. A Recompiled root must
  never fall through to the loose `list_dir` that reports "missing".

**Member paths.**
- `rendering/stadium/<stem>.rx2`, `rendering/jumbotron/texlib_<arenaartid>.rx2`, `rendering/icesurface/<stem>.rx2` and
  `rendering/banner/<stem>.rx2`: lower-case, exact, forward slashes.
- Archive search covers all 13 top-level EB archives; `icesurface` spans two of them.
- Listing a family means listing the archive index under that directory prefix. That replaces `list_dir`, with the same stem
  grammar (`parse_id`) and the same spare-key arithmetic.

**Index and duplicates.**
- Build one archive index per attached root (receipt 67's containment index is the precedent).
- Refuse conflicting duplicate paths within or across archives (0 measured).

**Status classes.**

| Case | Status |
|---|---|
| Stem in a readable complete index | present: `archive:member` |
| Stem absent, every archive listed | missing |
| Stem absent, any archive unlistable | **unknown**: "not proven absent" |
| Root with no archives and no loose tree | unknown (or the existing "attach" note) |

**Decode.**
- The shared unwrap (`read_member_unwrapped` / `read_eb_member`): chunklzx only, here. A decode failure is an error naming the member.
- Then the unchanged parsers: `venue::read` for packages; the existing texlib / texture path for jumbotron, ice and banner previews.

**Venue raster preview.**
- `venue_raster_png` is keyed by a loose path and the listed file SHA-256.
- The member variant keys by `archive`, `member` and the **unwrapped** SHA-256 the listing showed. It re-reads, re-unwraps and refuses
  on mismatch.
- Keep the arena-art-id file-name check (`registry.rs:3609`).

**Loose policy.**
- Loose first is Studio policy (18 K21). This install has no loose tree.
- A partial loose `rendering/<dir>` falls back per file, with the source shown.

**Provenance line.**
- "archive member `<archive>:<path>` (chunklzx, <stored> B; unwrapped <len> B, SHA-256 …)".
- "Matches X360 Legacy" may be shown only after hashing the current source against a recorded reference.
- Runtime selection is not observed.

**Not included:** any arena, board, ice or banner **write** on Recompiled or Xbox 360. Preview availability is not writer acceptance;
`board_replace` keeps refusing Xbox / Recomp as withheld.

## 5. Smallest regressions

**Positive** (registered install, both root forms; against the X360 Legacy loose tree as reference):
1. The arena card for one arena whose `arenaartid` has a package. The family statuses, `shipped_files` counts and spare keys equal the
   loose-tree card: stadium `_container_0` 64 (plus the 3 named ice bases), jumbotron 66, centre-ice logos 231, banners 218,
   bounce maps 37.
2. Venue art for that arena: the same package list. Each package's unwrapped SHA-256 equals the loose file's SHA-256, and the materials
   and rasters are identical.
3. One raster PNG is pixel-equal to the loose route.
4. An `icesurface` member from `renderboot.big` (`ice_normal_nm` / `ice_spec_sm`) is found, proving the search is not
   `cacherender.big` only.

**Negative:**
1. An owned synthetic EB with one corrupted chunklzx `stadium` member: an error naming it. Never "stored", never "missing".
2. A root with one unlistable archive: absent stems are **unknown**, not missing.
3. A root without archives: unknown or "attach", never "missing".
4. `nhllegacy` / Xbox project plus the Recompiled root (§3 edge): no "missing" status for shipped members.
5. A venue raster request with a stale unwrapped SHA-256, or a file outside the arena's art id: refused.
6. `board_replace` on Recompiled: still refused.
7. The X360 Legacy loose tree: unchanged.

## 6. Developer question from receipt 67, answered within bound: the two "unlistable" archives

Receipt 67 §3 shows 1 129 portraits as **unknown** because `gamedata/gps.big` and `gamedata/anim/faceposer.big` could not be listed.
Names only (R147b):

| Archive | Size | What it is | Paths under `rendering/` or `fe/` |
|---|---|---|---|
| `gamedata/gps.big` | 2 487 B | classic BIG4 with an **inline** directory: no `.bh` sidecar needed. Size word LE = file length; count 15 (BE); header length 206; rows BE offset / size + NUL name, all in bounds. Names: `ALL DL1 DL2 DL3 DL4 GOL GS1 GS2 GS3 GS4 H2H HUT OTP PRE V1.00` | **0** |
| `gamedata/anim/faceposer.big` | 8 050 064 B | EB version **0x00200400**. Name table: 904 distinct `gamedata/faceposer/faceposer_<N>.cba`. With the 16-byte row assumption the offset / size words are **not** all in bounds, so that version's row layout is unverified | **0** (names) |
| `gamedata/aidata.big`, `gamedata/nis.big` | 1 504 / 79 504 208 B | EB 0x00000400: 3 entries at the root; 321 `nis/…` | 0 |

**Consequence:**
- Neither archive can hold a portrait, head or arena member, by its names.
- A developer reader that lists both by name (BIG4 inline directory; EB 0x00200400 name table) would let the containment card
  decide those ids as missing instead of unknown.
- Offsets in `faceposer.big` are **not** established, so a name-only listing must not be used to read members.
- Whether Studio admits a name-only classification is the developer's / coordinator's decision. Research does not change the
  current honest "unknown".
