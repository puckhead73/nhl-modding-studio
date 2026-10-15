# Research lane status — overnight 2026-09-20

Campaign brief: the user's 20 September overnight research instruction (Priority 0 R7 contract → Priority 1 League blockers A–E →
Teams → Players → Equipment → Miscellaneous). Codex coordinates/reviews; the developer lane owns application code. This lane owns
`docs/research/overnight-2026-09-20/**`, `local/runs/overnight-research-2026-09-20/**` and
`local/coordination/overnight-2026-09-20/claims/research-*.json` only. Baseline: branch `feat/phase5-asset-tooling`, HEAD `8d60a6d`;
`docs/research/` untracked; developer lane closed at R1 corrections (its STATUS: "Active package — none"); no active developer claim.

Startup read (2026-09-20): repository instructions, the parity roadmap, the working agreement, the Phase 1 coverage matrix, both 19 September
reviews, the 19 September atlas/STATUS/handoffs 01–07, the 18 September atlas, the developer STATUS/claims. Latest corrections in force:
handoffs 02–06 rev. 2, handoff 01 rev. 2, handoff 22 rev. 3; NHL21-E1 accepted and closed (not reopened).

## R8 — Priority 0, the R7 evidence contract (2026-09-20, ~00:05–00:40) — handoff [01](01-ps4-jersey-triple-contract-and-r7-reconciliation.md)

### Plain-language summary

The three-number jersey join that yesterday's handoff 07 reported from deleted scripts is now produced by a durable probe on all seven
PS4 titles, with every unmatched and every doubly-matched triple listed and the numbers reconciled (handoff 07's figures reproduce
exactly; its three unrun titles are filled). The two-pack ambiguity is resolved at the data level: the packs differ in one typed on/off
attribute whose name resolves to `IsTopScorer`; the database does not carry that choice. The packaging corrections from the 19 September
review are restated for the developer unchanged.

### Changed paths

New: `00-ATLAS.md`, `STATUS.md`, `01-ps4-jersey-triple-contract-and-r7-reconciliation.md`, `data/r8-jersey-triple-joins.json` (16.7 MB),
`local/runs/overnight-research-2026-09-20/probes/r8_jersey_triples.py`, claim `research-r8-jersey-triple-contract` (`ready`).
Not changed: application code, defs, bindings, tests, the coverage matrix, developer files, installs, anchors, the 2026-09-19 handoffs
(handoff 07 is reconciled here, not edited). No commits or pushes.

### Checks actually executed (read-only)

`r8_jersey_triples.py` over the seven extracted `attrib/` trees + six anchors + the real NHL 20 install DB (≈ 4 min); ad-hoc python over
the JSON: pair-pattern census, `variantleadingscorer == variant` on every non-zero stock row (5 titles), residual breakdowns, the
`IsTopScorer` naming check (the Swiss pairs carry no `topscorer` token — the flag is the discriminator, the wording was corrected before
publication), the ≈ 25 000-candidate `lookup2` name search (`IsTopScorer` only). **Not executed:** launches, writes, any Studio binary.

### Limitations

Static correspondence only. NHL 19 Malmö pair unresolved; Int32 `bb93351e` opaque; runtime selection unobserved.

### Cleanup

Session scratchpad: `r8-test.json`, `r8-run.log` — deleted at the next checkpoint. No extracts, no processes.

### Next (done — R9 below)

Priority 1A — league audio.

## R9 — Priority 1A, SND10 sound banks and the league cue chain (~00:45–05:10) — handoff [02](02-snd10-audio-banks-and-league-cue-chain.md)

### Plain-language summary

The sound-bank index that blocked every audio cell is decoded: PS4 banks (`SBle`, all 418 in the extracted trees), the older speech/music
banks (`SBKR`, PS3 and Xbox Legacy) and the older sfx/horn banks (`ABKC`) now list their sounds, offsets, durations and codec families
with the offsets validated against the sample headers (36 780 / 36 780 PS4, 3 343 / 3 344 gen7). On NHL 21 a named commentary line
resolves to its exact samples (4 953 / 4 953 phrases), so the league-audio question is no longer "where is the sound" but "how to
encode a replacement" (EA-MP3 / Opus-shaped on PS4, EALayer3 v1 / XMA on gen7 — no encoder). Gen7 keeps one undecoded link
(event → sentence subscription) and the DB horn id → bank sound id mapping is still open.

### Changed paths

