# Research lane status — 2026-09-25 (overnight; asset / audio research continued from the 2026-09-24 correction checkpoint)

Opened 05:36 MDT (real clock, `date`). Baseline: HEAD `8d60a6d` on `feat/phase5-asset-tooling`; the developer's working tree dirty
(preserved, untouched); `docs/research/` and `local/` untracked. Probes: Python 3.12.10, numpy 2.5.1, ffmpeg 8.1.1 (`mp3float`,
`speex`, `libspeex`, `xma1` / `xma2`, `opus` decoders present) as decoder oracle only; no vgmstream binary on this machine, scipy absent —
no tool downloaded. Free space at open: E: 36 GB, H: 29 GB (scratchpad and previews), X: 3.9 TB (read-only source trees). No
launches, no listening, no writes outside this lane's roots, nothing staged / committed / pushed.

Lane roots this session: `docs/research/overnight-2026-09-25/**`, `local/runs/overnight-research-2026-09-25/**`,
`local/coordination/overnight-2026-09-25/claims/**`, plus dated correction notes inside the 2026-09-24 package (cards, queue, claims)
where a result of this session supersedes a statement there. Task queue: [TASKS.md](TASKS.md).

## Corrections carried forward (binding for every statement in this package)

1. `anchors/ps4/nhl20` is a mislabeled NHL 19 base copy (handoff 06 §F1) — never cited as NHL 20 evidence; real NHL 20 = CUSA15439
   install (`1e200478…`).
2. Table occupancy and observed distributions do not establish capacity limits or successful allocation (handoff 06 §F2).
3. An unused database art id is not necessarily asset-free (handoff 06 §F3: 156 "free" Legacy ids carry files).
4. Bank indexing and stream location (R89) do not establish decoding or replacement (handoff 06 §F4).
5. The NHL 21 single-block Speex correlation outlier (PS4 NHL 21 base `rink_ann_samples`, corr 0.46) is unresolved until this
   session's task 2c says otherwise, with its evidence.
6. The 509-sample Speex start is a count rule (block arithmetic + the pinned reference decoder's rule), not an observed game-playback trace.

Additional boundaries kept: "decoder returned audio" is decoder acceptance, never correctness; nothing here is game-verified; a
decoded preview is not replacement support; ~~the compiled crowd module's selection key is not in the database (R90)~~ **(withdrawn 14:20, R97 —
no selecting consumer traced either way; see correction 7)** and runtime selection is not demonstrated anywhere.

7. **(added 14:20, R97)** The chant-selection sentences "the only database value in 55–66 …", "only database path", "no database field selects
   them", "key is code-side" are withdrawn as stated ([handoff 10](10-correction-checkpoint-direction-audit.md) §1): the Finnish arena's chant,
   goal-horn and period-horn ids are all 61; SHL clubs without a club bank carry team `audioid` 64–66; no selecting consumer traced.
8. **(added 14:20, R97)** The NHL 21 single-block Speex record is "near-silent output under both tested decoders; unsuitable for validating
   onset or decoder fidelity" — never "silence" / "silent placeholder" / game-silent (handoff 10 §2). Correction 5 above reads accordingly.
9. **(added 14:20, R97)** Denominators: PS3 loose trees = NHL 11 / 14 / 15 / Legacy; controls = 15 named, 14 detected + 1 undetectable per
   platform; decoded = 14 (R92) + 106 (R96) vectors, never every stream; restart identity = reference-decoder property (handoff 10 §3).
   Archive-backed titles are R98's own denominators (handoff 11), not R91's.
10. **(added 14:20, R97)** Everything in this lane is research-owned; MANIFEST is a hash list, not writer-recovery evidence (handoff 10 §4).

## Progress log (appended after each meaningful result; times are real clock, MDT)

- 05:36 — package opened; prior handoffs 06–08, STATUS, queue, cards, claims and probes r82 / r83 / r84 / r89 / r90 read; tool
  inventory recorded above.
- 05:48 — first vector inspected (PS3 NHL 15 `playercalls.ast` stream 0, 34 blocks, 33 207 samples; X360 NHL 15 same record):
  PS3 block payloads are independent EA-frame chains (middle blocks tile with 0 trailing bytes; block 0 = 2 warm-up granules + one
  47-sample PCM frame; the last block ends in zero padding that absorbs the 0x40 alignment), X360 block payloads are `u32 w0 | one
  XMA2 packet | zero padding` with `w0 >> 2` = packet length + 4. This explains R89 §4's "does not tile": the earlier test parsed the
  last block's zero padding as a frame. Not yet a corpus result — see task 1.

