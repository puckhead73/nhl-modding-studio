# Handoff R81 — gen7 unresolved horn ids (Q-R78-1): the 11–14 non-NHL `goalhorn` / `periodhorn` values per title (30–39, 41, 45, 61, 136) are **absent from every inspected source** — the loose `audio/sfx` numbered banks, the numbered members of `nocache.big` / `audioboot.big` / `data0.big` (identical id sets, 0–29 + 100 [+ 194 on 14+]), and the `horns.abk` bank, which ships **empty** (S10A count 0) on all twelve loose trees; the horn module `aemshorn.csi` exposes a `h_goalHorn` handle with eight sustain / gap parameters — a **candidate** alternate consumer (synthesised horn), not a binding; NHL 09 X360 carries the numbered banks **inside** `nocache.big` only (R78's "no numbered banks" narrowed to "none loose")

Research lane, 24 September 2026 (brief item 2; Teams 2.7 gen7). Claim `research-r81-gen7-horn-sources`. Read-only: directory listings,
archive front tables (no extraction except the archived `aemshorn.csi` on trees without a loose copy), `horns.abk` headers, the r78 JSON.
Nothing game-verified; no runtime claim.

Machine-readable: [`data/r81-gen7-horn-sources.json`](data/r81-gen7-horn-sources.json) — per tree: loose id sets, per-archive
numbered member sets and horn-named members, `horns.abk` table count, `aemshorn.csi` source and MOIR names, every residual id with its
per-source presence and class, and the archive-vs-loose comparison. Probe `r81_gen7_horn_sources.py` (≈ 1 min).

## Plain-language summary

The horn numbers that the European, minor-league and international arenas use (30–136) name no file in the sound-effects folder. This
package looked in the other places the data could hold them: the audio archives carry exactly the same numbered horn files as the
loose folder (0–29 and 100, plus 194 from NHL 14), no more; a bank literally called `horns.abk` exists on every title but contains no
samples at all; and the event module for horns names a "goalHorn" handle with eight sustain and gap settings — the shape of a horn
that is *built* from segments rather than played from a file. So those numbers are absent from every place inspected, which is not
the same as absent from the game: the segment-built horn is a candidate explanation recorded as such, and only a runtime observation
can settle it. One earlier statement is narrowed: NHL 09 on Xbox 360 does have the numbered horn files, inside its archive rather
than loose.

## 1. Inspected sources (12 loose trees: X360 09–15 / Legacy, PS3 11 / 14 / 15 / Legacy; PS3 12 / 13 have no loose `audio` and were not inspected)

| Source | goal / period ids present | Result |
|---|---|---|
| loose `audio/sfx/<id>_goal.abk`, `<id>_period.abk` | 0–29 + 100 (NHL 10–13: 31 files); + 194 (NHL 14–Legacy: 32) | NHL 09 X360: **none loose** |
| `nocache.big` numbered members | identical to the loose set on 11 / 11 trees with both; NHL 09 X360: 0–29 + 100 **archive-only** | no id outside the loose set anywhere |
| `audioboot.big`, `data0.big` (`audio.big`, `audiostream.big` absent on these trees) | no numbered horn members; `audioboot.big` carries `audio/sfx/aemshorn.csi` | — |
| `horns.abk` (loose, every tree 10–Legacy; 3 268 bytes) | S10A table, **count 0** — no samples | an empty bank, not a source |
| `aemshorn.csi` (MOIR module, 1 236 bytes; loose on 10–Legacy) | handles `h_goalHorn`, `h_ducker`, `h_pause`, `h_service`, `h_stop`; parameters `firstSustain_snd` … `eighthSustain_snd`, `firstGap_snd` … `eighthGap_snd`, `hornDelayTrigger_snd`, `hornReplaySpeed_snd` / `hornReplayState_snd`, `hornFxLevel_snd`, `hornDryLevel_snd`, `goalHornMasterVolume_snd`, `goalHornDucker*`, `hornsKillDelayTime_snd`, `hornsPauseDelayTime_snd`, `m_goalHorn_v` | a **segment-built horn** interface (eight sustain / gap pairs) — candidate consumer for ids without a file; not established |

## 2. Residual ids, per title (from `r78` values without a loose file; class after inspecting the sources above)

| Title | `goalhorn` residual ids | `periodhorn` residual ids | class |
|---|---|---|---|
| NHL 10 / 11 (X360 = PS3) | 30–39, 41, 45, 61, 136 (14) | same | **absent in inspected sources** (all) |
| NHL 12 / 13 | 30–38, 41, 45, 61, 136 (13) | 30–36, 38, 41, 45, 61, 136 (12) | absent in inspected sources |
| NHL 14 / 15 / Legacy (X360 = PS3) | 30–36, 38, 41, 45, 61, 136 (12) | 30–34, 36, 38, 41, 45, 61, 136 (11) | absent in inspected sources |

Files no row names (0, 100, 194) are unchanged; **id 0 = shipped-but-unreferenced** on every title is a second candidate ("default horn
when the id has no file"), recorded as a candidate only.

## 3. Consequence (developer; not Codex-accepted)

- Arena page, gen7: for a residual id show **"no horn bank found in the loose folder or the audio archives (inspected: `audio/sfx`,
  `nocache.big`, `audioboot.big`, `data0.big`; `horns.abk` is empty)"** — never "no horn"; two candidates may be named behind
  disclosure: the segment-built horn of the `aemshorn` module and the unreferenced bank 0, both unestablished.
- NHL 09 X360: the numbered banks are `nocache.big` members — the horn file hop can be offered from the archive listing (names only,
  as for the PS3 12 / 13 kits); R78's refusal for NHL 09 becomes "archive members, not previewed".
- PS3 12 / 13: not inspected (archive-only audio); refuse with the reason.
- Q-R78-1 stays open; a runtime card (H-HORN-1) is the only way to close it: play a game at an arena whose `goalhorn` is a residual id and
  note whether a horn plays and whether it matches bank 0's.

## 4. Reproduce

```
python local/runs/overnight-research-2026-09-22-pm/probes/r81_gen7_horn_sources.py docs/research/overnight-2026-09-22-pm/data/r81-gen7-horn-sources.json
```
Executed three times (two guard fixes for NHL 09's missing family); inputs: the twelve loose trees' `audio/sfx`, their archives' front
tables, `r78-gen7-arena-sfx-bank-join.json`.
