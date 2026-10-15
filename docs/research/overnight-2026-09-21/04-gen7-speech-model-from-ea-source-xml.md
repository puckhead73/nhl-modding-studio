# Handoff R27 — The gen7 speech data model, read from EA's own source form (NHL 09 / 10 ship the tables as XML) and verified on every binary title (NHL 11–Legacy, PS3 + Xbox 360)

Research lane, 21 September 2026, Priority 1.1 (audio inspection prerequisites: "NHL 11 / 12 layout gaps, remaining title / platform
coverage, bank index and sample-range interpretation"). Claim `research-r27-gen7-speech-model`. Nothing game-verified; nothing written
outside research-owned paths (per-title temporary member copies for the existing readers were written to the session scratchpad and removed
by the probe); no launch; **no writer proposed**.

Machine-readable: [`data/r27-gen7-speech-model.json`](data/r27-gen7-speech-model.json) (38 MB: per tree the input identities, the full
sentence model — every sentence, body, phrase, fixed tag, event reference — the graffiti census, the parameter / enum tables, the
repetition pools, and the check counters). Probe: `local/runs/overnight-research-2026-09-21/probes/r27_gen7_speech_model.py` (≈ 16 s).

**Evidence classes:** *source* = stated by EA's XML; *parsed* = a binary reading verified on every record against the source semantics;
*measured* = a count; *candidate* = a reading no source or pointer confirms; *runtime* = unobserved.

## Plain-language summary

NHL 09 and NHL 10 ship the commentary tables in the form EA's tools wrote them — readable XML inside the archives — and those files spell out
the model that the later binary titles compiled: a **sentence** (named after the speech event that triggers it) is a list of **phrases**; each
phrase names one **sample group** by its "secret name" tag (e.g. `Cup_End_Cup_NHL`) plus zero or more **tags taken from the event's
parameters** (the team, the player, the clock time, the league …), and a **repetition pool**. A separate **graffiti database** lists, for every
tag value, the samples that carry it; the phrase's sample is one that carries all its tags. With that key, the binary sentence files of NHL 11
and 12 (never parsed before) and of NHL 13–Legacy (previously read as "words") decode completely: every one of 9 829–15 745 phrases per title
has exactly one secret-name pair and its event references name exactly the triggering event's parameters, and the graffiti database (now read
as tag → value → tag-count → sample ids) holds the table for **every** phrase on **every** title and references **every** sample of the bank.
The per-body constant word is the secret-name tag id; the "keywords" file of NHL 15 / Legacy is a separate index that cannot be derived from
the graffiti tables. The event ids are EA's `interfaceCrc` (stable across titles for an unchanged event), the enum vocabularies (`league`
= NHL / AHL / CHL / OHL / QMJHL / WHL / GENERIC …) are in the parameter tables of every binary title, and the parameter *names* label every tag.
NHL 07 / 08 use an older speech system (`pbp_eng.{big,csi,cti,evt,idx,nam}`) that this model does not cover.

## 1. The source form (NHL 09 / 10; *source*)

`audio/speech/pbp_eng_*.xml` — loose on Xbox 360 NHL 09 / 10, refpack members of `data0.big` (NHL 09 PS3) / `audioboot.big` (NHL 10 PS3):

| File | Content |
|---|---|
| `sentencedata.xml` | `<Sentence triggerName id priority numPhrases>` → `<Phrase numParameters repetitionPool repetitionId>` → `<TagBuilderFixed TagName="SecretName_SampleBank" TagId TagValueName TagValue/>` + `<TagBuilderEventRef TagName TagId EventIndex/>` |
| `graffitidata.xml` | `<GraffitiDatabase numTables numSamples>` → `<TagTable TagName="<tag>_<value>[_<numtags>]" TagId TagValue [NumTags] numRefs>` → `<DataRef value=sample id/>` (NumTags absent on NHL 09) |
| `speecheventsubscr.xml` | `<EventSystem systemCrc numParameters numEvents>` → `<parameter name id type="Int"/>`, `<enum name id type="EnumBit" numValues>` + `<enumerator name value/>`, `<function name interfaceCrc numParameters>` → `<parameter type name parameterId/>` |
| `repetitionpools.xml` | `<TimedRepetitionPool name id poolSize repeatTime>` / `<ShuffleRepetitionPool …>` / `<UseOnceRepetitionPool …>` (23 pools on NHL 10, 22 on NHL 09) |

