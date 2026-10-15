# Research lane task queue — overnight 2026-09-25 (asset / audio research; `.ast` framing, previews, R89 / R90 extensions) — morning queue closed 06:25 MDT; **afternoon continuation 13:56 → (see STATUS)** under the 25 September direction audit

Assignment: continue from the 2026-09-24 correction checkpoint (handoff 06) and the R89 / R90 packages. Authorization: read-only over
the source trees; scripts, parsers, decoders, fixtures, tests and generated previews under this lane's new roots only. No launches, no
installs, no commits / staging, no writes to game / anchor / vault / application paths, no touching the pre-existing `defs/enums/*.toml`
deletions. Generated audio stays local (`local/runs/overnight-research-2026-09-25/previews/`, never under `docs/`).

Roots: `docs/research/overnight-2026-09-25/` (this package), `local/runs/overnight-research-2026-09-25/{probes,blockmaps,previews}/`,
`local/coordination/overnight-2026-09-25/claims/`. Status classes: **completed / partial / falsified / blocked / human-review-pending**.

| # | Task | Status | Where |
|---|---|---|---|
| 0 | Package skeleton, provenance carry-forward (corrections list), input manifest with SHA-256 | **completed** (284 inputs, 79 outputs hashed) | STATUS.md, MANIFEST.md, data/manifest.json |
| 1 | **Q-R89-1** `.ast` block payload framing — representative set (PS3 / X360 NHL 15 `playercalls`, `crowds` incl. loop stub / body, cc 3 crowd stream; PS3 11 / X360 10 / Legacy variants) | **completed** | r91_ast_framing.py, AST_FRAMING.md |
| 1b | Standalone bounds-checked parser + block maps + negative controls + in-memory corruption controls | **completed** (15 named controls / platform; deep payload flips undetectable — stated) | r91_ast_framing.py, blockmaps/representative (288 maps) |
| 1c | Corpus expansion (11 loose trees, 137 `.ast`, 30 108 streams) after the controls pass; coverage table by title / platform / record class | **completed** (0 refusals; every file byte-accounted) | data/r91-ast-framing.json, blockmaps/r91-streams.jsonl, COVERAGE.md |
| 2 | Decode supported classes (EALayer3 v1 → MP3 rebuild → ffmpeg `mp3float`; EA-XMA → RIFF XMA2 → ffmpeg `xma2`); count / continuity / loop checks independent of parser success | **completed** (counts exact; restart bit-identical; PCM frame located) | r92_ast_decode.py, data/r92-ast-decode.json |
| 2b | Preview set (straightforward, multi-block, looping, platform variants, outlier) + PREVIEW_REVIEW.md | **completed** (51 rows) — **human-review-pending** (cards A-AST-1 / A-LOOP-1; nothing blocks on it) | previews/, PREVIEW_REVIEW.md |
| 2c | NHL 21 single-block Speex outlier revisited (framing / state / delay / count / amplitude explanations, no alignment fitting) | **completed — the "decoder disagreement" reading is falsified** (silent record → **"near-silent output under both decoders", R97 §2**) | r93_speex_outlier.py, data/r93-speex-outlier.json |
| 3 | R89 contract extension: S / X words (cross-file variation, controlled tests), name hash (explicit name / stored value pairs, held-out prediction) | **completed as a bounded result; S / X semantics and the 16-bit name key stay blocked** (no candidate fits; properties measured) | r94_abk_sx_hash.py, ABK_TREE_CONTRACT.md |
| 4 | R90 chant-selection evidence: `aemscrwd.csi` body — name presence vs reference vs reachable selection vs input; counterexample test; smallest additional observation | **completed as evidence; binding blocked offline** (loader-by-number gate found; module body / exe undecoded) | r95_crowd_script.py, r95b_controlcenter.py, CHANT_SELECTION.md |
| 5 | Replacement-readiness checklist (design only; explicitly unqualified where prerequisites are missing) | **completed** (both classes unqualified) | REPLACEMENT_READINESS.md |
| 6 | Cards / claims / queue updates with dated corrections; handoff; final checkpoint; cleanup | **completed** | USER-CARDS.md, claims/ (5), DEVELOPER-QUEUE.md, 09-*.md, CHECKPOINT.md |