New: `02-snd10-audio-banks-and-league-cue-chain.md`, `data/r9-snd10-bank-census.json` (2.0 MB), `data/r9-nhl21-ps4-pbp-sentences.json`
(6.4 MB), `data/r9-nhl15-ps3-pbp-sentences.json` (0.5 MB), probes `snd10_sbr.py`, `snd10_census.py`, `pbp_sentences.py`, claim
`research-r9-snd10-audio` (`ready`); atlas row 02 + Q-R9-*. Not changed: application code, defs, bindings, matrix, developer files,
installs, anchors. No commits or pushes.

### Checks actually executed (read-only)

Census over 418 PS4 + 874 PS3 banks (≈ 6 min, run three times while the mixed-bank rule and the codec labels were corrected; final data
published); NHL 21 sentence parse; PS3 NHL 15 gen7 parse; scratchpad checks listed in handoff 02 §9 (storage-mode hypotheses, PBP
id-space test, DSET-key search, MPEG-sync test, Xbox Legacy member extraction). **Not executed:** launches, writes, Studio binaries,
audio decode/playback.

### Limitations

Codec labels inferred from framing (MPEG sync / Opus-shaped packets), `0x09` unidentified; gen7 phrase word `X` in the bank for 2 106 /
3 812 phrases only; `ABKC` bank tree and 12 large `.abk` not decoded; NHL 08–10 and PS3 12/13 in-archive banks untested.

### Cleanup

Scratchpad: `pbp21.json`, `pbp-ps3-15.json`, `r9-ps3.json`, `r9-*.log`, the two extracted Xbox Legacy `.sbr` copies — deleted. No processes.

### Next

Priority 1B — changed-length packaging: a source-bound placement oracle (exact patched words / hashes for fit and growth cases per
chain) over the BIG4 + `.bh`, EB and inner BIGF layers, plus the NHL 07 embedded-index cascade and the unknown-field inventory.

## R10 — Priority 1B, changed-length placement oracle (~05:15–06:20) — handoff [03](03-changed-length-placement-oracle.md)

### Plain-language summary

An independent test oracle for the gen7 changed-length packaging the developer has not built yet: the inner package re-plan is proven
by reproducing all 129 measured packages byte-for-byte from source-derived rules (which corrected one earlier statement — the 16-byte
package tail is a per-asset constant, not zeros), growth cases carry exact table-word patches and hashes, the six early archives with a
`.bh` mirror get fit/append/reflow outcomes as word patches and whole-file hashes, the five later EB archives the append case, the NHL 07
two-level `.bh` cascade is pinned with its slack, and every unexplained word is inventoried for verbatim copying. One undecidable rule
(tight vs 64-aligned after a raw texture) is carried as two outputs.

### Changed paths

New: `03-changed-length-placement-oracle.md`, `data/r10-placement-oracle.json` (0.85 MB), probe `r10_placement_oracle.py`, claim
`research-r10-placement-oracle` (`ready`); atlas row 03 + Q-R10-* + correction entry. Not changed: application code, defs, bindings,
matrix, developer files, installs, anchors, the 2026-09-19 handoff 02 (its §2 tail statement is corrected here, not edited there).

### Checks actually executed (read-only)

The oracle in full (≈ 12 min: identity 129/129, growth variants, six BIG4 archives hashed in three variants incl. two 2 GB files in
two, five EB archives, NHL 07 `boot.big` members decoded and compared to the loose `.bh` 8/8, field census); the identity test four
times during rule correction; tail census (14 values / 104) and a 0-hit string-hash test. **Not executed:** writes, Studio binaries,
launches.

### Limitations

Oracle inputs are `stock ++ N zeros`, a test construction; append vs reflow loader tolerance unchanged (cards); NHL 08/09 PS3 `boot.big`
and the NHL 09 PS3 archives are not in the survey population and not covered.

### Cleanup

Scratchpad `r10-*.json`, `r10-full.log` deleted; no processes.

### Next

Priority 1C — localization: canonical tree rebuild / new-symbol contract, irregular-order dictionaries, PS4 non-English locale
inventory + vectors, NHL 07 label search coverage.

## R11 — Priority 1C, localization (~06:25–07:30) — handoff [04](04-localization-tree-rebuild-locales-and-nhl07-labels.md)

### Plain-language summary

The dictionary compression tree is pinned byte-exactly (re-serialised on all 175 dictionaries), shown to cover exactly the used
symbols with depth 10–12, and shown not to be a textbook Huffman tree (no standard construction or weight transform reproduces it) —
so a glyph-adding rebuild will always carry a new tree, whose acceptance is a runtime question with a card. The order / duplicate /
null census now covers 175 files (out-of-order files are concatenations of sorted blocks, NHL 08–10 and `lng_lg` only), the PS4 locale
inventory is complete (50 files), and the NHL 07 label search's size exclusion is closed (39 large files, 0 hits).

### Changed paths

