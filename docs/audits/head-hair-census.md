# Head / hair / facial-hair census — Phase 0

**Status:** first full inventory (2026-08-07), all 10 Xbox 360 titles (NHL 07 → Legacy) +
7 PS4 trees (NHL 15 → 21, mapped from each `sce_sys/param.sfo` CONTENT_ID), run against the
user's read-only extracted mounts. Engine stage:
`studio_domain::head_census` (`crates/studio-domain/src/head_census.rs`); re-run via the
mount-gated `head_census::tests::run_census` (`QA_CENSUS_MANIFEST` → per-title CSVs). Raw
per-file CSVs are user-mount inventory and are **not** committed; this doc keeps the
aggregate counts, the submesh vocabulary, and the conclusions.

## Why

Extraction gated a player's head on `pPaD == lOnr` and scanned only the `ubPc` bio table,
capturing **~246 of 5632** players on the Legacy anchor and **no** hair, facial-hair,
staff, or generic meshes. Before rebuilding that we needed to measure what actually ships:
where head/hair geometry lives per title, how it is named, and whether facial hair is ever
distinct geometry. This census is that measurement — it drives Phases 1–5.

## Headline: how much is being missed

`player_*` head files that physically exist on disk vs. what the equality gate would emit:

| Title | Player heads (files) | Old gate (~bespoke) | Separate hair meshes | Coach-head meshes |
|---|---|---|---|---|
| NHL 07 | 441 | small subset | 89 | 40 |
| NHL 08 | 445 | " | 336 | 40 |
| NHL 09 | 480 | " | 425 | 26 |
| NHL 10 | 742 | " | 0 (submesh) | 0 |
| NHL 11 | 821 | " | 0 | 0 |
| NHL 12 | 799 | " | 0 | 0 |
| NHL 13 | 1415 | " | 0 | 0 |
| NHL 14 | 1463 | " | 0 | 0 |
| NHL 15 | 1503 | " | 0 | 0 |
| NHL Legacy | **1589** | **~246** | 0 | 0 |

PS4 (title ← `sce_sys/param.sfo` CONTENT_ID `…-NHLICEHOCKEY20YY`):

| Title (CUSA) | Player heads | Separate hair | Generic pool heads | Coach heads | Equipment (excluded) |
|---|---|---|---|---|---|
| NHL 15 (CUSA00561) | 420 | 0 | 183 | 43 | 360 |
| NHL 16 (CUSA01909) | 550 | 0 | 211 | 43 | 454 |
| NHL 17 (CUSA04326) | 626 | 71 | 211 | 43 | 499 |
| NHL 18 (CUSA07569) | 659 | 86 | 211 | 43 | 527 |
| NHL 19 (CUSA11126) | 777 | 172 | 211 | 43 | 582 |
| NHL 20 (CUSA15439) | 991 | 516 | 211 | 267 | 582 |
| NHL 21 (CUSA18182) | 1103 | 516 | 211 | 272 | 622 |

The gate isn't dropping a handful of edge cases — on Legacy it emits ~15 % of the heads that
exist, and **zero** hair.

### Phase 1 fix: uniqueness, not a pool threshold (verified across the committed anchors)

Diagnosing the gate against every committed anchor DB + mount showed **two id conventions**, so
the old `pPaD >= 60000` pool cutoff is not portable:

- **NHL 08–14** pack generic heads into **low, shared** ids (max `pPaD` ~10k–30k; **zero** ids
  ≥ 60000). A fixed 60000 cutoff catches nothing here, so gating on it would attribute one shared
  low-id head to thousands of players.
- **NHL 15 / Legacy** use the **60000+ pool** for generics; bespoke heads sit at low unique ids.

The portable signal is **uniqueness**: a `pPaD` used by exactly **one person** is that person's
own head; one used by two or more different people is generic — regardless of numeric value.
Counting *people* (raw name + birth fields), not records, so all-star/legends duplicates of a star
don't demote their real head. This lands a stable ~250–500 bespoke heads per title (vs the old
equality's ~250 that also included false-bespoke shared heads), recovers the divergent-id heads the
equality dropped, and never names a shared head after an individual. Implemented in
`scan_player_assets`; proven by `assets::tests::phase1_file_truth_recovers_divergent_id_heads`
(mount-gated). Shared/pool heads are captured once by the Phase-4 sweep, not per player.

## Verified layout (what lives where)