Out of this queue by instruction: team allocation, shared-art re-keying, shadPS4 performance work, any replacement.

## Afternoon continuation (13:56 MDT →; direction: [research-20260925-misrouted-prompt-audit.md](../../validation/research-20260925-misrouted-prompt-audit.md) + the original 2026-09-24 brief's nonblocking League / Team queue)

| # | Task | Status | Where |
|---|---|---|---|
| 7 | Register the unrecorded R96 decode-matrix run (13:36–13:38) with a bounded summary | **completed** | handoff 10 §6, claim `research-r96-decode-matrix` |
| 8 | R97 correction checkpoint: audit corrections 1–4, R88 count, `*artid*` namespaces; whole-database 55–66 census; dated notes on 14 documents; cards rev. 3 | **completed** | [10-correction-checkpoint-direction-audit.md](10-correction-checkpoint-direction-audit.md), `data/r97-chant-id-field-census.json` |
| 9 | R98 archive-backed `.ast` framing (PS3 09 / 10 / 12 / 13, X360 09) from member bytes; X360 09 header variant discriminated (R98b) | **completed as a bounded result** (X360 09 residuals: 87 middle blocks, header values, 4 loop bodies; no decode) | [11-ast-framing-archive-backed-titles.md](11-ast-framing-archive-backed-titles.md) |
| 10 | R99 Q-R88-2: team-art member spans + entry fields in the gen7 EB archives; PS4 `word_12` bounded test | **completed as a bounded result** (in-payload references, nameplate id position, NHL 12 sock grammar, PS4 `word_12` open) | [12-team-art-archive-spans-and-entry-fields.md](12-team-art-archive-spans-and-entry-fields.md) |
| 11 | Representative decode of archived PS3 09 / 10 / 12 / 13 streams (R92 route on member bytes); an X360 NHL 09 packet-walk decode attempt | **completed** (16 vectors; X360 09 decodes through the unchanged R92 route) | handoff 13 §1, `data/r100-archived-decode.json` |
| 12 | Nameplate id position (DB join `name_{a}_{b}_{c}` vs `stockteamjerseys.teamartid`), NHL 11 / 12 sock grammar, in-payload path / hash references in kit `.rx2` members | **completed as a bounded result** (nameplate = jersey key; PS3 12 sock four-number grammar and in-payload references still open) | handoff 13 §2, `data/r101-nameplate-kit-key-positions.json` |

Out of scope by instruction (unchanged): production writers, live tests, launches, listening, staging / commits; Projects / Builds.

**Next task (not started):** in-payload references of kit members (does a jersey `.rx2` / `.rpsgl` name its nameplate / crest by path or by djb2 hash?) on X360 / PS3 Legacy, read-only — the last unmeasured element of the archive-side rename cascade (handoff 12 §5); then the PS3 NHL 11 / 12 sock grammar.

## Evening continuation (21:58 MDT →; direction: [research-20260925-r97-r101-review.md](../../validation/research-20260925-r97-r101-review.md))

| # | Task | Status | Where |
|---|---|---|---|
| 13 | Correction checkpoint F1–F3: EB rename layout constraints (R102), per-row nameplate / rink-logo joins (R103), alias identity (R103b), X360 09 byte-identical controls (R100b), R96 exceptions, queue split | **completed** | [14-correction-checkpoint-r97-r101-review.md](14-correction-checkpoint-r97-r101-review.md) |
| 14 | In-payload references of team-art / kit members on Legacy X360 / PS3 with the real container reader (RW4 / rx2 / rpsgl): parsed reference fields, then scoped string / hash search; inspection graph | **completed** (R104–R106; no writer) | [15-team-art-payload-references-and-inspection-graph.md](15-team-art-payload-references-and-inspection-graph.md) |
| 15 | R104b: payload-reference check on X360 / PS3 NHL 15 | **completed** (same result) | handoff 15 addendum |

**Next task (not started, 22:22):** discriminate the "rink logo" family statically — decode the RW4 raster descriptors (`0x20003`: dimensions,
format, mip count) per member and compare the `logo_{a}_{b}_{c}_{k}` raster set with the jersey's crest placements; then PS3 NHL 12 / 13
payload references from archive members (chunkzip).

