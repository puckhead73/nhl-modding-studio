# Final receipt — research lane, 24 September 2026 (04:57 → 05:56 MDT; correction checkpoint 13:25 MDT)

Written for: Codex (coordinator review) and the Studio developer. Everything below is researcher-verified static evidence pending
coordinator review; nothing is game-verified, nothing was heard, nothing was written to any game, anchor or application path, and
nothing was staged, committed or pushed.

## Plain-language results

- The older-console audio coverage table has no "not surveyed" cell left: PlayStation 3 NHL 09, 10, 12 and 13 music and sound-effect
  banks are measured from their archives and decode under the established grammar; Xbox 360 NHL 09's archived banks are EA's XMA.
- The one sample that failed yesterday was a bank-record misreading, not a codec problem. The corrected record grammar (loop records
  carry a loop-start word and one or two audio segments; stream records are stubs; a table offset of all-ones is an empty slot) tiles
  every one of 35 706 records on nine titles with zero failures. Four- and six-channel entries are interleaved stereo streams — named as
  classes, not decoded.
- The last unidentified speech codec (`0x09`) is Speex ultra-wideband: read from the data first (block sizes, frame counts, header
  bits), then matched to the pinned public decoder, including the 509-sample delay. Two separate Speex decoders return the predicted
  sample counts on 96 / 96 vectors across 25 announcer / play-by-play families on Xbox 360, PS3 and PS4 15–21.
- EA-XMA has a bounded diagnostic-preview route: each block is one XMA2 packet; a plain RIFF wrapper decodes to the declared count on
  91 / 91 vectors, and a control shows that count comes from the bitstream, not from the wrapper.
- The gen7 game's own load file names every horn and chant bank by id; the European / minor-league horn ids are unlisted there — the
  strongest static statement available. The four-digit chant banks are goalie chants keyed by goalie announcer ids. The PS4 goal-horn
  consumer is located (a Harmony patch) but not decoded.
- A team's art id is a shared key (a club and its Winter Classic / Alumni / Circuit rows), so any re-key would have to move every row
  and every keyed file of an id or refuse. **Corrected (R88):** the re-key is an unfinished contract — destination occupancy across
  files and archives, sharing, atomic restore and PS4 vault references are open; the shared-id inspection is what is usable now.
- Every X360 / PS3 08–Legacy database is packed to its slot counts, so a no-growth in-place "create a team" is refused there; whether
  a table can grow is an engine question. **Corrected (R88):** "PS4 18–21 can host only a non-NHL team" is withdrawn — the stat tables
  hold NHL and AHL season participants, not one row per club; what remains is measured headroom plus a proposed allocation experiment.

## League / Team blockers removed versus still open

| Blocker | Before | After |
|---|---|---|
| gen7 audio coverage (1.5 / 2.7) | PS3 09 / 10 / 12 / 13 music + sfx unsurveyed; chant residual | **closed** (handoff 01) |
| `.abk` parser contract | R80 entry rule (non-loop only) | **replaced** by the record grammar (handoff 01 §1 / §6) |
| speech codec `0x09` | unidentified | **identified** (EA-Speex), diagnostic route (handoff 02 A); start rule carded (A-SPX-1) |
| EA-XMA | no route | **diagnostic route** with the decoder constraint stated (handoff 02 B); encoder absent |
| residual horn ids | absent in folders / archives | **unlisted in the loader's asset table** (handoff 03 §1); runtime card H-HORN-1 rev. 2 |
| PS4 horn → sound | file named | consumer **located**, format undecoded (handoff 03 §3) — open |
| Team `artid` re-identification | cascade contract not accepted | **still contract-blocked** — shared-key inspection ready (handoff 04 §1–2); writer gaps named (handoff 06 §F3); T-ARTID-1 narrowed |
| team / league creation | contract missing | **still contract-blocked** — capacities measured with corrected provenance (handoff 06 §F1); no-growth refusal on gen7 scoped; PS4 18–21 / real NHL 20 headroom measured, runtime requirements unresolved, experiment proposed (handoff 06 §F2) |
| Xbox packed tail (R5) | H1 candidate | unchanged — no new method arose; no package spent |
| NHL 07 labels, CHL participants, NHL 08 / 09 settings, PS4 kit duplicate-pack selection, venue re-pointing | open | unchanged (runtime or no new method) |

