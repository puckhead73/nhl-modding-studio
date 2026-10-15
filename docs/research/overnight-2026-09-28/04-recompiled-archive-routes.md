# Handoff 04 (28 Sep) — Legacy Recompiled: archive-member routes per Studio operation, resolver behaviour on the registered install, save sidecars

Research lane, 28 September 2026, published 05:25 MDT (system clock). Prompt package 5. Claim `research-q5-recompiled-routes`.
Read-only:
- archive listings of the registered install;
- 52 members unwrapped and compared with the X360 Legacy loose files;
- six save sidecars parsed structurally (no display names copied, no save payload decoded);
- Studio source read, not executed.

Nothing launched; no other repository entered.

Machine-readable: [`data/r138-recompiled-archive-routes.json`](data/r138-recompiled-archive-routes.json). Probe:
`local/runs/overnight-research-2026-09-28/probes/r138_recompiled_archive_routes.py` (`9c14ebab…`). Builds on 27 Sep handoff 21 (R128), as
corrected in [00](00-correction-checkpoint.md) K5.

## Plain-language summary

The Recompiled install keeps everything inside its game archives; there is no extracted folder. Every file that Studio reads from the
Xbox 360 Legacy folder tree exists exactly once as a member of one of those archives. Every member research checked is byte-for-byte
the same file once unwrapped. So each Studio operation has a known place to find its bytes in Recompiled: which archive, which member,
and how it is wrapped. That is not the same as Studio working there today:
- Studio's picture and sound routes look for extracted folders, which this install does not have;
- its Xbox sound previews exclude Recompiled by name;
- which copy the recompiled program loads, and whether it accepts a changed archive, are still unobserved.

The save sidecars match the layout Studio already writes.

## 1. The registered install (`H:\Emulators\games\XBOX\NHL Legacy Decomp Test\`)

