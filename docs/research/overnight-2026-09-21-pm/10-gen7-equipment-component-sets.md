# Handoff R52 — Gen7 equipment: the complete file set per equipment id (required components, variants `n2` / `n3`, texture kinds) on X360 11 / 13 / 15 / Legacy and PS3 12 / 13 (archive member listing) / 15 / Legacy — what an existing-id reassignment must find and a new-id registration must place

> **Reading note — 2026-09-22 pm ([research R74](../overnight-2026-09-22-pm/06-developer-observations-reconciled.md), developer observation O-9).** Blocker / trapper ids are keyed at the second numeric position (`blocker_<style>_<id>_…`); the single-number stems `blocker_0_am` / `trapper_0_am` are style-level files, so the id counts of record are 68 / 66 (not 69 / 67). NHL 13 mask paint 260 ships `goaliehelmet_{1..7}_260_cm` without a shell-0 file — the one exception to "shell 0 always" on that title.

Research lane, 21 September 2026 pm, roadmap step 4.x gen7 (prompt step 5 "Equipment: complete per-title grammar, required components,
variant semantics, meshes / textures and existing-id reassignment before new-id registration"; Q-R38-1 remainder). Claim
`research-r52-gen7-equipment-components`. Read-only directory listings; nothing game-verified; no writer.

Machine-readable: [`data/r52-gen7-equipment-component-sets.json`](data/r52-gen7-equipment-component-sets.json) (per tree / family / part:
ids, signatures, majority signature, required / optional components, incomplete ids, `n2` / `n3` values, kinds; shared meshes per part;
PS3 vs X360 per-id signature equality). Probe `r52_gen7_equipment_component_sets.py` (≈ 20 s).

## Plain-language summary

For the equipment families whose files are keyed by the equipment id, the shipped trees show one dominant file set per family: a glove id
has seven textures (`am`, `nm`, `sm` per id plus `dm` / `tm` for variant 0 and `tm` for variants 1 and 2) on every one of the 53 Legacy ids;
a goalie-pad id the same set minus `am` (57 of 67 ids carry the full set, 10 lack the third variant); a blocker or trapper id carries the
set twice (once per style 0 / 1: `nm`, `sm`, `dm`, `tm` ×3 — 49–57 of 67–69 ids complete, the rest lack the third variant or the normal
map); a goalie stick id two textures (`dm`, `tm`, all 38 ids); a mask paint id one `cm` per shell it exists for (0 always; 1 on 81, 6 on 75,
2 on 52 … of 280 paints); hands one `cm`. The front-end (`fe*`) copies carry the `dm` / `tm` subset only. The meshes are shared per part
(3–8 containers per family, keyed by style, not by equipment id), so a **new equipment id needs no mesh** — only the texture set — and an
existing id is complete when it has the family's majority set. PS3 and Xbox 360 carry the same per-id sets (851 / 853 Legacy, 789 / 791
NHL 15, 648 / 655 NHL 13 by the PS3 archives' member listing equal; the differences are the helmet `helmet_0_*` texture library on X360 with one extra brand). The helmet, jersey, pant, sock and
name families use other keys (brand in the second number for helmet textures; team art id and kit for jerseys / pants / socks) and are not
censused as equipment ids here.

## 1. Required set per family (X360 Legacy; NHL 13 / 15 identical in shape; counts in the JSON)

| Family / part | id position | ids (min–max) | majority signature = components per id | ids complete | incomplete ids (examples) | optional |
|---|---|---|---|---|---|---|
| `glove/glove` | n1 | 53 (15–101) | `_am`, `_nm`, `_sm`, `_0_dm`, `_0_tm`, `_1_tm`, `_2_tm` (7 files) | **53 / 53** | — | — |
| `glove/feglove` (front end) | n1 | 53 | `_0_dm`, `_0_tm`, `_1_tm`, `_2_tm` | 45 | 15, 18, 20, 23, 24, 28, 33, 39 (no `_0_dm`) | `_0_dm` 45 |
| `goaliepad/goaliepad` | n1 | 67 (1–84) | `_nm`, `_sm`, `_0_dm`, `_0_tm`, `_1_tm`, `_2_tm` | 57 | 12, 14, 15, 17, 18, 22 … (no `_2_tm`) | `_2_tm` 57 |
| `goaliepad/fegoaliepad` | n1 | 67 | `_0_dm`, `_0_tm`, `_1_tm`, `_2_tm` | 57 | same ids | |
| `blocker/blocker` | **n2** (n1 = style 0 / 1) | 69 (0–84) | per style s ∈ {0, 1}: `s_<id>_nm`, `_sm`, `_0_dm`, `_0_tm`, `_1_tm`, `_2_tm` (12 files) | 49 | 0 (`am` only), 11, 12, 14, 15, 17 … (no `_2_tm` or no `nm`) | `_2_tm` 54, `nm` 64 |
| `blocker/feblocker` | n2 | 68 | per style: `_0_dm`, `_0_tm`, `_1_tm`, `_2_tm` | 54 | 14 ids without `_2_tm` | |
| `trapper/trapper` | n2 | 67 (0–84) | as blocker | 57 | 0, 12, 14, 15, 17, 22 … | `_2_tm` 58 |
| `trapper/fetrapper` | n2 | 66 | as feblocker | 58 | 8 ids | |
| `goaliestick/goaliestick` | n1 | 38 (6–43) | `_dm`, `_tm` | **38 / 38** | — | — |
| `goaliehelmet/goaliehelmet` (mask paint) | **n2** (n1 = shell 0–7) | 280 (0–291) | `<shell>_<paint>_cm` for shell 0 | 280 / 280 (shell 0) | — | shell 1: 81, 6: 75, 2: 52, 3: 18, 4 / 5: 16, 7: 1; `am` / `nm` on 2 |
| `body/hand` | n1 | 10 (1–10) | `_cm` | 10 / 10 | — | — |
| `playerstick` / `skate` / `helmet` / `boot` (texture libraries) | `texlib_<id>` | 75 / 33 / 20 / 7 | one `texlib_<id>.rx2` container each (999 = default) | all | — | (handoff 12 §2: brand ids) |

Shared meshes (not per id): `glove` 0 / 2 / 3; `goaliepad` 0 / 2 / 3; `blocker` / `trapper` styles 0 / 1 with variants 0 / 2 / 3; `goaliestick` /
`playerstick` 0 / 1 with 2 / 4; `goalieskate` / `playerskate` / `skatelace` / `skatetongue` 0 / 2 / 3; `cage` 0–7, mask `helmet` 0–7 and
`throatprotector` 0 / 6 (0 / 2 / 3); `hand` 0 / 2 / 3. Variant semantics (`n2` / `n3`): `_0 / _1 / _2` on the `tm` textures of gloves / pads /
blockers / trappers are three team-colour tint maps of one id (candidate: home / away / third — name-only; the `dm` diffuse exists for
variant 0 only); blocker / trapper `n1` = style 0 / 1 = the two mesh styles; mask `n1` = shell 0–7 = the eight mask meshes. NHL 11's
`texlib_<id>` families (handoff 12 addendum) have one container per id (no component set).

## 2. Contract input (developer, Phase 4 gen7)

- **Existing-id reassignment** (the P5 check, receipt 12): "id has files" should become "id has the family's majority set"; an id in the
  incomplete list is valid as shipped (the game ships it) but the card should say which components it lacks (e.g. "no third tint map").
- **New-id registration** (Q-R38-1 remainder): the file set to place is the majority signature of §1 for the family (textures only; meshes
  are shared and need no new file); the front-end copy (`fe*`) is a second set; there is no registration table — the id becomes valid by
  the files existing (static claim; runtime acceptance of an unshipped id is card material, unchanged).
- **Refusals:** an id whose part keys by `n2` (blocker / trapper / mask paint) must be validated in that position; the helmet / jersey /
  pant / sock / name families are not equipment-id keyed (team art id, brand-in-`n2`) and stay under handoff 12's separate grammar.

## 3. Not claimed

The meaning of `n2` beyond "three tint variants" (no consumer trace); which `am` / `nm` / `sm` maps the shaders require (the game may
fall back — unobserved); NHL 11's per-id sets (texture libraries, one file per id); NHL 12's PS3 census is by archive listing only (texture-library form like NHL 11: gloves / pads / skates as `texlib_<id>`); runtime.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r52_gen7_equipment_component_sets.py docs/research/overnight-2026-09-21-pm/data/r52-gen7-equipment-component-sets.json   (≈ 20 s)
```
Executed four times (grammar refinement: meshes separated, `n2`-keyed parts; PS3 Legacy path corrected to the `- Vanilla` tree; PS3 12 / 13 through the EB archives' member listing — no extraction).
