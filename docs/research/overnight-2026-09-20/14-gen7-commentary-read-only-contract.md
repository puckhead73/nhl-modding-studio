# Handoff R23 — Read-only implementation contract: gen7 commentary listing (PS3 / Xbox 360 NHL 13–Legacy) from the parsed `EVT0` / `MOIR` / sentence / keyword files

> **Superseded (2026-09-21)** by [handoff 03 rev. 2](../overnight-2026-09-21/03-gen7-commentary-read-only-contract-rev2.md): per-word resolution *states* (NHL 13 / 14 have no keyword table), the pointer-walked `CTX0` record listing, the source rule (loose = archive; NHL 13 PS3 archive-only; NHL 14 X360 stream from `nocache.big`). Kept for the record.

Research lane, 20 September 2026. A proposal for the developer, scoped to what handoff [13](13-gen7-speech-events-evt0-and-sentence-join.md)
rev. 4 established by parsing; it is **not** Codex-accepted, proposes **no writer**, and stops where the evidence stops (context firing,
bytecode, runtime). Claim `research-r20-gen7-speech-events`.

## Coverage (measured sets only)

| Title | Platforms measured | `EVT0` | `MOIR` | sentences (hash rule) | keywords → samples | contexts (records) | bytecode (bounded) |
|---|---|---|---|---|---|---|---|
| NHL 13 | Xbox 360 (loose tree) | yes | yes | yes | **no** (keywords file in-archive; words stay identifiers) | yes | yes |
| NHL 14 | PS3, Xbox 360 | yes (equal) | yes | yes | **no** (same) | yes | yes |
| NHL 15 | PS3, Xbox 360 | yes (equal) | yes | yes | yes | yes | yes |
| NHL Legacy | PS3, Xbox 360 | yes (equal) | yes | yes | yes | yes | yes |
| NHL 11 / 12 | — | v0 / v1.2, unparsed / partial | — | unparsed | — | — | table differs |

Anything outside this table (Xbox NHL 14 / 15 / Legacy files inside `audioboot.big` rather than the loose trees, PS3 NHL 13, PS4) is not
covered by this contract. Loose-tree identities (SHA-256, size) are in `data/r20-gen7-speech-events.json` `inputs`.

## Inputs and parsers (all read-only, big-endian)

1. `audio/speech/pbp_eng_speecheventsubscr.bin` (`EVT0` v3.3 / v1.2) → events: `{id (u16), name, params[]}` from the main record list **and**
   the secondary list (`groups[1:]`); parameter table `{id, type, name}`. Reject unknown versions (NHL 11).
2. `audio/speech/pbp_eng_speecheventdef.csi` (`MOIR`) → `{id → [names]}` (stride 24 @0x40 on PS3, 12 @0x28 on X360, chosen by the rule
   `first string ptr == table start + stride × count`). Keep the list per id: ids are not unique.
3. `audio/speech/pbp_eng_sentencedata.bin` (`SPC0`) → sentences `{name, slot, bodies[{id, phrases[{type[4], words[]}]}]}`; assert
   `slot == FNV-1_32(name) mod N` per sentence (refuse the file on any miss).
4. `audio/speech/pbp_eng_keywords.bin` (`KWD0`, when present) → `{keyword id → [sample ids]}`.
5. `audio/speech/pbp_eng_samples.sbr` (`SBKR`) → `{sample id → .sbs offset}` (handoff 02).
6. `audio/context/pbp_eng_contexts.bin` (`CTX0`) → context records `{context id, name, items[{event id, inline label}]}` (record bounds from
   the pointer list; items = in-record `ns<<16|id` words in order).

## Operations (list / inspect only)

- **Events:** list events with parameters; for each event, the sentence whose name equals the event name, or the explicit state
  *"no sentence carries this event name"* (never "silent", never "bucket empty").
- **Sentences:** list name, bodies, phrases; per word: `keyword → sample ids` when the keywords file is present, else "identifier
  (unresolved)"; mark the per-body constant (`287` on NHL 15 — present in every body) as a control value; keep unresolved words visible.
- **Contexts (candidate links, shown apart from the parsed event → sentence data):** list each context's items in order with *all* MOIR
  candidates for the id and the inline label; when the label equals exactly one candidate, display that as "label matches", not as the
  resolution. Header words `w1 / w2 / w4 / w8` are opaque — show raw only behind disclosure.
- **Samples:** resolve keyword → sample ids → `.sbs` ranges for preview (handoff 02 §6 preservation rules apply to any later write; none here).

## What this contract does not claim

Which context fires, what the `.gas` bytecode does, how the engine treats an event without a matching sentence, how ambiguous ids are
resolved at runtime, and which body / take is played. Any UI must say "sentence sharing this event's name", not "the line this event speaks".
