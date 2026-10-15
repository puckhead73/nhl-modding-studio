# Deferred user cards — research lane, 2026-09-22

Observation cards for questions the shipped data cannot settle. Each names the exact observation, a control, and what each outcome
would establish. None is a request to change anything; all visual and listening reviews are the user's, when convenient. The pm
campaign's cards (2026-09-21-pm `USER-CARDS.md`) stay open unchanged.

## A-MP3-1 — EA-MP3 first 24 ms: which splice is right? (Q-R67-1; handoff 03)

> Note 2026-09-22 pm (R69, review F2): renderings (A) / (B) / (C) are all *candidates*; none is verified game output. Research R71 measures the junction and delay evidence first; the card stays open for the listening decision only.

- **Observation:** for one NHL 15–17 speech sample decoded by the handoff 03 contract, listen to two renderings of the first second:
  (A) the embedded 1 152 PCM samples followed by the decoded stream from sample 1 152; (B) the decoded stream from sample 0 with nothing
  prepended. A third (C), the embedded PCM followed by the decoded stream from sample 0, is the pre-roll alternative.
- **Control:** a sample whose junction metric was tiny under A (e.g. NHL 16 `3428`, jump 0.004) and one where it was large (NHL 15
  `35147`, 2.4) — both are in `data/r67-ea-mp3-sized-frames.json`.
- **Outcome:** an audible click or dropout at the start under A but not C (or vice versa) settles the role of the embedded PCM; no
  audible difference means the preview may use either, labelled. Research will then close Q-R67-1 with the chosen rule.

## A-ICE-1 — gen7 ice layers: which arena column selects the centre line / lightmap / bounce map? (Q-R62-1; pm handoff 19 §3)

- **Why data cannot answer:** the `icesurface` families have 4–37 ids and four to six arena columns each cover the whole id set, so every
  candidate column "fits". Only a runtime change can discriminate.
- **Observation (any gen7 title with a loose tree, e.g. NHL 13 X360):** in an exhibition game at one arena, note the centre-line style
  and the ice tint; change **one** candidate column of that arena's row (`centerline`, then `bouncemap`, then `puck`, then `powerring`)
  to the id of a visibly different shipped file, restart, and note what changed. Restore the row between trials.
- **Control:** the unchanged arena in the same session; the `arenaartid` swap (established to change the venue model) as the positive
  control that the edit path works.
- **Outcome:** the column whose change alters the ice layer becomes an established key for that family; columns that change nothing stay
  "candidate, no runtime effect observed" — never "not consumed".

## T-VEN-1 — PS4 venue by vault level name: one positive check (handoff 04)

> Note 2026-09-22 pm (R69, review F3): pick an arena whose `arenaartid` is **not** a shared level id (ID 0 / 18 / 28 on NHL 16, ID 0 on 17–21); the JSON of R73 lists the unambiguous ones. A positive result promotes the *listing* binding; runtime selection (Q-R69-1) needs the reassignment control as well, which is not requested.

- **Observation:** on any PS4 title, pick an arena row and confirm in game that the venue rendered is the one whose folder is
  `stadiums/<ge_level.DebugName>/` for that row's `arenaartid` (the JSON lists the name per level). One arena is enough.
- **Control:** a second arena whose level name differs.
- **Outcome:** promotes the folder binding from "observed on every shipped title" to game-verified for that title; a mismatch reopens
  Q-R64-1 with the specific level named.
