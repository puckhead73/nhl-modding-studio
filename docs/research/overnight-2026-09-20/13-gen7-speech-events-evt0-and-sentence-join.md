# Handoff R20 (rev. 4) — Gen7 commentary data (PS3 / Xbox 360 NHL 13–Legacy): context records, speech-event ids, names, hashed sentences, keyword-resolved samples — parsed; the bytecode bounded; runtime not observed

> **Supersession note (2026-09-21):** §3's item routine was a record-bounded *scan* (review F1); the `CTX0` structure is now decoded by pointer walk in [2026-09-21 handoff 01](../overnight-2026-09-21/01-ctx0-context-records-decoded.md) (items include 74–562 context-namespace items per title the scan missed; all names by pointer; no 0-item records; w8 = trigger-name offset; conditions carry blob indices). §2 / §6 "keywords file in-archive on NHL 13 / 14", "explains the Xbox NHL 13 mismatch" and "287 is a control value" are superseded by [2026-09-21 handoff 02](../overnight-2026-09-21/02-nhl13-14-keyword-resolution-and-source-identity.md): no keyword table exists on NHL 13 / 14; 257 / 287 / 298 is the `GRF0` word-type tag. `EVT0` / `MOIR` / sentence-hash / `KWD0` / `SBKR` findings stand.

Research lane, 20 September 2026. Rev. 2 applied the coordinator review
([docs/validation/research-r19-r20-review-2026-09-20.md](../../validation/research-r19-r20-review-2026-09-20.md)); rev. 3 found the MOIR
name table and the sentence hash rule; **rev. 4** (this session) decodes the `CTX0` context records so the context → event references are
record fields rather than scan candidates, bounds the `.gas` bytecode by its section table, and corrects the reading of the sentence
phrase words (keyword ids, not bank sample ids). Gen7 1.5 (Q-R9-3 / Q-R20-1). Claim `research-r20-gen7-speech-events`. The read-only
implementation contract for the proven parts is a separate document: [14](14-gen7-commentary-read-only-contract.md). Nothing here is
game-verified; nothing written; no launch; **no writer proposed**.

Machine-readable: [`data/r20-gen7-speech-events.json`](data/r20-gen7-speech-events.json) (41.6 MB, rev. 4, compact) and
[`data/r22-gas-sections.json`](data/r22-gas-sections.json) (rev. 1). Probes: `r20_gen7_speech_events.py` rev. 4 (≈ 5 min), `pbp_sentences.py`
rev. 3, `r14_audio_keywords.py` (2026-09-20 R14, reused), `r22_gas_sections.py`.

**Evidence classes:** *parsed* = typed pointers / ids resolved inside a file, or a rule verified on every record; *heuristic* = labelled;
*candidate* = a reading not confirmed by a pointer; *runtime* = nothing observed.

## Plain-language summary

The gen7 commentary is five data files plus a bytecode file. **Context records** (`audio/context/pbp_eng_contexts.bin`, 3 371–4 183 per
title) each carry a name, a few header numbers whose meaning is unknown, and an ordered list of one or two *speech items*; every item is
a typed reference to a speech event (`namespace << 16 | event id`) followed, inside the record, by the event's name spelled out. The
**MOIR** file maps event id → name (a plain table); **EVT0** lists each event's parameters; the **sentence file** is a hash table keyed by
`FNV-1(name)` (verified on every sentence), each sentence having one or more bodies whose phrases are lists of **keyword ids**, and the
**keywords file** maps keyword id → the bank sample ids (the actual audio takes). So the static chain
context → event → name → sentence → keywords → samples is parsed end to end on NHL 15 / Legacy (both platforms), and up to the keyword
step on NHL 13 / 14, whose keywords file is not loose. Event ids are 16-bit and repeat (149–221 ids carry several names); the record's
inline name string matches exactly one candidate in every such case (395 / 395 items on NHL 15) — the JSON keeps all candidates and
marks which one the label matches; nothing is chosen for the reader. On NHL 15 / Legacy only 1 489 / 1 718 of 3 714 / 3 982 events have
a sentence carrying their name; all 83 / 88 AHL / CHL / OHL / QMJHL / WHL / tournament cup events are among those without — their contexts
exist and reference them, so what the engine does when the lookup finds no sentence is a runtime question. The **bytecode**
(`*_contextgasscript.gas`, `CSIS`) is bounded to a table of 1 739–2 681 blobs with no identifier strings; which blob a context runs, and
what the instructions do, are not decoded. Which context fires, and which body/take is chosen, are runtime and unobserved.

**Corrections carried in this revision:** rev. 3's "phrase words are sample ids" — they are keyword ids (18 407 / 19 658 words on NHL 15 are
in the keywords table; the Xbox NHL 13 bank ids start at 77 269 and cannot match them; NHL 14's 31 325 / 31 490 "in bank" of rev. 3 was
a coincidence of id ranges); the constant word `287` sits once in **every** body (3 337 / 3 337) and is a control value, not a keyword,
although keyword 287 exists. Rev. 3's scan-derived context labels are replaced by record names (201 disagreements on NHL 15 kept in the
JSON: 183 where the scan had no label, 18 where its pointer heuristic pointed at a neighbouring record's string).