- 05:50 — **R91 framing, corpus (task 1 / 1b / 1c complete).** `r91_ast_framing.py` walked every stream entry of every loose bank on
  the 11 trees (29 962 referenced streams + 146 loop bodies, 137 `.ast`, 1.45 GB) in 57 s: 0 refusals; every block payload tiles
  under its codec's grammar (PS3: EA-frame chains, zero padding only in the terminal block; X360: `w0 | XMA2 packet` sub-blocks, 1 per
  stream); every `.ast` is byte-accounted (header + block headers + payloads + terminal padding + zero inter-stream gaps = file size, 0
  unexplained bytes); block sample words: middle blocks exact, first block decodable − declared = 1152 per stream (EAL3) / 384 (XMA)
  on every non-loop multi-block stream, terminal trim 1153–2304 samples (EAL3) / 0–884 (XMA); MPEG-1 granule index alternates across
  block boundaries on 100 % of PS3 streams (per de-interleaved sub-stream); loops 146 / 146 (stub = one block = words (a, b); body
  unreferenced, fresh start, stub + body = num_samples); 14 / 15 in-memory corruption controls detected per platform, the deep
  main-data / packet-body flip undetectable structurally (stated). Data: `data/r91-ast-framing.json`, per-stream JSONL and block maps
  under `local/runs/overnight-research-2026-09-25/blockmaps/`.
- 05:56 — **R92 decode + previews (task 2 / 2b complete).** 14 representative vectors (PS3 11 / 14 / 15 / Legacy EAL3 mono / stereo /
  4-channel / loops; X360 10 / 12 / 15 / Legacy XMA mono / stereo / 4-channel loop): EAL3 decodes to granules × 576 on every
  sub-stream; XMA decodes to exactly the declared count with the RIFF header shown not to drive it; a restart decode from the middle
  block converges to **bit-identical** output at the block map's predicted offset (EAL3 after 2 granules, XMA immediately) —
  independent evidence for the block → sample map and the state-continuity requirement; the 47-sample PCM block frame of block 0
  equals decoder output samples 1105–1151 (corr ≥ 0.995 on every non-silent vector; **planar** per channel on stereo) — the
  intended output start is therefore a data-supported candidate (PCM frame + decode from 1152), recorded as a construction, not a rule.
  47 WAVs (62 MB) local under `previews/`. Nothing listened to.
- 05:58 — **R93 Speex outlier (task 2c complete): explained, not a decoder disagreement.** The PS4 NHL 21 base `rink_ann_samples`
  single-block vector is silence: both decoders output peaks of 5 / 4 LSB, rms 0.40 / 0.90 LSB, max abs difference 2 LSB; 10 of 12
  frames are Speex narrowband mode 1 (lowest); correlation 0.46 is rounding noise between two near-zero signals (0.83 on the two
  non-lowest frames). The same title's `_inxp` first vector (143 frames, speech) shows corr 0.98 like the other 95. Consequence: the
  class "single-block NHL 21 base bank" is a silent placeholder record, not a decoder-disputed class; neither decoder is *validated*
  by it (nothing can be), and the 509-count rule is untestable on a silent sample. `data/r93-speex-outlier.json`.

- 06:05 — **R94 bank-tree words (task 3 complete as a bounded result).** The u32 before each name = `u16 bank key | u16 name key`
  (name key constant per name across banks / platforms, 0 exceptions on 1 217 names; bank key constant per bank, shared by families);
  29 hash algorithms × 8 string variants × 4 folds on the name key: best 1 / 244 (chance) — unidentified; X constant per list, clusters
  by channel config, uninterpreted; S platform-independent, 4-byte multiples, cumulative in a non-list non-index order, increments
  2·ch·n + 0.3–1.4 %, none of 14 formulas fits — interpretation open, not needed for addressing. [ABK_TREE_CONTRACT.md](ABK_TREE_CONTRACT.md).
- 06:12 — **R95 chant selection (task 4 complete as evidence, no binding).** Chant banks load by number (loader `descr`, like goal
  horns); the only database value in 55–66 is the Finnish arena's chant id 61; `aemscrwd.csi` = 196-name MOIR table + 12 KB undecoded
  body (u16 words contain 8 name ids, none a club picker); `controlCenter.abk` defines the 12 club pickers as empty class-2 message
  endpoints (no parent, no constants). R90's "selected by the compiled module" narrowed; T-CHANT-1 rev. 2 (observe the Finnish arena
  first). [CHANT_SELECTION.md](CHANT_SELECTION.md).
