# Deferred user cards — research lane, 2026-09-25

Observation cards the shipped data cannot settle. No listening was performed this session; none is required to continue. The
2026-09-24 cards A-XMA-1, T-ARTID-1, H-HORN-1 rev. 2 stay open unchanged; A-SPX-1 and T-CHANT-1 are revised below.

## A-AST-1 — `.ast` stream previews: is the candidate start (PCM frame + decode from 1152) audibly right? (AST_FRAMING.md §6; PREVIEW_REVIEW.md)

- **Known statically:** the block-0 PCM frame equals decoder output 1105–1151 (corr ≥ 0.995); the trimmed previews start with those 47
  samples and continue from decoder index 1152; the raw previews include the 1152-sample warm-up.
- **Observation:** `A-ps3-15-playercalls-0…__trimmed.wav` vs its `__raw.wav` (skip the first 1105 / 1152 samples by ear or in an editor):
  is the onset of the call clean in the trimmed copy (no click, no lost attack)? Same for `I5-ps3-14-sfx…` (a loud sfx).
- **Control:** `F-x360-15-playercalls-0…` (the same call, X360 encode, no PCM-frame construction).
- **Outcome:** clean onset → the construction is audibly plausible for the diagnostic label; a click / missing attack → the PCM frame is
  placed elsewhere (the 1152-start copy without PCM is the fallback); says nothing about the console's own rendering.

## A-LOOP-1 — loop stub + body: seamless at the loop point? (R91 loops; `D-…__loop_x2_s0.wav`, `E-…__loop_x2.wav`, `H-…__loop_x2_s0.wav`)

- **Observation:** listen across the two junctions (stub → body at 29 / 24 samples; body → body at stub + body).
- **Outcome:** seamless → stub = intro, body = loop is the right reading of words (a, b); a gap or click → the loop point or the
  trim of the body's tail is misread (competing readings: body trim not applied at the loop; crossfade by the engine).

## A-SPX-1 rev. 2 — EA-Speex 509-sample start (unchanged observation; scope note)

> **Rev. 3 wording (2026-09-25 14:20, R97 §2, [10-correction-checkpoint-direction-audit.md](10-correction-checkpoint-direction-audit.md)):** "is silence (R93)" reads **"decodes to near-silent output under both tested
> decoders (R93) and is unsuitable for validating onset or decoder fidelity"** — no placeholder / game-silent reading.

- The R88 outlier (PS4 NHL 21 base bank, single block) is silence (R93) and cannot test the start; the observation stays on the X360
  NHL 15 `ann_eng` offset 16 and PS4 NHL 15 `rink_ann_samples` offset 32 vectors as written on 2026-09-24. `OUTLIER-control_inxp-…`
  WAVs (both decoders, raw, no 509 drop) are provided for the NHL 21 `_inxp` line if convenient.

## T-CHANT-1 rev. 2 — gen7 club chant banks: loader-by-number gate (CHANT_SELECTION.md §3; supersedes rev. 1's first observation)

> **Rev. 3 (2026-09-25 14:20, R97 §1.3, [10-correction-checkpoint-direction-audit.md](10-correction-checkpoint-direction-audit.md)) — observations unchanged, "known statically" and outcomes reworded:**
> *Known statically:* chant banks are loader assets listed by number; of the 17–20 audio-named fields per title (all tables, 18 anchors)
> only the Finnish arena's `chantaudioid` equals a club-bank number (61) — and that arena's `goalhorn` / `periodhorn` are 61 too; no club's
> own arena or team row carries its bank number; no selecting consumer traced. *Outcomes:* 1 yes → **consistent with** the chant loader
> keying on the arena field, not proof that `chantaudioid` caused it (the same rows carry 61 in the horn fields; a code path could load
> by any field or none); 2 yes → a club-chant path this census did not trace (not "code-side"); nothing → shows neither unreachable nor
> unused. *Next discriminating control (not authorised):* an explicit treatment of the candidate input on a disposable copy (another
> arena's `chantaudioid` set to 61, horn fields untouched, active source verified).

- **Known statically:** chant banks load by number (`descr`); the only database value in 55–66 is the Finnish arena's `chantaudioid` 61
  (→ `061_chant.abk`, Linköping); the crowd module and the control bank name the 12 club pickers without any selecting data found.
- **Observation 1:** X360 NHL 15, exhibition game at the **Finnish arena** (chantaudioid 61), any teams: a Linköping-specific chant
  audible?
- **Observation 2 (control):** Frölunda at home (arena → generic bank 45): a Frölunda-specific chant audible?
- **Observation 3 (control):** Växjö at home (no club bank).
- **Outcome:** 1 yes / 2 no → chant banks load by the arena's field and club banks are reachable only that way (authoring inconsistency
  on 61); 2 yes → a team-keyed path exists outside the database (code-side); 1 no / 2 no → the club pickers are never triggered on this
  title. None identifies a key or authorises an edit.