New: `04-localization-tree-rebuild-locales-and-nhl07-labels.md`, `data/r11-loc-tree-census.json`, `data/r11-nhl07-label-grep-large.json`,
probes `loc_tree_rebuild.py`, `nhl07_label_grep_large.py`, claim `research-r11-localization` (`ready`); atlas row 04 + Q-R11-*. Not
changed: application code, defs, bindings, matrix, developer files, fixtures, installs, anchors.

### Checks actually executed (read-only)

Tree census over 25 anchors + 100 private + 50 PS4 files (≈ 4 min); large-member grep over 39 files (≈ 3 min); scratchpad: tree-hash
comparison, corpus/weight-transform hypotheses (0 hits), length-limited optimum. **Not executed:** writes, Studio binaries, launches.

### Limitations

EA's tree algorithm unknown (two approaches failed; stopped); no font/glyph inspection; runtime acceptance of rebuilt trees, grown files
and unsorted rows unobserved.

### Cleanup

Scratchpad `r11-anchors.json`, `r11.log` deleted; no processes.

### Next

Priority 1D — different-dimension logo replacement contract (texture metadata / mips / Apt geometry / container consequences) and the
league → imagery bindings; then 1E (competition structure, schedules, settings serialization).

## R12 — Priority 1D, different-dimension logo contract (~07:35–08:00) — handoff [05](05-different-dimension-logo-replacement-contract.md)

Plain-language: every word that carries a logo's size was measured per gen7 family — the texture container's own header words, six
floats in the 288-byte geometry record and two floats in the Apt object of member `0` (the latter not named before) — and the NHL 07
witness (100-unit quad on a 128-pixel texture) shows those floats are the display quad, not the texture size; the contract splits
"change the texture's pixels" (container + placement only) from "change the displayed size" (the eight floats), with the UI's reaction
to a mismatch as the runtime card. PS4 has no geometry record (registry directory + GNF header). No writer. Probe `r12_dimension_words.py`;
claim `research-r12-dimension-contract` (`ready`). Checks: package member diffs on Legacy Xbox/PS3, NHL 10, NHL 07 (read-only).

## R13 — Priority 1E, schedule formats + competition-structure status (~08:05–08:40) — handoff [06](06-schedule-formats-observed-and-competition-structure-status.md)

Plain-language: the stock NHL schedules follow exact per-era quotas by opponent class (NHL 08: 8/4/1; 09–13: 6/4/1+2×3; 14–17: 4–5/3/2;
18–21: 4–5/3/2 on 31 teams) with even home/away splits, measured on all 25 anchors — the input a "stock-shaped" generator needs and the
oracle to judge it; the AHL tables are real-season replicas with no class rule (import or user quotas only); `nhl32`/`ahl31`/`ahl32` are
an even 2/3 mix with placeholder slots. The remaining 1.2 questions (77/78 names, NHL 07 semantics, tournament slots, older settings,
save serialisation, expansion) are tabled with their obstacle and next step — no new evidence tonight, no saves generated. Probe
`r13_schedule_structure.py`, data `r13-schedule-structure.json`; claim `research-r13-schedule-formats` (`ready`). NHL 07 anchors are
skipped by the probe (thin team table).

### Priority 1 closeout statement

League research now has, per operation: 1.1 (unchanged, accepted E1); 1.2 schedules — quotas delivered, generation contract proposed,
runtime card unchanged; 1.2 rules/settings — obstacles named (§3 of handoff 06); 1.3 — tree rebuild contract + locale inventory +
NHL 07 closure (handoff 04); 1.4 — dimension contract (handoff 05) and placement oracle (handoff 03) on top of the accepted logo codecs;
1.5 — bank index decoded and the NHL 21 cue chain (handoff 02), blockers = encoders, gen7 event → sentence link, horn id mapping.
Nothing here is implemented or game-verified by this campaign.

### Next

Priority 2 — Teams, in roadmap order, starting from the existing PS4 maps (handoffs 06/07 of 2026-09-19, 01 of today): identity joins,
localization keys, logos/textures, arenas/ice, kits, audio.

## R14 — Priority 2, Teams (~08:45–09:40) — handoff [07](07-teams-reference-census-vaults-and-audio-keywords.md)

