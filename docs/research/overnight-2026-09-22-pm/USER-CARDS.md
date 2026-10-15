# Deferred user cards — research lane, 2026-09-22 pm

Observation cards for questions the shipped data cannot settle. Each names the exact observation, a control, and what each outcome would
establish. None is a request to change anything; visual and listening reviews are the user's, when convenient. The morning campaign's
cards (2026-09-22 `USER-CARDS.md`) stay open as revised by R69; the pm 2026-09-21 cards are unchanged.

## A-MP3-1 rev. 2 — EA-MP3 first 24 ms: confirm candidate A by ear (Q-R67-1; handoffs 02 / 03)

> Note 2026-09-23 (R75): a listening pass establishes *audible plausibility* of a rendering; it does not by itself identify the game's exact reconstruction rule. B is "not supported by the junction test", not impossible.

- **What changed:** the static evidence now favours A (the embedded PCM replaces the decoder's first frame; the join is continuous at
  sample 1 152 on 20 / 24 vectors) and rules out B (the decoded stream starts with 161–880 exactly silent samples). Only A and the
  plain MPEG-only start remain worth hearing.
- **Observation:** for NHL 16 `3428` (junction cost 0.07) and NHL 15 `5671` (one of the four inconclusive vectors), listen to the first
  second of (A) `PCM + decoded[1152:]` and (M) `decoded[0:]` (MPEG only; begins with the silent warm-up). The rendering hashes for both
  are in `data/r71-ea-mp3-embedded-pcm.json` (`renderings`).
- **Outcome:** A plays a clean onset and M a late / clipped one on both → Q-R67-1 closes on A and the preview label can drop "candidate";
  a click or dropout under A on `5671` but not `3428` → the four inconclusive vectors need a second look before the label changes.

## T-VEN-1 rev. 2 — PS4 venue by vault level name: one positive check on an unambiguous arena (handoff 04)

- **Observation:** on any PS4 title, pick an arena row classed `unambiguous` in `data/r73-ps4-venue-listing.json` (e.g. TD Garden,
  `arenaartid` 2 → `boston`) and confirm in game that the venue rendered is that folder's. Do **not** use Honda Center (shared id 0) or,
  on NHL 16, Barclays Center / Rogers Arena.
- **Control:** a second unambiguous arena with a different level name.
- **Outcome:** promotes the *listing* binding to game-verified for that title. It does not settle Q-R69-1 (runtime selection through
  `arenaartid` and what a re-pointed id would change), which is not requested.

## K-SOCK-1 — NHL 11 / 12 sock pairs: which file dresses which leg? (Q-R74-1; handoff 06)

> Note 2026-09-23 (R75): the pairs are clear mirrors on two thirds, identical on a sixth, mirror-like or other on the rest; the card applies to any pair class.

- **Why data cannot answer:** the `_1_` file is the mirror of the `_0_` file; the runtime's assignment of the two to the left and right
  leg is not in the data.
- **Observation (optional; nothing in Studio depends on it):** on X360 NHL 11 or 12, replace one team's `sock_<style>_0_<art>_<var>_cm`
  with a visibly marked copy, restart, and note which leg shows the mark; restore.
- **Outcome:** names the leg per sub-index; without it the listing simply shows "mirrored pair (2 files)".

## A-EAL3-1 — gen7 EALayer3 v1: the first 1 152 + 47 samples (Q-R77-1; handoff 09)

- **Observation:** for one PS3 NHL 15 (44.1 kHz) and one PS3 NHL 13 (24 kHz) speech sample decoded by the handoff 09 rebuild, listen
  to the first second of (D) the decoded stream from sample 0 (two warm-up granules included), (E) the decoded stream from sample 1 152
  with the 47 PCM samples prepended, and (F) from sample 1 152 without them. Hashes for (D) are in `data/r77-gen7-ealayer3-v1-grammar.json`.
- **Control:** a second sample of each rate.
- **Outcome:** establishes audible plausibility of one start rule for the diagnostic preview label; it does not by itself identify the
  game's exact reconstruction rule (the same distinction as A-MP3-1).

## H-HORN-1 — gen7 arenas whose horn id has no bank (Q-R78-1; handoff 13)

- **Observation (any gen7 title with a loose tree, e.g. X360 NHL 15):** play an exhibition game at an arena whose `goalhorn` is a
  residual id (e.g. a DEL arena, ids 30–41) and score; note whether a horn plays and whether it sounds like `000_goal.abk` (bank 0) or a
  built tone (sustain / gap pattern).
- **Control:** an NHL arena with a filed id (1–29).
- **Outcome:** a horn identical to bank 0 → the default-bank candidate; a tone pattern → the `aemshorn` segment-built candidate; silence →
  neither; in every case the listing text stays "no horn bank found in the inspected sources" until a rule is established.