## Measured title / platform coverage (this session)

- `ABKC` grammar: loose PS3 11 / 14 / 15 / Legacy (18 124 records), archived PS3 09 / 10 / 12 / 13 + X360 09 (17 582 records).
- Music: PS3 09 / 10 / 12 / 13 (3 vectors each). `.abk` decode: PS3 09 / 10 / 12 / 13 (every ram entry of goal / period / chant / sfx).
- EA-Speex: X360 11 / 12 / 13 / 14 / 15 / Legacy `ann_eng`; X360 11 / 12 / 13 / 14 + PS3 14 `pbp_eng`; PS4 15–21 rink announcer (4 vectors each).
- EA-XMA: X360 09 / 10 speech, X360 09–12 music, X360 09 (archived) and 10–Legacy loose `.abk` (91 vectors, 40 families).
- Loader table: all 16 gen7 trees. `artid` / allocation: 25 anchors (X360 / PS3 08–Legacy, PS4 15–21).

## Ready contracts, by kind

- **Inspection / listing:** `.abk` record grammar and classes; loader asset table join; goalie chant join; `artid` row-set listing; allocation
  headroom and refusal texts.
- **Diagnostic (decoder acceptance, not heard):** EA-Speex rewrap; EA-XMA rewrap; the widened EALayer3 v1 coverage.
- **Writer proposal:** none accepted. The `artid` re-key of handoff 04 §4 is an unfinished contract (handoff 06 §F3); the shared-id
  inspection is the usable part.
- **Blocked / unknown, named:** every audio replacement (encoders; `.abk` 0x78 tree; `.DUR` mirrors; PS4 patch data); team creation on
  gen7 (engine growth); PS4 vault writes.

## Exact checks and residuals

- Zero grammar failures (T1 / T2 / T4) on 35 706 records; T3 residuals: 6 one-sample placeholders, 4 entries at exactly 4-granule trim,
  29 six-channel entries unclassified for decode.
- Speex: 96 / 96 structure + decode; 1 vector skipped (X360 14 `pbp_eng`, truncated loose stream, known); decoders differ 7–29 % rms on 95 vectors and **0.89 rms ratio / 0.46 correlation on the PS4 NHL 21 single-block base-bank vector** (outlier; cause unidentified; neither decoder validated there).
- EA-XMA: 91 / 91 decode = declared; 15 entries with a different packet-header class (`000_period.abk`, NHL 09 archived) — decode unaffected;
  1 vector skipped (X360 11 music beyond the loose stream).
- Malformed controls: refused on 25 / 25 (Speex), 40 / 40 (XMA), 16 / 16 (`.abk` archived classes) families.

## Developer claim pointers

`local/coordination/overnight-2026-09-24/claims/`: `research-r0-blocker-map-20260924`, `research-r82-gen7-archive-audio-coverage`,
`research-r83-codec09-easpeex`, `research-r84-ea-xma-preview`, `research-r85-audio-consumers`, `research-r86-team-artid-graph`,
`research-r87-team-row-allocation` — all `ready`. Developer queue rows 1–10 in [DEVELOPER-QUEUE.md](DEVELOPER-QUEUE.md). The developer's
own `dev-p0-reviewed-texture-write` claim (complete) has no overlap.

## Deferred user cards

A-SPX-1 (Speex start), A-XMA-1 (XMA rewrap), T-ARTID-1 (art-id re-key), H-HORN-1 rev. 2 — [USER-CARDS.md](USER-CARDS.md); earlier cards unchanged.