Plain-language: (1) a reference census over eight main DBs lists every column that carries team indexes / art ids / arena art ids —
the disclosure list for identity edits — with value-based ambiguity flagged honestly (small-integer columns match every key set);
(2) the numbered stadium vaults are associated to their levels by their own string pools (75 / 81 on NHL 21) while their naming key
stays underived after two more approaches (hash families, attribute/id search); (3) the team, arena, city and chant "audio ids" are
keyword ids of `pbp_keywords.bin` (100 % on PS4 15–19 / 21 and PS3 15, whose table equals PS4's), and on NHL 21 an AHL club's keyword
tags exactly its two half-second name-call samples — the team-name audio binding, with a reassignment operation and a card; player
audio ids do not follow it (recorded negative). Data `r14-team-reference-census.json`, `r14-audio-keywords.json`; probes
`r14_team_reference_census.py`, `r14_audio_keywords.py`; claim `research-r14-teams` (`ready`). NHL 20's extracted keyword table is
incomplete (coverage gap). Not executed: writes, launches, Studio binaries.

### Next

Priority 3 — Players: portrait/head/asset-key relationships (vault head classes, Q-R7-4), name audio (open after R14), identity FKs
(covered by the census); then Priority 4 equipment catalogs.

## R15 — Priority 3, Players (~09:45–10:20) — handoff [08](08-players-head-hair-binding-resolved.md)

Plain-language: the NHL 21 head audit's "no id → head binding anywhere in the data" is superseded at the data level — every head
collection and head texture pack carries a typed `SkaterNameAttribute` whose value is the DB `headid` (766 / 766 on NHL 21, 604 / 605
DB ids resolve, 364 exact + 31 name-drift bespoke checks; all seven PS4 titles), hair binds through `GenericHairStyle` (58 / 58).
Reassigning heads/hair is a DB edit validated against the vault; creating a new head still needs the vault writer (keys underived).
Data `r15-player-asset-binding.json`; probe `r15_player_asset_binding.py`; claim `research-r15-players` (`ready`). The audit file is
cited, not edited. Not executed: writes, launches, Studio binaries.

### Next

Priority 4 — Equipment: catalog → DB selector → vault/registry model/texture chains per category (sticks, gloves, helmets, skates,
goalie gear), existing-entry replacement vs new entry.

## R16 — Priority 4, Equipment (~10:25–11:05) — handoff [09](09-equipment-selector-to-catalogue-chain.md)

Plain-language: the per-player equipment numbers in the PS4 databases are values of typed enums on the vault's equipment collections
(`StickBrand`, `GloveBrand`, `HelmetBrand`, `SkateBrand`, `HelmetVisorType`, tape styles, `GoalieMaskTexture`, `GoalieStickBrand`,
`GoalieBrands`) — every used value exists in the catalogue on all seven titles (one `helmetbrand 99` sentinel and a few NHL 16/17 mask
ids excepted) and each resolves to a named collection with its texture files; the catalogues hold many spare entries (reassignment
targets), while a new brand needs the vault writer. Geometry binding and gen7 equipment are not covered. Data `r16-equipment-chain.json`;
probe `r16_equipment_chain.py`; claim `research-r16-equipment` (`ready`). Not executed: writes, launches, Studio binaries.

### Next

Priority 5 — Miscellaneous presentation (UI / boards / scorebug / menu music): reuse the bank census (menu music = `musicbank.sbr` /
`music_inxp*.sbr` on PS4, `SBKR` music on gen7) and the registries; then closeout.

## R17 — Priority 5, miscellaneous presentation (~11:10–11:40) — handoff [10](10-misc-presentation-menu-music-and-status.md)

Plain-language: the gen7 music box table is decoded — every music-bank track has a record naming the situation it plays in (menu
tunes, goals, period ends, anthems, stoppages, penalties, win/lose, playoffs), 100 % of the bank on NHL 11 / 14 / 15 / Legacy — which,
with the bank decode, gives the complete gen7 menu/arena music chain; PS4 banks are inventoried (423 + 285 + 70 + 1 tracks) but their
selection is compiled. UI, boards and scorebug get a status table with the bounded next step each. Data `r17-musicbox.json`; probe
`r17_musicbox.py`; claim `research-r17-misc-presentation` (`ready`). Not executed: writes, launches, Studio binaries.

## R18 — Phase 2.5 / 5.2 follow-up, arena packs and boards (~11:45–12:30) — handoff [11](11-arena-presentation-packs-and-boards-status.md)

Plain-language: the arena record's ~100 typed pack slots are unset in the main registry (0 resolved on every level); the packs are keyed
on their own attributes — chairs/scarves/kits/stanchion lights/nets by `ArenaID` (chairs cover 44 of 62 NHL 21 arenas), centre/spotlight/BAP
logos by `Team`, and the Creation-Zone catalogue packs (centre ads, jumbotron, stanchion material, goal light, entrance prop, netting,
one dasher) by their own enum. The numbered stadium vaults are lighting only (light rigs + 755 lightmaps for Pittsburgh). Boards are
per-arena meshes (61 / 80) with 63 shared `adboard_*` textures whose material binding is not decoded — the named next step for 5.2.
Data `r18-arena-packs.json`; probe `r18_arena_packs.py`; claim `research-r18-arena-packs` (`ready`). Also: NHL 20's base
`pbp_keywords.bin` verified as a 91-pair stub in `inxp_40.big` (handoff 07 §3 corrected in place). Not executed: writes, launches,
Studio binaries.

## R19 — Phase 5.2 follow-up, the dasher-board ads binding chain (~12:50–14:40) — handoff [12](12-dasher-board-ads-binding-chain.md)

Plain-language: the boards' sponsor panels are not painted into the mesh or its materials. Every arena's board mesh has 38–42 numbered
ad materials that all bind the same blank base textures plus a transparent decal slot; the logos are separate `boards_ads_decal_<sponsor>`
textures, and the vault assigns them per panel through the arena's boards scene object — a default list (one entry per panel, in panel
order; NHL 21 adds a trailing blank) plus game-mode override groups (HUT, European leagues, Champions Hockey League, World-of-CHEL
tiers, AHL). Measured on all seven PS4 titles: 32/32 … 61/61 arenas with a boards model have a default list of exactly the panel count
(+1 on 21); the default record is shared on 16–21 (NHL 21: one record feeds 37 rinks). The `adboard_*` textures of handoff 11 are the
static building signage (stands / roof / scoreboard / VIP boxes; 56 materials). Unknown: how a short override group maps onto the
panels, the GameMode / BoardVariation value names, runtime acceptance (card R19-ADS-1). Data `r19-board-ads-chain.json`; probe
`r19_board_materials.py`; claim `research-r19-board-ads-chain` (`ready`). Q-R18-1 resolved at the data level; Q-R19-1 added.
Executed: the probe (five runs while the chain parser was corrected — instance-blob arrays with empty headers, 20-byte scene-schema
entries, game objects selected by name because NHL 15's instances list other objects), heap owner attribution, a DB metadata grep
(no ad/board/sponsor fields; only `boardtrimzone1..3` / scoreboard zones), a non-texture tree grep (no board-ad identifiers outside
vaults). Not executed: writes, launches, Studio binaries. Cleanup: scratchpad `nv2.py`, `mdl.py`, `r19.log`, `upd.py` and
`probes/__pycache__` removed; no extracts created.

## R20 — gen7 1.5 follow-up, speech events `EVT0` and the sentence join (~14:45–15:30) — handoff [13](13-gen7-speech-events-evt0-and-sentence-join.md)

Plain-language: the gen7 event table does not point at sentences — it lists each event's parameters (team, player, league, tournament
round, period, penalty type, …); which sentence an event speaks is decided by the compiled MOIR script, whose string table names every
event and sentence. A sentence carrying an event's exact name is that event's line (649 / 700 on NHL 15, 2 212 / 2 233 on NHL 14,
751 / 796 on Legacy); the AHL / CHL cup events carry a `cup_endings_sequence_number` parameter and have no sentence of their own, so
their lines are script-routed (unobservable statically). Decoded on 8 loose sets (PS3 = Xbox 360 per title; fixed part 72 / 48 bytes);
NHL 11 (`EVT0` v0) and the NHL 11 / 12 sentence files stay unparsed. Data `r20-gen7-speech-events.json`; probe `r20_gen7_speech_events.py`;
claim `research-r20-gen7-speech-events` (`ready`). Q-R9-3 resolved at the data level; Q-R20-1 added. Executed: the probe (three runs
while the parameter-table / padding rules were established) and the MOIR string census. Not executed: writes, launches, Studio
binaries. Cleanup: scratchpad `upd2.py` and `probes/__pycache__` removed; no extracts created.