- 06:19 — documents published: AST_FRAMING, COVERAGE, PREVIEW_REVIEW (51 review rows), ABK_TREE_CONTRACT, CHANT_SELECTION,
  REPLACEMENT_READINESS (both classes unqualified), MANIFEST + `data/manifest.json` (284 inputs hashed, 79 outputs), handoff 09,
  USER-CARDS (A-AST-1, A-LOOP-1, A-SPX-1 rev. 2, T-CHANT-1 rev. 2), DEVELOPER-QUEUE (6 rows); 5 claims `ready`; dated notes on the
  2026-09-24 USER-CARDS, DEVELOPER-QUEUE, handoffs 07 / 08 and STATUS.

- 13:36–13:38 — **R96 decode-contract matrix** ran (106 vectors, exit 0) by an unrecorded continuation after the 06:25 closeout; found at the
  14:00 reopen and registered at the R97 checkpoint (handoff 10 §6): counts confirmed on all 106; restart continuity confirmed; the PCM-frame
  match holds on **loud** frames only (12 / 14 at ≥ 0.95; quiet frames uninformative); terminal flushes carry audio on 15 / 49; int-vs-float
  decoders are not independent. Claim `research-r96-decode-matrix` (created late, real timestamps).
- 13:56 — **session reopened** under the user's instruction to execute the 24 September campaign review and continue the queue; the review's
  F1–F4 were already answered by R88 (handoff 06, re-read, standing); the 25 September direction audit
  ([research-20260925-misrouted-prompt-audit.md](../../validation/research-20260925-misrouted-prompt-audit.md)) had not been applied —
  its corrections become the checkpoint. HEAD still `8d60a6d`; developer tree dirty and untouched; developer claims all `complete`
  (Players / Equipment packages, none on League / Team archive work); disk E: 31.6 GB, H: 28.2 GB.
- 14:20 — **R97 correction checkpoint published** ([handoff 10](10-correction-checkpoint-direction-audit.md)): whole-database 55–66 census
  on the 18 gen7 anchors (`data/r97-chant-id-field-census.json`, 9 s) — the R90 table row "0 arenas reference a club bank" was a transcription
  error (the R90 JSON holds `[61]` on 11 / 11 trees) and that arena carries 61 in `goalhorn` / `periodhorn` as well; Speex wording, denominators,
  boundary, R88 count 27, `*artid*` namespaces, R96 inventory; dated notes on 14 documents of the 09-24 / 09-25 packages; card T-CHANT-1 rev. 3,
  A-SPX-1 rev. 3. Claim `research-r97-correction-checkpoint-audit`.
- 14:35 — **R98 archive-backed `.ast` framing published** ([handoff 11](11-ast-framing-archive-backed-titles.md)): PS3 NHL 09 / 10 / 12 / 13 and
  X360 NHL 09 from their own members (1 240 inputs hashed; every `.ast` member stored, not wrapped): 9 987 streams + 28 loop bodies, 0 refusals,
  byte-accounted; R91 grammar unchanged on PS3; **X360 NHL 09 is a packet-header variant** (header word constant `0x08000000` / `0x20000000`, not
  a frame count; up to 2 packets per sub-block; the sample rule recovered by an in-packet frame walk on 270 / 270 first blocks and 19 138 / 19 225
  middle blocks; 87 middle blocks + the header values + 4 loop bodies kept as residuals; no decode). Claim `research-r98-ast-framing-archived`.
- 14:40 — **R99 started**: Q-R88-2 — team-art members inside the gen7 EB archives (Legacy X360 / PS3, NHL 15 X360 / PS3, PS3 12 / 13): span
  sharing across art ids, `path_hash` formula (rename cascade input), the unknown `w4` word, a bounded PS4 `word_12` hash test. Read-only.

- 15:05 — **R99 published** ([handoff 12](12-team-art-archive-spans-and-entry-fields.md)): gen7 EB archives of Legacy X360 / PS3, NHL 15 X360 / PS3,
  PS3 12 / 13 — **0 duplicate and 0 overlapping member spans on 242 429 rows**; entry row = `block | 0 | size | djb2(lowercase path)` on
  242 429 / 242 429 (+ PS4 NHL 21 `path_hash` 2 950 / 2 950); team-art families re-enumerated from the real stems (small crest `s{id}` absent);
  PS4 `word_12` sorted / non-hash, unknown → PS4 renames stay refused. Claim `research-r99-team-art-archive-spans`.