Facts read from the source: a phrase = one fixed `SecretName_SampleBank` tag (the sample group, with a human-readable `TagValueName`) +
event-reference tags whose value is the event's parameter at `EventIndex`; `numParameters` counts both; sentence `id` and `priority` (1–4)
exist; `interfaceCrc` is the event id (equal to the `MOIR` table of the same title, 2 751 / 2 751 and 2 096 / 2 096 pairs); a parameter has a
*type* and a *name* (`pID_home` / `pID_away` are of type `player_ID`); enums are bit flags.

## 2. The binary forms (*parsed* with the source semantics; 16 sets)

| Set | sentence file | phrases (one secret pair each) | event refs = event's parameter at the index | secret tag id | graffiti | tables | bank samples referenced by the graffiti | enums / parameters |
|---|---|---:|---|---|---|---:|---|---|
| NHL 09 X360 = PS3 | XML | 10 490 / 10 490 | 7 218 / 7 218 | 2870 | XML (no NumTags) | 9 842 | 45 589 / 45 589 | 33 / 48 |
| NHL 10 X360 = PS3 | XML v3.5 | 13 331 / 13 331 | 8 416 / 8 416 | 2903 | XML v2.1 | 11 133 | 53 366 refs vs bank 53 276 / 53 278 (90 ids referenced but not in the bank) | 36 / 52 |
| NHL 11 X360 = PS3 | `SPC0` v4.2 (4-byte magic) | 14 060 / 14 060 | 8 723 / 8 723 | 2918 | `GRF0` v2.1 (named) | 12 624 | 61 572 / 61 576 (4 unreferenced) | 34 / 50 |
| NHL 12 X360 = PS3 | `SPC0` v4.4 | 15 309 / 15 309 | 9 066 / 9 066 | 2969 | `GRF0` v2.1 (named) | 15 515 | 73 194 / 73 194 | 35 / 58 |
| NHL 13 X360 = PS3 | `SPC0` v5.3 | 14 696 / 14 696 | 8 732 / 8 732 | 257 | `GRF0` v3.3 | 17 392 | 83 453 / 83 453 | 60 / 84 |
| NHL 14 X360 = PS3 | v5.3 | 15 745 / 15 745 | 8 796 / 8 796 | 257 | v3.3 | 18 525 | 89 734 / 89 734 | 61 / 85 |
| NHL 15 X360 = PS3 | v5.3 | 9 829 / 9 829 | 4 019 / 4 033 (14 refs tag `player_ID` for parameters `pID_home` / `pID_away` — type match, §1) | 287 | v3.3 | 8 368 | 45 442 / 45 442 | 64 / 108 |
| Legacy X360 = PS3 | v5.3 | 11 465 / 11 465 | 4 429 / 4 443 (same 14) | 298 | v3.3 | 10 819 | 59 938 / 59 940 (2 unreferenced) | 58 / 101 |

PS3 and Xbox 360 give identical counts on every row (the decoded members of `audioboot.big` on PS3 NHL 12 / 13, loose files elsewhere). The
`FNV-1(name) mod N` slot rule holds on every binary sentence (2 835 / 3 390 / 3 215 / 3 378 / 1 489 / 1 718); every sentence name is an event
of its title (`sentence_has_event` = sentences on all 16 sets); every phrase's graffiti table `(secret tag, value, NumTags = numParameters)`
exists (`fixed_table_exists` = phrases on all 16 sets; on NHL 09, keyed without NumTags).

Layouts (all big-endian):

