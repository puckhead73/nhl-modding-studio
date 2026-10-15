# Receipt 07 (P4.5 i) — the gen7 diagnostic preview reachable from every measured catalogue row

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to.** HEAD
`8d60a6d` (unchanged). UI only; no backend, binding or dependency change. Claim:
`dev-p45-preview-hooks`. Clock: 14:16 → 14:24 (MDT).

## 1. Plain-language summary

Receipt 05 put the gen7 speech preview behind the team name-call chips. The same preview is now
reachable from the two other places that list gen7 sample ids: the commentary catalogue's phrase
rows (each candidate sample id of a phrase is a chip; the play-by-play bank) and the player card
(name chips use the play-by-play bank, announcer chips the announcer bank). One component, one
label, one owned audio element; the backend refuses tuples outside the research's measured set as
before (X360 announcer banks, for example, are another codec and refuse by name).

## 2. Files changed

| Path | Change |
|---|---|
| `app/src/components/SpeechCatalogueCard.tsx` | phrase rows show up to 8 candidate sample id chips; a selected id mounts `Gen7SpeechPreview` (`pbp_eng`) under the sentence's bodies |
| `app/src/components/PlayerNameCallsCard.tsx` | sample id chips per group; the selection carries the bank stem (`pbp_eng` for name rows, `ann_eng` for announcer rows); the preview renders at the card's foot |
| `app/src/components/Gen7SpeechPreview.test.tsx` | 2 new cases: catalogue chip → exact request; player card announcer chip → `ann_eng`, then a name chip → `pbp_eng` with the previous preview dropped |

## 3. Checks ([`p45-checks.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p45-checks.log))

| Run | Result |
|---|---|
| `npx vitest run` (full) | **35 files, 229 passed** |
| `npx tsc --noEmit` | clean |

## 4. Limitations

- The announcer route is measured on **PS3** `ann_eng` only (R80); on Xbox 360 the same chip
  refuses with the tuple named (the banks there are codec `0x09`), which is the intended reading.
- Streams shipped only inside archives still refuse (receipt 05 §5); the chips do not know in
  advance whether a title's stream is loose, so the refusal is the first thing a user on such a
  title sees — card V-52.

## 5. Cleanup

Scratchpad `patch_p45_catalogue.py`, `patch_p45_player.py` deleted after this receipt; no temp roots; no processes.

## 6. Next action

P4.5 (iii): read gen7 speech streams that ship only inside `nocache.big` / `audiostream.big` /
`audioboot.big` / `data0.big` through Studio's existing archive readers (bounded reader change, no
writer), so X360 15 / Legacy pbp and PS3 09 / 10 / 12 / 13 speech preview instead of refusing.
