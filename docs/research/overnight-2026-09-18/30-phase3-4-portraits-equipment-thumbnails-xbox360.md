# Handoff P3/P4-X — portrait and equipment-thumbnail locations on the Xbox 360 family

Research lane, overnight 2026-09-18. Claim `p3-portrait-equipment-thumbs-xbox360`. Consumer: developer Phase 3.4
(portraits) and Phase 4 (equipment catalog thumbnails); closes the AS7 open item "Legacy Xbox 360 portrait /
create-player thumbnail parent archive unresolved". Machine-readable: `data/xbox360-assets-playerhead.csv`,
`data/xbox360-assets-createplayer.csv` (every member, archive, entry, offset, size, magic). Probe: `asset_grep.py`.

Evidence: **observed** from the archive directory tables on the X: trees (member bytes not decoded except
the league-logo specimens in P1.4-X, which share the same package shape); id joins to `ubPc.lOnr`/`pPaD`
were **not** recomputed tonight (AS7's join records stand).

## Portraits (`fe/ion/artassets/playerheads*`)

| Title | Large portraits | Small portraits | Layout | Count (large) | Ids |
|---|---|---|---|---|---|
| NHL 08 | `data0.big` `fe/ion/artassets/playerheads/p{N}.big` | `data0.big` `…/playerheadssmall/p{N}.big` | **flat** (no buckets) + `h{N}` ×2, one unnumbered | 3 493 | 0..10 314 |
| NHL 10 | `fe.big` `…/playerheads/{p0_4000,p4001_8000,p8001_12000}/p{N}.big` + `silhouettes/p6000{0..3}.big` | `fe.big` `…/playerheadssmall/<same buckets>` | bucketed | 3 637 | 1..8 331 |
| NHL 12 | **`nocacherender.big`** `…/playerheads/<buckets>` | **`cache.big`** `…/playerheadssmall/<buckets>` | bucketed | 6 158 | 1..9 798 |
| NHL 15 | **`nocache.big`** `…/playerheads/<buckets>` | `cache.big` `…/playerheadssmall/<buckets>` | bucketed | 8 362 | 1..11 384 |
| NHL Legacy | `nocache.big` `…/playerheads/<buckets>` | `cache.big` `…/playerheadssmall/<buckets>` | bucketed | 9 224 (+4 silhouettes) | 1..12 401 |

So the AS7 leads were right in kind and wrong in archive: on Legacy Xbox 360 the large portraits are in
**`nocache.big`** (not `cacherender`/`nocacherender`), the small ones in **`cache.big`**; NHL 12 is the odd one
(`nocacherender.big` for large). NHL 13/14 follow the same split as 15/Legacy (see CSV). The disc ships
these archives only; the X: trees also carry the loose extraction under `fe/ion/artassets/playerheads/`
(same relative paths). `patch.big` (PS3) has no Xbox analogue on these installs (no title update present).
Extras: `cards/playerheads_special_{N}.big` (197 members, NHL 12+, `cache.big`) — HUT/card art variants;
`fe/ion/game/components/nhlplayerheadloader.big` (the ION component that loads them — the runtime lookup
logic, if data-driven, lives there); `editplayer/playerheads/h{N}` (08/10).

Package shape: the same six-member BIGF → RefPack → texture chain as league logos (XPR2/TX2D on 12–Legacy,
RW4 on ≤11 — *inferred from the league-logo family; verify one portrait per era before relying on it*).
Studio's `tdb-rx2::xpr` fixture list already names `playerheadssmall_p1512.xpr2`.

## Equipment / appearance thumbnails (`fe/ion/artassets/createplayer/<family>/<family>{N}.big`)

Counts per family (= the visible catalog size the create-player screens can show; not necessarily the DB
catalog size — compare with `equipatrib.bin` / the equipment tables):

| Family | 08 | 10 | 12 | 15 | Legacy | Notes |
|---|---|---|---|---|---|---|
| `sticks/stick{N}` | 14 | 44 | 66 | 101 | 106 | 4.1 |
| `gloves/gloves{N}` | 9 | 32 | 56 | 74 | 83 | 4.2 |
| `helmet/helmet{N}` | 6 | 24 | 29 | 34 | 38 | 4.3 (+ `visor/visor{0..2}` = 3 everywhere) |
| `skates/skate{N}` | 9 | 27 | 37 | 52 | 58 | 4.4 |
| `pads/pads{N}` | 19 | 38 | 64 | 87 | 92 | 4.5 |
| `blocker/blocker{N}` | 18 | 37 | 65 | 88 | 93 | 4.5 (Legacy also one stray `gloves{N}` in this folder) |
| `trapper/trapper{N}` | 18 | 37 | 63 | 86 | 91 | 4.5 |
| `goaliestick/goaliestick{N}` | 39 | 50 | 50 | 48 | 54 | 4.5 (cf. AS7 `rendering/goaliestick` ids 6–43 and the `fMVl.wXFx` question) |
| `mask/mask{N}` (10+) / `maskpattern` 23 + `maskstyle` 3 (08) | — | 81 | 88 | 88 | 88 | 4.5 mask/paint |
| `head/head{N}` | 9 | 24 | 208 | 210 | 421 | 3.4 create-player heads (Legacy doubled) |
| `hairstyle` (08: 17) / `haircolor` 6 / `eyebrow` 8 / `eyebrowcolor` 6 / `eyecolor` 8 / `facialhair` 8 / `facialhaircolor` 6 | | | | | | 3.4 appearance pickers (stable 10–Legacy) |
| `brands/brand{N}` (+ `brands_small`) | 9 | 13 | 16 | 18 | 19 | equipment brand logos |
| `boost_applied` 23–25, `max` 13+1, `proshop` 10 | | | | | | pro-shop/boost UI |

All in `cache.big` on 12–Legacy, `fe.big` on 10, `data0.big` on 08. These are thumbnails; the geometry/
textures for the on-ice models are under `rendering/<family>/` (not surveyed tonight beyond goalie sticks in AS7).

## What development can do now

- Point the portrait matcher at the right archive per title (table above) with the bucket rule
  `p0_4000 / p4001_8000 / p8001_12000` (N ≤ 4000 / 4001..8000 / 8001..12000; silhouettes 60000..60003) and
  the flat rule for NHL 08.
- Use the thumbnail counts as the upper bound for each equipment picker per title, and the thumbnail member
  as the preview source for Phase 4 catalogs.

## Not established

- Runtime selection between loose tree, `cache.big` and `nocache.big` copies; whether `nhlplayerheadloader`
  hard-codes the bucket rule; the texture formats per era for portraits (assumed as league logos).
