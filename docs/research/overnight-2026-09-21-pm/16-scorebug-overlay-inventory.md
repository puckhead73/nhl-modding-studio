# Handoff R59 — Miscellaneous 5.3 (scorebug / HUD overlays): member inventory per title — PS4 `.gfx` (Scaleform `CFX` / `chun`-wrapped) overlays with their `.gnf` siblings, gen7 Ion `overlays/*.big` packages — no decoding

Research lane, 21 September 2026 pm, roadmap step 5.3 (the 2026-09-20 handoff 10 §3 "inventory the `.gfx` members and their embedded
image / text resources — new format work, not started"). Claim `research-r59-overlay-inventory`. Read-only archive listings; nothing decoded
beyond the first bytes of each member; nothing game-verified.

Machine-readable: [`data/r59-scorebug-overlay-inventory.json`](data/r59-scorebug-overlay-inventory.json) (PS4: every `.gfx` / `.swf` member
with archive, path, size, magic; `.gnf` siblings per overlay folder; gen7: overlay packages per title). Probe
`r59_scorebug_overlay_inventory.py` (≈ 2 min).

## Plain-language summary

On PS4 the scorebug and the other in-game overlays are Scaleform movies (`.gfx`, magic `CFX` + version byte, or `chun`-wrapped = EA's
chunkzip around a `CFX`) with their image resources as separate `.gnf` textures in a folder named after the overlay — `scoreclock`
(477 textures on NHL 15, 202 on 21), `endofperiod`, `lowerthirds`, `teamintro`, `goalscored` … — spread over the `overlays_2x.big` /
`overlays.big` archives (the small, patch-able set) and the big `game_61` / `inxp_10` / `patch_inxp00` archives (the UI library). So the
scorebug's *pictures* (sponsor plates, logos, frames) are ordinary GNF members the developer's C1 / GNF routes already replace, while its
*layout and text* live inside the `.gfx` movie (Scaleform GFx — undecoded, the next format). On the older consoles the overlays are 56–57
numbered Ion packages plus a few named ones (`timer`, `fight_bar`, `line_change`, `logo_pill`, `snow`, `pressstart`, `overlayfontcache`)
in `cacheoverlay.big` (13–Legacy) / `cache.big` (12) / `fe.big` (11); NHL 09 / 10 keep them inside the BIG4 `fe.big` (not listed by this
probe). Nothing here identifies which numbered gen7 package is the scorebug.

## 1. PS4 inventory

| Title | `.gfx` / `.swf` members | archives holding them | overlay folders with `.gfx` | `.gnf` siblings (top folders) | scorebug movie |
|---|---|---|---|---|---|
| NHL 15 | 71 (58 `chun`, 13 `CWS`) | `inxp10` 26, `patch_inxp00` 37, `overlays` 6, `patch_overlays` 2 | commonflares, endofperiod, playerinterstitial, robocam, **scoreclock**, teamvsteam | scoreclock 477, robocam 182, endofperiod 99, teamvsteam 60 | `overlays.big` → `scoreclockframe.gfx` 167 886 B (`chun`); patched copy 165 756 B in `patch_overlays.big` |
| NHL 16 | 236 | inxp / overlays | commonflares, endofperiod, nbcsnlogo, playerinterstitial, robocam | — | (no `scoreclock` folder listed — the scorebug movie sits elsewhere; not identified) |
| NHL 17 | 331 | | + as 16 | | idem |
| NHL 18 | 573 | | + `goalscored.gfx` | | idem |
| NHL 19 | 1 074 (967 `CFX\x0f`) | | + `goalscoredproam.gfx` | | idem |
| NHL 20 | 781 | | 16 folders: clearplayerintros, commonflares, endofperiod, energygain, faceoffstrategy, goalscored, goalscoredproam, leagueleaders, lowerthirds, matchidentity, nbcsnlogo, playerinterstitial, … | | (see JSON) |
| NHL 21 | 696 (610 `CFX\x0f`, 23 `chun`, 11 `CWS`) | `game_61` 609, `inxp_10` 69, `overlays_20..24` 2 / 11 / 1 / 1 / 3 | 14 folders incl. **scoreclock**, goalscored, goalscoredrushthrees, lowerthirds, teamannouncement, guestinthebooth, simtonextshift | scoreclock 202, endofperiod 91, teamintro 50, lowerthirds 38, leagueleaders 22, 3on3 18 (+ sponsor plates `scoreclock_sponsor_1.gnf` …) | `overlays_21.big` → `scoreclock.gfx` 102 260 B (`chun`); `overlays_22 / 23` → `goal.gfx` / `goal_fr.gfx` (`CFX\x11`) |

Magics: `CFX` + version byte (0x0b … 0x29 — Scaleform GFx versions), `CWS` (compressed SWF — the `application.swf` / `as3bootstrapper.swf`
UI shells), `chun` (EA chunkzip wrapper around a movie — the same codec the developer's chunkzip encoder handles).

## 2. Gen7 inventory

| Title | archive | overlay members | named | numbered `overlay_<n>.big` |
|---|---|---|---|---|
| NHL 11 | `fe.big` (EB) | 61 | fight_bar, line_change, logo_pill, logo_pill_small, pressstart, timer | 55 |
| NHL 12 | `cache.big` (EB) | 64 | + snow | 57 |
| NHL 13 / 14 / 15 / Legacy | `cacheoverlay.big` (EB) + `cache.big` (pressstart, playnowtutorialoverlay) | 63 + 3 / 4 | fight_bar, line_change, logo_pill, logo_pill_small, overlayfontcache, snow, timer | 56 |
| NHL 09 / 10 | `fe.big` is BIG4 (not walked by this probe) | — | — | — |

## 3. Consequences (developer)

- 5.3 listing: an "overlays" page can list the PS4 movies by folder with their texture siblings (which are replaceable through the
  existing GNF route — a sponsor plate on the scorebug is a `.gnf` in `overlays/scoreclock/`), and the gen7 packages by name / number.
- Text and layout inside a `.gfx` stay "not decoded (Scaleform GFx)" — the next format if the roadmap wants scorebug text edits; the
  `chun` wrapper is already handled by the chunkzip codec, so the movie bytes are reachable.
- Which `.gnf` the scorebug actually draws is a runtime observation (card material) — the folder name is the only static binding.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-21-pm/probes/r59_scorebug_overlay_inventory.py docs/research/overnight-2026-09-21-pm/data/r59-scorebug-overlay-inventory.json   (≈ 2 min)
```
Executed once (final).
