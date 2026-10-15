# Handoff R24 — Gen7 `CTX0` context records decoded from their pointers: trigger, conditions, speech/context items with typed parameters (PS3 / Xbox 360 NHL 13–Legacy)

Research lane, 21 September 2026. Applies the coordinator review
[research-r22-r23-review-2026-09-21.md](../../validation/research-r22-r23-review-2026-09-21.md) F1 (the R20 rev. 4 item routine was a
scan, not a parse) and its "next research prompt" step 3 (decode the speech-list structure independently), then step 4 (the header words).
Claim `research-r24-ctx0-structure`. Nothing here is game-verified; nothing written outside research-owned paths; no launch; **no writer proposed**.

Machine-readable: [`data/r24-ctx0-records.json`](data/r24-ctx0-records.json) (35 MB, compact; every record of every set with header
words, items, parameters, conditions, extra words, the scan comparison, the platform comparison and the input identities). Probe:
`local/runs/overnight-research-2026-09-21/probes/r24_ctx0_records.py` (≈ 5 s, read-only; imports the 2026-09-20 `r20_gen7_speech_events.py`
for the `EVT0` / `MOIR` readers and for the rev. 4 scan it is compared against).

**Evidence classes** (as in handoff 13 of 2026-09-20): *parsed* = a typed pointer / offset resolved inside the file, or a rule verified on
every record with zero exceptions; *candidate* = a reading that a pointer does not confirm; *not decoded* = raw values recorded only;
*runtime* = nothing observed.

## Plain-language summary

The context file that decides which commentary line is spoken is now read the way the game reads it — by following its pointers — instead of
by searching record bodies for likely words. Every one of the 3 371–4 183 records per title parses without a single refusal, and every record
turns out to have the same shape: a **trigger** (the game event that starts the evaluation, named through a typed offset into the file's
trigger-name pool), a list of **conditions** (each a named test that points at one compiled expression in the bytecode file, with 0–4 integer
arguments), one or two **items** (what to do when the context is chosen: speak a speech event, or raise a context event back to the engine),
each item carrying **one parameter value per parameter the event declares** (immediate integers, or values computed by another bytecode
expression), and a few header numbers that remain undecoded. The old scan had found all the speech items (100 % agreement on the 3 492–4 321
it found) but missed the 74–562 items per title that raise a context event instead of speaking, and it had named 58–66 contexts per title with
a regex that could not read names containing `+`. PS3 and Xbox 360 carry identical records field for field on NHL 14, 15 and Legacy. What the
bytecode expressions compute, which context the engine picks when several triggers' conditions hold, and every runtime behaviour remain
unobserved — but the *static* structure is no longer heuristic, and the "blob ↔ context" link the R22 review asked about is a typed index.

## 1. File layout (parsed; NHL 13 X360, 14 / 15 / Legacy PS3 + X360; NHL 11 / 12 refused)

All big-endian. Magic `CTX0PS30` / `CTX0X360`, then `u16 4, u16 5`.

| Offset | Field | Verified rule |
|---|---|---|
| 0x10 | `N` record count | equals the count at 0x18 on every set |
| 0x14 | `A` | 96–127 per title; the table at 0x30 is exactly `(k, 0)` for `k = 0..A−1` (meaning not decoded; A ≠ the speech `EVT0` secondary-list count: 108 vs 108 on NHL 15 is a coincidence — 127 vs 85 on NHL 14) |
| 0x28 / 0x2c | pool 1 / pool 2 offsets | pool 1 = NUL-terminated **condition names** (1 835–2 050); pool 2 = **trigger names** (137–153); pool 2 runs to EOF |
| T1 = 0x30 + 8A | `N × u32` record pointers, file order = record index | distinct; every pointer inside `[T3 end, pool 1)`; `min == T3 end` |
| T2 = T1 + 4N | `N × (u32 1, u32 → T3 row i)` | exact on every set |
| T3 = T2 + 8N | `N × (u32 context id, u32 record ptr)` sorted by id | ids distinct, pointer set == T1 set; first record starts at T3 end |

Records are contiguous from T3 end to pool 1 (bounds = sorted pointers, as R20 rev. 4 had). NHL 11 (PS3 + X360) and NHL 12 (X360) files
are `CTX0` version `(0,0)` with a different header (`\0\3\0\1`) — refused by the parser, not attempted.

## 2. Record layout (parsed)

Header = 15 u32 (60 bytes), then in-record data reached only through the header pointers:

| Word | Content | Status |
|---|---|---|
| w0 | context id | == record index on every record; == the T3 id for its pointer |
| w1 | integer, 189–249 distinct values per title (10, 100, 120, 150, 200, 400, 2000, 5000, 6001, 9999 …) | **not decoded** ("priority" is a guess only; within one trigger the values vary, e.g. `CUP_ENDINGS` 10 / 399 / 400 / 2001 / 2002 / 6000 / 6001 / 9999) |
| w2 | `u8 ncond │ u8 nextra │ u8 b2 │ u8 100` | ncond and nextra verified against the pointed lists on every record; `b2 = 1` on exactly the 22 Legacy records that carry the slot-w11 block, 0 elsewhere; byte 3 is 100 on every record |
| w3 | 0 or `0x01000000` (126–289 records) | **not decoded** |
| w4 | `u16 hi` (25–32 distinct: 0, 30, 60, 90, 120, 180, 240, 300, 600, 2000 …), `u16 lo` (1; 20 / 25 / 28 on 35–116 records) | **not decoded** (hi looks like a time in some unit; unverified) |
| w5, w6, w7 | 0, 0, `ffffffff` | on every record |
| w8 | offset into **pool 2** → the trigger name; `ffffffff` = no trigger (51 / 116 / 102 / 57 records on 15 / 14 / 13 / Legacy) | **parsed** — resolves on every record; this closes the R22 "is w8 a trigger id" question: it is a *name offset*, not an id, and no census against crowd contexts was needed |
| w9, w10 | item slots: pointer or `ffffffff` | 1 or 2 items per record (never 0 items with a trigger; the no-trigger records all have exactly 1 speech item) |
| w11 | `ffffffff`, or (Legacy only, 22 records) a pointer to a 4-word block `1 │ 3 │ 0 │ n` (n = 1 / 2 / 3) or `0 │ 7 │ 6 │ 4` | block **not decoded** (recorded raw; never an item) |
| w12 | pointer to the condition pointer list, `ffffffff` iff ncond = 0 | list length `(first ptr − list) / 4 == ncond` on every record |
| w13 | pointer to `nextra` u32 words, `ffffffff` iff nextra = 0 | words are all multiples of 8 (≤ 1 056; 102 distinct on NHL 15) — **not decoded** (not pool offsets, not blob indices) |
| w14 | pointer to the context name (NUL string inside the record) | resolves on every record (R20 rev. 4 needed a regex fallback on 58–66 records: names containing `+`, e.g. `Goal_Ram_PER_1_2Q_Down_3+`, `+Tendencies_slideshow_killer` — the 61–69 "context name disagreements" in the JSON are exactly those) |

### Item (36 bytes fixed on PS3, 28 on Xbox 360, then the parameter table)

`u32 0 │ u32 ref │ u32 0 ×4 (PS3) / ×2 (X360) │ u32 nparam │ u32 0 │ ptr name │ nparam × (u32 kind, ptr)`

- `ref = namespace << 16 | event id`. Namespace = the speech module's (`pbp_eng_speecheventdef.csi`) → a **speech item**; = the context
  module's (`pbp_eng_contexteventdef.csi`) → a **context item** (raise a trigger back to the context module). No other namespace occurs.
- `name` = the event's name spelled inside the record. It is in the MOIR candidate list of its id on **every** item (4 522 / 4 522 on NHL 15;
  the 305–469 items whose 16-bit id is shared by several names all spell exactly one of them — still recorded as "label matches one
  candidate", the engine's own resolution being unobserved).
- `nparam == the number of parameters the event declares in its EVT0 record` on **every** item of every set (speech events via
  `pbp_eng_speecheventsubscr.bin`, triggers via `pbp_eng_contexteventsubscr.bin`) — this is the check that types the parameter table.
- parameter `kind 1` → pointer to one immediate u32; `kind 2` → pointer to a **call block** `u32 0 │ u32 blob │ u32 nargs │ (nargs == 0 ?
  ffffffff : 0) │ nargs × (u32 1, ptr → u32)`; no other kind or shape occurs (any would be a refusal). Kind-2 blobs: 229–266 distinct per title.
- Context items name 8–18 triggers per title, dominated by `trigger_HIGH_INTENSITY_PBP_HAS_PLAYED` (325 / 360 on 15 / Legacy) and
  `trigger_GOAL_REPLAY_FOLLOW` (111); NHL 13 / 14 have 74 / 112 such items.

### Condition (16 bytes, one per pointer of the list)

`u32 nargs │ ptr-or-ffffffff (→ nargs × u32) │ u32 blob index │ u32 name offset in pool 1`

