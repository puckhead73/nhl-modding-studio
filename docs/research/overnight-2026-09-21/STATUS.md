# Research lane status — overnight 2026-09-21

Campaign brief: the user's 21 September overnight research instruction (Priority 0: apply
[research-r22-r23-review-2026-09-21.md](../../validation/research-r22-r23-review-2026-09-21.md) and finish the audio contract → Priority 1
League blockers 1–5 → Teams → Players → Equipment → Miscellaneous). Codex coordinates/reviews; the developer lane owns application code.
This lane owns `docs/research/overnight-2026-09-21/**`, `local/runs/overnight-research-2026-09-21/**` and
`local/coordination/overnight-2026-09-21/claims/research-*.json`, plus concise supersession notes in its own earlier campaigns' files.
Baseline: branch `feat/phase5-asset-tooling`, HEAD `8d60a6d`; `docs/research/` untracked; developer lane: campaign of 2026-09-20 closed
(receipt 10), R1 / R2 corrections reviewed, **OVERNIGHT-DEV-20260920-R3** (colour acknowledgement ordering, frontend only) dispatched —
no developer claim under `local/coordination/overnight-2026-09-21/`; nothing of the developer's is touched here.

Startup read (2026-09-21, ~03:40): repository instructions, the parity roadmap, the working agreement, the Phase 1 / Phase 2 matrices, the
R22/R23 review, the 2026-09-20 atlas / STATUS / handoffs 13 rev. 4 and 14 / claims, the developer STATUS, receipt 10, the R2 / R3 dispatch,
the audio-framework module XMLs (`*_moduledef.xml`, new evidence: `ContextModule maxNumValidContexts / numPassives / maxNumWindows`,
`Sample Not Found → Csis Failed`).

## R24 — Priority 0 C / D, `CTX0` decoded by pointer walk (2026-09-21, ~03:50–05:15) — handoff [01](01-ctx0-context-records-decoded.md)

Plain-language: the context file is parsed by following its pointers; every record has a trigger, conditions with bytecode indices and
names, and one or two items (speak an event, or raise a trigger) whose parameter lists match the events' declared parameters exactly. The
old scan is reproduced item-for-item on the speech items and is shown to have missed the context-raising items and mis-named the `+` names.
PS3 = Xbox 360 on every decoded field. w8 turned out to be the trigger-name offset, so the review's step 4 census was unnecessary.

Changed paths (new): handoff 01, `data/r24-ctx0-records.json` (35 MB), probe `r24_ctx0_records.py`, claim `research-r24-ctx0-structure`.
Executed: the probe (9 development runs + 1 final), hex dumps of records 0 / 1 / 10 / 15 / 52 / 65 / 68 (NHL 15), 648 / 1185 / 1813 / 1814
(Legacy), record 1 (NHL 15 X360), the crowd-file check. Not executed: launches, writes, Studio binaries, `.gas` decoding.

## R25 — Priority 0 B, NHL 13 / 14 keyword resolution + source identities (~05:15–06:00) — handoff [02](02-nhl13-14-keyword-resolution-and-source-identity.md)

Plain-language: no keyword table exists on NHL 13 / 14 (every archive of both platforms read), so the review's extraction step has a
negative result; the graffiti index is keyed by event parameter ids and by every sentence word; the per-body constant is its word-type tag;
the loose gen7 speech files equal the archive members everywhere except PS3 NHL 13 (archive-only) and the Xbox NHL 14 stream, whose loose
copy is truncated. Three resolution states replace the single keyword chain. A Players / Teams lead (DB audio ids ⊆ graffiti keys) is recorded.

Changed paths (new): handoff 02, `data/r25-gen7-speech-sources.json`, probe `r25_gen7_speech_sources.py`, claim
`research-r25-gen7-speech-sources`. Executed: r25 (3 runs), the member-vs-loose comparison, bank / stream size checks, GRF0 type census,
the anchor DB audio-id test (scratch). Temporary: `%TEMP%\r25_*.bin` written and removed by the probe. Not executed: launches, writes.

## R26 — Priority 0 A, contract revision + supersession (~06:00–06:20) — handoff [03](03-gen7-commentary-read-only-contract-rev2.md)

