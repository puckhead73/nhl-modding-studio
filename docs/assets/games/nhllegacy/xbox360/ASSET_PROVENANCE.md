# NHL Legacy — Xbox 360 — asset provenance (Phase 5.0)

> **Status:** Phase 5.0 inventory + container reverse-engineering (2026-08-02/03). The `EB\0\3`
> `0x0000_0400` container is **reversed, productionized, and validated**; in-archive payloads are LZX
> (`chunklzx`, decode not yet implemented), but the **user's extracted tree ships decompressed
> `RW4`/`RX2` assets** usable directly.
> **Source (user-provided, read-only):** `X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\` — a
> dynamic mount exposing **both** the packed `.big` archives **and** an extracted tree (`rendering/`,
> `fe/`, `gamedata/`, …). (The mount also transiently presents a `…\NHL Legacy ISO\` view of the same
> archives; the drive spins down between reads.)
> **Tooling:** `tdb-archive-big` (`cargo run -p tdb-archive-big --example big_ls -- <archive.big>`).
> **Clean-room:** this document records archive/path metadata only; **no game asset bytes are copied
> into the repo.**

## 1. Build identity

| Field | Value |
|---|---|
| Title | NHL Legacy Edition |
| Platform | Xbox 360 |
| Install/dump root | `X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy ISO\` |
| Extracted-files root | same (archives are packed; not unpacked) |
| Archives inspected | 13 top-level `.big` + `gamedata\*.big` |
| Date inspected | 2026-08-02 |

## 2. Container format — a two-level `EB\0\3` / nested-`.big` structure

**Every top-level `.big` is an `EB\0\3` archive with version tag `0x0000_0400`** — a **distinct EB
revision from the PS4 one** (`0x0011_0400`). **Reversed + validated (Phase 5.0):**

- **Header + name/dir tables are identical to PS4's** (paths parse perfectly; counts + `archive_size`
  match the file length).
- **The only difference is the file-table entry stride: 16 bytes, not 20.** Field order is the same —
  `[offset_block:u32 @0 (offset = block<<4)] [reserved:u32 @4] [size:u32 @8] [crc:u32 @12]`; the PS4
  variant just carries one extra trailing `u32`. Confirmed against `data0.big`: every payload lands
  16-byte-aligned at `block<<4` with a valid magic (`GD.STRM` for `.cba`, `chunklzx` for `.xml`), and
  the last entry ends exactly at `archive_size`.
- **Productionized:** `tdb_archive_big::eb` now accepts both version tags and picks the stride via
  `entry_size_for(version)` (unit-tested; `EbHeader::version` / `entry_size()` exposed). An end-to-end
  extract through `EbArchive` reads e.g. `rendering/logo/texlib_0_0_0.rx2` at its correct offset/size.

> **EB writer — DONE (write path, 2026-08-03):** `tdb_archive_big::eb::EbFile` re-emits an archive byte-identically
> (verified on real `data0.big`) and `replace_payload` reflows blocks — so the container **rebuild**
> mechanism exists. **Read** proceeds from the extracted tree (payloads ship decompressed as raw
> `RW4`/`RX2`), side-stepping LZX. The remaining **install** blockers are (1) payload codec — in-`.big`
> payloads are `chunklzx` (LZX, decode unimplemented); a write must either LZX-compress or rely on the
> loader accepting a substituted magic-dispatched codec (`chunkzip`/raw — unproven, needs in-game
> test), and (2) the `+12` per-entry checksum (algorithm unidentified). See
> [`PLATFORM_LIMITATIONS.md`](PLATFORM_LIMITATIONS.md).

The front-end archives (`cache.big`, `nocache.big`) are a **second level**: their entries are themselves
thousands of nested `.big` bundles (e.g. `fe/ion/artassets/playerheadssmall/p0_4000/p1.big`), so the
actual asset textures live one archive-level deeper (inner paths like `fe/artassets/playerhead…`).

## 3. Archive inventory

See [`archives.csv`](../../../../audits/data/assets/nhllegacy/xbox360/archives.csv). Summary:

| Archive | Entries | ~Size | Holds |
|---|--:|--:|---|
| `cacherender.big` | 12,845 | 1.46 GB | **stock rendering**: team logos (`rendering/logo/texlib_*.rx2`), players, equipment, ice, banners, crowd |
| `nocacherender.big` | 3,237 | 1.94 GB | **customization rendering**: `jersey` (1,869), `czlogo` (1,065), `czhead`, `czhair`, `czjersey`, `czpant`, `czsock`, `czfont` |
| `cache.big` | 17,760 | 312 MB | **front-end (`fe/ion`)** + nested `.big` bundles (portraits, heads, cards, FE logos) |
| `nocache.big` | 9,707 | 2.30 GB | `fe/ion` + `audio/*` + `rendering/movies` |
| `boot.big` / `renderboot.big` / `cacheboot.big` | 599 / 277 / 61 | — | boot/FE bring-up |
| `audioboot*.big` / `cacheaudio.big` / `cachemovie.big` | — | — | audio / movies |
| `cacheoverlay.big` / `data0.big` | 63 / 5 | — | overlay/UI, misc |

## 4. Graphical asset family map (by content-derived path)

### Portraits / player heads — front-end nested bundles in `cache.big`
- **`fe/ion/artassets/playerheadssmall/…`** — 9,230 nested bundles: the **small player portraits**
  (inner texture path `fe/artassets/playerheadsmall…`).
- **large `playerhead`** — the full-size portraits live inside the same nested bundles
  (inner path `fe/artassets/playerhead…`); a couple of FE loaders reference them
  (`fe/ion/game/components/nhlplayerheadloader.big`).
- **`fe/ion/artassets/cards/playerheads_special/…`** — hockey-card player heads.
- **`fe/ion/artassets/createplayer/head/head*.big`** — create-player 3-D head pool.

### Logos — three distinct sources
- **stock team logos:** `cacherender.big` → `rendering/logo/texlib_*.rx2` (1,926) — texture-library RX2
  (`tdb_rx2::texlib`).
- **front-end logos (multi-context):** `cache.big` → `fe/ion/artassets/teamlogos`, `teamlogosdynasty`,
  `teamlogoswide`, `teamlogossmall`, `teamlogoscalendar` (≈318 each), plus `twittericonsteam`, `flags*`.
- **customization logos:** `nocacherender.big` → `rendering/czlogo/*` (1,065).

### Jerseys / uniforms
- `nocacherender.big` → `rendering/jersey/*` (1,869) + `czjersey`; FE art in `cache.big` →
  `fe/ion/artassets/jerseys` (910); `teamnumbers` / `teamnumbersflat` (600) for nameplate numbers.

### Equipment / appearance (render)
- `cacherender.big` → `rendering/{blocker,trapper,pant,sock,goaliepad,goaliehelmet,glove,helmet,
  goaliestick,playerstick,…}`; customization heads/hair in `nocacherender.big` (`czhead`, `czhair`).

## 5. Format taxonomy (by content)

| Format | Where | Status |
|---|---|---|
| `EB\0\3` v`0x0000_0400` container | every top-level `.big` | listed; **payload read unsupported (blocker above)** |
| nested `.big` bundle | `fe/ion/artassets/**/*.big` | inner archives, one level down; unreadable until the outer read path works |
| `.rx2` (EA RenderWare container) | `rendering/**` (12,705 in `cacherender`) | decoder exists (`tdb_rx2`); **Xbox 360 texture variant CONFIRMED** against real extracted logos (2026-08-03) |
| `texlib` `.rx2` | `rendering/logo/texlib_*.rx2` | texture-library RX2 (`tdb_rx2::texlib`) — **confirmed decode**: DXT5 256×128 color map + Xbox tiling + 16-bit-pair swap → correct `Red Wings` wordmark etc. |
| `BIGF` portrait bundle | `fe/ion/artassets/playerheads[small]/p*/p*.big` | classic EA `BIGF` → RefPack → `XPR2`/`RX2` raster; **confirmed decode** via `decode_portrait` (24/24 real bundles → correct headshots) |
| `.ttf` / `.dds` / `.png` / `.fsh` / `.cwd` / `.dat` | scattered | catalogued; decode TBD |

## 6. Per-operation capability (this title, all graphical families)

Confirmed families: **portraits, logos, uniforms (jersey/pant/sock), equipment** (helmet/glove/
blocker/trapper/goaliepad/goaliehelmet/goaliestick/playerstick). Per-family detail +
`reassign-existing` in [`capability.csv`](../../../../audits/data/assets/nhllegacy/xbox360/capability.csv).

`discover = pass` · `identify = pass` (logo↔team, portrait/head↔player, uniforms↔team all resolved to
real names; **equipment = partial** — goalie masks team-keyed, glove/stick brand-keyed player link
deferred; see [`IDENTITY_MAP.md`](IDENTITY_MAP.md)) · `preview =
pass` (**confirmed 2026-08-03** — real extracted logos, portraits, uniforms + equipment decode
correctly, see below) ·
`export = pass` (same decode → PNG/DDS) · `replace-modified = blocked` (needs LZX compress + EB repack)
· `rebuild = blocked` (LZX + EB writer) · `install = blocked` · `game-verified = not-run` (manual
emulator/console gate). The container is no longer a blocker; the remaining blocker is **LZX
(de)compression + an EB `0x0000_0400` writer** for the write path.

**Preview/export confirmation (Phase 5.1, 2026-08-03).** The gated tests
`head_preview::tests::legacy_x360_{logos,portraits}_decode_from_extracted_tree` (studio-domain) drive
the production decode functions against the read-only extracted tree:
- **Logos** — 24 `rendering/logo/texlib_*.rx2` → 72 rasters via `tdb_rx2::texlib::parse` +
  `linear_to_rgba`. Every raster's dims/format/base-mip size validate; 20/24 carry a rich DXT5 256×128
  color map (the rest are unused-team placeholder slots). Eyeball: `texlib_10` → a clean **Red Wings**
  wordmark (team 10 = Detroit — id matches).
- **Portraits** — 24 `fe/ion/artassets/playerheadssmall/p0_4000/p*.big` → **24/24** via
  `decode_portrait` (BIGF → RefPack → XPR2/RX2 → RGBA). Correct, upright headshots with intact alpha.

The tests skip cleanly when the mount is absent (CI / clean-room); set `QA_ASSET_OUT` to dump PNGs.
No new decode code was needed — the existing `tdb_rx2` + `decode_portrait` paths already handle the
Xbox 360 variant; this step *verified* them against real Legacy files.

**Uniforms + equipment confirmation (Phase 5.2 / 5.3, 2026-08-03).** Test
`assets::tests::legacy_x360_outfit_and_equipment_decode_from_extracted_tree` decodes 11
`rendering/<family>` folders content-driven (try `texlib::parse`; a mesh `.rx2` yields zero textures
and is skipped, mirroring `process_one_outfit`). All confirmed (global max 64 colors):
- **Uniforms** — `jersey/`, `pant/`, `sock/`: color maps decode correct (striped socks, jersey
  nameplates/numbers). Team-keyed; resolved by the existing `export_team_outfits` /
  `derive_team_index` (the team-id token position is title-specific, derived at runtime — **not**
  hard-coded; `identify = pass`).
- **Equipment** — `helmet/`, `glove/`, `blocker/`, `trapper/`, `goaliepad/`, `goaliehelmet/`,
  `goaliestick/`, `playerstick/`: color maps decode correct. Eyeball: `goaliehelmet_0_0_cm` → a clean
  **Anaheim Ducks** goalie mask (team 0 = Anaheim — id matches identities.csv). Helmet/pant folders
  are mostly *meshes* (chinstraps/cages), correctly skipped. Equipment `identify = partial`: goalie
  masks are team-keyed, but glove/stick/pad art is **brand/equipment-keyed** — the player↔equipment
  link (via the equip tables) is deferred.

## 7. Honest unknowns / next steps (for review before any codec work)

1. ~~Reverse the `EB 0x0000_0400` file-table layout~~ — **done** (16-byte stride; productionized +
   validated).
2. ~~Confirm the `RW4`/`RX2` Xbox-360 texture variant decodes with `tdb_rx2`~~ — **done 2026-08-03**
   (logos DXT5 + portraits BIGF/XPR2/RX2 → correct images; `preview`/`export` = pass). See the
   confirmation box in §6.
3. **LZX (`chunklzx`) decompression + an EB `0x0000_0400` writer** — required for reading straight from
   the `.big` and for the `replace`/`rebuild`/`install` write path (now the #1 remaining blocker).
4. Descend the nested `fe/ion/artassets/**/*.big` bundles to catalog the inner texture layout
   (`fe/artassets/playerhead…`, `playerheadsmall…`).
5. Build the **identity map** (portrait/head/logo/jersey ↔ team/player DB ids) — `IDENTITY_MAP.md`.
6. Whether this same `EB 0x0000_0400` variant is used by the other Xbox 360 titles (NHL 07–15) or is
   Legacy-specific.