- nargs 0–4 (0 on ~60 %, 1 on ~37 %); the argument pointer is `ffffffff` iff nargs = 0.
- The name offset resolves in pool 1 on every condition; **every pool-1 string is referenced** except 52 on NHL 15 (listed in the JSON —
  e.g. `Playoffs_Calder_Cup_Playoffs_Over`, `any_team_shorthanded`).
- **blob index ↔ name is 1 : 1** on every set (1 835 / 1 954 / 1 980 / 2 050 blobs ↔ as many names; no blob with two names, no name with two
  blobs). The indices range 1..count of the `.gas` `CSIS` blob table (2 261 / 2 386 / 2 535 / 2 681, handoff r22), the kind-2 parameter blobs are
  **disjoint** from the condition blobs, and their union leaves 197 / 198 / 303 / 365 blobs unreferenced by the pbp contexts.

Worked example (NHL 15, both platforms — the R20 rev. 4 example re-read): record 1 `Cup_End_Cup_Presentation_NHL`: trigger
`trigger_CUP_ENDINGS`, w1 6001, w4 60/1, conditions `cup_type_Stanley_Cup` (blob 246), `+_Context_RUN_ONCE` (97), `+Group_Cup_Sequence_5_run_once`
(127), extra [128], item 1 = speech `CUP_END_CUP_NHL` with 1 parameter = immediate 0. Record 2 `Cup_End_Cup_Presentation_AHL`: identical but
`cup_type_Calder_Cup` (239) and w4 300. Record 52 `Playoff_End_Stanley_Cup_Presentation`: conditions `+_Context_RUN_ONCE`,
`playoff_cup_type_Stanley_Cup` (1260), `+Group_Cup_Sequence_5_run_once`; items `CUP_END_CUP_NHL` then `SILENCE_HALF_SECOND` (0 parameters).

## 3. Coverage and comparison with the R20 rev. 4 scan

| Set | records | pool 1 / pool 2 | items (speech + context) | ambiguous-id items, all spelling one candidate | items / record | no trigger | conditions | blobs referenced (cond + param) of `.gas` count | scan items = parsed speech items | scan-missed context items | name disagreements (regex fallbacks) |
|---|---:|---|---|---|---|---:|---:|---|---|---:|---|
| NHL 13 X360 | 3 371 | 1 835 / 137 | 3 566 (3 492 + 74) | 305 | 1: 3 176, 2: 195 | 102 | 13 128 | 1 835 + 229 = 2 064 of 2 261 | 3 492 = 3 492 | 74 | 69 (66) |
| NHL 14 PS3 = X360 | 3 583 | 1 954 / 142 | 3 796 (3 684 + 112) | 343 | 1: 3 370, 2: 213 | 116 | 14 134 | 1 954 + 234 = 2 188 of 2 386 | 3 684 = 3 684 | 112 | 68 (66) |
| NHL 15 PS3 = X360 | 3 866 | 1 980 / 153 | 4 522 (4 007 + 515) | 395 | 1: 3 210, 2: 656 | 51 | 15 440 | 1 980 + 252 = 2 232 of 2 535 | 4 007 = 4 007 | 515 | 61 (58) |
| Legacy PS3 = X360 | 4 183 | 2 050 / 149 | 4 883 (4 321 + 562) | 469 | 1: 3 483, 2: 700 | 57 | 16 521 | 2 050 + 266 = 2 316 of 2 681 | 4 321 = 4 321 | 562 | 65 (62) |

- **Platform comparison:** PS3 and Xbox 360 records are equal in order on every decoded field (name, trigger, w1, w3, w4, ncond, nextra,
  extra words, slot-w11 block, items with kinds / names / parameter values, conditions with names / arguments / blob indices): 3 583 / 3 583,
  3 866 / 3 866, 4 183 / 4 183. Only the item fixed part differs (36 vs 28 bytes) and hence the file bytes. The `.gas` files are byte-identical
  across platforms on all three (same SHA-256 per title).
- **Scan vs parse:** every scan hit is a parsed speech item and vice versa (offset-for-offset); no label disagreement on the 3 492–4 321
  common items; the scan had no way to see the 74–562 context items; the 58–66 regex-derived context names are replaced by pointer-resolved
  names (all 61–69 differing names listed under `vs_r20_scan.context_name_disagreements`).