Handoff 14 (2026-09-20) superseded by rev. 2; supersession notes placed in 2026-09-20 handoffs 13 / 14, the r20 probe docstring and the
2026-09-20 atlas; new atlas, this file, [DEVELOPER-QUEUE.md](DEVELOPER-QUEUE.md), [USER-CARDS.md](USER-CARDS.md).

**Priority 0 checkpoint published (06:20).** Continuing to Priority 1 without a review stop, per the brief.

## R27 — Priority 1.1, the gen7 speech model from EA's XML source form (~06:25–07:45) — handoff [04](04-gen7-speech-model-from-ea-source-xml.md)

Plain-language: NHL 09 / 10 carry the commentary tables as readable XML, which spells out the model the later binaries compiled — a sentence
is a list of phrases, each naming one sample group plus tags taken from the event's parameters; the graffiti database maps tag values to
samples. With that key the NHL 11 / 12 sentence files parse for the first time, the NHL 13–Legacy "words" become (tag, value) pairs, and every
phrase on every title resolves to a graffiti table that exists; the graffiti references every bank sample; the enum vocabularies (league,
game mode …) and parameter names are decoded on every binary title; event ids are EA's `interfaceCrc`. The keyword file stays a separate
mention index. Contract 03 revised (rev. 2a); handoff 02 §3 superseded by note.

Changed paths (new): handoff 04, `data/r27-gen7-speech-model.json` (38 MB), probe `r27_gen7_speech_model.py`, claim
`research-r27-gen7-speech-model`; edited: handoffs 02 (note), 03 (rev. 2a), atlas, DEVELOPER-QUEUE, this file. Executed: r27 (7 runs),
XML / hex dumps, the `interfaceCrc` hash search (negative), KWD0-vs-graffiti tests (negative). Temporary: `r27_*` / `r27-test.json` /
`nhl10_pbp_*.xml` in the scratchpad (removed at the next boundary). Not executed: launches, writes.

## R28 / R29 — Priority 1.1, gen7 banks, codecs, decoder feasibility (~07:50–08:35) — handoff [05](05-gen7-speech-banks-codecs-and-decoder-feasibility.md)

Plain-language: every gen7 commentary bank on both consoles is measured (aliases, packing, codec per title); ~half the sample ids are
aliases from NHL 10 on, samples are packed with 0–12 bytes of slack, and the codec differs by title and platform (XMA, an unidentified
`0x09`, EALayer3). The locally installed ffmpeg decodes none of the five classes as shipped; EA-Opus (PS4 18–21) is the first realistic
preview target because its packet framing is verified. No encoder exists locally for any class.

Changed paths (new): handoff 05, `data/r28-gen7-bank-streams.json`, `data/r29-codec-feasibility.json`, probes `r28_gen7_bank_streams.py`,
`r29_codec_feasibility.py`, claim `research-r28-r29-gen7-banks-codecs`; atlas, DEVELOPER-QUEUE, this file. Executed: r28 (2 runs), r29 (5
runs incl. development). Temporary: per-class sample copies + 2 partial WAVs + `r28-test.json` + `dec_try.py` in the scratchpad — removed.
Not executed: launches, writes, encoders.

## R30 — Priority 1.2, chunklzx store route (~08:40–09:05) — handoff [06](06-chunklzx-store-route.md)

Plain-language: an encoder that writes any file as a valid `chunklzx` member using LZX's uncompressed block type — no compressor needed,
container identity preserved — round-trips all 50 shipped members and 16 synthetic sizes through the validated research decoder. It
makes gen7 Xbox 360 rink / ice / logo write-back a static possibility at the cost of size; whether the console's decoder accepts
uncompressed blocks is card R30-LZX-1.

Changed paths (new): handoff 06, `data/r30-chunklzx-store-roundtrip.json`, probes `chunklzx_store.py`, `r30_chunklzx_store_roundtrip.py`,
claim `research-r30-chunklzx-store`; atlas, DEVELOPER-QUEUE, USER-CARDS, this file. Executed: r30 (1 run). Not executed: archive writes,
launches. **Background:** `r31_gnf_header_census.py` (PS4 NHL 21 + NHL 16 GNF headers, ≈ 2 h on the slow H: drive) started 09:10 —
PID recorded in the scratchpad `r31.log`; it is this lane's only process.

