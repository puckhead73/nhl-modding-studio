# Handoff R38 — Gen7 equipment (Phase 4): the database selector → file-name grammar chain on PS3 / Xbox 360 NHL 11–Legacy

Research lane, 21 September 2026, Priority 4 (gen7 counterpart of the PS4 chain in 2026-09-20 handoff 09). Claim
`research-r38-gen7-equipment`. Read-only over the loose `rendering/<family>` trees (X:) and the PS3 anchors; nothing game-verified; no writer.

Machine-readable: [`data/r38-gen7-equipment-file-grammar.json`](data/r38-gen7-equipment-file-grammar.json) (per title / platform: every
family's file-name grammar — prefixes, `n1` / `n2` number sets, texture-kind suffixes — and the heuristic column matches; the containment
table below was computed from it plus the anchor columns). Probe `r38_gen7_equipment_file_grammar.py` (≈ 1 s).

## Plain-language summary

On the old consoles the equipment catalogue is not a vault: it is the file names. Each family directory holds meshes and textures named
`<part>_<brand id>[_<variant>]_<kind>`, and **every** brand / model id the database assigns to a roster player has its files on every title
measured — gloves, helmets (with chin strap, jaw protector, padding and visor parts), skates, sticks, goalie pads / blocker / trapper /
stick, mask paints and mask shells — while the directories hold 1.5–4× more ids than the rosters use (the unused catalogue). Jerseys,
pants and socks are keyed by the team's art id. So a gen7 equipment reassignment is a database edit validated against the file numbers of
that title, the existing-entry replacement is a texture / mesh replace of the named files (the RW4 rules of handoff 11), and a *new* brand
is a new file set plus a new id — no registration table exists to update, which is both simpler and unverified at runtime.

## 1. Grammar (measured; identical on PS3 `.rpsgl` and Xbox 360 `.rx2`)

`<prefix>_<n1>[_<n2>[_<n3>]][_<kind>].<ext>` — `kind` ∈ `dm` (diffuse), `tm` (tint / team-colour mask), `nm` (normal), `sm` (specular), `am`,
`cm` (colour), `csm`, `rm`; no kind = mesh. `fe<part>` = the frontend (menu) copy of the same part. Families and the id position:

| Family dir | Prefixes (Legacy) | Id position | Other numbers |
|---|---|---|---|
| `glove` | `glove`, `feglove` | `n1` = glove brand id | `n2` 0–2 = texture set |
| `helmet` | `helmet`, `chinstrap`, `jawprotector`, `padding`, `texlib`, `visor` | `n1` = helmet brand id (visor: `n1` = visor type) | `n2` = variant |
| `skate` | `texlib` (textures), `playerskate` / `goalieskate` / `skatelace` / `skatetongue` (meshes 0 2 3) | `texlib n1` = skate brand id | — |
| `playerstick` | `texlib` (textures), `playerstick` / `playersticktape` (meshes 0 1; tape `n2` 2 4) | `texlib n1` = stick brand id | — |
| `goaliestick` | `goaliestick`, `goaliesticktape` | `n1` = goalie stick id | `n2` 2 4 |
| `goaliepad` | `goaliepad`, `fegoaliepad` | `n1` = pads id | `n2` 0–2 |
| `blocker` / `trapper` | `blocker` / `feblocker`, `trapper` / `fetrapper` | **`n2`** = blocker / trapper id (`n1` 0 / 1 = side or set) | `n3` |
| `goaliehelmet` | `goaliehelmet`, `helmet`, `cage`, `throatprotector`, `texlib` | `n1` = mask type (shell 0–7), **`n2` = mask paint id** (243–280 paints) | — |
| `jersey` / `pant` / `sock` | `texlib`, `name`, `shirt`, `jerseytie` / `texlib` / `sock`, `playersock`, `goaliesock` | `n1` 0 / 1 (home / away set), **`n2` = team art id** | `n3` = variant |
| `puck` | `puck` | `n1` = puck id (249 pucks) | — |

## 2. Containment of the database ids in the file numbers (PS3 anchors; files on both platforms unless stated)