## Owned cleanup

Scratchpad emptied by literal path (decode scratch, reference copies, smoke JSONs, run logs, inventory scripts); `__pycache__` created by
this lane removed; `%TEMP%` clean of this lane's copies; no lane process running; durable evidence retained under
`docs/research/overnight-2026-09-24/data/` (six JSONs) and the six probes under `local/runs/overnight-research-2026-09-24/probes/`.
Corrections this session: clock stamps re-derived from file times; handoff 01 re-created after an encoding fault emptied it (noted in its header).

## One exact next task

Decode the `ABKC` bank tree at 0x78 on the loose PS3 trees (sound → sample index; stream records → `.ast` block offset), anchored on the
R82 record grammar, with `crowds.abk` (86 stream records vs 89 chained `.ast` streams) as the first discriminating vector — a
read / selection / stream-address contract that unblocks the sfx listing's stream previews and sound names. **It does not by itself
qualify replacement** (handoff 06 §F4 lists the separate prerequisites of same-length substitution and arbitrary import).

## Correction checkpoint (13:25 MDT)

Applied the coordinator review's F1–F4 ([handoff 06](06-correction-checkpoint-campaign-review.md), claim
`research-r88-correction-checkpoint`, `data/r88-correction-checkpoint.json`): NHL 19 / NHL 20 rows re-measured on witnessed sources with
hashes (the `ps4/nhl20` anchor retained as a mislabeled NHL 19 base measurement); stat-table membership audited and the "one row per NHL
club" / "non-NHL only" statements withdrawn; the R86 re-key downgraded to inspection + unfinished contract; audio boundaries re-bounded
(0x78 = indexing, the NHL 21 outlier carried, u8 domain 1..255, 509 = count rule). Dated notes on handoffs 02–05, the queue, atlas,
STATUS and cards; no evidence replaced. Two archive members were extracted to the scratchpad and deleted after hashing.

## R89 addendum (13:47 MDT)

The "one exact next task" was executed: the `ABKC` bank tree is decoded as a read / selection / stream-address contract on the 11 loose
trees ([handoff 07](07-abk-bank-tree-read-selection-stream-address.md), claim `research-r89-abk-bank-tree`, `data/r89-abk-bank-tree.json`):
every sample record named once by a sample list, every stream record addressed in its `.ast`, loops explained, three controls refusing
on every tree. Not established: stream payload decode (next package, Q-R89-1), the S / X words, the name hash, node classes 1 / 2 / 4,
the archived trees. Replacement unchanged (handoff 06 §F4). Scratch: one smoke JSON and the aggregation script, deleted.

## R90 addendum (13:54 MDT)

Highest-impact ready League / Team blocker after R89: the "league-keyed cues selected by compiled conditions" item. Result
([handoff 08](08-gen7-team-chant-binding.md), claim `research-r90-team-chant-binding`): twelve club chant banks identified by name
(Swedish clubs), known to the crowd script on every title, addressed by no database field — the selection key is script-resident;
arena-keyed join re-measured per title; counterexample kept; card T-CHANT-1. Static evidence cannot discriminate the script's
comparison key, so the card is published and the item is closed as "no DB key" (bounded).

> **Note 2026-09-25 14:20 (R97, [overnight-2026-09-25/10-correction-checkpoint-direction-audit.md](../overnight-2026-09-25/10-correction-checkpoint-direction-audit.md)):** "addressed by no database field — the selection key is script-resident" and "closed as
> 'no DB key'" are withdrawn as conclusions: the Finnish arena's chant id **is** 61 on every anchor (a transcription error hid it; its horn
> ids are 61 too), and no selecting consumer has been traced either way. The item is **open**: club association by name measured, loading by
> number measured, binding untraced (card T-CHANT-1 rev. 3). The R88 source count is 27, not 28.