## R35 — Phase 2.7 / 3.5 (consumed early), gen7 name-audio bindings (~10:20–10:45) — handoff [07](07-gen7-name-audio-bindings.md)

Plain-language: with the speech model in hand, one probe joins every database audio-id column of the PS3 anchors 09–Legacy against the
commentary and announcer tag tables: play-by-play player names are keyed by `playernames.audioid`, announcer names by
`exhibitionplayerbiotable.audioid`, team names by `exhibitionteams.audioid`; the arena ids' tag stays open. Listing is static; reassignment
is a DB cell edit against the key set (runtime card R35-AUD-1).

Changed paths (new): handoff 07, `data/r35-gen7-audio-id-bindings.json`, probe `r35_gen7_audio_id_bindings.py`, claim
`research-r35-gen7-audio-id-bindings`; atlas, DEVELOPER-QUEUE, USER-CARDS, this file. Executed: r35 (1 run) + the NHL 09 announcer XML
check. **Background at this point:** r31 (GNF census, H:), r32 (RW4xb2 census, X:), r33 (RW4ps3 census, X:), r34 (fonts / encoding) —
all this lane's, logs in the scratchpad.

## R36 — Priority 1.5, tournament templates + group ids (~10:50–11:15) — handoff [08](08-tournament-templates-and-group-ids.md)

Plain-language: the tournament schedule tables are templates whose slot ids are resolved by standings at runtime, so there is no
participant-slot binding to edit; the PS4 group ids that P1-C could only show as numbers are the EBEL / Allsvenskan / CHL-guest /
Spengler-group / Alumni-All-Time / Pro-Am pseudo-leagues, nameable by rule. The domestic CHL participant selector is not in the DB.

Changed paths (new): handoff 08, `data/r36-tournament-templates-and-groups.json`, probe `r36_tournament_templates_and_groups.py`, claim
`research-r36-tournament-groups`; atlas, DEVELOPER-QUEUE, this file. Executed: r36 (1 run) + 2 scratch scripts (removed).

## R34 — Priority 1.4, dictionary encoding + font glyph coverage (background 09:20–11:20; written up ~11:40) — handoff [09](09-dictionary-encoding-and-font-glyph-coverage.md)

Plain-language: every localization dictionary is UTF-8; the shipped TrueType fonts cover the Latin / Cyrillic / Greek characters the
locales use, missing only the CJK language names, a few Polish / Hungarian capitals in the language-neutral gen7 file and some format
characters — a static glyph gate the rebuild preview can apply. Changed paths (new): handoff 09, `data/r34-font-glyph-coverage.json`,
probe `r34_font_glyph_coverage.py`, claim `research-r34-fonts-encoding`; atlas, DEVELOPER-QUEUE, this file.

## R37 — Priority 2, team re-identification contract (~11:45–12:00) — handoff [10](10-team-reidentification-contract.md)

Policy proposal only (no new measurement): refuse index renumbering, allow artid swaps and pre-placed artid moves, refuse on PS4 until a
vault writer exists. Changed paths: handoff 10, claim `research-r37-team-reid-contract`, atlas, DEVELOPER-QUEUE, this file.

## R38 — Priority 4 (gen7 counterpart of PS4 handoff 09), equipment file grammar (~12:05–12:40) — handoff [12](12-gen7-equipment-file-grammar.md)

Tokenised every `rendering/<family>` file name on 12 trees (11–Legacy, both platforms), joined the number sets with the anchor equipment
columns: 100 % containment on 13 / 15 / Legacy for every equipment id column; jersey / pant / sock keyed by team art id; NHL 11 grammar
differs (Q-R38-1). Changed paths: handoff 12, `r38_gen7_equipment_file_grammar.py`, `r38-gen7-equipment-file-grammar.json`, claim
`research-r38-gen7-equipment`, atlas, DEVELOPER-QUEUE, this file.

