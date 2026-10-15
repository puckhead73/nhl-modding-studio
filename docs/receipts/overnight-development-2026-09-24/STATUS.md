# Developer lane status — overnight 2026-09-24 (Leagues and Teams)

Campaign: the fresh overnight brief
[overnight-development-2026-09-24-leagues-teams.md](../../plans/overnight-development-2026-09-24-leagues-teams.md):
close the 24 Sep review's F1 first, reconcile a completion ledger, then finish League / Team
packages with established contracts, then Teams 2.7 audio as diagnostics — without routine review
stops. Codex is architect / reviewer; the research lane owns `docs/research/**` and
`research-*` claims (nothing there edited). Projects / Builds untouched; NHL 21 E1 not reopened.

**Starting state, recorded rather than assumed:** HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051`
(branch `feat/phase5-asset-tooling`), working tree dirty from the 18–23 September campaigns (790
paths) and preserved. Private fixture root `E:\nhl-studio-fixtures` present (no env var set;
passed explicitly). Game trees on `X:` (Xbox 360, PS3) and the extracted PS4 banks on `H:` read
only. No staging, commits, pushes, launches, live writes, anchor or save writes; every mutation in
memory or on disposable copies. Clock: 04:54 MDT start; the session clock jumped 05:53 → 13:05
between packages 03 and 04 (recorded, not explained here).

Every checkpoint is **developer-verified, not Codex-accepted, not desktop-exercised, not listened
to, not game-verified**. Claims: `dev-*` under `local/coordination/overnight-2026-09-24/claims/`.

## Checkpoints

- **01 — P0 / F1: the texture write is bound to the plan the operator saw** — delivered:
  [receipt 01](01-p0-f1-reviewed-texture-write.md). Red reproduction recorded, then a server-held
  review (target bytes, raster, source identity, displayed policy) that both import routes must
  name; drift refuses before backup / mutation; the dialog re-plans for display only; the nameplate
  editor's Save is a reviewed two-step. 8 route cases + 4 new jsdom cases; private-fixture sweeps
  green under the new signature.
- **P1 — completion ledger** — [LEDGER.md](LEDGER.md): per operation and family, user-facing
  complete vs internal vs blocked vs n/a; no offline-actionable Phase 1 gap without a new contract;
  three Teams packages selected.
- **02 — T1 Teams 2.2 relationships parity** — delivered: [receipt 02](02-t1-team-relationships-parity.md).
  Arena assignment with a tenant / vacancy disclosure before the write (`arena_assignment_preview`,
  the `ArenaSelect` two-step); 27-anchor sweep: home arena on 15 anchors (13–Legacy + PS4; 07–12
  measured without the link), retired numbers 26, draft trades 18, strategies 27 — each edit →
  validation → save → reopen → undo.
- **03 — T2 Teams 2.6 PS3 NHL 12 / 13 kit members replaced in their render archives** — delivered:
  [receipt 03](03-t2-ps3-kit-member-replacement.md). R76 consumed (members are `chunkzip`; the
  receipt-04 / STATUS blocker wording corrected by dated notes); the existing archive-member route
  proven end to end on a synthetic two-archive install and planned on the real NHL 13 member (1.6 GB
  archive not copied); kit card offers Replace… (loose) / Replace in archive… (archived); the
  NHL 11 / 12 sock pair is not offered file-by-file.
- **04 — P4.1 + P4.2 PS4 EA-MP3 diagnostic preview** — delivered:
  [receipt 04](04-p4-ps4-ea-mp3-diagnostic-preview.md). Decoder decision: `symphonia-bundle-mp3`
  0.6.1 (pure Rust, MPL-2.0). New crate `tdb-snd` (SND10 banks, SNS blocks, the R67 / R71 refusing
  grammar, frame-by-frame decode, candidate A vs MPEG-only renderings, WAV). 24 / 24 R71 vectors;
  the real NHL 17 tree lists 4 banks (40 018 samples) and previews. Speech-page card with an owned
  audio element.
- **05 — P4.3 + P4.4 gen7 EALayer3 v1 preview and horn sources** — delivered:
  [receipt 05](05-p4-gen7-ealayer3-preview-and-horn-sources.md). Granule grammar, reservoir rebuild
  and decode on the measured tuples only; 116 / 116 R77 + R80 vectors (both rate classes, mono and
  stereo); real X360 15 pbp / PS3 15 ann previews; name-call chips decode. Arena venue columns read
  against the numbered `audio/sfx` banks under R78 / R79 / R81 (file / archive member / chant
  candidate / no binding / inspected-sources residual); real X360 NHL 15 + NHL 09 match the R81 classes.

- **07 — P4.5 (i) preview hooks** — delivered: [receipt 07](07-p45-preview-hooks.md). The gen7
  diagnostic preview is reachable from the commentary catalogue's phrase chips and the player
  card's name / announcer chips (bank stem carried per row); UI only; vitest 35 files / 229.

- **08 — P4.5 (iii) archived gen7 banks / streams** — delivered: [receipt 08](08-p45-archived-gen7-banks.md).
  The gen7 preview resolves banks and streams from the audio archives when not loose (EB / BIG4;
  chunkzip / RefPack); real PS3 NHL 13 and NHL 09 archived banks preview; gen7 music banks (SBKR under `audio/music`, stereo) preview through the same route (API only). Domain + client type.

- **09 — P4.6 gen7 bank listing card** — delivered: [receipt 09](09-p46-gen7-bank-listing.md). Play-by-play,
  announcer and music banks listed from their SBKR table with header-measured codecs; music admitted
  (SBKR, measured) and previewed stereo; real X360 NHL 15 music 263 / 263; vitest 36 / 231.

- **10 — R85 consumed (rows 5 / 7): loader asset table + PS4 horn consumer** — delivered (25 Sep, 05:05–05:50):
  [receipt 10](10-r85-loader-table-and-ps4-horn-consumer.md). The gen7 horn / chant notes read
  `audio/na_en_loadfile.xml` (loose or archived: chunkzip / chunklzx / RefPack measured on four real
  trees); residual ids say "no loader asset for this id"; goalie vs team chant banks told apart; PS4
  `goalhorn` names `goalhornpatch.ptc` (undecoded), `periodhorn` 0–2 the NHL 21 enum candidate. One
  departure from R85 pinned: X360 NHL 15 lists chant 0587 but ships 0584. Read-only, no UI change.

- **11 — P3.1-A Players 3.1 identity data on every anchor; weight base fixed** — delivered (25 Sep, 05:50–06:15):
  [receipt 11](11-p31a-player-identity.md). 27-anchor sweep: all identity columns present; headers
  compose from the probed encodings; names are UTF-8 everywhere; seven identity edits → validate →
  save / reopen → undo on every anchor; refusals read as text. **Defect fixed red → green:** the
  weight base came from the rating-scale probe and was 30 lb wrong on PS4 (display and writes) and
  NHL 08; now probed from the weight column (150 on NHL 07–10, 120 after incl. PS4), pinned 27 / 27.

- **12 — P3.2-A roster-list Overall column era-gated** — delivered (25 Sep, 06:15–06:45):
  [receipt 12](12-p32a-roster-overall-column.md). The list / lines / contracts OVR ran the NHL 14 / 15
  regression on every title (NHL 13 ~13 low; NHL 07–12 printed 1–6); it now follows the Attributes
  tab's rule (derived on `c_*` with coverage, else stored, else nothing). Red → green on 8 anchors;
  NHL 14–21 unchanged. studio-domain lib 222 passed, 0 failed, 2 ignored.

- **13 — P3.2-B ratings / style / potential swept on 27 anchors; six-bit scale probe narrowed** — delivered
  (25 Sep, 06:45–07:03): [receipt 13](13-p32b-ratings-scale-and-sweep.md). Skater + goalie per anchor:
  rating → 88, style, potential; refusal by range; save / reopen / undo. **Defect fixed red → green:**
  NHL 08 was shown on the 36 scale (every rating 14 low) because the probe read any six-bit value
  above 49 (33 goalie `g_positioning` = 50 cells); it now counts rating cells only with a 1 % rule
  (50 on NHL 07–10, 36 on 11–Legacy, PS4 0..99). Attributes caption follows the offset; raw table id
  behind a tooltip. vitest 37 / 234.

- **65 — consolidated receipt for the dispatch after receipt 60**: receipts 61–64, r64 bundle, remaining beta scope, next task PS4 hair. [65](65-consolidated-after-receipt60.md)
- **64 — artwork kept for internal builds (recorded, rights unchanged); NHL 11–13 companion-style lock; r64 bundle** — delivered (28 Sep). [64](64-internal-bundling-and-r64.md). r64: `E:\nhlms-review-build-r64-2026-09-28\NHL Modding Studio rc-20260928-r64\` (exe `b7ab3ce5…`, zip `6f88b5f7…`), receipts through 64. Cards V-94, V-95.
- **63 — PS4 head reassignment; applies need the checked plan** (R15 / R124) — delivered (28 Sep). [63](63-ps4-head-reassign.md)
- **62 — Recompiled player edits through the public path (regression)** — delivered (28 Sep). [62](62-recompiled-public-edits.md)
- **61 — Hockey Shop: a failed undo keeps the original (coordinator F1)** — corrected (28 Sep). [61](61-hockey-shop-rollback-f1.md)
- **60 — consolidated receipt for the dispatch after receipt 45** (28 Sep, 04:54 → about 08:10 MDT): receipts 46–59, r59 bundle,
  ranked October 1 blockers, next task PS4 head reassignment. [60](60-consolidated-after-receipt45.md)
- **r59 review bundle (28 Sep 07:50–08:00)** — `E:\nhlms-review-build-r59-2026-09-28\NHL Modding Studio rc-20260928-r59\` (exe `940a24c0…`), the verified extraction of
  `rc-20260928-r59.zip` (`aaf1e4fb…`); receipts through 59; inputs identical after the build; bundle tests from an unrelated folder
  pass; internal candidate, not release-ready by its record. Cards V-91 … V-93, A-XMA-8.
- **59 — Legacy Recompiled: player edits unblocked (defect: title id `legacyrecomp` unmatched) and sound banks from its own archives** (R138) — delivered (28 Sep; after r56). [59](59-recompiled-id-and-audio.md)
- **58 — PS3 venue-art pictures; X360 NHL 11 / 12 / 14 and PS3 NHL 14 in the gallery** (R137) — delivered (28 Sep; after r56). [58](58-venue-art-ps3-previews.md)
- **57 — PS4 gear and mask names in the title's own words** (R141) — delivered (28 Sep; after r56). [57](57-ps4-creation-zone-names.md)
- **r56 review bundle (28 Sep 07:01–07:10)** — `E:\nhlms-review-build-r56-2026-09-28\NHL Modding Studio rc-20260928-r56\`
  (exe `6eea54c1…`), the verified extraction of `rc-20260928-r56.zip` (`d918ee6a…`); receipts through 56; inputs identical
  after the build; bundle tests from an unrelated folder pass; internal candidate, not release-ready by its record. Cards V-89, V-90, A-XMA-7.
- **56 — X360 NHL 10 / 15 / Legacy stream sounds of every shape** (R140b) — delivered (28 Sep). [56](56-ast-xma-streams-10-15-legacy.md)
- **55 — portrait / head reassignment on archive-held installs (PS3 NHL 09–14, Recompiled)** (R134 / R138) — delivered (28 Sep). [55](55-archive-held-reassign.md)
- **54 — PS4 goalie mask pictures** (R135 §5) — delivered (28 Sep). [54](54-ps4-goalie-mask-thumbnails.md)
- **r53 review bundle (28 Sep 06:23–06:26)** — `E:\nhlms-review-build-r53-2026-09-28\NHL Modding Studio rc-20260928-r53\`
  (exe `13965696…`), the verified extraction of `rc-20260928-r53.zip` (`9899c076…`); receipts through 53; inputs identical
  after the build; internal candidate, not release-ready by its record. Cards V-86 … V-88, A-XMA-5 / 6.
- **53 — NHL 11 / 13 / 14 `.ast` XMA streams** (R140) — delivered (28 Sep). [53](53-ast-xma-streams-11-13-14.md)
- **52 — PS4 skater gear lists and pictures** (R136) — delivered (28 Sep). [52](52-ps4-skater-gear-catalogue.md)
- **51 — PS4 goalie mask pairs (G3) and a Rust vault reader** (R135 / R7) — delivered (28 Sep). [51](51-ps4-goalie-mask-pairs-g3.md)
- **50 — X360 in-bank XMA residuals; (0, 0x7FFF) false refusal fixed** (R139) — delivered (28 Sep). [50](50-xma-inbank-residuals.md)
- **49 — read-only gen7 venue-art gallery** (R127) — delivered (28 Sep). [49](49-venue-board-art-gallery.md), observation [49](observations/49-R127f-slot1-past-block.md)
- **r48 review bundle (28 Sep 05:22–05:25)** — `E:\nhlms-review-build-r48-2026-09-28\NHL Modding Studio rc-20260928-r48\`
  (exe `73e16eb5…`), the verified extraction of `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r48.zip`
  (`5f59eba7…`); receipts through 48; internal review candidate, **not release-ready** by its own record (copyright holder;
  artwork rights). Evidence `review-build-r48/`. Cards V-82 … V-85.
- **48 — grouped schedule generation (R123 single phase)** — delivered (28 Sep): groups mode; shipped groups only by exact
  R113 identity (41 anchor instances + genuine NHL 20); host-rule values and per-pair hosts checked; independent post-check.
  [48](48-schedule-grouped-single-phase.md), observation [48](observations/48-R123-validator-gaps-and-phase-decision.md)
- **47 — Hockey Shop NHL 12 write regression** — delivered (28 Sep): applied create / overwrite on a disposable copy; title
  from the open project; reviewed-plan apply; rollback. **Defect fixed:** NHL 12 shared meshes read as pads 0 / 2 / 3
  (receipt 45's "44" was 41 + 3). [47](47-hockey-shop-write-regression.md)
- **46 — MIT metadata, About & Licences, notices, release gate** — delivered (28 Sep): 11 missing texts retrieved at exact
  upstream versions, 4 packages measured out of the bundle, WebView2 SDK notice, prerequisites corrected, startup message;
  holder and artwork rights pending. [46](46-release-notices-mit.md)

- **45 — consolidated close of the dispatch after receipt 40** — [45](45-consolidated-after-receipt40.md): corrected beta
  blocker list, archive / bundle paths and hashes, checks, next task.
- **44 — NHL 08–11 conference / division labels (R133); NHL 07 header division defect** — delivered (27 Sep, after the r43
  bundle). [44](44-group-names-nhl08-11.md)
- **43 — release-candidate archive + r43 review bundle** — delivered (27 Sep): one verified zip with the helper's source and
  notices; bundle `E:\nhlms-review-build-r43-2026-09-27`. [43](43-release-archive-candidate.md)
- **42 — team goalie mask paints (G2)** — delivered (27 Sep). [42](42-team-goalie-mask-paints.md)
- **41 — schedule generation keeps retained rows in place; V-76 recipe** — delivered (27 Sep): red → green on both
  generators. [41](41-schedule-preservation.md)
- **40 — goalie gear file check** — delivered (27 Sep, after the r39 bundle): NHL 12 pads read at the measured position
  (defect; pads 54: 0 → 20 files), NHL 08 / 09 mask paint checked. [40](40-goalie-gear-file-positions.md)
- **39 — consolidated close of the overnight 27 Sep run** — [39](39-consolidated-overnight-0927.md): the October 1
  blocker list, provenance, the final review bundle and checks.
- **38 — portrait / head reassignment; NHL 08 / 09 goalie mask paint** — delivered (27 Sep): guarded reassign to shipped
  assets per R124 (X360 09 → Legacy, PS3 15 / Legacy, PS4 portraits), API + UI; `maskcolor` notes + regression. [38](38-portrait-head-reassign.md)
- **37 — logo resize on the NHL 07 → 11 package form** — delivered (27 Sep): ARGB8 RW4xb2 resize under R132's rule
  (4 634 / 4 634 descriptors reproduced), Apt rewrite incl. wrapped member 0 and in-place same-length members; PS3 package
  slot resize. [37](37-old-logo-resize.md)
- **36 — PS3 NHL 09 / 10 / 12 / 13 music; X360 NHL 09 banks** — delivered (27 Sep): R82 12 / 12; R131 reproduced through
  Studio's helper (471 streams, 1 901 records, the 2 short refused). [36](36-audio-admissions-r82-r131.md)
- **35 — style labels per title (defect R122 S1 / S2)** — delivered (27 Sep): red → green. [35](35-style-label-scoping.md)
- **34 — user-supplied pair-quota schedules; round-tag refusal** — delivered (27 Sep), + R123 Q11 / Q12 addendum.
  [34](34-schedule-pair-quotas.md)
- **33 — XMA speech / music listing, `.ast` XMA streams, Cancel** — delivered (27 Sep). [33](33-xma-listing-previews.md)
- **32 — the XMA2 helper built** — delivered (27 Sep): MSYS2 toolchain (user-approved), pinned LGPL FFmpeg 8.1.3,
  reproducible, 83 / 83 bit-identical. [32](32-xma2-helper-build.md)
- **31 — consolidated close of the after-receipt-27 prompt** — delivered (26 Sep): goalie gear 4.5 swept on 25
  anchor directories + genuine NHL 20 (stored numbers, refusal, undo / redo, save / reopen); SBKR reader skips
  all-zero slots → X360 NHL 14 announcer Speex admitted; British nationality code kept as "England" (NHL 10–12)
  after the coordinator's R119–R121 review. Next: wire the SNS EA-XMA route into the X360 NHL 10–12 speech /
  music listing behind the helper gate. [31](31-consolidated-after-receipt27.md)
- **30 — EA-XMA helper route (Studio side)** — delivered (26 Sep): RIFF XMA2 wrapping, a controlled helper
  runner (explicit path, no shell / `PATH`, bounded, cancellable, kill-and-reap, hidden window), X360 NHL 10 →
  Legacy ram records through an installed helper, framed-only without one; 83 R84 vectors exact through the
  runner with a local oracle. **Helper build blocked** (no make / MinGW / MSYS). [30](30-xma2-helper-invocation.md)
- **29 — EA-Speex diagnostic preview** — delivered (26 Sep, after receipt 28): pinned upstream libspeex
  1.2.1 (tarball hash = Xiph's list; vendored unmodified) behind a `cc`-compiled C shim, no libclang;
  96 R83 vectors within 1 LSB of the libspeex oracle; 10 gen7 tuples and the PS4 NHL 15–21 rink-announcer
  banks preview through the domain; X360 NHL 14 `ann_eng` refused (bank-reader blocker). Not in the 26 Sep
  review build. [29](29-easpeex-preview.md)
- **28 — after receipt 27, first checkpoint** — delivered (26 Sep, after the receipt-27 review):
  receipt-27 corrections (Speex evidence narrowed, ledger tournament row, consistency import through
  save / reopen); **title-scoped nationality labels** (red: NHL 10 raw 81 shown as "Croatia"; green on
  25 anchor directories, import lookup, birth-state country region); range audit on real NHL 20 /
  NHL 21 (0..255, no truncation; stored-code box beside labelled dropdowns); read-only goalie-Overall
  input comparison (formula unchanged; PS4 15 / 16 proposal). [28](28-after-receipt27-corrections-labels.md)
- **27 — packages 23–26 review continuation (consolidated)** — delivered (26 Sep, 03:40 onward):
  [receipt 27](27-review23-26-continuation.md). Goalie `consistency` stored-value classification (red → green on 7 titles;
  goalie-Overall fingerprints unchanged); the ledger's current-authority section; the decoder recommendation (libspeex
  for Speex, XMA2 waits on the licence decision); read-only tournament browse labels (World Cup names by R115 J1); the
  4.2 / 4.3 glove + helmet / visor sweep on 27 anchors and the real NHL 20.

- **26 — P2.6-N nameplate association** — delivered (26 Sep, 03:28–03:35):
  [receipt 26](26-p50-nameplate-association.md). Each kit row names its `name_<style>_<art id>_<variant>_cm`
  sheet on the six R103 trees (present on 659 / 703 / 659 / 703 / 676 / 684 rows = R103), shown beside the
  kit, never counted in completeness; other titles not looked up.

- **25 — P4.9 four- / six-channel and loop sound-bank previews** — delivered (26 Sep, 03:20–03:28):
  [receipt 25](25-p49-multichannel-and-loop-previews.md). Channel configs 3 / 5 preview one stereo pair
  at a time (research's inferred de-interleave, labelled); stream loops as stub PCM + one body pass
  with the loop point shown (body framed at the stub's end, stub + body = count). R100's cc 3 vectors
  and PREVIEW_REVIEW #4–#9 reproduce exactly; NHL 13's one cc 5 entry decodes as three pairs.

- **24 — P4.8 archive-backed PS3 sound banks, arena chant wording, horn-to-bank links** — delivered
  (26 Sep, 02:40–03:12 real clock): [receipt 24](24-p48-archive-sound-banks-chant-wording-horn-links.md).
  PS3 NHL 09 / 10 / 12 / 13 banks listed, framed and previewed from each title's own archive members
  (NHL 10's four loose streams included) with the stream id checked on the reading handle; the census
  equals R82 / R98 on every title (186 / 199 / 219 / 213 banks; 506 / 1 108 / 3 719 / 3 708 streams
  framed, 0 refused) and R100's vectors reproduce (81 768 / 49 382 / 33 207 / 133 749 samples). X360
  NHL 09 refused. Chant ids follow R97 §1.4 (never resolved or linked); gen7 goal / period horns link
  to their bank. Two defects found and fixed in the run: a cc 1 loop marked previewable in the tree,
  and 30 GB of archive hashing per census (new `BoundBig4::read_directory`).

- **23 — review F1 / F2: stream identity on the reading handle; bounded layout signature** — delivered
  (26 Sep): [receipt 23](23-f1f2b-review-corrections.md). Missing / malformed / mismatched /
  replaced stream files refuse stream entries in the tree and the direct route; ram entries unaffected.
  `schema_family` is a descriptor signature measured on all 27 anchors; unmatched layouts are raw and
  read-only; an unsupported declared title / platform is a conflict; both import seams refuse under it.

- **22 — P4.7 gen7 sound banks (`.abk` / `.ast`): listing, picker tree, framing, diagnostic decode** — delivered
  (25 Sep, 15:10–22:07 real clock): [receipt 22](22-p47-gen7-sound-banks.md). New `tdb_snd::abk` (R82 / R89 / R91 /
  R92 / R94) and `studio_domain::gen7_sfx_banks`; 3 routes, 6 DTOs; Speech catalogue card. Real PS3 / X360
  NHL 15 pinned to the R89 counts and PREVIEW_REVIEW #1 (33 207 samples). X360 framed only (no XMA2
  decoder); archive-backed titles and PS4 refused in words. Closing aggregate: domain lib 222 / 0 / 2;
  operation matrix 1, no CSV drift; vitest 40 / 243; tsc clean; bindings in sync.
- **League / Team ledger refreshed against R88–R101** — [LEDGER.md](LEDGER.md) §"Refresh 25 Sep".

- **21 — review item 8e: title-scoped equipment vocabularies** — delivered (25 Sep, 14:40–15:10):
  [receipt 21](21-p8e-title-scoped-equipment-vocabularies.md). `[scope] games = ["nhllegacy"]` on the
  ten Legacy-established equipment TOMLs; the enum registry resolves a scoped vocabulary only for a
  project opened as an in-scope title; `FieldViewDto.vocabulary_note` (bindings regenerated) carries
  provenance or the "no catalogue for <title>" reason; FieldRow shows the marker; hockey-shop /
  equipment-file labels scoped by the project's title. Writes unchanged — any in-range id round-trips
  (27-anchor sweep). tdb-enums 7; equipment_scope 2; player_sticks 1; vitest 39 / 239; tsc clean.

- **20 — F1 / F2 encoding contract: weight, rating, birth-date and cap-hit encodings from the schema family, not the cells; cap-hit units witnessed by 6–25 public contracts per family** — delivered
  (25 Sep, 13:20–14:40): [receipt 20](20-f1-f2-encoding-contract.md). New `studio_domain::encoding`
  (`SchemaFamily` from the schema shape; `PlayerEncoding` with evidence levels; title / schema conflict
  → encoded cells refuse in words; unresolved schema → raw, read-only). **Red → green** on disposable
  copies: bulk 170 lb / all-zero weights / lowered or raised ratings no longer move the base or scale
  through display, edit form, re-applied value, undo, redo, save → reopen; the import seam follows.
  NHL 08 cap-hit unit measured (7 contracts; receipt 17's Crosby explanation withdrawn); real NHL 20
  private fixture carries the PS4 contract. Domain lib 222 passed, 0 failed, 2 ignored; 27-anchor
  identity / ratings sweeps green.

- **19 — P4.1-A hockey sticks swept on 27 anchors** — delivered (25 Sep, 07:50–08:00):
  [receipt 19](19-p41a-sticks-sweep.md). Stick model / curve / flex / tape columns recorded per
  title (widths, ranges, catalogue labels), edited, range-refused, saved / reopened, undone; NHL 07
  has no companion (stated). Finding for the coordinator: the Legacy-anchored stick catalogue
  labels are applied unscoped on every title — provisional off Legacy.

- **18 — P3.3-A player text inventory (biography strings vs dictionary tokens vs audio-name ids), 27 anchors** — delivered
  (25 Sep, 07:44–07:50): [receipt 18](18-p33a-player-text-inventory.md). Names / nicknames / birth
  cities are inline UTF-8 strings everywhere; eight PS4 17–21 "Joe NHL" template rows carry
  `$BURNABY` (dictionary-resolvable); `audioid` (+ NHL 07 `defaultaudioid`, NHL 21 `surnameid`) is
  the audio-name reference. Read-only; ledger 3.3 filled.

- **17 — P3.2-F contract dollars: NHL 08–10 unit measured (×$25 000), PS4 18–21 unit confirmed (×$5 000)** — delivered
  (25 Sep, 07:38–07:44): [receipt 17](17-p32f-contract-dollar-units.md). Crosby's stored 348 reads
  $8 700 000 on NHL 09 / 10, McDavid's 2 500 reads $12 500 000 on PS4 19–21; 28 public cap hits
  pinned through the player card; the "salary scale unconfirmed" note leaves NHL 08–10. Domain lib 222 passed, 0 failed, 2 ignored.

- **16 — P3.2-E inline strings carry their byte width; over-long names refused before the write** — delivered
  (25 Sep, 07:26–07:40): [receipt 16](16-p32e-string-width-precheck.md). `FieldViewDto.text_max_bytes`
  (bindings regenerated); the inline editor counts UTF-8 bytes and shows the refusal beside the
  value with nothing sent; identity sweep pins the width and the writer's matching refusal.
  vitest 38 / 236; tsc clean.

- **15 — P3.2-D full roster block refuses a move / added membership in words, 27 anchors** — delivered
  (25 Sep, 07:20–07:34): [receipt 15](15-p32d-roster-full-refusal.md). The refusal names the team
  (abbreviation fallback on PS4 16–21 without a dictionary) instead of "team 0 roster block is full";
  nothing moves, no undo entry; new engine variant `EditError::Refused` (verbatim), also used by
  the NHL 07 slot-key refusal. Engine 41 passed (6 targets); domain lib 222 passed, 0 failed, 2 ignored.

- **14 — P3.2-C historical import on 27 anchors; NHL 07 refusal in words; Phase 3 ledger** — delivered
  (25 Sep, 07:03–07:20): [receipt 14](14-p32c-import-sweep-and-phase3-ledger.md),
  [LEDGER-PHASE3.md](LEDGER-PHASE3.md). Every probed encoding flows through the import writer and
  reads back on 25 anchors (save / reopen); NHL 07 refuses create / clone / import before the
  transaction with the reason in words (was a raw field id). Operation matrix 18 passed across the five targets (operation matrix 1 passed, no CSV drift, 334 s).

## Checks

Per-package logs under
[`docs/validation/evidence/overnight-development-2026-09-24/logs/`](../../validation/evidence/overnight-development-2026-09-24/logs/):
`p0-f1-{red,green,private-fixtures}.log`, `t1-team-relationships-sweep.log`, `t2-checks.log`,
`t2-ps3-kit-member-real-plan.log`, `p4-checks.log`, `p4-ea-mp3-real-vectors.log`,
`p4-ps4-speech-bank-real-nhl17.log`, `p4-ealayer3-real-vectors.log`, `p4-gen7-speech-preview-real.log`,
`p4-gen7-horn-sources-real.log`, `p47-r85-loader-table-real.log`, `p47-gen7-sfx-banks-real.log`, `closeout2-{domain-lib,operation-matrix,crates,frontend}.log`, `closeout2-csv-{before,after}.txt`, `p8e-equipment-scope-sweep.log`, `p8e-tdb-enums.log`, `f1-encoding-contract-{red,green,families}.log`, `f1-domain-{focused-suites,lib}.log`, `f1-api-sweeps.log`, `f2-contract-witness-candidates.log`, `f2-real-nhl20-encoding.log`, `p31a-player-identity-sweep.log`, `p31a-weight-base-{measurement,red,green}.log`, `p32a-roster-overall-{red,green}.log`, `p32b-rating-scale-{measurement,green}.log`, `p32b-player-ratings-sweep.log`, `p32c-import-sweep.log`, `p32c-operation-matrix-rerun.log`, `p32d-roster-full-{red,green}.log`, `p32d-regression-suites.log`, `p32f-contract-scale-{measurement,green}.log`, `p33a-player-text-columns.log`, `p41a-player-sticks-sweep.log`, and the closeout aggregate `closeout-aggregate.log` (its results are
summarized in the [closing receipt](06-closeout.md)). `zz_diag` was never invoked. Pre-existing
repo-wide fmt / clippy failures (C0.2) are untouched and not re-measured.

## Blockers (recorded, not campaign stoppages)

- Xbox 360 packed-tail layout (Q-R65-1; H1 a candidate): resized writes into mipped rasters stay
  refused; same-size writes stay disclosed partial replacements.
- League / team creation, cloning, deletion: no reference / allocation contract.
- Schedule generation for NHL / Swiss / AHL formats; tournament table binding; Season / Franchise
  schedule copies: research unknown.
- Gen7 settings (compiled `default.xex` table); X360 07 dictionary labels.
- Audio: replacement / encoding / event and bank mutation — no contract; the audible start of both
  previews (A-MP3-1, A-EAL3-1) is the user's to judge; horn banks are located, not decoded; music
  banks preview through the API but have no listing card (receipt 08 closed the archived-bank and music-index gaps).
- NHL 11 / 12 sock pairs: per-pair mirror class not measured (R74 aggregates only).

## Research published during this session (read at closeout)

The research lane's 2026-09-24 campaign published R82–R90 (`docs/research/overnight-2026-09-24/`,
queue rows 1–13) while these packages ran. **R85 was consumed by package 10 (25 Sep)**; the rest is
unconsumed and nothing here depends on it. Recommended next consumers, in the queue's order:

- ~~**R85 loader asset table** (row 5 / 7)~~ — consumed: [receipt 10](10-r85-loader-table-and-ps4-horn-consumer.md).
- **R82 archive-backed coverage** (row 2): `music_samples` on PS3 09 / 10 / 12 / 13 from the archives
  — the receipt-08 archive resolution likely covers it once the tuples are added to `MEASURED`.
- **R83 EA-Speex (codec 0x09)** and **R84 EA-XMA** (rows 3–4): two further diagnostic-preview
  contracts (X360 announcer / pbp 11–Legacy, PS3 14 pbp; EA-XMA sfx / music on X360) — each needs its
  own decoder decision (Speex UWB; XMA2) before any route.
- **R89 `ABKC` bank tree** (row 12) and **R90 team-chant binding** (row 13): the `.abk` horn / chant
  banks' sample lists and the club-chant banks — listing contracts for the arena page.
- **R86 / R87 with the R88 corrections** (rows 9–11): the team art-id graph and row-allocation
  constraints — read-only inspection is consumable; the re-key and creation contracts stay unfinished.

## Deferred cards

[MORNING-REVIEW.md](MORNING-REVIEW.md): desktop V-45 … V-53, listening A-MP3-1 / A-EAL3-1, runtime
R36-PS3-KIT-1 / H-HORN-1. Earlier campaigns' cards stand; NHL 21 E1 closed.

## One exact next task

See the [closing receipt](06-closeout.md).
