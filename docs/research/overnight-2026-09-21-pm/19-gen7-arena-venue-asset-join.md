# Handoff R62 — gen7 arena / ice assets joined to the DB: `exhibitionarena.arenaartid` names the venue's shipped files (`stadium/stadium_<artid>_container_*`, `jumbotron/texlib_<artid>`) on **100 % of arena rows** on NHL 09–Legacy, both consoles; the centre-ice logo and the rafter banners are keyed by **team** art id, not by the arena; the remaining ice layers stay candidates

> **Scope note — 2026-09-22 pm ([research R74](../overnight-2026-09-22-pm/06-developer-observations-reconciled.md), developer observation O-7).** NHL 09 and NHL 11 carry no `kOtt.buDa` home-arena link, so §2's "show the centre logo / banners through the home team" has no key on those titles (list as "no tenant link on this title"). On 13 / 15 / Legacy the team-keyed files resolve for 152 / 156 / 165 of 174 / 178 / 187 arenas (centre logo) and 171 / 170 / 177 (banners); the residual arenas are those whose tenant's art id ships no file.

Research lane, 22 September 2026, roadmap step 2.5 (Teams: arena / ice — the asset half of handoff 09's field inventory). Claim
`research-r62-arena-asset-join`. Read-only over the 27 anchors' `nhlng.db` and the loose `rendering` trees / EB archive member listings on
`X:`; nothing decoded, no writer, nothing game-verified.

Machine-readable: [`data/r62-gen7-arena-asset-join.json`](data/r62-gen7-arena-asset-join.json) (per title: arena rows and columns, each
file family's grammar, id range, files per id, named members, unparsed sample, the ranked candidate keys with rows hit / distinct values /
share of the shipped id set, both residual directions, and the overlap of each family's ids with `exhibitionteams.artid`). Probe
`r62_gen7_arena_asset_join.py` (≈ 3 min for 14 trees).

## Plain-language summary

Every arena row in the database carries an art id, and that art id is literally the name of the arena's model and jumbotron files on disk:
arena art id 12 means `rendering/stadium/stadium_12_container_0.rx2` and `rendering/jumbotron/texlib_12.rx2`. This holds for **every**
arena row on NHL 11–Legacy on both consoles and for the stadium file on NHL 09 / 10 as well, with no row pointing at a missing file; ten to
fourteen stadium files per title belong to no row (spare venues EA ships but does not list). The two families one might expect to belong to
the arena — the centre-ice logo (`icesurface/centerlogo_<id>_cm`) and the rafter banners (`banner/banner_<id>…`) — are keyed by **team** art
id instead: 191 of 218 centre-logo ids and 210 of 218 banner ids are team art ids, and no arena column explains them. That matches what
the PS4 vault shows (the centre logo binds a Team), so the same presentation rule holds on both generations. The other ice layers (the
bounce map, the centre-line art, the line art and the ice lightmap) have small id spaces that several arena columns can cover, so only the
bounce map has a candidate worth naming (`bouncemap`, which every row satisfies and whose name matches the `_bm` files); the rest stay
unresolved and must not be presented as bindings.

## 1. Established joins (both consoles; "rows hit" = arena rows whose value names an existing file)

| Title | arena rows | `arenaartid` → `stadium/stadium_<id>_container_*` | `arenaartid` → `jumbotron/texlib_<id>` | stadium ids with no arena row |
|---|---|---|---|---|
| NHL 09 (X360) | 188 | 188 / 188 | — (no `jumbotron` dir) | 4 of 43 |
| NHL 10 (X360) | 182 | 182 / 182 | — | 6 of 44 |
| NHL 11 | 242 | 242 / 242 | — (shipped from 12) | 10 of 51 |
| NHL 12 | 243 | 243 / 243 | 243 / 243 | 11 of 57 |
| NHL 13 | 174 | 174 / 174 | 174 / 174 | 10 of 61 |
| NHL 14 | 177 | 177 / 177 | 177 / 177 | 11 of 64 |
| NHL 15 | 178 | 178 / 178 | 178 / 178 | 12 of 64 |
| NHL Legacy | 187 | 187 / 187 | 187 / 187 | 12 of 64 (14 jumbotron) |

PS3 11 keeps `rendering` inside archives this probe could not list (recorded as "no families"); PS3 12 / 13 / 14 / 15 / Legacy reproduce the
X360 numbers exactly. **Zero** arena rows on any title name a stadium or jumbotron file that does not exist, so a listing can show the
venue's model and jumbotron for every arena, and a refusal is only needed for the reverse direction (a spare shipped id).

The `stadium` directory also holds three or four **named** ice bases per title — `ice_nhl_container_0`, `ice_intl_container_0`,
`ice_nhl_full_lm_container_0` (NHL 09 additionally `ice_ahl`, `ice_practice`) — which no id names; they are selected by something other
than the arena art id (unresolved).

## 2. Team-keyed, not arena-keyed (observation, both consoles, NHL 11–Legacy)

| Family | ids | of which are `exhibitionteams.artid` | best arena column |
|---|---|---|---|
| `icesurface/centerlogo_<id>_{cm,nm,sm}` | 218 (13) … 231 (Legacy) | 191 / 218 … 196 / 231 | `index`, only 42–46 % of the id set — **no established binding to the arena table** |
| `banner/banner_<id>[_<k>][_<map>]` | 218 | 210 … 203 of the team art ids | `index`, 50–53 % — likewise none |

So the centre-ice logo and the banners are team presentation assets that an arena page should show via the **home team**, not via the
arena row. (Consistent with the PS4 vault, where the centre logo's texture reference binds a Team.)

## 3. Candidates, explicitly not established

- `bouncemap` → `icesurface/icesurface_<id>_bm`: every arena row's value names an existing file (174 / 174, 187 / 187) and covers 95–97 %
  of the shipped ids; the field name and the `_bm` suffix agree. Strong **candidate**; `puck` and `powerring` also fit the id space, so the
  distinction is not decided by the data.
- `centerline` → `icesurface/ice_cline_<id>_cm` / `ice_lines_<id>_cm` / `ice_lightmap_<id>_lm`: name-consistent, but these families have
  only 4–37 ids, and four to six arena columns cover the id set completely. **Not established** — do not label the field as their key.
- `crowd/chair_<id>_<n>` (2 ids), `banner/header_<id>_<k>_cm` (2 ids): too small to key; listed for completeness.

## 4. Consequence (developer)

- An arena page can name and preview the venue's model and jumbotron files directly from `arenaartid` on every gen7 title (2.5), and list
  the spare shipped venues as "shipped, no arena row".
- Centre-ice logo and banners belong to the **team** surface, keyed by team art id; an arena page that shows them must resolve them through
  the home team and say so.
- The bounce map is a candidate binding; the other ice layers are "shipped files, no established binding" — never "not consumed".
- Refusals: an `arenaartid` with no `stadium_<id>_container_*` (none shipped, so this is a user-created-arena case), and any write to a
  family in §3 while its key is a candidate.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r62_gen7_arena_asset_join.py docs/research/overnight-2026-09-21-pm/data/r62-gen7-arena-asset-join.json   (≈ 3 min)
```
Executed: three runs (the first two with a coarser family set and a join ranking that small id spaces defeated — corrected to rank by the
share of the shipped id set and to split `icesurface` into its five grammars), the third final; two shell listings of the NHL 13
`rendering` sub-directories. Inputs: `anchors/**/databases/nhlng.db` + `-meta.xml`, the loose `rendering` trees on `X:` (paths in
`r56_gen7_kit_selector_join.py`).