## R32 / R33 — Priority 1.3, gen7 texture descriptors (background 09:30–12:30; slot / mip rule census to 13:20; written up ~13:35) — handoff [11](11-gen7-texture-descriptor-rules.md)

Censused every loose `RW4ps3` (9 PS3 trees, 102 669) and `RW4xb2` (10 X360 trees, ≈ 190 000) descriptor. PS3: two dimension words +
linear chain. Xbox: the Xenos fetch-constant fields; the mip-address rule (`tiled base / 4096`, cubes 6×, single-mip 0) and the slot
rule (`DW8` upper = 2 × index) verified on Legacy / 15 / 11 / 09 by a scratch census (`r32b` data). Corrections during the work: 6-bit
format ids, 32-element tile alignment, `DW13` byte layout, and an earlier "mip − base" formulation that only held for slot 0 — the
handoff carries the corrected rule; the r32 JSON's `mip_addr_eq_base_plus_tiled_base` counter is superseded by `r32b`. Changed paths:
handoff 11, probes `r32_…` / `r33_…`, data `r32` / `r32b` / `r33`, claim `research-r32-r33-gen7-descriptors`, atlas (row 11, Q-R32-1),
DEVELOPER-QUEUE row 11, card R32-TEX-1, this file.

## R39 — Priority 1.5 (open since 09-18), NHL 07 competition membership + schedules (~13:40–13:55) — handoff [13](13-nhl07-competition-membership-and-schedules.md)

Pair-class analysis on the gen6 anchors with `stockteamtable` groups (the step the 2026-09-20 handoff 06 §3 named): membership lives on
the team rows, the thin selector is an identity map, schedule ids are league ranks, the NHL quota is NHL 08's. Changed paths: handoff 13,
probe `r39_…`, data `r39-…`, claim `research-r39-nhl07-competition`, atlas row 13, DEVELOPER-QUEUE row 12, this file.

## R40 — Phase 3.5 PS4, player-name audio binding (~14:00–14:35) — handoff [14](14-ps4-player-name-audio-binding-negative-results.md) (negative)

Tested `audioid` as keyword / commentary sample / announcer sample, the `surnameid → playernames` route and constant offsets on NHL 15–21;
all excluded by density checks (one intermediate reading withdrawn before consumption, §3 of the handoff). Changed paths: handoff 14,
probe `r40_…`, data `r40-…`, claim `research-r40-ps4-player-name-audio`, atlas row 14 + Q-R40-1, DEVELOPER-QUEUE row 14, card
R40-AUD-1, this file.

## R31 — Priority 1.3, PS4 GNF header census (background 09:20–14:10; written up ~14:40) — handoff [15](15-ps4-gnf-header-rules.md)

NHL 16 + NHL 21 trees (216 000 files): size / pitch / mip rules verified, pitch rounding rule found on the naive-rule failures (6 000-file
sample check), cube and BC6 exceptions listed. Changed paths: handoff 15, probe `r31_…`, data `r31-…`, claim `research-r31-ps4-gnf`,
atlas row 15 + Q-R31-1, DEVELOPER-QUEUE row 13, card R31-GNF-1, this file.

## R38 addendum — NHL 11 equipment grammar (~14:45–14:55)

Scratch census of the NHL 11 X360 / PS3 `rendering/<family>` trees against the NHL 11 anchor: texture-keyed families, 100 % containment on
every equipment column; handoff 12 §2 addendum + §4 updated, Q-R38-1 grammar part closed in the atlas.

## R41 — Q-R24-2, `CTX0` v3.1 (NHL 11 / 12) decoded (~15:00–15:35) — handoff [16](16-ctx0-v31-nhl11-12-context-records.md)

Pointer-walk parser written and checked on X360 NHL 11 / 12 and PS3 NHL 11 (`pbp_eng` + `crowds`): all triggers and speech items resolve
by id and inline name, conditions well-formed, PS3 = X360 2 833 / 2 833. Corrections during the work: condition names follow the inline
arg pointers (first read at the wrong offset), PS3 item block has two extra zero words, NHL 12's MOIR header namespace ≠ item namespace
(ids / names agree), kind-0 params outside the trigger are immediates. Changed paths: handoff 16, probe `r41_…`, data `r41-…` (5 MB, 400
records per set), claim `research-r41-ctx0-v31`, contract handoff 03 (rev. 2b note), atlas row 16 + Q-R24-2 resolved, DEVELOPER-QUEUE
row 15, this file.

