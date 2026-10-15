# Handoff P1.4-X — league logo assets: DB code → executable logo index → FE package → texture

Research lane, overnight 2026-09-18. Claim `p1.4-xbox360-league-logo-assets`. Consumer: developer Phase 1.4
(league logos / selection tiles), S5/C4 logo resolver. Machine-readable:
[`data/xbox360-league-logo-map.json`](data/xbox360-league-logo-map.json) (per title: the executable table,
every code's logo index and package paths, every `leaguelogos*` package with archive/entry/offset/size and
decoded texture format/dimensions) and [`data/xbox360-leaguelogo-members.csv`](data/xbox360-leaguelogo-members.csv)
(all 621 `*leaguelogo*` members across the ten titles). Previews (Legacy Xbox 360, decoded tonight with a
port of `tdb-rx2`'s untile/XPR2 logic): [`previews/leaguelogos/`](previews/leaguelogos/) incl. the contact
sheet `contact-nhllegacy-x360-leaguelogosbig.png`. Probes: `asset_grep.py`, `league_logo_map.py`, `fe_pkg_probe.py`, `xpr_preview.py`, `refpack.py`.

Evidence: **observed** on the X: trees and executables; the *meaning* of each logo index below is my reading
of the decoded Legacy previews (**inferred**, queued for the user's visual check); runtime consumers unknown.

## Plain-language summary

League logos on the Xbox 360 titles are *not* addressed by the league code directly. The executable holds a
second small table — league code → **logo index** — and the frontend loads `fe/ion/artassets/leaguelogos/l<index>.big`
(plus `leaguelogosbig/`, `leaguelogossmall/`, `leaguelogoswide/` variants of the same index). The logo index
is a historically stable enum (NHL 08's league order with later leagues appended), which is why `l8`/`l9`
(Russian Superliga, Historic teams) vanished after NHL 10 while the numbering of later leagues stayed. Several
codes deliberately share one logo (Winter Classic / United States / Russia / Rest-of-World / HUT → NHL
`l0`; Custom / Legend / EASHL → the EA Sports tile `l5`; Prospects and CHL → the CHL crest `l14`).
The packages are six-member BIGF files with the texture RefPack-compressed inside: XPR2/TX2D on NHL
12–Legacy (Studio's existing portrait codec), RenderWare `RW4xb2` on NHL 07–11 (Studio's `.rx2` texlib
family — different chain, same tiling), and DDS on PS3 (same file names and index). Everything needed to
preview, replace and repack an existing league logo is therefore already known; adding a *new* logo index
would need the executable table changed (out of scope).

## 1. The executable table (observed; u32 BE array, one entry per league code)

| Title | VA | Values (code order) |
|---|---|---|
| NHL 09 | `0x82098854` | 0 1 2 3 6 7 **8** 4 **9** 5 |
| NHL 10 | `0x820ea330` | 0 1 2 3 6 7 10 4 **9** 5 5 |
| NHL 11 | `0x82251278` | 0 1 2 3 6 7 10 4 12 13 11 5 5 14 |
| NHL 12 / 13 / 14 / 15 | `0x8212dab8` / `0x820408f0` / `0x82036648` / `0x82036a20` | 0 1 2 3 6 7 10 4 12 13 11 14 5 5 0 14 0 0 0 0 (**identical on all four**) |
| NHL Legacy | `0x820369f0` | 0 1 2 3 6 7 10 **15** 4 12 13 11 14 5 0 5 14 0 0 0 0 5 |

Located by the sequence `0,1,2,3,6,7` (unique in every image) directly after a float/constant block; length
equals the league-key count of the same title (P1.1-X), which is the structural check that ties the two tables.

**Cross-generation identity (observed):** on PS4 the same enum is stored in the database as
`stockleagues.artid` (`fsLK.lOnr`): NHL 15 PS4's 20 rows carry `0 1 2 3 6 7 10 4 12 13 11 14 5 5 0 14 0 0 0 0` —
byte-for-byte the NHL 15 Xbox 360 executable table — and NHL 21 extends it (17 ECHL, 18 EBEL, 19 Allsvenskan,
20 Champions League, 21 Spengler, 22 3-on-3, 23 USA, 24 Russia, 25 NHL Alumni/All-Time; `renderid` is a second,
different index). So "league art id" is one family-wide enum: executable table on gen7 Xbox 360 (and by
identity PS3), DB column on PS4 — and on PS4 the FE files are named by it directly
(`nhl/ui/kettle_assets/leaguelogos/size256/l{artid}.gnf`, NHL 21 `inxp_10.big`; §3). The same enum also tags every row of the competition-group table
(handoff P1.2-X, `xbox360-competition-groups.json`).

## 2. Logo index → asset (Legacy Xbox 360 previews; index meaning inferred from the decoded art)

| idx | Art (Legacy `leaguelogosbig/l{idx}.big`) | Used by codes (Legacy) | Notes |
|---|---|---|---|
| 0 | NHL shield | 0 NHL, 14 Winter Classic, 17 US, 18 Russia, 19 ROW, 20 HUT | HUT also has its own tile `leaguelogos/lut.big` (64×64) — which consumer uses it is unknown |
| 1 | AHL | 1 | |
| 2 | SHL wordmark | 2 elitserien | NHL 12/13 era art would read "Elitserien" — per-title art differs, index is stable |
| 3 | Liiga | 3 SMLIIGA | |
| 4 | National teams (globe/puck) | 8 NATIONAL | |
| 5 | EA SPORTS tile | 13 Custom, 15 EASHL, 21 Legend | also `leaguelogos/lea.big` (128×128) |
| 6 | DEL | 4 | |
| 7 | ELH (Czech) | 5 O2 Extraliga | |
| 8 | (absent from NHL 11 on) | NHL 09 code 6 Superliga | present on NHL 09/10 only |
| 9 | (absent from NHL 11 on) | NHL 09/10 code 8 Historic teams | present on NHL 09/10 only |
| 10 | National League (Swiss) | 6 | first appears NHL 10 |
| 11 | WHL | 11 | |
| 12 | OHL | 9 | |
| 13 | QMJHL (LHJMQ) | 10 | |
| 14 | CHL | 12 Prospects, 16 CHL | |
| 15 | Norway flag | 7 TXT_LEAGUE_NORWAY | Legacy only; `leaguelogoswide/l15` exists from NHL 11 (unrelated content? not decoded) |

`mudleaguelogos/l0,l5` (NHL 13+) and `onlineleaguelogos*/l0..l16` + `lockimg` (NHL 08+) are separate
families (online/EASHL tiers; "mud" unknown) — do not fold them into the league-code map.

## 3. Package families and codecs (observed)

| Titles | Outer | BIGF members | Texture payload | Studio codec |
|---|---|---|---|---|
| NHL 07, 08, 09, 10 | **RefPack'd** BIGF | 4: `0` Apt Data, `1` Apt const, `<n>` raw texture, `<n+1>` 288-byte record | `\x89RW4xb2` (RenderWare 4, uncompressed inside) | `tdb-rx2` texlib path (RW4 `.rx2`); *not verified tonight on these members* |
| NHL 11 | BIGF | 6: `0`, `<n+1>`, `sg1`, `<n>` RefPack, `sg2`, `1` | RefPack → `\x89RW4xb2` | same |
| NHL 12, 13, 14, 15, Legacy | BIGF | 6 (same shape) | RefPack → `XPR2` / `TX2D` | `tdb-rx2::xpr` (the portrait chain) — **decoded tonight**: `leaguelogos` 64×64 (`lea` 128×128) 8_8_8_8; `leaguelogosbig` 256×256 8_8_8_8; `leaguelogoswide` 256×256 8_8_8_8; `leaguelogossmall` **128×64 DXT5**; `onlineleaguelogos` 256×256 / 32×32 DXT5; DW10 `0x00000C14` (mipmapped) |
| PS3 Legacy (disc `USRDIR/fe/ion/artassets/…` loose **and** `cache.big` members, EB v0x400) | BIGF | 6 (same shape, same member names `0/17/sg1/16/sg2/1`) | RefPack → **DDS** | PS3 DDS path (O5) |
| PS4 NHL 21 (`databigs/inxp_10.big`, EB v0x00110400) | — | `nhl/ui/kettle_assets/leaguelogos/size{64,256,512}/l{artid}.gnf` (+ `l{artid}_fr.gnf` French variants) | `chunkzip` → **GNF** | Studio's PS4 GNF path. Indices present: 0–7, 10–21, 23–25 (= every `stockleagues.artid` except 22 = 3-on-3) plus extras 99/100 — the same family-wide art id |

The XPR2 members are byte-identical in size across NHL 13/14/15/Legacy for the same index (e.g.
`leaguelogos/l0.big` 8 176 B), so most logo art is shared across the family — hash equality per member is in
the JSON offsets/sizes; content hashes were not computed (cheap to add).

## 4. Consumers, precedence, gaps

- Which screens draw which family (`leaguelogos` vs `big` vs `wide` vs `small`) is untraced; Apt/ION screen
  data (`fe/ion/game/screens/*.big`) references them by path — tomorrow's visual pass on the Legacy team-select
  and roster screens will settle it faster than static tracing.
- Xbox: the packages live in `cache.big` (12–Legacy) / `fe.big` (09–11) / `data0.big` (07–08) and are also
  present loose under the X: extraction trees; the disc ships only the archive copies (11+). The `cache:`
  precedence caveat from the source inventory applies to the archive.
- Same-length replacement: a re-encoded 64×64 8_8_8_8 XPR2 has a fixed data size (16 384 B) but the RefPack
  member length varies with content, so an in-place member replacement needs the BIGF repack (member
  offsets) and, in the outer EB archive, either a same-length RefPack stream or a re-planned archive — the
  S2-A planner's same-length rule will usually *not* hold for logos. Padding the RefPack stream to the original
  length (RefPack tolerates trailing bytes after the stop code) is the cheap route to test.
- NHL 07/08: their `leaguelogos` sets are tiny (07: `l0,l1`; 08: `l0..l5`+`lea`), matching their 6-league
  tables 1:1 by code (index == code there, by count — not verified by decode).

## 5. Reproduction

```
python probes/asset_grep.py leaguelogo data/xbox360-leaguelogo-members.csv
python probes/league_logo_map.py                     # needs scratch/xex/*.bin (xex_image.py) and the key-table JSON
python probes/eb_list.py "<cache.big>" --extract fe/ion/artassets/leaguelogosbig/l7.big out.big
python probes/xpr_preview.py out.big out.png          # XPR2 packages only (12–Legacy)
```