## R21 — Correction checkpoint after the Codex R19/R20 review, then the gen7 trace and the board-consumer search (2026-09-20, second session) — handoffs [12](12-dasher-board-ads-binding-chain.md) rev. 2 and [13](13-gen7-speech-events-evt0-and-sentence-join.md) rev. 3

Plain-language: the review ([docs/validation/research-r19-r20-review-2026-09-20.md](../../validation/research-r19-r20-review-2026-09-20.md))
was right on all four points. (F1) The board handoff now separates *parsed* pointers from *name associations* and *candidate readings*;
"element *i* = panel *i*" is a candidate, the array record sharing is a parsed fact, the JSON note and probe docstring say the same thing,
and the building-signage sharers are a reproducible census in the JSON. (F2) Card R19-ADS-1 is rewritten with a stock baseline, two rinks
(shared and unshared record), treatment A/B, active-source and rollback checks, and bounded verdicts. (F3/F4) The speech handoff keeps every
event record, every candidate join, input hashes and explicit SBR failures; and the continuation the review asked for changed the picture:
the `MOIR` file is an id → name table (not a script), the sentence file is a hash table keyed by `FNV-1(name)` (verified on every sentence
of four titles), and the context file references events by typed ids — so context → event → name → sentence → samples is a *parsed* static
chain; only firing, body choice and the behaviour of a missing lookup (every AHL / CHL cup event on NHL 15 / Legacy) remain runtime
questions. That also uncovered a defect in the R9 sentence parser (one body per sentence, wrong name offset): handoff 02 §4.2 carries a
correction note and its data was regenerated. For the boards, static data does not resolve the indexing: the NHL 21 eboot carries the
attribute names next to a `StadiumBoardWrapFeature`, the scenario schema shows `ArenaBoardVariation` is a per-game input, and GameMode
value names exist nowhere in data — the corrected card stays the discriminating step.