## 1. `EVT0` and `MOIR` (parsed; unchanged from rev. 3 except the coverage table)

| Set | `EVT0` magic / ns | events (main + secondary) | params | param refs resolved | MOIR entries / distinct ids / ids with 2+ names |
|---|---|---:|---:|---|---|
| PS3 + X360 NHL 14 | `0x7850` | 3 413 (3 328 + 85) | 85 | 3 303 / 3 315 (`256`) | 3 413 / 3 245 / 166 |
| PS3 + X360 NHL 15 | `0x6c8d` | 3 714 (3 606 + 108) | 108 | 4 332 / 4 341 (`286`) | 3 714 / 3 522 / 189 |
| PS3 + X360 Legacy | `0x79b7` | 3 982 (3 881 + 101) | 101 | 4 580 / 4 585 (`297`) | 3 982 / 3 757 / 221 |
| X360 NHL 13 | `0x0514` | 3 248 (3 164 + 84) | 84 | 3 231 / 3 243 (`256`) | 3 248 / 3 097 / 149 |
| X360 NHL 12 | v1.2 `0x23ea` | 3 400 | 58 | 3 276 / 3 290 | sentence file version unparsed |
| PS3 / X360 NHL 11 | `EVT0` v0 | — | — | — | unsupported |

Layouts as in rev. 3 (event record fixed part 72 B PS3 / 48 B X360; MOIR stride 24 @0x40 PS3 / 12 @0x28 X360). PS3 and Xbox 360 `EVT0`
tables are equal record-for-record on 14 / 15 / Legacy (`platform_comparison`).

## 2. Sentence file (parsed) and phrase words (rev. 4 reading)

Slot = `FNV-1(name) mod N` on every sentence (1 489 / 3 378 / 1 718 / 3 215 on NHL 15 / 14 / Legacy / 13). Sentence = `c` bodies; body = phrase
list; phrase = `4 type bytes | u8 count | ff ff ff | count × u32 word`. **Words:** on NHL 15 / Legacy (keywords file loose) 18 407 / 19 658 and
20 922 / 22 930 words are keyword ids of `pbp_eng_keywords.bin` (`KWD0`, 26 109 / 26 069 pairs, 2 121 keywords; 22 080 / 26 109 keyword sample
ids are in the NHL 15 bank); `287` occurs once per body (control value); 513 / 755 words are neither keyword nor bank id (unresolved, listed
per body). On NHL 13 / 14 no keywords file is loose (in-archive) — words stay **unresolved identifiers**; their equality with bank ids
(31 325 / 31 490 on NHL 14, 0 / 29 392 on Xbox NHL 13) is reported as a coincidence measure only. Phrase type words (first byte 1 / 3 / 12 /
22 / …, third byte 1–6) are control values of unknown meaning (census in the JSON).

## 3. `CTX0` context records (parsed; rev. 4)

`CTX0PS30 | u16 4, u16 5 | 0 | u32 record count | u32 A | u32 record count | … | @0x28 u32 string pool 1 | @0x2c u32 string pool 2 | @0x30 A × (u32 k,
u32 0) | record pointer list (count × u32, unsorted) | count × (u32, u32) | @47 304 count × (u32 context id, u32 record ptr) sorted by id` —
the id ↔ pointer table agrees with the records' first word on 3 866 / 3 866. **Record** (bounded by the sorted pointers; last ends at pool 1):
`u32 context id | u32 w1 | u32 w2 | u32 w3 | u16 w4, u16 1 | 0 | 0 | ffffffff | u32 w8 | ptr speech list | ptr/ff | ff | ptr/ff | ptr/ff | ptr name`, then
in-record data: the speech items (`0 | ns<<16|id | … | name string of the event`), pointers into the record, and the context name. Names
resolve through the `+56` pointer on 3 808 / 3 866 (fallback: last identifier string in the record, 58; all named). `w1` (10 / 100 / 200 /
400 …, 224 values), `w2`, `w4`, `w8` (154 values): **not decoded**.

| Set | records | speech items | items whose inline label equals a MOIR candidate | items with an ambiguous id / label matches exactly one | items per record | scan refs = record items |
|---|---:|---:|---:|---|---|---|
| NHL 13 (X360) | 3 371 | 3 492 | 3 492 | 305 / 305 | 1: 3 242, 2: 125, 0: 4 | 3 492 = 3 492 |
| NHL 14 (PS3 = X360) | 3 583 | 3 684 | 3 684 | 343 / 343 | 1: 3 474, 2: 105, 0: 4 | 3 684 = 3 684 |
| NHL 15 (PS3 = X360) | 3 866 | 4 007 | 4 007 | 395 / 395 | 1: 3 721, 2: 143, 0: 2 | 4 007 = 4 007 |
| Legacy (PS3 = X360) | 4 183 | 4 321 | 4 321 | 469 / 469 | 1: 4 041, 2: 140, 0: 2 | 4 321 = 4 321 |

