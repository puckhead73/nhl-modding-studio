# Receipt 06 — Closing receipt, overnight 2026-09-24 (Leagues and Teams)

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` throughout; nothing staged,
committed, pushed or launched; no live install, archive, image, anchor or save was written. Session
clock 04:54 → 14:05 MDT (the wall clock jumped 05:53 → 13:05 between packages 03 and 04).

## 1. Plain-language summary

The night started with the coordinator's open defect: the texture Replace dialog could be shown one
plan and then perform a different, undisclosed operation if the file changed underneath it. That
is closed first, with the write now bound to a server-held review of exactly what was displayed,
and proven red then green. A ledger of every League and Team operation followed; it found no Phase
1 gap that could be closed offline without a new contract, so the campaign moved through the Teams
queue: the arena assignment now shows who shares an arena before it writes and is proven to save,
reopen and undo on every anchor that has the link (with retired numbers, draft trades and strategy
cells proven the same way); the PS3 NHL 12 / 13 kit textures inside their render archives can now
be replaced through the existing archive route (the "no encoder" blocker was a wrong codec label,
corrected on the record); and audio moved from listing to **diagnostic previews** — a pure-Rust MP3
decoder was chosen and recorded, PS4 EA-MP3 speech and gen7 EALayer3 speech decode on the
research's measured grammars (24 / 24 and 116 / 116 research vectors), and the arena page reads its
horn ids against the actual bank files. Every preview carries a label saying it is diagnostic and
not game-verified; nothing claims replacement support. What remains is listed exactly, and the
morning cards say what only a listener or a console can decide.

## 2. Packages and receipts

| # | Package | Receipt | Claim |
|---|---|---|---|
| 01 | P0 / F1 — reviewed texture write (file + bytes routes) | [01](01-p0-f1-reviewed-texture-write.md) | `dev-p0-reviewed-texture-write` |
| — | P1 — League / Team completion ledger | [LEDGER.md](LEDGER.md) | — |
| 02 | T1 — Teams 2.2 relationships parity (arena disclosure; 27-anchor sweep) | [02](02-t1-team-relationships-parity.md) | `dev-t1-team-relationships` |
| 03 | T2 — Teams 2.6 PS3 12 / 13 kit members replaced in their archives | [03](03-t2-ps3-kit-member-replacement.md) | `dev-t2-ps3-kit-member-replacement` |
| 04 | P4.1 + P4.2 — decoder decision; PS4 EA-MP3 diagnostic preview | [04](04-p4-ps4-ea-mp3-diagnostic-preview.md) | `dev-p4-ps4-ea-mp3-preview` |
| 05 | P4.3 + P4.4 — gen7 EALayer3 v1 preview; gen7 horn sources | [05](05-p4-gen7-ealayer3-preview-and-horn-sources.md) | `dev-p4-gen7-ealayer3-and-horn-sources` |
| 07 | P4.5 (i) — the gen7 preview reachable from the catalogue phrase and player name-call chips (after this closeout; UI only; vitest 35 / 229) | [07](07-p45-preview-hooks.md) | `dev-p45-preview-hooks` |
| 08 | P4.5 (iii) — archived gen7 banks / streams resolved by the preview route (after this closeout; domain only; real PS3 13 / 09) | [08](08-p45-archived-gen7-banks.md) | `dev-p45-archived-gen7-banks` |
| 09 | P4.6 — gen7 bank listing card incl. music (after this closeout; real X360 NHL 15 music 263 / 263; vitest 36 / 231) | [09](09-p46-gen7-bank-listing.md) | `dev-p46-gen7-bank-listing` |
| 10 | R85 consumed — gen7 loader asset table in the horn / chant notes; PS4 horn consumer notes (after this closeout, 25 Sep; read-only; four real trees) | [10](10-r85-loader-table-and-ps4-horn-consumer.md) | `dev-r85-loader-table` |
| 11 | P3.1-A — Players 3.1 identity data swept on 27 anchors; **weight base defect fixed red → green** (PS4 / NHL 08 were 30 lb off on display and writes) (after this closeout, 25 Sep) | [11](11-p31a-player-identity.md) | `dev-p31a-player-identity` |
| 12 | P3.2-A — roster-list Overall column era-gated like the Attributes tab (NHL 13 ~13 low, NHL 07–12 nonsense → stored / none) (after this closeout, 25 Sep) | [12](12-p32a-roster-overall-column.md) | `dev-p32a-roster-overall-column` |
| 13 | P3.2-B — ratings / style / potential sweep on 27 anchors; **six-bit scale probe narrowed to rating cells** (NHL 08 was 14 low) (after this closeout, 25 Sep) | [13](13-p32b-ratings-scale-and-sweep.md) | `dev-p32b-ratings-sweep` |
| 14 | P3.2-C — historical import round trip on 27 anchors; NHL 07 create / clone / import refused in words; **Phase 3 ledger** (after this closeout, 25 Sep) | [14](14-p32c-import-sweep-and-phase3-ledger.md), [LEDGER-PHASE3](LEDGER-PHASE3.md) | `dev-p32c-import-sweep` |
| 15 | P3.2-D — full roster block refuses move / added membership in words on 27 anchors; engine `EditError::Refused` (after this closeout, 25 Sep) | [15](15-p32d-roster-full-refusal.md) | `dev-p32d-roster-full-refusal` |
| 16 | P3.2-E — inline string byte width on the field view; the editor refuses over-long text before the write (DTO + bindings + jsdom) (after this closeout, 25 Sep) | [16](16-p32e-string-width-precheck.md) | `dev-p32e-string-width` |
| 17 | P3.2-F — contract-dollar units measured against public cap hits: ×$25 000 on NHL 07–10, ×$5 000 on NHL 11–PS4 21 (after this closeout, 25 Sep) | [17](17-p32f-contract-dollar-units.md) | `dev-p32f-contract-units` |
| 18 | P3.3-A — player text inventory: inline biography strings vs the `$BURNABY` template token vs audio-name ids, 27 anchors (read-only) (after this closeout, 25 Sep) | [18](18-p33a-player-text-inventory.md) | `dev-p33a-player-text` |
| 19 | P4.1-A — hockey sticks swept on 27 anchors (model / curve / flex / tape; save / reopen / undo); catalogue-label scoping flagged (after this closeout, 25 Sep) | [19](19-p41a-sticks-sweep.md) | `dev-p41a-sticks` |
| 31 | Consolidated close (after receipt 27): goalie gear 4.5 database parity (25 anchors + genuine NHL 20); SBKR empty slots → X360 NHL 14 announcer Speex; "England" correction. **Next: SNS EA-XMA route in the X360 NHL 10–12 speech / music listing behind the helper gate** (26 Sep) | [31](31-consolidated-after-receipt27.md) |
| 30 | EA-XMA: Studio-side helper route + RIFF XMA2; 83 R84 vectors exact through the runner (oracle); helper build blocked (no toolchain) (26 Sep) | [30](30-xma2-helper-invocation.md) |
| 29 | EA-Speex diagnostic preview: pinned libspeex 1.2.1 + C shim; 25 R83 families within 1 LSB of the oracle; gen7 + PS4 rink-announcer routes; X360 NHL 14 announcer refused (bank reader) (26 Sep) | [29](29-easpeex-preview.md) |
| 28 | After-receipt-27 first checkpoint: receipt-27 corrections; **title-scoped nationality labels** (NHL 10 / 11 / 12 fixed; import + birth-state follow); real NHL 20 / 21 range audit + stored-code box; goalie-Overall input comparison (proposal only) (26 Sep) | [28](28-after-receipt27-corrections-labels.md) |
| 27 | Packages 23–26 review, tasks 1–4 (consolidated): **goalie consistency** not a rating (formula input preserved); **ledger current authority**; **decoder recommendation**; **tournament browse labels** (read-only); **4.2 gloves / 4.3 helmets** sweep incl. real NHL 20 (26 Sep 03:40 onward) | [27](27-review23-26-continuation.md) | `dev-c15-goalie-consistency`, `dev-ledger-refresh-26sep`, `dev-decoder-recommendation`, `dev-tournament-browse-labels`, `dev-p42-43-gloves-helmets` |
| 26 | Ledger row **2.6 nameplate keying** (R101 / R103): the row's own triple names its nameplate sheet on the six measured trees; association only, not completeness (26 Sep 03:28–03:35) | [26](26-p50-nameplate-association.md) | `dev-p50-nameplate-association` |
| 25 | Ledger row **cc 3 / 5 and loop previews**: one stereo pair at a time (inferred de-interleave, labelled); loops = stub PCM + body, loop point shown (26 Sep 03:20–03:28) | [25](25-p49-multichannel-and-loop-previews.md) | `dev-p49-multichannel-loop-previews` |
| 24 | Review 26 Sep items after F1 / F2 — **archive-backed PS3 NHL 09 / 10 / 12 / 13 sound banks** from each title's own members (R98 census and R100 vectors reproduced); **arena chant wording per R97 §1.4**; gen7 **horn-to-bank links**; `BoundBig4::read_directory` (26 Sep 02:40–03:12) | [24](24-p48-archive-sound-banks-chant-wording-horn-links.md) | `dev-p48-archive-sound-banks` |
| 23 | Review 26 Sep **F1** (stream identity checked on the reading handle) and **F2** (bounded layout signature; unsupported title / platform = conflict; import seams gated) (26 Sep) | [23](23-f1f2b-review-corrections.md) | `dev-f1f2b-review-corrections` |
| 22 | P4.7 — **gen7 sound banks**: `.abk` listing, named picker tree, `.ast` framing, diagnostic decode (PS3 EALayer3, one sub-stream); X360 framed only; R82 / R89 / R91 / R92 / R94 consumed (after this closeout, 25 Sep 15:10–22:07) | [22](22-p47-gen7-sound-banks.md) | `dev-p47-gen7-sound-banks` |
| 21 | Review item 8e — **title-scoped equipment vocabularies**: `[scope]` on the ten Legacy-established equipment TOMLs; labels only on a project opened as NHL Legacy, numbers + reason elsewhere; `FieldViewDto.vocabulary_note` (bindings regenerated); writes unchanged (after this closeout, 25 Sep 14:40–15:10) | [21](21-p8e-title-scoped-equipment-vocabularies.md) | `dev-p8e-equipment-scope` |
| 20 | F1 / F2 (development audit 25 Sep) — **encoding contract**: weight base / rating scale / birthdate index / cap-hit unit from the schema family, never the cells; title-vs-schema conflict refuses in words; cap-hit units witnessed by 6–25 public contracts per family (NHL 08 measured); real NHL 20 fixture witness (after this closeout, 25 Sep 13:20–14:40) | [20](20-f1-f2-encoding-contract.md) | `dev-f1f2-encoding-contract` |

## 3. Closeout aggregate ([`closeout-aggregate.log`](../../validation/evidence/overnight-development-2026-09-24/logs/closeout-aggregate.log))

| Suite | Result |
|---|---|
| `cargo test -p tdb-snd` | 12 passed (lib); the two real-vector cases ignored by default (each **1 passed** with `--ignored`, receipts 04 / 05) |
| `cargo test -p studio-domain --lib -- --skip zz_diag` | **222 passed, 0 failed, 2 ignored** (was 219 at the 23 Sep closeout) |
| `cargo test -p studio-domain --test ps4_speech_bank --test gen7_speech_preview --test gen7_horn_sources --test arena_asset_join --test field_profile` | 9 passed, 3 ignored (real trees) |
| `cargo test -p studio-api` (lib + every test target, 44 targets) | **135 passed, 0 failed, 35 ignored** (private fixtures / real trees); includes the 27-anchor relationship sweep (204 s) |
| `export-bindings --check` | in sync (156 roots) |
| `npx vitest run` (full) | **35 files, 227 passed** (was 31 / 209) |
| `npx tsc --noEmit` | clean |
| Real-tree / fixture runs of this campaign (each `--ignored`, each failing loudly without its input) | `loose_texture_route` + `gnf_real_resize` (8), `ps3_kit_member_route` real plan (1), `ea_mp3_real_vectors` (24 vectors), `ps4_speech_bank` real NHL 17 (1), `ealayer3_real_vectors` (116 vectors), `gen7_speech_preview` (2 trees), `gen7_horn_sources` (2 trees) — all passed, logs named in STATUS |

`zz_diag` was excluded explicitly (`--skip zz_diag`) and never invoked. Repo-wide `cargo fmt` /
`clippy -D warnings` remain the recorded C0.2 baseline defects and were not re-measured or repaired.

## 4. Focused coordinator review list

Risky writers, dependency changes and measurements that departed from a handoff — in that order.

1. **New dependency** — `symphonia-bundle-mp3` 0.6.1 + `symphonia-core` 0.6.1 (MPL-2.0, pure Rust,
   MSRV 1.85; transitive deps MIT / Apache-2.0). No repository dependency policy exists to cite;
   the decision and the alternatives are in receipt 04 §2. Lockfile +151 lines.
2. **Writer contract change** — `import_texture` / `import_texture_bytes` now require a
   `review_id` from `texture_replace_plan` / `_bytes`; the bytes route no longer takes the PNG (the
   review holds it). The review is consumed on use and bounded to 8 held; drift refuses before
   backup or mutation; the writer's notes are compared to the reviewed notes (receipt 01 §6).
3. **Archive write route widened in use, not in code** — PS3 NHL 12 / 13 kit members go through
   the unchanged `asset_member_edit` route; the real 1.6 GB archive was **planned only**, never
   copied or applied (receipt 03 §3, card R36-PS3-KIT-1).
4. **Arena assignment** — still the ordinary validated `set_field`; the disclosure is UI-bound, not a
   server-held review (a deliberate difference from the texture route, receipt 02 §5).
5. **Departures from handoff text, recorded as observations for research** (also in the claims):
   (a) the EALayer3 header-constant check must exclude `mode_extension` (joint-stereo music varies
   it per granule; R80 says "carried as read"); (b) R81 §1 text places NHL 09 X360's numbered horn
   banks in `nocache.big`, the r81 JSON in `data0.big` — the data was followed and matched on the
   real tree; (c) the PS4 bank sample counts aggregate every DSET (NHL 17: 40 018) where R71 used
   the largest DSET (3 863); (d) Symphonia output is not bit-exact with ffmpeg on any vector (RMS
   agrees within 0.1 %); (e) the PS4 banks' path buffer carries placeholder text, no stream path; (f) [package 10]
   X360 NHL 15's loader table lists goalie chant 0587 (not shipped) and omits the shipped 0584 —
   R85 §1's chant "100 %" was not measured against members; PS3 NHL 09 lists 32 goalie chants.
6. **Matrix stale wording flagged, not edited** — Phase 1 matrix 1.4 "different dimensions — X360
   12–15/Legacy: blocked (RW4xb2)" predates C6 / receipt 13 (LEDGER note).
7. **Dated correction notes** added to developer-owned records only: 23 Sep receipt 04 §5 and
   23 Sep STATUS blocker (chunkzip, not chunklzx). No research file was edited.
8a. **[package 11] Weight base contract change** — `ubPc.sNZW` display and edit offset now follow
   `probe_weight_base` (median non-zero raw < 60 → 150, else 120) instead of the rating-scale probe;
   measured 150 on NHL 07–10 and 120 on NHL 11–Legacy + PS4 on all 27 anchors. Any weight a user
   staged on a PS4 title **before** this fix was written 30 lb light (the roster edits of earlier
   campaigns did not touch weight; the historical import lowers through the same offset and is
   therefore corrected too). Review the probe threshold (means 44–46 vs 69–73) and the NHL 08 pin (Kukkonen 187 lb).
8e. **[package 19] Equipment catalogue labels are title-unscoped** — `defs/enums/stick-model.toml`
   (75 ids, anchored on NHL Legacy PS3 texlibs) binds by table / field id only, so its labels show
   on NHL 08 (ids 0..190) through PS4 21 (0..1023). Decision needed: per-title vocabulary scope in
   the enum registry (and the same question for gloves / helmets / skates / tape colours) before
   4.2 / 4.3 pickers are called human-readable off Legacy. **Decided and implemented (package 21,
   receipt 21):** scope on the definitions, resolved with the project's title; the Legacy labels
   show on NHL Legacy (PS3 / X360; Recomp by identity) only; every other title shows the number with
   the reason; writes unchanged. Review: the `FieldViewDto` contract addition (`vocabulary_note`),
   the two shop commands' optional game id, and whether the appearance vocabularies should follow.
8d. **[package 17] Cap-hit unit change** — `cap_hit_scale` now returns ×$25 000 for the 10-bit
   NHL 08–10 field (was `None` → raw number shown); pinned by 28 public cap hits (Crosby, McDavid) —
   NHL 08's own figure is EA's pre-extension $7.0M, inferred not public. Review the evidence basis
   (public contract figures used as ground truth for a stored unit).
8c. **[package 15] Engine error variant** — `tdb_edit::EditError::Refused(String)` added (displayed
   verbatim); the domain maps `TeamRosterFull` and the NHL 07 slot-key absence onto it with resolved
   team names. Review: whether other engine refusals should route through the same variant.
8b. **[package 13] Rating scale probe contract change** — `sixbit_rating_offset(db, meta)` now
   counts rating-classified six-bit cells over ≤ 1 024 records per companion table and returns 36
   only when > 1 % exceed 49 (else 50). Measured: 0 / 33 / 0 / 0 cells above 49 on NHL 07 / 08 / 09 / 10,
   2–4 % on 11–Legacy. NHL 08 ratings previously displayed and **wrote** 14 low. The NHL 07–10 "+50"
   reading itself is a distribution inference (receipt 13 §5–6), not in-game verified — review the
   threshold and whether a research card should close it.
8f. **[package 20] Encoding contract replaces the population probes (audit F1)** — `encoding::schema_family`
   classifies the layout from the schema shape (`yeardrafted` width 6 / 7 / 8, `helmetbrand` width,
   `draftyear` presence, byte order) and `encoding_for_family` fixes weight base / six-bit rating
   offset / birthdate index / cap-hit unit per family with an evidence level; `read_field`,
   `CompanionReader`, the header, `cap_hit_scale`, the import lowering and the player-asset slug all
   consume it. `probe_weight_base` / `probe_birthdate_encoding` / `sixbit_rating_population_offset`
   remain as `content_diagnostics` only. **New refusal:** `prepare_field_edit` refuses weight, birth
   month / day, six-bit ratings and `contractdollars` when the declared title's family differs from
   the schema's (text names both) or the schema is unresolved. Review: the classifier's features and
   whether the conflict should also surface on the project summary (it is a write-time refusal only
   today). Items 8a and 8b are superseded by this contract; the NHL 08–10 "+50" reading stays
   `Inferred`.
8g. **[package 20] Cap-hit evidence basis (audit F2)** — item 8d's "28 public cap hits" were three
   players repeated across anchors; the candidate table now matches 6–25 distinct public contracts
   per family, NHL 08 included (×$25 000 measured: Iginla, Lidstrom, Luongo, Pronger, Richards,
   Thornton, Kovalchuk). Receipt 17's explanation of Crosby's NHL 08 figure is withdrawn (dated note).
   The anchors are 27 directories, not 27 titles (`ps4/nhl20` = NHL 19 base; the real NHL 20 witness
   is the private fixture with pinned hashes).
8h. **[package 22] New read-only codec surface** — `tdb_snd::abk` parses the bank tree and frames
   streams on the measured grammars; the decode route trims PS3 terminal zero padding before the
   R77 walk (a departure the research did not spell out for Studio's parser) and emits the R92
   candidate construction under a fixed "not game-verified" label. No dependency added. Review: the
   padding trim rule, and that X360 stays framed-only pending an XMA2 decoder decision.
8. **Coverage pins** — the 27-anchor sweep asserts the exact family sets (home-arena link on 15,
   `wcSL` absent on NHL 07, draft picks from NHL 11); the gen7 preview offers only the `MEASURED`
   tuple table; the export-root count pin moved 148 → 156.

## 5. Owned artifact and process cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad patch scripts (`patch_p0_*.py`, `patch_t1_*.py`, `patch_t2_frontend.py`, `patch_p4_*.py`) | deleted at closeout after this receipt was written; scratchpad verified empty |
| Temp roots created by this campaign's tests (`nhlms-review-*`, `nhlms-teamrel-*`, `nhlms-ps3kit-*`, `nhlms-ps4snd-*`, `nhlms-horns-*`, `nhlms-c1-loose-*`, `studio_api_import_texture`) | removed on drop; the one leftover `nhlms-horns-131860` (killed mid-run once) removed by literal path; verified absent |
| Older `nhlms-*` roots in `%TEMP%` (`nhlms-league-campaign-all-*`, `nhlms-logos-gen7-*`, `nhlms-p3-op-*`, `nhlms_ps3_dds_portrait`) | **not touched** — earlier campaigns' or another lane's scratch, ownership not established by name |
| Evidence logs under `docs/validation/evidence/overnight-development-2026-09-24/logs/` (13 files) | retained (durable evidence) |
| `app/src/bindings/ContextCatalogueDto.ts` | regenerated during package 04 and restored — byte-identical to HEAD |
| Background processes | none at closeout (the aggregate finished; no cargo / node processes of this session left running) |
| Shared build cache (`target/`), other lanes' files, game trees on `X:` / `H:`, anchors, private fixtures, user saves | untouched; nothing deleted by age |

**Receipts 23–26 (26 Sep, 01:00–03:40):** owned patch scripts in the session scratchpad deleted by literal
path (20 files; scratchpad empty); the alternate build directory `E:/nhlms-target-check` (11 GB, created so
work could continue while the F1 / F2 aggregate held the main target) removed; the runaway census
test process stopped through its owning background task (a direct process kill was refused by the
harness); disposable test roots are removed by their tests' drop guards. Left in place, not attributable
to these packages: `nhlms-league-campaign-all-*`, `nhlms-logos-gen7-*`, `nhlms-p3-op-*`,
`nhlms-teamrel-sweep-*` under the system temp directory (older test runs); four node processes not
started by this session. No background task of this session is still running.

## 5b. Research published during this session (read at closeout)

The research lane's 2026-09-24 campaign published R82–R90 (`docs/research/overnight-2026-09-24/`,
queue rows 1–13) while these packages ran. **R85 was consumed afterwards by package 10** (receipt 10,
25 Sep); the rest is unconsumed and nothing here depends on it. Recommended next consumers, in the
queue's order:

- ~~**R85 loader asset table** (rows 5 / 7)~~ — consumed by [receipt 10](10-r85-loader-table-and-ps4-horn-consumer.md);
  its one departure from the handoff (X360 NHL 15 chant 0587 listed / 0584 shipped) is in that
  receipt §5 and the claim.
- **R82 archive-backed coverage** (row 2): `music_samples` on PS3 09 / 10 / 12 / 13 from the archives
  — the receipt-08 archive resolution likely covers it once the tuples are added to `MEASURED`.
- **R83 EA-Speex (codec 0x09)** and **R84 EA-XMA** (rows 3–4): two further diagnostic-preview
  contracts (X360 announcer / pbp 11–Legacy, PS3 14 pbp; EA-XMA sfx / music on X360) — each needs its
  own decoder decision (Speex UWB; XMA2) before any route.
- **R89 `ABKC` bank tree** (row 12) and **R90 team-chant binding** (row 13): the `.abk` horn / chant
  banks' sample lists and the club-chant banks — listing contracts for the arena page.
- **R86 / R87 with the R88 corrections** (rows 9–11): the team art-id graph and row-allocation
  constraints — read-only inspection is consumable; the re-key and creation contracts stay unfinished.

## 6. One exact next task

P4.5 (i) / (iii) and P4.6 were delivered after this receipt (07 / 08 / 09). The audio queue's remaining rows all need contracts (PS4 announcer codec census, `.abk` S10A reading for horn banks, every replacement / mutation). R85 was consumed by package 10 and Phase 3.1 identity opened and swept by package 11 (25 Sep, receipts 10 / 11). Package 12 then closed the recorded roster-OVR defect (the `sixbit_rating_offset` question turned out moot for ratings: PS4 ratings are classified 0..99 from metadata and never reach that probe). Package 13 delivered the ratings / style / potential sweep and narrowed the six-bit scale probe. Package 14 delivered the import sweep and the Phase 3 ledger. Packages 15–17 closed the packed-roster refusal, the string-width pre-check and the contract-dollar units. The remaining 3.2 rows are research or scope questions (ledger). Package 18 delivered the 3.3 inventory; Phase 3's developer-actionable rows are closed or research-owned (LEDGER-PHASE3). Package 19 delivered the 4.1 stick sweep. Package 20 (25 Sep, after the development audit) replaced the cell-population probes with the schema-family encoding contract and witnessed every cap-hit unit (F1 / F2). Package 21 delivered the title-scoped equipment vocabularies (review item 8e); the ledger was refreshed against R88–R101 and package 22 delivered the gen7 sound-bank listing / tree / framing / diagnostic decode. Receipt 23 closed the 26 Sep review's F1 / F2 and receipt 24 delivered the archive-backed PS3 sound banks, the R97 chant wording and the horn-to-bank links. Receipt 27 (the packages 23–26 review's tasks 1–4) corrected the goalie consistency classification, made the ledger's current authority explicit, published the decoder recommendation, added read-only tournament browse labels and swept gloves / helmets. **The next task is equipment 4.4 skates, then goalie gear (masks / pads / blocker / trapper / goalie stick), on the same sweep pattern including the real NHL 20** — unless the coordinator's decoder decision (libspeex; XMA2 licence) or a League / Team defect from the morning review comes first. Formerly: receipt 25 delivered the cc 3 / 5 and loop previews and receipt 26 the nameplate association. The ready League / Team rows of the refreshed ledger are now closed; what remains there needs a decision or a contract (Speex / EA-XMA preview: a decoder decision; art-id re-key and archive rename: R99 / R102 writer contracts; rink-logo keys: R103 candidates only). The next task is therefore the **equipment fallback, Phase 4.2 gloves then 4.3 helmets**, on the 27-anchor sweep pattern of receipt 19 with the title-scoped vocabularies of receipt 21 — unless the morning review returns a League / Team defect or a decoder decision, which take precedence — formerly the nameplate keying — formerly the cc 3 / 5 and loop previews — formerly the archive-backed gen7 sound banks and the chant wording — formerly the ledger refresh and the loose `.abk` listing — formerly the equipment scoping — formerly Phase 4.2 gloves and 4.3 helmets, formerly **Phase 3 players, step 3.1 in the parity roadmap's order**, starting from the existing player inspection foundations — read the roadmap's 3.1 statement and the ledger vocabulary, inventory what the player surfaces already do per family, and open the first bounded package with the same red / green, per-title, save → reopen → undo discipline used tonight; unless the morning review returns a League / Team defect, which takes precedence.