| Class | NHL 07 | NHL 08/09 | NHL 10 → Legacy | PS4 |
|---|---|---|---|---|
| **Player head** | `player/player_{id}/head_{id}.rx2` (nested) | `player/head_{id}.rx2` | `player/player_{id}.rx2` (hair bundled as submesh) | `player/{slug}.mcd`, `player/head_generic_player_{id}.mcd` |
| **Player hair** | with head | **separate** `player/hair_{id}_0_0_0.rx2` | head submesh (`haircap_`/`cards`) | **separate** `player/hair_{slug}.mcd`, `hair_generic_player_{id}.mcd` |
| **Coach head** | `coachhead/coachhead_{id}` | `coachhead/head_{id}.rx2` (+`hair_`,`eyel/eyer/eyes`) | ❌ dropped — staff reuse pool heads | `characters/coach/…` (dedicated) |
| **Referee / trainer** | `referee_0`/`trainer_0` body | body only | body only | `characters/referee/`: `refereejersey/pants` + `generic_referee_{id}.mcd` — **headless in sample** |
| **Generic pool head** | — | (via ids) | `player_{id≥60000}` | `player/generic_player_{id}.mcd`, `head_generic_player_{id}` |
| **Create-a-player** | — | `czhead/`,`czhair/` = **textures only** | textures only | in-engine |

### Three facts that shape the build

1. **Hair is era-split.** Separate hair *mesh files* on NHL 08/09 (`hair_*`) and PS4
   (`hair_*.mcd`); a bundled `haircap_`/`cards` *submesh* on NHL 10 → Legacy. The extractor
   must handle both to produce a single per-player hair asset (merging all haircaps/cards,
   per the requested `ovechkin_alex_hair_nhl14` rule).

2. **Create-a-player is texture-only.** `czhead`/`czhair` carry only `*_cm`/`*_am` rasters
   (`eyebrow_*`, `blackeye_*`, `haircp_*`) — 190 files/title on Xbox. There is **no**
   separate custom-head geometry; custom appearance = base head mesh + these swatches. So
   "custom heads" becomes a **texture** catalog, not a mesh one.

3. **Facial hair is not distinct geometry.** Across every sampled decode (12/class/title,
   both platforms) **zero** submesh matched a facial-hair keyword (beard/moustache/goatee/
   stubble/…). This confirms the donor-registry note: vanilla facial hair is **painted into
   the head texture**. Per the agreed decision, facial-hair capture emits a mesh only where
   real geometry is found (none in vanilla so far) and otherwise a flagged **texture entry**.
   *One curio to investigate:* PS4 carries `geo_facegear_cardsa/b/haircapShape` and
   `geo_toque_facegear_…` — face-attached *card* geometry (visor or beard-cards?), 2–4
   occurrences. It is the only candidate for facial-hair geometry and is flagged for a
   targeted look before Phase 3.

## Submesh vocabulary (the classifier's Phase-2 input)

**RenderWare (`.rx2`)** — head-core: `head_`, `head__1`, `eyel_`, `eyer_`, `mouthbag_`.
Hair: `haircap_`, `cardsa_`, `cardsb_`, and (08/09 separate files) `hair_`, `haira_`,
`hairb_`. Staff: `coachhands_`, `coachsuit_`.

**PS4 (`.mcd`, `geo_*Shape`)** — head-core: `geo_headShape`, `geo_head_lowShape`,
`geo_leyeShape`, `geo_reyeShape`, `geo_eyesShape`, `geo_eyes_lowShape`, `geo_eyelashesShape`.
Hair: `geo_haircapShape`, `geo_cardsaShape`, `geo_cardsbShape` + gear-context variants
(`geo_headgear_*`, `geo_hat_*`, `geo_toque_*`, `geo_facegear_*`). Referee body:
`geo_referee_jersey_{high,low}Shape`, `geo_referee_pants_{high,low}Shape`. Unnamed MCD
containers fall back to `mesh_0/1/2` (equipment / non-head).

→ **Hair-split rule:** everything matching `hair*`/`haircap*`/`cards*` (incl. gear-context
`*_haircapShape`/`*_cardsaShape`) merges into the single hair asset; `head/eye/mouthbag`
stays in the head asset.

## Staff → pool linkage (Xbox 10 → Legacy, and PS4 referees)

- **Coaches** have dedicated head meshes on NHL 07/08/09 (`coachhead/`) and all PS4 trees
  (`characters/coach/`, 43–272). **Xbox 10 → Legacy dropped `coachhead/`** entirely.
- **PS4 referee models** (`generic_referee_{id}.mcd` + `refereejersey/pants`) decoded to
  jersey/pants submeshes with **no head submesh in the sample** → referees appear to reuse a
  generic player head rather than carry their own. Coach = dedicated; referee = shared.
- **Open item:** confirm which pool head id the 10→Legacy staff systems reference (DB
  official/coach table vs. a fixed id) so staff heads on those titles get correct provenance.
  Tracked into Phase 4.

## Reproduce

```
QA_CENSUS_MANIFEST=<tsv of "platform<TAB>title<TAB>rendering_root"> \
QA_CENSUS_OUT=<dir> QA_CENSUS_SAMPLE=12 \
cargo test -p studio-domain --lib head_census::tests::run_census -- --ignored --nocapture
```