(The PS3 and Xbox `contexts.bin` differ in bytes — different hashes — but give identical record / item counts; not compared field-wise.)
Two-item records are sequences such as `Playoff_End_Calder_Cup_Presentation` → [`CUP_END_CUP_AHL`, `SILENCE_HALF_SECOND`]. Records with 0
items: the two `BIT_*_Boardplay_Offense_Restart` contexts (NHL 15).

## 4. The static trace (parsed hops; every candidate kept)

context record item → every MOIR name for the id (`candidates`; `inline_label` and `label_matches_this_name` recorded) → sentence at slot
`FNV-1(name) mod N` (present, or **no sentence carries this event name** — other sentences may share the bucket) → bodies → keyword ids →
sample ids (where the keywords file is loose). NHL 15 (both platforms): 3 714 names reached, 381 through an ambiguous id, 1 489 with a
sentence, 2 225 without, 83 league-shaped without (all `CUP_END_*_{AHL,CHL,OHL,QMJHL,WHL,TOURNAMENT}[_CPU]`, `CUP_END_CUP_NHL_CPU`,
`CUP_END_HANDSHAKE_CPU`, `CUP_END_PHOTO_GENERIC`, `CUP_END_STANLEY_CONN_SMYTHE`, `INTRO_{AHL,CHL}_PLAYOFFS`, `GAMEOVER_CALDER_CUP_*`,
`PLAYOFF_EXTRO_*`), 27 league-shaped with. Legacy: 1 718 / 2 264 / 88 / 28. NHL 13 / 14: 3 215 / 33 / 0 and 3 378 / 35 / 0. Worked example
(NHL 15 PS3): `Cup_End_Cup_Presentation_NHL` (record 1) and `Playoff_End_Stanley_Cup_Presentation` (record 52) → `CUP_END_CUP_NHL` (id 29243)
→ slot 228 → body 4322 → words [140, 287, 3439] → keywords 140 (→ 23 sample ids incl. 7660–7662, 8965, …), 287 = control, 3439 unresolved.

## 5. `.gas` bytecode — structural bound only ([`r22-gas-sections.json`](data/r22-gas-sections.json))

`u32 0 | u32 2 | u16 3, u16 1 | 'CSIS' | u32 0 | 'CSS_' | u32 0x1c | u32 1 | u32 0 | u32 count | (count + 1) × u32 offset` (sorted; offset[0] = table
end); blob *k* = offset[k] .. offset[k+1] (last to EOF); each blob starts `u32 k | 0 | 0 | 1 | u32 ptr | …` (ids sequential 1..count on 13 / 14 /
15 / Legacy; *not* on NHL 11 / 12, whose layout differs — bounded, not parsed). Counts: 2 261 / 2 386 / 2 535 / 2 681 blobs (13 / 14 / 15 / Legacy),
sizes 0–52 504 B, 0 identifier strings inside; the PS3 and Xbox files are byte-identical on NHL 15 (same SHA-256). **Not decoded:** the
instruction set, the blob ↔ context binding (no header word of the context record indexes the blob table unambiguously — `w8` has 154
values, `w1` 224), and runtime firing.

## 6. What is developer-ready, what is candidate, what is unknown

- **Developer-ready (read-only; NHL 13 X360, NHL 14 / 15 / Legacy PS3 + X360):** the contract in [handoff 14](14-gen7-commentary-read-only-contract.md).
- **Candidate (kept visibly separate):** inline-label → candidate resolution of ambiguous event ids; the meaning of the record header
  words; sequence semantics of two-item records (order observed, timing unknown).
- **Unknown / runtime:** which context fires; bytecode semantics; behaviour when no sentence carries the event name; body / take
  selection; NHL 11 / 12 formats; the 513 / 755 unresolved words; phrase / parameter type codes; the `league` parameter's values.

## 7. Reproduce

```
python local/runs/overnight-research-2026-09-20/probes/r20_gen7_speech_events.py docs/research/overnight-2026-09-20/data/r20-gen7-speech-events.json   (≈ 5 min)
python local/runs/overnight-research-2026-09-20/probes/r22_gas_sections.py docs/research/overnight-2026-09-20/data/r22-gas-sections.json         (≈ 10 s)
```
Inputs (read only; SHA-256 + size per file in the JSON `inputs`; NHL 15 PS3: `EVT0` `f96dc45d…`, sentences `2ae33820…`, MOIR `4bec3c34…`, SBR
`ee90d0fc…`, keywords `8ebba42f…`, contexts `0699651c…`): `audio/speech/pbp_eng_*` and `audio/context/pbp_eng_{contexts.bin,contextgasscript.gas}`
under the PS3 / Xbox 360 trees named in the probe. Executed: r20 rev. 4 (five runs while the record parser, label matching and keyword
resolution were added), r22 (one run), the record dumps of §3. **Not executed:** launches, writes, Studio binaries.