- **`SPC0` v5.3 phrase** (NHL 13–Legacy): `u8 repetitionId │ u8 0 │ u8 numParameters │ u8 0 │ u8 nwords │ ff ff ff │ nwords × u32 = (TagId, TagValue)
  pairs │ (numParameters − nwords / 2) × event ref `u8 1 │ u8 EventIndex │ u8 EventIndex + 1 │ u8 EventIndex │ u32 TagId │ u32 ffffffff`` (shape
  verified on all 4 033–8 796 refs per title). Body: `u32 id │ u32 100 │ u32 1 │ u32 nphrases │ ptrs` (the `100 / 1` words are constant — not
  the XML `priority`; priority is not located in v5). This supersedes handoff 13 rev. 4's "phrase = type bytes + count × sample-id / keyword
  words": the "words" were (tag, value) pairs and the "constant 287" the tag id.
- **`SPC0` v4.2** (NHL 11, 4-byte magic `SPC0`, slot table at 0x14) / **v4.4** (NHL 12): body `0 │ 0 │ u32 nphrases │ u32 id │ u32 w4 │ ptrs`;
  phrase `u32 0 │ u32 repetitionId │ u32 0 │ [u32 0 on v4.4] │ u32 numParameters │ numParameters × (u32 kind, u32 TagId, u32 value)`, kind 2 =
  fixed (value = TagValue), kind 1 = event ref (value = EventIndex); no other kind occurs.
- **`GRF0` v2.1** (NHL 11 / 12): `GRF0 │ u16 2, u16 1 │ 0 │ 1 │ u32 slots │ 0 │ @0x18 slots × (u32 count, u32 ptr rel. 0x18)` → count × 16-byte
  entries `u32 TagId │ u32 TagValue │ u32 NumTags │ u32 ptr` → `u32 numRefs │ numRefs × u32 sample id │ name` (`<tag>_<value>_<numtags>`, NUL,
  4-aligned). **v3.3** (NHL 13+): 8-byte magic, slots at 0x14, table at 0x1c, same entries, data without the name. The 2026-09-21 handoff 02
  §3 reading (`n │ ptr → n words` with a "leading word") is **superseded**: the third entry word is NumTags and the data begins with numRefs.
- **`EVT0` parameter table** (binary): entry `u32 3 │ id │ 0 │ 0 │ 0 │ name` = Int parameter; `u32 1 │ id │ u32 count │ 0 │ ptr │ name` = enum,
  `ptr → count × ptr → u32 0 │ u32 value │ name` (the XML `<enumerator>`). NHL 11's 4-byte-magic `EVT0` v1.2 is the v1.2 layout with the header
  fields 4 bytes earlier (parsed by rebuilding an 8-byte-magic image, +4 on the header pointer lists).

## 3. What the model gives the developer (developer-ready, read-only; all titles 09–Legacy)

1. **Sentence listing with meaning:** for each sentence (= event name) its phrases in order, each as *sample group* (secret tag value; on
   09–12 with the name from the XML / the graffiti entry name; on 13+ as a number — the names are not shipped) + *parameters* (the event's
   parameter names at the referenced indices) + *repetition pool* (name on 09–10; id elsewhere — 23 pools, ids stable: 1 Generic 15-minute
   timed pool, 12 Recap, 17 Faceoff, 22 Intermission …).