## Cleanup ledger (closed at the receipt)

- Scratchpad (`H:\TEMP\claude\…\scratchpad`): every file removed at closeout (`dump_ctx.py`, `show24.py`, `slotrule.py`, the `r31 / r31b /
  r32 / r33 / r34 / mipbase / slotrule` logs; the earlier `dump_hdr.py`, `pools.py`, `ctxparse.py`, `ctxev.py`, `patch24*.py`, `cmp_member.py`,
  `r24-test.json`, `r24-all.json` were removed at the mid-session boundary; `receipt.md` — this text's staging file — removed last).
- `local/runs/overnight-research-2026-09-21/`: `probes/` only (18 probes + `chunklzx_store.py`); the r29 by-product `ps4-nhl15-pbp.bin.stripped.wav`
  removed; `__pycache__` removed here and under the 2026-09-18 / 19 / 20 probe directories (imports of their readers); the 2026-09-18
  `pylib/` vendored caches untouched.
- Background processes: the four-title GNF census (`--titles nhl15,nhl17,nhl18,nhl19`, started ~14:50 UTC) was **stopped at closeout** before
  finishing its first title; its partial output file was deleted; NHL 15 / 17 / 18 / 19 remain "not censused" as handoff 15 §4 says. No
  process of this lane remains (the `python.exe` instances left are the Claude Desktop Blender extension's).
- Not written: anything under `X:`, `H:`, `E:\nhl-studio-fixtures`, `anchors/`, application code, defs, bindings, tests, matrices,
  developer files. No commits, pushes, launches; no extractions retained.

## Closing receipt (2026-09-21, ~15:50 UTC / 07:50 local)

### Plain summary

Sixteen handoffs tonight. The gen7 commentary question is now answered end to end at the data level — context records (NHL 11–Legacy,
two file versions), the speech model from EA's own XML source form (every phrase = one sample group + event-parameter tags → graffiti
table → samples, verified on all 16 sets), the name-audio bindings for players / teams, the bank / codec facts and what a decoder can and
cannot do today — and written up as the read-only contract rev. 2b. Around it: a codec-preserving store route for Xbox 360 `chunklzx`
members (round-trips verified), the exact descriptor rules for resizing PS3 / Xbox 360 / PS4 textures (293 000 + 216 000 descriptors,
with a correction to Studio's Xbox slot formula), the tournament / group-id semantics, UTF-8 + shipped-font glyph coverage for the
dictionary rebuild, a team re-identification contract proposal (already consumed by the developer's P3), the gen7 equipment file
grammar (NHL 11 included), NHL 07's competition structure, and one honest negative: the PS4 player-name audio binding stays open after
five hypotheses were excluded. Every result is static and read-only; nothing is implemented, accepted or game-verified by this lane.

### Completed handoffs (all `docs/research/overnight-2026-09-21/`; claims under `local/coordination/overnight-2026-09-21/claims/research-*.json`)

| # | Topic | Claim |
|---|---|---|
| 01 | `CTX0` v4.5 records decoded (13–Legacy; 0 refusals; PS3 = X360) | r24-ctx0-structure |
| 02 | NHL 13 / 14 keyword resolution (no table exists) + source identity (loose = archive; Xbox NHL 14 stream truncated) | r25-gen7-speech-sources |
| 03 | Read-only commentary contract rev. 2 / 2a / **2b** | (covered by r24 / r27 / r41) |
| 04 | Gen7 speech model from EA's XML source form; 16 sets verified | r27-gen7-speech-model |
| 05 | Banks, codecs, decoder feasibility (32 banks) | r28-r29-gen7-banks-codecs |
| 06 | `chunklzx` store route (50 / 50 + 16 / 16 round trips) | r30-chunklzx-store |
| 07 | Gen7 name-audio bindings (players / teams) | r35-gen7-audio-id-bindings |
| 08 | Tournament templates + PS4 group ids 62–64 / 75–78 | r36-tournament-groups |
| 09 | Dictionary encoding (UTF-8) + font glyph coverage (175 files, 27 font trees) | r34-fonts-encoding |
| 10 | Team `index` / `artid` refuse-or-cascade contract proposal | r37-team-reid-contract |
| 11 | RW4ps3 / RW4xb2 descriptor rules (`DW8` slot correction, `DW12` mip rule) | r32-r33-gen7-descriptors |
| 12 | Gen7 equipment file grammar (13 / 15 / Legacy + NHL 11 addendum) | r38-gen7-equipment |
| 13 | NHL 07 competition membership + schedules | r39-nhl07-competition |
| 14 | PS4 player-name audio binding — **negative results** (5 hypotheses + an eboot value scan) | r40-ps4-player-name-audio |
| 15 | PS4 GNF header rules (216 000 files) | r31-ps4-gnf |
| 16 | `CTX0` v3.1 (NHL 11 / 12) records decoded; PS3 = X360 | r41-ctx0-v31 |

Data: 17 JSON files (86 MB; `r24` 35 MB, `r27` 38 MB, `r41` 5 MB). Probes: 18 + the `chunklzx_store.py` encoder. Claims: 14.

### Prioritised developer unblockers (DEVELOPER-QUEUE rows; nothing here is accepted)

1. Commentary catalogue (rows 1, 1a, 2–5, 15): contract rev. 2b — the developer's P2-E `tdb-speech` crate already reproduces the model
   on 10 trees; row 15 adds the v3.1 parser rules for NHL 11 / 12; row 2 the source / truncation refusals; row 5 the sample-listing facts.
2. Texture resize (rows 11, 13): PS3 two-word rule, Xbox 360 fetch-constant rules with the `DW8` slot correction, PS4 GNF rules — lifts the
   C6 refusals for the common 2D block-format cases with explicit refusals for cubes / tiny / BC6 / NHL 07 / 08.
3. `chunklzx` store route (row 6) for X360 `rendering/*` write-back (the developer's P2-A promoted the decoder; the store encoder is the
   next piece; card R30-LZX-1 gates runtime).
4. Leagues: tournament templates + derived group labels (row 7), NHL 07 membership (row 12), UTF-8 + static glyph gate (row 8).
5. Teams: re-identification contract (row 9; P3 consumed it). Equipment: gen7 file-grammar catalogue (row 10). Players: gen7 name-audio
   listing / reassignment (row 1a); PS4 player names explicitly **not** bindable (row 14).

### Remaining blockers (by phase / target)

- 1.5 / 2.7 / 3.5 gen7: bytecode semantics and context selection (Q-R20-1 / Q-R24-1); v3.1 script-id → `.gas` binding; codec `0x09`
  identity, EALayer3 / EA-XMA / EA-MP3 decoders and every encoder (Q-R28-1); ~~NHL 13 word → sample resolution beyond the graffiti (bank
  ids 77 269+)~~ (**correction R3, 2026-09-21 pm:** obsolete — handoff 04 resolves every NHL 13 phrase through its graffiti table, 14 696 /
  14 696, and the graffiti references all 83 453 bank ids; the residual is not NHL-13-specific: the runtime take selection *within* a
  table (repetition pools / multi-tag intersection) on every title, atlas Q-R25-1); arena audio-id tag (Q-R35-1); NHL 07 / 08 speech
  system (older form, not covered).
- 3.5 PS4: player-name binding (Q-R40-1 — next: a disassembly-level search or the `0TVE` parameter typing; card R40-AUD-1 after a team
  reassign ships); the PS4 phrase-item tag namespace.
- 1.3 / 2.5 textures: `RW4xb2` `DW8` exceptions in multi-slot containers (~10 %, Q-R32-1); NHL 07 mip placement; GNF BC6H / `d5` / `d6`
  / `USER` (Q-R31-1); NHL 15 / 17 / 18 / 19 GNF trees not censused; runtime acceptance of any resize (cards R32-TEX-1, R31-GNF-1).
- 1.2 / 1.4 packaging: `chunklzx` loader acceptance of uncompressed blocks (card R30-LZX-1); a real LZX compressor (not attempted).
- 1.2 competition: CHL domestic participants / WC rosters selector (Q-R36-1); runtime honouring of edited template rows; NHL 07 league /
  division dictionary keys.
- 1.3 localization: system-font fallback for the 9 CJK characters; PS3 08–10 / 12 fonts (in-archive); the runtime glyph card stays.
- 2.1 teams: cascade contract acceptance (Codex); PS4 art-id moves need a vault writer (Q-R7-1).
- 4.x equipment: `n2` / `n3` variant semantics, gear mesh containers, required texture kinds, unregistered-id runtime acceptance (Q-R38-1
  remainder).

### Deferred user review cards (USER-CARDS.md; none requested now)

- Xbox 360 NHL 14: R25-SRC-1 (truncated loose stream — which file the game plays).
- PS3 / Xbox 360 NHL 15: R24-CTX-1 (context listing vs. what is heard; only if a listing is built).
- Xbox 360 NHL Legacy: R30-LZX-1 (uncompressed-LZX member acceptance; after the EB append route places a member), R32-TEX-1 (resized
  `RW4xb2` texture; after a descriptor-regenerating resize exists).
- PS3 NHL 15: R35-AUD-1 (announcer name reassignment; after a DB edit path for the audio-id cells).
- PS4 NHL 21: R31-GNF-1 (resized GNF; after the writer), R40-AUD-1 (does the commentary consume the player `audioid`; listening review
  after PS4 team-name reassignment ships).

### Cleanup and retained dependencies

Retained: the 16 handoffs + atlas / STATUS / queue / cards, 17 data files, 19 probe files, 14 research claims, supersession notes in the
2026-09-20 handoffs 13 / 14, atlas and the `r20` docstring. Dependencies of the probes: the 2026-09-18 readers (`tdb.py`, `schema_dump.py`,
`eb_list.py`, `big4_list.py`, `refpack.py`, `chunklzx.py`), the 2026-09-20 readers (`r14_audio_keywords.py`, `r20_gen7_speech_events.py`,
`snd10_sbr.py`, `pbp_sentences.py`), the installs on `X:` / `H:` (read only), the extracted PS4 trees, the private fixture root, the
anchors, local ffmpeg 8.1 (decode attempts only). Cleanup as the ledger above; nothing of another lane touched; no background jobs.

### Exact next task

Gen7 commentary, static and read-only, in this order: (1) **v3.1 script-id → `.gas` binding** — census the 1 525 / 1 758 condition ids
against the `.gas` blob headers of NHL 11 / 12 (the blob header word that carries an id, if any; compare with the v4.5 1 : 1 rule of
handoff 01 §2) so the NHL 11 / 12 listing can show the script like 13+; (2) the `w0 / w1 / w2` time-like words across both versions
(a census against the `ContextModule maxNumWindows / numPassives` XML attributes and the `crowds` siblings); (3) only then the PS4
player-name binding by the `0TVE` parameter typing of the goal / shot events (handoff 14 §4 item 2). The `.gas` instruction set stays out
of scope. Nothing here requires a launch, a write or the user.

## Corrections applied by the continuation campaign (2026-09-21 pm, 16:58–17:03 MDT)

The coordinator review [overnight-research-2026-09-21-review-r1.md](../../validation/overnight-research-2026-09-21-review-r1.md) R1–R3
was applied in place with dated notes (superseded text preserved): handoff 11 (`DW8` bounded to a majority observation + preserve-only
contract; NHL 07 `DW12` figure marked unreliable; r32b restored as a runnable probe), handoff 15 (coverage classes separated; NHL 16 / 21
only; NHL 20 explicitly not censused; pitch sample replaced by a full-population per-title probe; mipped-cube ratio), handoff 14 (direct-join
scope; "not consumed" not claimed), USER-CARDS (three cards superseded by observation-only restatements), DEVELOPER-QUEUE rows 1 / 11 /
13, this file's blocker list (NHL 13 residual), and the atlas (rows 11 / 15, Q-R32-1 / Q-R31-1). The continuation lives in
[overnight-2026-09-21-pm](../overnight-2026-09-21-pm/STATUS.md).