- 15:30 — **R100 / R101 published** ([handoff 13](13-archived-stream-decode-and-kit-name-key-positions.md)): archived streams decode with
  exact counts (PS3 15 / 15 sub-streams; **X360 NHL 09 3 / 3 through the unchanged R92 route** — handoff 11's expectation withdrawn by note);
  kit / nameplate / rink-logo name keys settled by value-set match (`{style}_{teamartid}_{variant}`; rink logo `texlib_{artid}_{style}_{variant}`);
  PS3 NHL 12 socks = four-number grammar, open. Claims `research-r100-archived-decode`, `research-r101-kit-name-key-positions`.
- 15:35 — **afternoon closeout.** Delivered R96 (registered) + R97–R101; documents: handoffs 10–13, dated notes on 16 documents across the
  09-24 / 09-25 packages, cards rev. 3, queue rows 7–12, TASKS rows 7–12, MANIFEST addenda, 6 new claims. **Falsified this afternoon:** R90's
  "0 arenas reference a club bank" (transcription error; 61 on every anchor); handoff 11's "R92 XMA route not expected to apply on X360 09".
  **Blocked / open:** club-chant selecting consumer (offline: none traced; next lever = an explicit treatment on a disposable copy, not
  authorised); X360 09 residual middle blocks (87) and header values; PS4 `word_12`; PS3 NHL 11 / 12 sock grammar; in-payload references of
  kit members (next task). **Human-review-pending:** unchanged (PREVIEW_REVIEW packet, cards A-AST-1 / A-LOOP-1 / A-SPX-1 rev. 3 / T-CHANT-1
  rev. 3). **Cleanup:** every scratch script deleted (session scratchpad empty); the r100 decode scratch directory removed; no background
  process of this lane running; `gen_docs.py` not re-run (it would re-stamp the claim files); nothing under X:, H: (outside the session
  scratch), `anchors/`, `defs/`, bindings, developer paths or application code written; nothing staged, committed or pushed; `docs/research/`
  and `local/` remain untracked; the developer's working tree as found. **One exact next task:** in-payload references of kit members
  (jersey `.rx2` / `.rpsgl` → nameplate / crest by path or djb2) on Legacy X360 / PS3, read-only (TASKS.md).

- 21:58 — **evening session opened** under the R97–R101 review ([research-20260925-r97-r101-review.md](../../validation/research-20260925-r97-r101-review.md)).
  HEAD `8d60a6d`; developer tree untouched; no new developer claims since 15:35; scratchpad empty.
- 22:07 — **R102 correction checkpoint published** ([handoff 14](14-correction-checkpoint-r97-r101-review.md)): F1 — rename sentence narrowed; EB layout measured on
  71 archives (payload-offset row order, name slot = longest basename, header word `0x10` unexplained, 0 hash collisions); loose copies =
  byte-identical aliases of nameplate / rink-logo members on Legacy / NHL 15 (R103b). F2 — nameplate per-row association (every row, 6
  trees, every discriminating row); rink-logo positions 2–3 candidates; PS3 12 sock "grammar" = R56 / R74 two-file form (closed by existing
  evidence). F3 — X360 09 decode narrowed to 3 vectors with byte-identical PCM under shipped / synthesised / zeroed header words; R96's two
  loud exceptions named. Developer queue split (current C1–C8 + preserved history, row 6 repaired). Claim
  `research-r102-correction-checkpoint-r97-r101`; revisions on r96 / r99 / r100 / r101.

- 22:10 — **coordination:** developer claim `dev-p47-gen7-sound-banks` (complete 22:07) consumed R91 / R92 / R94 and R98 / R100 **as refusal
  wording only** — unaffected by handoff 14's narrowing (refusals stay conservative). Its four observations are acknowledged as research
  facts to carry: (1) the R89 JSON `owner_examples` lists are truncated to eight records (e.g. `hiAmbColorSamplePicker02_msg` holds 21–29,
  `boardPlaySamplePicker_msg` 38–55) — a JSON-sampling limit, not a grammar result; (2) the name-directory class is the high byte of the
  big-endian flags word (consistent with R89's class byte 0 / 1 / 2); (3) `100_goal.abk` (0 records) carries one empty sample list, not
  counted by R89; (4) PS3 terminal-block zero padding must be trimmed before the R77 granule walk (= R91 §1's terminal-padding rule). No
  research claim overlaps the developer's current work; research continues on read-only team-art payload references (R104).

- 22:18 — **R104–R106 published** ([handoff 15](15-team-art-payload-references-and-inspection-graph.md)): Legacy X360 / PS3 kit, nameplate, "rink logo", centre-logo RW4 members
  (5 871 + 5 873, 0 refusals): subreferences 0 everywhere, external arenas = three fixed defaults, name dictionaries name only the member's
  own rasters with its own id tuple (a rename dependency; runtime use open); scoped ASCII / djb2 search of non-payload bytes negative;
  loose = unwrapped member on 72 / 72 samples per tree; "rink logo" = candidate per-kit logo set over the (style, variant) grid (label
  unverified); inspection graph `data/r106-…json` with typed edges and open requirements. Claims r104 / r105 / r106. One interrupted r104
  run (stopped for speed after its X360 half finished; superseded by the complete second run).

- 22:19 — **clock correction:** handoffs 14 / 15, their dated notes, the queue header, CHECKPOINT title, three STATUS entries and claims
  r104–r106 had been stamped with estimated later times (22:40 / 22:45 / 22:50 / 23:30); replaced by the real file times (22:07 / 22:08 /
  22:10 / 22:18). No content changed.

- 22:22 — **R104b** (NHL 15 X360 / PS3): same payload-reference result on 4 743 / 4 745 members; handoff 15 addendum; queue C9 denominators updated.
- 22:22 — **evening closeout.** Delivered R102 correction checkpoint (handoff 14; queue split C1–C9 + history) and R104–R106 + R104b (handoff 15).
  **Withdrawn / narrowed this evening:** R99 rename sentence; R101 "settled"; R101's "PS3 12 sock grammar open" (closed by R56 / R74 + developer
  resolver); R100's decoder-internal explanation; R96 "established on loud frames". **Open:** rink-logo meaning and slots; internal RW4 raster
  names as a rename dependency (runtime use); EB header word `0x10`; loose-vs-archive precedence; PS3 12 / 13 and other titles' payload
  references; PS4 `word_12`; X360 09 residual blocks; club-chant consumer. **Cleanup:** scratchpad empty (patch / exploration scripts deleted;
  two scratch results promoted to lane probes with their JSON / text outputs); no process of this lane running (monitors stopped / ended);
  nothing under X:, anchors, defs, bindings, application or developer paths written; nothing staged, committed or pushed. **One exact next
  task:** static discrimination of the "rink logo" family via RW4 raster descriptors (TASKS.md).

> **Note 14:20:** the "Final checkpoint" and "Actual stopping reason" below describe the 06:25 closeout of the morning assignment. They are
> superseded by this afternoon's continuation (R96–R99, this log; CHECKPOINT.md and TASKS.md rewritten at the afternoon closeout).

## Final checkpoint (see CHECKPOINT.md for the restart commands)

**Completed:** tasks 0–6 of TASKS.md. **Partial:** none. **Falsified:** the R88 "decoder disagreement" reading of the NHL 21 outlier
(silence); R89 §4's "does not tile" (terminal padding); R89's 32-bit hash reading (two u16 keys); R90's "selected by the compiled
module" (narrowed). **Blocked:** the S / X semantics and the 16-bit name key (no candidate fits; no further offline lever named);
the crowd module body and the executable (opcode format unknown; exe encrypted; no tool authorised); archived-title `.ast` members
(not extracted). **Human-review-pending:** the 51-row PREVIEW_REVIEW packet and cards A-AST-1 / A-LOOP-1 / T-CHANT-1 rev. 2 —
none blocks further research.

**Actual stopping reason:** the assignment's queue is exhausted (tasks 1–6 delivered); no independent queued work remains inside the
authorised scope (team allocation and art re-keying are excluded by instruction).

**Owned processes / cleanup:** no background process of this lane running (the single background job exited 0 at 05:50); the
previews' `_scratch` and `_scratch93` directories removed after use; the session scratchpad's helper scripts (`peek_ast.py`,
`tile_test.py`, `x360_inspect.py`, `jsonl_analysis.py`, `sx_explore*.py`, `crowd_look.py`, `patch_r92*.py`, `patch_r95b.py`,
`r91_smoke.*`) deleted at closeout — their durable equivalents are the lane probes; `local/runs/overnight-research-2026-09-25/`
holds probes (8), block maps (46 MB), previews (62 MB, 51 WAV) and two run logs; nothing under X: / H: / anchors / application paths
written (every control re-verified the original file hash); nothing staged, committed or pushed; `docs/research/` and `local/` remain
untracked; the developer's working tree as found.

## Restart checkpoint

See [CHECKPOINT.md](CHECKPOINT.md) (rewritten after each task).