Changed paths (research-owned): handoffs 12 / 13, handoff 02 (note only), `00-ATLAS.md`, this file, probes `r19_board_materials.py` rev. 2,
`r20_gen7_speech_events.py` rev. 3, `pbp_sentences.py` rev. 3; data `r19-board-ads-chain.json`, `r20-gen7-speech-events.json`,
`r9-nhl15-ps3-pbp-sentences.json` regenerated; claims r19 / r20 updated. Executed: r19 probe (1 run), r20 probe (6 runs), sentence probe
(1 run), the hash-function search, eboot / extracted-tree string searches (NHL 21). Not executed: launches, writes, Studio binaries, the
`.gas` bytecode. Cleanup: scratchpad `fix12.py`, `patch20.py`, `fixnul.py`, `upd3.py`, `r19.log`, `r20.log`, `t.json` removed;
`probes/__pycache__` removed; one stuck helper interpreter of this lane (PID 55356, a NUL-repair loop) was terminated; no extracts created.

## R22 — gen7 context records, keyword resolution, bytecode bound, read-only contract (2026-09-20, third session) — handoffs [13](13-gen7-speech-events-evt0-and-sentence-join.md) rev. 4 and [14](14-gen7-commentary-read-only-contract.md)

Plain-language: the context file is now parsed record by record (3 371–4 183 contexts per title): each has a name, opaque header numbers
and an ordered list of one or two speech items, each a typed event reference followed by the event's name spelled out — so the earlier
scan candidates are now record fields (every scan hit lies inside a record item; the 170–202 label disagreements per title are kept). The
inline name matches exactly one MOIR candidate for every ambiguous event id (all candidates are kept; nothing is chosen). The sentence
phrase words turned out to be keyword ids, resolved to sample ids through `pbp_eng_keywords.bin` where it is loose (NHL 15 / Legacy) — this
explains the Xbox NHL 13 "bank mismatch" (its bank ids start at 77 269) and retires the rev. 3 sample-id reading; the constant `287` in every
body is a control value. The bytecode is bounded to a blob table (no strings, no decoded instructions, no context ↔ blob link). A read-only
listing contract for the parsed parts is published (handoff 14), scoped to the measured sets. Boards stay on card R19-ADS-1 rev. 2 (no new
static evidence). Changed paths (research-owned): handoffs 13 (rev. 4), 14 (new), atlas, this file; probes `r20_gen7_speech_events.py` rev. 4,
`r22_gas_sections.py` (new); data `r20-gen7-speech-events.json` (41.6 MB compact), `r22-gas-sections.json`; claim r20 updated. Executed:
r20 (five runs), r22 (one), record dumps, keyword tests. Not executed: launches, writes, Studio binaries. Cleanup: scratchpad `patch21.py`,
`patch22.py`, `patch23.py`, `upd4.py`, `r20.log` removed; `probes/__pycache__` removed; no extracts created.

## Closing receipt — research lane, 2026-09-20 (00:05–15:30; R21 and R22 appended in later sessions)

### Plain-language summary

Thirteen handoffs, each with a durable probe and machine-readable data, cover the campaign's five priorities. The evidence contract
of yesterday's vault work is closed (three-part jersey join, duplicate discriminator `IsTopScorer`); the sound-bank index that blocked
every audio cell is decoded on PS4 and gen7 with the NHL 21 commentary chain complete to the sample; an independent placement oracle
reproduces all 129 measured logo packages and gives hash-level expectations for changed-length writes; the dictionary tree is pinned
for the glyph-adding rebuild; different-dimension logo edits have a word-level contract; the stock NHL schedule quotas per era are
measured; team, player, equipment and arena assets on PS4 are joined from database numbers to vault collections to texture files
(including the head binding the earlier audit could not find); the gen7 music box is decoded. Remaining blockers are named per item
with the next discriminating step; nothing is implemented, accepted or game-verified by this campaign, and no visual/listening review
was requested.