- **R20 rev. 4's "0-item records"** (2 on NHL 15: `BIT_*_Boardplay_Offense_Restart`) were an artefact of the scan; every record has ≥ 1 item.
- **Trigger names vs the context eventdef:** 147 / 153 pool-2 names on NHL 15 (and Legacy) are `pbp_eng_contexteventdef.csi` triggers; the
  other 6 (`trigger_DYNASTY_PASSIVES`, `trigger_GAMEOVER_FOLLOW`, `trigger_PERIOD_END_MONTAGE`, `trigger_FIGHT_REPLAY_PT2`,
  `trigger_REPLAY_HIGHLIGHTS_FOLLOW`, `trigger_SAVE_PBP_FOLLOW`, 149 records) exist only as **speech-module** events — a context triggered by the
  speech module (candidate reading: a follow-up chain). NHL 13 / 14: 137 / 137 and 142 / 142 resolve in the context eventdef.
- **Generality check:** the same parser reads `crowds_contexts.bin` (crowd-reaction module; 1 164 records, 1 171 items all in the crowd
  namespace, names 100 % in `crowds_contexteventdef.csi`, 0 refusals on NHL 13 X360 / NHL 15 PS3 / Legacy PS3). Not in the JSON; rerun with
  `parse_ctx0(<crowds_contexts.bin>, None, <crowd namespace>)`.

## 4. What is parsed, candidate, not decoded, runtime

- **Parsed:** the file tables; record bounds; the trigger name (w8); the context name (w14); item count, kind (speech / context), event id, spelled
  name, parameter count (typed by EVT0), parameter values (immediate) and the call-block shape (blob index + immediate arguments); condition
  count, names, argument lists and blob indices; the extra-word count; the Legacy slot-w11 block bounds.
- **Candidate:** that a condition's blob index selects the `.gas` blob that *evaluates* the condition, and a kind-2 parameter's blob the value —
  supported by the 1 : 1 blob ↔ name map, the disjoint index sets inside one table and the exact range, but no blob has been decoded; the
  "follow-up chain" reading of speech-named triggers; the `SILENCE_*` second items as sequencing.
- **Not decoded:** w1, w3, w4 (hi/lo), the extra words, the Legacy slot-w11 block, the `A` table, the meaning of condition / parameter argument
  integers (values 0–30 — plausibly enum values of the game variables named by the conditions; unverified).
- **Runtime:** context selection among triggered candidates, priority, windows (`maxNumWindows="25"`, `numPassives="200"` in
  `pbp_eng_contextmoduledef.xml` are the only static hints), `+_Context_RUN_ONCE` / `+Group_*_run_once` semantics, what a context item's raised
  trigger does, sentence / body / take choice, the missing-sentence behaviour.

## 5. Parser refusal behaviour (for the developer contract; every case is exercised by the checks above or by construction)

Header: wrong magic / version, `N ≠ N'`, pools outside the file, non-zero reserved words, a record pointer outside `[T3 end, pool 1)` or
duplicated → refuse the file. Pools: a missing final NUL → refuse the file. Per record (record marked `malformed`, listed in `refusals`, the
rest of the file still parsed): record shorter than 60 bytes; w8 not a pool-2 string start (other than `ffffffff`); any pointer (item, name,
parameter, call block, argument, condition list, condition, extra) outside `[record + 60, record end)`; unterminated string; item fixed words
not zero; `nparam > 64`; parameter kind ∉ {1, 2}; call-block shape violation or argument kind ≠ 1; condition list length ≠ ncond; condition
name offset not a pool-1 string start; argument pointer `ffffffff` with nargs > 0 or vice versa; nargs > 16; nextra words beyond the record.
Not refused (recorded): an item whose spelled name is not among its id's candidates (0 observed), a blob index above the `.gas` count (0
observed), trigger `ffffffff`. Nothing is silently dropped: 0 refusals on all 8 sets, the crowd files included.

## 6. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r24_ctx0_records.py docs/research/overnight-2026-09-21/data/r24-ctx0-records.json   (≈ 5 s)
```
Inputs (read-only; SHA-256 + size in the JSON `inputs` per set): `audio/context/pbp_eng_{contexts.bin,contexteventdef.csi,contextgasscript.gas}`,
`audio/speech/pbp_eng_{speecheventdef.csi,speecheventsubscr.bin}` of the loose PS3 / Xbox 360 trees named in the probe (NHL 15 PS3 contexts
`0699651c…`, NHL 15 X360 `a5ee5c5d…`, Legacy PS3 `5a001d99…`, X360 `95ed9350…`, NHL 14 PS3 `abd8b561…`, X360 `c324df7c…`, NHL 13 X360 `9d65cbe1…`).
Executed: 9 probe runs while the layout was established (the intermediate dumps are not retained), 1 final run, the crowd-file check, the
scan comparison. Not executed: launches, writes, Studio binaries, `.gas` decoding.