2. **Sample resolution per phrase:** the graffiti table `(secret tag, value, numParameters)` → its sample ids → `SBKR` → `.sbs` offsets. When
   the phrase has event-reference tags, the runtime narrows to the samples that also carry those tags — statically listable as "candidates
   (n) for this group; the event's *team / player / …* selects among them"; the intersection with the parameter-value tables is computable
   once the value is chosen (e.g. a team's audio id).
3. **Team / player name calls (Phase 2.7 / 3.5):** the event-reference tag `team_ID` / `player_ID` takes the event parameter's value, and the
   DB audio ids are those values: `exhibitionteams.audioid` ⊆ graffiti `team_ID` keys 207 / 211 (NHL 15), `playernames.audioid` ⊆ `player_ID`
   keys 1 169 / 1 172 (handoff 02 §4; a Players-phase probe should make it durable across titles). So "which samples say this team's name" =
   graffiti `(team_ID, audioid, *)` — a listing, and a reassignment = a DB edit validated against the key set.
4. **Vocabularies:** parameter names (48–108) and enum values (33–64 enums) per title in the JSON (`param_names`, `events.enums`), e.g. `league`
   NHL 1 / AHL 2 / CHL 4 / OHL 8 / QMJHL 16 / WHL 32 (+ GENERIC 64 on Legacy), `game_mode` PLAY_NOW 1 / SHOOTOUT_MODE 2 / DYNASTY 4 /
   TOURNAMENT 8 / HUT 16. This answers the "`league` parameter values" item of Q-R20-1.
5. **Event ids:** `interfaceCrc`, 15-bit, stable per name + signature (`CUP_END_CUP_NHL` = 29 243, `CUP_END_CUP_AHL` = 19 835, `GOAL_SCORED_GENERIC`
   = 27 202 on NHL 10 … Legacy; `FACEOFF_WON_CRITICAL` 30 609 → 13 390 between NHL 10 and 11 = signature change). The hash was not identified
   (CRC-16 families, FNV, djb2, sdbm, CRC-32 folds over name / name + parameters: no match) — the MOIR table remains the name ↔ id source.

## 4. Candidate, not decoded, runtime

- *Candidate:* the exact runtime selection among a table's samples (repetition pools, "NumTags" as the count of tags a sample carries, the
  intersection rule for multi-tag phrases — all stated by the source form's vocabulary, not observed); the v5 body words `100 / 1`.
- *Not decoded:* the `interfaceCrc` hash; the `.gas` bytecode; `pbp_eng_acmedata` / `acmevents` (ACME module); `pbp_eng_starplayers.bin`
  (09 / 10); the `REP0` binary pool records beyond the count; the `KWD0` keyword index's producer (it is **not** derivable from the graffiti
  tables: 0 / 2 121 keywords equal any single-type or all-type union of tables with that value — it stays the *mention* index of handoff 07
  (2026-09-20 §3), used by the team / arena `audioid` joins there; both joins hold).
- *Runtime:* everything about playback; unchanged.

## 5. Corrections carried

- 2026-09-20 handoff 13 rev. 4 §2 / handoff 14 "phrase words = keyword ids (NHL 15 / Legacy) / unresolved identifiers (13 / 14)" and the
  2026-09-21 handoff 02 §3 "resolution states 1 / 2 / 3" → **superseded**: on every title the phrase resolves through its secret-name graffiti
  table; the KWD0 coincidence on NHL 15 / Legacy (2 061 / 2 976 values also being keyword ids) is a shared id space, not a lookup path.
  Handoff 02's source-identity findings (§1–§2) and its DB-audio-id lead (§4) stand.
- 2026-09-21 handoff 02 §3 GRF0 entry reading (`n │ ptr → n words`, "leading word") → superseded by §2 above (NumTags + numRefs).
- The 2026-09-20 R20 probe's NHL 11 / 12 "unparsed" status → parsed here (v4.2 / v4.4).

## 6. Reproduce

```
python local/runs/overnight-research-2026-09-21/probes/r27_gen7_speech_model.py docs/research/overnight-2026-09-21/data/r27-gen7-speech-model.json   (≈ 16 s)
```
Inputs: `audio/speech/pbp_eng_{sentencedata,graffitidata,repetitionpools,speecheventsubscr}.{xml,bin}`, `pbp_eng_speecheventdef.csi`,
`pbp_eng_samples.sbr` — loose or from `audioboot.big` / `audio.big` / `data0.big` (BIG4 refpack on 09 / 10, EB chunklzx / chunkzip on 11+)
of the 16 tree roots in the probe; every input's source, codec, size and SHA-256 is in the JSON `inputs`. Executed: 6 development runs
(layout discovery on NHL 10 XML / NHL 11 / 13 / 15 hex dumps), 1 final run, the hash search, the KWD0-vs-graffiti tests. Not executed:
launches, writes, Studio binaries.