### Delivered (researcher-verified, pending Codex review)

Handoffs 01–11 (`docs/research/overnight-2026-09-20/`), 14 data files under `data/` (≈ 29 MB, the largest `r8-jersey-triple-joins.json`
16.7 MB and `r9-nhl21-ps4-pbp-sentences.json` 6.4 MB), 15 probes under `local/runs/overnight-research-2026-09-20/probes/`, eleven
`research-*` claims (all `ready`) under `local/coordination/overnight-2026-09-20/claims/`. Atlas rows 01–11, questions Q-R8-1 … Q-R18-1,
corrections to handoff 07 (2026-09-19) §4, handoff 02 (2026-09-19) §2 (tail) and the head audit §8 (superseded at the data level; audit
not edited).

### For the developer (its lane was active in parallel tonight — claims `dev-c0` … `dev-c6`; nothing of theirs was touched)

1. **C3/C4 gen7 placement (delivered):** run the writers against [handoff 03](03-changed-length-placement-oracle.md)'s
   `r10-placement-oracle.json` — identity re-plan 129 / 129, growth variants, BIG4 + `.bh` fit / append / reflow hashes, EB append
   hashes, NHL 07 cascade slots. The tail correction the developer found independently (C1) agrees with §1 there.
2. **C5 loc rebuild (delivered):** [handoff 04](04-localization-tree-rebuild-locales-and-nhl07-labels.md) §1 — a rebuilt tree must be a
   full prefix tree with ≤ 256 leaves = exactly the used symbols and max depth ≤ 12 (observed ceiling; EA's own trees are not textbook
   Huffman and byte identity is not a target); the 175-file census gives the class-A/B order structure and the PS4 locale files.
3. **C6 dimension change (active):** [handoff 05](05-different-dimension-logo-replacement-contract.md) — besides the six floats of the
   288-byte record there are **two floats in Apt member `0`** (bytes 156/160 or 140/144 by variant) carrying W/H, and the floats are the
   display quad, not the texture size (NHL 07 witness); contract T (texture dims) vs Q (quad size), cards R12-DIM-1/2.
4. **Phase 1.5 / 2.7 audio:** [handoff 02](02-snd10-audio-banks-and-league-cue-chain.md) — bank index reader (`snd10_sbr.py`), the
   NHL 21 sentence → sample chain, the preservation contract for in-place sample replacement; [handoff 07](07-teams-reference-census-vaults-and-audio-keywords.md)
   §3 — team/arena audio ids are PBP keyword ids (reassign = DB edit); [handoff 10](10-misc-presentation-menu-music-and-status.md) — the
   gen7 music box.
5. **Phase 2–4 joins on PS4:** handoffs [01](01-ps4-jersey-triple-contract-and-r7-reconciliation.md) (jerseys), [07](07-teams-reference-census-vaults-and-audio-keywords.md)
   (identity dependency map), [08](08-players-head-hair-binding-resolved.md) (heads/hair), [09](09-equipment-selector-to-catalogue-chain.md)
   (equipment), [11](11-arena-presentation-packs-and-boards-status.md) (arena packs) — all "reassign among existing = DB edit validated
   against the vault; new entry = vault writer (blocked on collection keys)".
6. **Schedules:** [handoff 06](06-schedule-formats-observed-and-competition-structure-status.md) — per-era NHL quotas + JSON oracle for a
   "stock-shaped" generator; AHL = import only.
7. **Boards (5.2):** [handoff 12](12-dasher-board-ads-binding-chain.md) rev. 2 — read-only listing per arena (ad materials, the decal array
   labelled "entry *i*", override groups, sharers of the array record) and the `adboard_*` sharer census; **no** per-panel editor contract
   (candidate experiment only, card R19-ADS-1 rev. 2).
8. **Gen7 commentary (1.5):** [handoff 14](14-gen7-commentary-read-only-contract.md) (contract) on [handoff 13](13-gen7-speech-events-evt0-and-sentence-join.md)
   rev. 4 — events + parameters, MOIR id ↔ name (all candidates per id), sentences (name, hashed slot, bodies, keyword-resolved samples),
   context records with candidate links shown apart; say "no sentence carries this event name" for the AHL / CHL cup events on NHL 15 /
   Legacy, never "silent".

### Remaining blockers (by phase / family)

- 1.2: PS4 group names 77/78, NHL 07 membership semantics, tournament slot semantics, older-title settings, save serialisation of
  chosen settings (needs game-generated specimens) — handoff 06 §3.