| Column | NHL 11 (X360 = PS3) | NHL 13 (X360; PS3 has no loose `rendering`) | NHL 15 | Legacy |
|---|---|---|---|---|
| `exhibitionskaterequipment.gloves` → `glove n1` | 14 ids **0 / 14** (NHL 11 glove files use another grammar — 2 numbers only; open) | 13 / 13 of 32 file ids | 15 / 15 of 50 | 14 / 14 of 56 |
| `.helmetbrand` → `helmet n1` | 7 / 7 of 10 | 9 / 9 of 14 | 10 / 10 of 18 | 10 / 10 of 19 |
| `.skatebrand` → `skate/texlib n1` | 8 / 8 of 17 | 10 / 10 of 20 | 9 / 9 of 31 | 8 / 8 of 33 |
| `.stick` → `playerstick/texlib n1` | no `playerstick/texlib` on 11 (open) | 16 / 16 of 41 | 19 / 19 of 69 | 19 / 19 of 75 |
| `.visortype` → `helmet/visor n1` | no files on 11 | 3 / 3 of 13 | 3 / 3 of 17 | 3 / 3 of 19 |
| `exhibitiongoalieequipment.pads` → `goaliepad n1` | no files on 11 | 30 / 30 of 53 | 28 / 28 of 67 | 29 / 29 of 70 |
| `.blocker` → `blocker n2` / `.trapper` → `trapper n2` | no files on 11 | 31 / 31 of 54; 28 / 28 of 52 | 29 / 29 of 68; 26 / 26 of 66 | 29 / 29 of 71; 26 / 26 of 69 |
| `.stick` → `goaliestick n1` | 13 / 13 of 16 | 17 / 17 of 26 | 19 / 19 of 35 | 12 / 12 of 40 |
| `.maskid` → `goaliehelmet n2` | 188 / 188 of 243 | 184 / 184 of 249 | 182 / 182 of 256 | 173 / 173 of 280 |
| `.masktype` → `goaliehelmet n1` | 5 / 5 of 7 | 4 / 4 of 8 | 4 / 4 of 8 | 3 / 3 of 8 |
| `exhibitionteams.artid` → `jersey` / `pant` / `sock` `texlib n2` | 212 / 213 (the missing id is the `20054` sentinel) | 212 / 213 | 212 / 213 | 217 / 218 |

NHL 12 / 14 anchors were not joined (the probe covers them; the same grammar is in the JSON).

**NHL 11 (addendum, same session, X360 = PS3):** the families are *texture-keyed*: the meshes are generic (`glove_<side>_0_<lod>`,
`skatelace_…`, `sticktape_…`, no id) and the brand id sits on the texture library `texlib_<id>` (`glove`, `stick` — not `playerstick` —,
`skate`, `goaliepad`, `blocker`, `trapper`), while `helmet` / `visor` / `goaliestick` / `goaliehelmet` keep the part-keyed grammar above.
Containment on the NHL 11 anchor: gloves **14 / 14** of 17 `texlib` ids, helmet brand 7 / 7 of 10, skate brand 8 / 8 of 17, stick 14 / 14
of 28, visor type 3 / 3 of 12, pads 32 / 32 of 33, blocker 33 / 33, trapper 32 / 32, goalie stick 13 / 13 of 14, mask paint 188 / 188 of
243, mask type 5 / 5 of 7 — the "0 / 14" glove cell above was the probe reading the mesh prefix; Q-R38-1's grammar part is resolved. The
contract input (§3) holds with "catalogue = `texlib` ids" for the texture-keyed families on NHL 11.

## 3. Contract input (proposal)

- **Listing:** per family, the catalogue = the file ids present in the title's tree (loose) — for the archive-only cases the same members
  live in `cacherender.big` / `nocache.big` (2026-09-18 asset census); rosters reference a subset. Show "no files for id n" when a DB value has
  none (0 observed on the stock anchors).
- **Reassignment:** a DB cell edit of the equipment column validated against the family's id set of *that title* (ids differ per title —
  e.g. gloves 15–40 on 11, 27–83 on Legacy).
- **Existing-entry replacement:** the `<part>_<id>_*` files of the family (all kinds; `fe*` copies too) through the RW4xb2 / RW4ps3 texture
  rules of handoff 11 and the mesh containers (not covered here); archive placement per the packaging handoffs.
- **New entry:** a new id = a complete new file set (mesh + every kind + `fe*`); no table registers it — runtime acceptance of an unknown id
  with files present is unobserved (card, only if built).

## 4. Open (Q-R38-1)

~~NHL 11 grammar for gloves / sticks / goalie gear~~ (resolved above); the meaning of `n2` variants (0–2 texture sets) and `n3`; the mesh containers' internals
(RW4 FXRS — Studio's `playermesh.rs` reads heads, not gear); which `kind` textures a part actually requires; whether the game tolerates a
missing `fe*` copy.

## 5. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r38_gen7_equipment_file_grammar.py docs/research/overnight-2026-09-21/data/r38-gen7-equipment-file-grammar.json   (≈ 1 s)
```
The containment table was computed by a scratch join of the JSON's number sets with the anchor columns (removed); the JSON keeps the
number sets so it can be recomputed.
