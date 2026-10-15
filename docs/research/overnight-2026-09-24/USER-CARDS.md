# Deferred user cards — research lane, 2026-09-24

Observation cards for questions the shipped data cannot settle. Each names the exact observation, a control, and what each outcome
would establish. None is a request to change anything; listening and visual reviews are the user's, when convenient. No listening was
performed this session. The 2026-09-22 pm cards (A-MP3-1 rev. 2, T-VEN-1 rev. 2, K-SOCK-1, A-EAL3-1, H-HORN-1) and the League cards
L-CHL-1 / L-SET-09 stay open unchanged.

## A-SPX-1 — EA-Speex start: the 509-sample encoder delay (handoff 02 A; Q-R83-1)

- **What is known statically:** every `0x09` stream's first block declares 11 × 640 − 509 samples; the pinned public decoder discards 509.
  Nothing has been heard.
- **Observation:** for X360 NHL 15 `ann_eng_samples.sbs` offset 16 (SNS SHA-256 `6157e642…`) and PS4 NHL 15 `rink_ann_samples.sbs`
  offset 32, listen to the `libspeex` rendering (S) from sample 509 and (S0) from sample 0. Hashes of the full renderings are in
  `data/r83-codec09-speex.json`.
- **Control:** the same line's PS4 `_inxp` copy (offset 32, 143 frames) and one X360 NHL 11 announcer vector.
- **Outcome:** clean onset at S and a click / dead air at S0 → the 509 rule is audibly plausible for the diagnostic preview label;
  no difference → the delay is inaudible and either start is acceptable for a preview; it does not identify the game's reconstruction.

## A-XMA-1 — EA-XMA diagnostic rewrap (handoff 02 B)

- **Observation:** X360 NHL 10 `ann_eng` offset 16 (24 kHz mono) and X360 NHL 15 `000_goal.abk` entry 0 (44.1 kHz stereo) through the
  RIFF XMA2 rewrap → ffmpeg `xma2` → WAV: start, end and pitch plausible?
- **Control:** an unrelated X360 `sfx.abk` entry (36 kHz) — pitch must differ accordingly.
- **Outcome:** audible plausibility of the diagnostic route only; nothing about writing.

## T-ARTID-1 — team art-id re-key: is the static graph complete? (handoff 04; Q-R86-1)

- **Baseline:** X360 NHL 15 loose tree copy (disposable), one non-NHL club with a single `exhibitionteams` row and a complete file set
  (an AHL club whose crest `t{artid}`, `s{artid}`, rink `texlib_{artid}_*`, puck and kit files all exist).
- **Treatment:** re-key its `artid` to a free id (handoff 04 §4) in the DB and rename every keyed file accordingly; leave saves untouched.
- **Observation:** FE crest, rink art and kits of that club in an exhibition game vs an untouched club (control); restore afterwards.
- **Outcome (narrowed, R88 §F3):** art follows the id on the observed surfaces (FE crest, rink art, kits) → those surfaces, on that
  title and file set, resolve through the renamed families — **not** completeness of the graph; art missing → competing explanations
  (a family not in §3, archive / cache placement, consumer behaviour, a code-side table) that this observation cannot separate — record
  which surface failed; the club appears twice / vanishes → a further consumer keys by art id.

## H-HORN-1 rev. 2 — residual horn ids (handoff 03 §1 adds the loader table)

- Unchanged observation (play an exhibition game at an arena whose `goalhorn` is a residual id; note whether a horn plays and whether it
  sounds like bank 0 or a built tone); the static statement now reads "unlisted in the loader's asset table", which the observation
  tests directly: any horn at all → the engine resolves unlisted ids somewhere (bank 0 or the segment-built module); silence → it does not.

## T-CHANT-1 — gen7 club chant banks: are the script-named club banks what plays? (handoff 08; Q-R90-1)

- **What is known statically:** banks 055–066 are club chants named Brynäs … Timrå inside the bank and in `aemscrwd.csi`; no arena or
  team field addresses them; every SHL club's arena carries the generic chant bank 45.
- **Observation:** X360 NHL 15, exhibition game as Frölunda (bank 058, team present) at home; note whether a club-specific chant plays.
- **Controls:** Växjö (no club bank); Djurgården (bank 056; team present on NHL 15, absent on 13 / 14).
- **Outcome:** chant for Frölunda, none for Växjö → the script binds by team identity to the named banks (key still unknown); a chant
  for both → the club banks are not what plays or generic chants mask it; none → the club banks are unused on this title. Nothing here
  identifies the key or authorises any edit.

> **Note 2026-09-25 14:20 (R97, [overnight-2026-09-25/10-correction-checkpoint-direction-audit.md](../overnight-2026-09-25/10-correction-checkpoint-direction-audit.md)):** T-CHANT-1's "no arena or team field addresses them" is corrected (the Finnish arena's
> `chantaudioid`, `goalhorn` and `periodhorn` are all 61); rev. 3 wording in the 2026-09-25 cards. A-SPX-1: "is silence" reads "near-silent
> output under both tested decoders; unsuitable for validating onset or decoder fidelity".

> **Note 2026-09-25 (R93 / R95, [overnight-2026-09-25/USER-CARDS.md](../overnight-2026-09-25/USER-CARDS.md)):** A-SPX-1's outlier remark
> is superseded — the NHL 21 single-block vector is silence and cannot test the 509 start (A-SPX-1 rev. 2). T-CHANT-1's first observation
> is replaced by **T-CHANT-1 rev. 2**: chant banks load by bank number (loader `descr`) and the only database path to a club bank is the
> Finnish arena's chant id 61, so the observation that separates the readings is a game at that arena, with Frölunda-at-home as the
> control. Two new cards A-AST-1 / A-LOOP-1 cover the `.ast` previews.