| Item | Result |
|---|---|
| `game\` | 13 EB archives (`audioboot`, `audioboot2`, `boot`, `cache`, `cacheaudio`, `cacheboot`, `cachemovie`, `cacheoverlay`, `cacherender`, `data0`, `nocache`, `nocacherender`, `renderboot` `.big`), `gamedata\` (4 `.big`), `default.xex`, `default - Copy.xex`, `nxeart`, `options.viv` |
| `_compiled\`, loose `rendering\`, loose `db\` | **absent** |
| Archive members | 44 602 (e.g. `cache.big` 17 760, `cacherender.big` 12 845, `nocache.big` 9 707, `nocacherender.big` 3 237) |

## 2. Coverage and identity against the X360 Legacy loose tree (the tree Studio's loose routes read)

- The X360 Legacy tree has 44 606 files below its root folder:
  - **44 602** have exactly **one** Recompiled archive member with the same lower-case path (no path in two archives);
  - the other 4 are `gamedata\*.big`, which Recompiled ships as the same files (R128).
- *(Scope note 28 Sep 13:21, [09](09-after-receipt5-corrections.md) K9: the 44 602 are **path** matches. Bytes were compared only for the 52 samples below; whole-archive identity is R128's separate archive-to-archive evidence.)*
- Identity sample: 52 members across 13 classes (the smallest and middle-sized per class). **52 / 52** are byte-identical to the X360 loose
  file once unwrapped (`stored` or `chunklzx`).

| Studio operation class | Recompiled archive (member folder) | Wrapping (sampled) | Sample (member → SHA-256 of the unwrapped bytes) |
|---|---|---|---|
| Main / aux databases | `cacheboot.big` (`db/`) | stored | `db/nhlng.db` = anchor `105d2186…` (R128); `db/dynamic_ratings.db` `f1efa6a2…` |
| Localization dictionaries | `cacheboot.big` (`fe/loc/`, 8 `.db` + 8 `.xml`) | stored | `nhl_eng_us.db` (the 18 Sep league-parity archive route) |
| Sound banks `.abk` | `nocache.big` (206), `audioboot2.big` (10), `audioboot.big` (1) | stored | `audio/sfx/024_goal.abk`, `0293_chant.abk` |
| Sound streams `.ast` | `nocache.big` (7), `cacheaudio.big` (7) | not sampled here | — |
| Speech / music | `nocache.big`, `audioboot.big` (`.sbr` / `.sbs` / `.bin` / `.csi`) | stored or chunklzx | `audio/music/music_samples.sbr` (chunklzx) |
| Kits / nameplates | `nocacherender.big` (1 869), `renderboot.big` (14) | chunklzx | `rendering/jersey/name_0_326_2_cm.rx2` `e8c153c5…` |
| Rink-logo family | `cacherender.big` (1 926) | chunklzx | `rendering/logo/texlib_30_0_0.rx2` `4cad4102…` |
| Heads / player meshes | `cacherender.big` (1 589) | chunklzx | `rendering/player/player_186.rx2` `bd1cd73c…` |
| Venue packages | `cacherender.big` (67) | chunklzx | `rendering/stadium/stadium_326_container_0.rx2` (all 67 through the identical X360 Legacy archive: [03](03-board-art-replacement-boundaries.md) §5) |
| Team / league logos, calendars, jerseys (FE) | `cache.big` (`fe/ion/artassets/teamlogos*`, `leaguelogos*`, …) | stored | `fe/ion/artassets/teamlogoscalendar/c154.big` |
| Portraits | `nocache.big` (`playerheads`, 9 228); small portraits `cache.big` (`playerheadssmall`, 9 229) | stored | `fe/ion/artassets/playerheads/p4001_8000/p5135.big` |

## 3. What Studio does on this install (source reading, 28 Sep working tree)

- **Art roots** (`assets.rs` `AssetRoots::resolve`): anchor = the `.xex` folder (`game\`), preferring `game\_compiled\rendering` / `fe`,
  else `game\rendering` / `fe`. Neither exists here, so every **loose** art route finds no files; the code calls this "not extracted yet".
- **Discovery** (`discovery.rs` `detect_recomp`): database paths come from `resolve_db_paths` and `note_extract_needed`. The main database
  is an archive member here (`cacheboot.big`, stored).
- **Archive-member routes that already name Recompiled:**
  - the gen7 logo route (`gen7_logo_archive` → `cache.big`; test `gen7_asset_member_route.rs`);
  - the 18 Sep league-parity dictionary route (`cacheboot.big:fe/loc/nhl_eng_us.db`, recorded as experimental with loader behaviour
    unknown).
- **Audio:**
  - `gen7_sfx_banks.rs`' coverage table lists "Xbox 360 NHL 10 → Legacy (+ Recompiled by lineage), loose `audio/sfx`". On this install
    that loose route has **no files**; the banks are archive members (§2).
  - `xma_gate` refuses every non-Xbox platform by name ("no measured EA-XMA vector on this source"). Both statements are true of the
    current code, but "by lineage" names a route this install cannot serve. **Observation for the developer:** wording or route mismatch,
    not a wrong write.
- **Portrait / head reassignment** refuses Recompiled (`player_reassign.rs`: archive-held, no loose tree). That is consistent with §2.

## 4. Why an Xbox loose-file test cannot stand in for a Recompiled archive route

A loose-file test reads `X:\…\NHL Legacy\audio\sfx\024_goal.abk`. A Recompiled route must:
1. find the member in the right archive by path hash (EB rows: djb2 of the lower-case path);
2. unwrap it (stored / `chunklzx`);
3. for a write, re-wrap it and re-plan the archive (`chunklzx` store acceptance is still card R30-LZX-1; stored members need no codec).

*(Narrowed 28 Sep 13:21, [09](09-after-receipt5-corrections.md) K9: the archives are identical to the X360 Legacy archives (R128). Equality with the X360 **loose** file is measured for the 52 samples and, via R137, the 67 venue packages; for other members it holds only as far as each loose file equals its own archive member.)* The **bytes** are identical, so every codec, framing and decoder fact measured on the X360 Legacy files (R84 / R92 vectors, texlib
rules, venue rules) **applies to these members**. The route, the write path and the loader's choice between copies are separate facts.
Byte identity says nothing about loader precedence (`update:` / `cache:` probing is only seen in one old log and is not relied on).

## 5. Save sidecars (Studio `content_header.rs` layout)

| Save folder (type token at offset 0 of the save file) | Sidecar | Type word | Const | Content name = file stem | Profile / title words = directory names | Tail 0 | Save version word |
|---|---|---|---|---|---|---|---|
| ROSTER (`RosterFile`) | 328 B | 1 | 1 | yes | yes / yes | yes | 4 |
| PROFILE (`UserProfileFile`) | 328 B | 1 | 1 | yes | yes / yes | yes | 4 |
| HOCKEYCARD (`HockeyCards`) | 328 B | 1 | 1 | yes | yes / yes | yes | 4 |
| DYNASTY × 2 (`Dynasty…`) | 328 B | 1 | 1 | yes | yes / yes | yes | **5** |
| BUILDYOURAI (`TXT_BUILD_YOUR_…`) | 328 B | 1 | 1 | yes | yes / yes | yes | 4 |

**Findings:**
- All six sidecars match the documented 0x148-byte layout; the folder and file names agree with the stored words.
- **Studio treatment:**
  - `kind_from_magic` classifies BUILDYOURAI as `unknown`, so it is not listed;
  - the DYNASTY saves report container version 5, which the codec does not write (listed read-only). That is existing, documented
    behaviour.
- These are the user's own saves. Research read only structure words, and the contents are not compared with any anchor.

## 6. Contract for the developer (routes; no new write authorisation)

1. **Source identity per operation.** For Recompiled, bind:
   - the install's archive by SHA-256 (R128: identical to the X360 Legacy tree's archives);
   - the member path, its wrapping, and the unwrapped SHA-256.

   Cite R128 + this handoff to reuse an X360 Legacy codec fact on identical member bytes.
2. **Read / preview routes** may read these members directly (§2 table). The logo route already does. For audio, an archive-member bank
   route mirrors the PS3 archived-bank route of receipt 36. Admission then follows the X360 Legacy vectors *for these bytes*. It stays
   labelled "Recompiled: bytes identical to X360 Legacy; runtime unobserved".
3. **Writes** go only through the existing archive-member stager and copied-target contract, stored members first. `chunklzx` members
   (all `rendering/`, some audio) need the LZX store route and card R30-LZX-1 before any claim.
4. **Keep refusing** where a route reads loose folders. Say "this install keeps its files inside its archives", not "not extracted", when
   `_compiled\` is absent by design.

## 7. Not established (runtime or other builds)

- Which copy the recompiled executable loads (archive vs any loose / `_compiled` folder, `update:` / `cache:`).
- Whether it accepts a re-planned archive.
- Save compatibility beyond the sidecar layout.
- Any other Recompiled build.

## 8. Residual XMA shapes (package 5, second half)

- Recompiled had the concrete new lead, so it was taken first.
- The XMA residuals (two-segment in-bank records, looping ram records on NHL 10+, > 2 channels, NHL 11 / 13 / 14 `.ast` shapes) are
  picked up later in this run if time allows (see STATUS). None is claimed here.

## 9. Reproduce

```
python -B local/runs/overnight-research-2026-09-28/probes/r138_recompiled_archive_routes.py <out.json> [--per-class 4]   (≈ 3 min)
```