- 1.3: EA's tree algorithm (byte identity only), runtime acceptance of rebuilt trees / grown files / unsorted rows, font glyph coverage,
  locale selection — handoff 04 §6.
- 1.4: UI reaction to texture ≠ quad; the other Apt words; PS4 size-directory semantics — handoff 05 §4.
- 1.5 / 2.7 / 3.5: codec `0x09`; encoders for EA-MP3 / Opus-shaped / EALayer3 / XMA; gen7 runtime (bytecode semantics and context ↔ blob
  binding, body / take choice, missing-lookup behaviour for the AHL / CHL cup events, runtime resolution of non-unique event ids; `league`
  values); NHL 11 / 12 layouts; NHL 13 / 14 keywords files (in-archive); DB `goalhorn` → hornbank id; player-name audio; NHL 20's
  club-mention tagging — handoffs 02 §7, 07 §4, 13 §6.
- 2.5 / 5.2: numbered-vault naming key (handoff 11 §5); the consumer / indexing of the boards decal arrays, GameMode names, runtime acceptance — handoff 12 §4–4a (references parsed; consumer inferred only).
- 2.x / 3.x / 4.x creation: vault collection-key derivation (every "new entry" case) — Q-R7-1.
- 5.1 / 5.3: Apt object census; `.gfx` scorebug inventory — handoff 10 §3.
- gen7 equipment / heads not re-measured (handoff 30 of 2026-09-18 stands).

### User review cards (batched; none requested now)

- NHL 21 PS4: R8-KIT-1 (top-scorer kit), R9-AUD-1 (only if a reassign is built), R14-AUD-1 (club-name keyword reassignment),
  R15-HEAD-1 (head reassignment), R16-EQ-1 (stick brand reassignment), R12-DIM-1/2 (dimension / quad), R11-LOC-1 (rebuilt tree glyph), R19-ADS-1 rev. 2 (board decal-array swap with controls).
- NHL 10 Xbox 360: R2-BIG4-1 (unchanged from 2026-09-19); NHL Legacy Xbox: R2-LZX-1 (unchanged).
- PS3 NHL 15 / Legacy: R17-MUS-1 (music-box context re-point).
- Carried forward unchanged: P1BS-CSV-1, R2-EB-1, R3-IMG-1/2, R4-AUD-1, R5-07-1.

### Cleanup and retained artifacts

- Retained (durable): the 14 handoffs, 17 data files, 18 probes, 13 claims. Dependencies of the probes: the 2026-09-18 probes
  (`tdb.py`, `schema_dump.py`, `eb_list.py`, `big4_list.py`, `refpack.py`, `chunklzx.py`, `ps4_leaguelogos.py`), the 2026-09-19 probes
  (`vlt_collections.py`, `ps4_team_field_map.py`, `viv4_index.py`, `nhl07_label_grep.py`) and their data (`r2-packaging-survey.json`,
  `r2-fit-audit.json`); the installs on `X:`/`H:` (read only), the extracted PS4 trees, the private fixture root, the anchors.
- Removed: every session scratchpad file (`r8-*`, `pbp21.json`, `pbp-ps3-15.json`, `pbp20.json`, `r9-*.json/log`, the two extracted
  Xbox Legacy `.sbr` copies, `r10-*.json/log`, `r11-*`, `r16-21.json`, `nv.py`, `adb.py`, `tailtest.py`, `closeout.py`, later `nv2.py`, `mdl.py`, `r19.log`, `upd.py`, `upd2.py`); `probes/__pycache__`
  removed at closeout (again after R19 and R20). No extracts under `local/runs/overnight-research-2026-09-20/` other than `probes/`.
- Processes: none of this lane's remain (the four `python.exe` seen at closeout belong to the Claude Desktop Blender extension, started
  2026-09-19 12:42 — not touched).
- Not written: anything under `X:`, `H:`, `E:\nhl-studio-fixtures`, `anchors/`, application code, defs, bindings, tests, the matrix,
  developer files. No commits, no pushes, no launches.

### Exact next continuation task

Gen7 commentary, still static and read-only: (1) extract the NHL 13 / 14 `pbp_eng_keywords.bin` members from `audioboot.big` (EB tables,
streaming extractor of handoff 40 / 2026-09-18) into a scratch path, rerun `r20_gen7_speech_events.py` with them so those titles' phrase
words resolve, then delete the extracts; (2) the `CTX0` header words `w1 / w2 / w4 / w8` — census against the `crowds_contexts.bin` sibling
and the 178 `trigger_*` names of `pbp_eng_contexteventdef.csi` (MOIR-shaped) to test whether `w8` is a trigger id; (3) only if (2) yields a
typed link, the context → `.gas` blob binding. The `.gas` instruction set stays out of scope.
