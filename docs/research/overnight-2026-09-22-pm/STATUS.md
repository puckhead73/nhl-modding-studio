# Research lane status — 2026-09-22 pm (fresh extended continuation)

Campaign brief: the fresh-session prompt in [research-20260922-delivery-review.md](../../validation/research-20260922-delivery-review.md)
— publish F1–F3 (R66–R68) first, then strengthen the R66 classifier, investigate the EA-MP3 embedded PCM and open gen7 EALayer3, publish the
bounded PS4 venue listing, continue the roadmap. Codex coordinates and reviews; the developer lane owns application code. Baseline: branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d`; `docs/research/` and `local/` untracked; the dirty working tree preserved; nothing of the
developer's touched. No launches, no writes to any source tree, no staging, no commits, no pushes. Listening and visual reviews are the
user's. Session clock: started 22 September 14:55 MDT; the system clock read 23 September 05:17 MDT at closeout (the claim files for
R70 / R71 / R73 carry approximate 22-September evening timestamps written before that was noticed).

Startup read (14:55–15:10 MDT): repository instructions, the delivery review, the morning campaign's atlas / status / queue / cards and
handoffs 02–04, the r66 / r67 / r68 probes and JSONs, the pm campaign's queue and handoffs 13 / 21, the 2026-09-21 handoff 05 (gen7
banks), the developer's closeout, receipt 10 and `OBSERVATIONS-FOR-RESEARCH.md` (read only), the parity roadmap phases.

## R69 — Correction checkpoint F1–F3 (R66–R68) — handoff [01](01-correction-checkpoint-f1-f3-r66-r68.md)

Plain-language: the review's three findings are applied before any new work. H1 becomes a candidate with partial static support (its
predicate never tested uniqueness, margin or informativeness; `clipboard_0_0_am` is an uninformative level, not a flat-crowd artefact;
DXN had leaked into the totals); the EA-MP3 contract is bounded to a diagnostic preview whose first 24 ms are unresolved, with the two
ffmpeg decoders named as one code base; the venue counts are made exact (NHL 16 / 17 have no level without a folder) and a residual the
morning missed is published (level ids shared by several names, Honda Center among them); re-pointing is withdrawn as a proposal; R60
is marked INVALID / SUPERSEDED. Raw evidence untouched.

Changed paths: new handoff 01 + claim `research-r69-corrections-f1-f3-r66-r68`; dated notes on 09-22 handoffs 02 / 03 / 04, queue rows
1–3 (rev. 1a), atlas header + Q-R64-1, STATUS pointer, cards A-MP3-1 / T-VEN-1; 09-21-pm handoff 17 status line + queue row 17 pointer.
Executed: verification queries over `r66-contract-check.json` / `r66-rw4xb2-level-addresses.json` / `r68-…json` / r7 levels (the cited
figures — p05 63.75 / median 255.0; `disagree packed 69` including DXN; the shared ids — were confirmed before publication). **Published
first, as required; no acknowledgement awaited.**

## R70 — `RW4xb2` level classifier with independent controls (prompt item 1) — handoff [03](03-rw4xb2-level-classifier-with-controls.md)

Plain-language: the same 142 descriptors re-classified with one rule and four controls; 359 levels are located and every one sits at
H1; 320 are set aside as ambiguous / uninformative; 15 fail one control narrowly on banner text (at H1, not counted). Synthetic controls
show flat / repeated content is never "found" and a deliberately shifted level is detected. H1 stays a candidate rule.

Changed paths (new): handoff 03, `data/r70-rw4xb2-level-classifier.json`, probe `r70_rw4xb2_level_classifier.py`, claim
`research-r70-rw4xb2-level-classifier`. Executed: three synthetic-control runs (two exposed a flaw in the synthetic generator — flat
blocks fail the decoded test by construction — fixed with a real mip chain + minimal DXT1 encoder), one aborted four-title run (output
directory missing after a failed shell command; restarted), the final four-title run (≈ 10.5 min). Scratch JSONs removed.

## R71 — EA-MP3 embedded PCM + robustness (prompt item 2a) — handoff [02](02-ea-mp3-embedded-pcm-and-robustness.md)

Plain-language: the embedded PCM joins the decoded stream continuously at exactly sample 1 152 (candidate A) on 20 / 24 vectors with a
per-vector control; the decoded stream's first 161–880 samples are silent, so B is impossible; the parser refuses seven mutation
classes once the block count is checked against the header; the reservoir is consistent on every frame. Contract rev. 2: diagnostic
preview, start rendered by A, labelled not game-verified; Q-R67-1 narrowed to A pending the listening card.

Changed paths (new): handoff 02, `data/r71-ea-mp3-embedded-pcm.json`, probe `r71_ea_mp3_embedded_pcm.py`, claim
`research-r71-ea-mp3-embedded-pcm`. Executed: a 3-vector smoke run (exposed a control-loop bug), two full 24-vector runs (the second
after adding the Σ-samples check and the first-non-silent measurement). Temporary `.mp3` / `.wav` removed by the probe; the scratch
directory removed at closeout.

## R72 — gen7 EALayer3 v1 on its own vectors (prompt item 2b; the morning's named next task) — handoff [05](05-gen7-ealayer3-v1-structure.md)

Plain-language: 48 samples on six trees share a two-byte block header (`00` + an MPEG version / rate / mono byte), declare 16 or 9
granules per block with 47 extra samples on the first block, and match the header total exactly; the PS4 grammar does not apply, and a
bounded search for a per-granule side-info chain finds nothing consistent — the frame grammar stays unidentified, failure recorded.

Changed paths (new): handoff 05, `data/r72-gen7-ealayer3-structure.json`, probe `r72_gen7_ealayer3_structure.py`, claim
`research-r72-gen7-ealayer3-structure`. Executed: three smoke runs (a `None` guard; floor / ceil granule counts; the generalised
`(h, a, b)` search + byte-aligned variant), one full run (≈ 12 min). `%TEMP%` `.sbr` copies written and removed by the r28 locator.

## R73 — PS4 venue listing contract (prompt item 3) — handoff [04](04-ps4-venue-listing-contract.md)

Plain-language: row by row, 79 / 80 arena rows resolve to one level and one folder on six titles, 77 / 80 on NHL 16; the ambiguous rows
are named (Honda Center everywhere from NHL 16; Barclays / Rogers on 16), no vault attribute disambiguates them, and a level record also
binds three ice texpacks, a rink id and typed placement references — so re-pointing is a cascade and the listing is read-only.

Changed paths (new): handoff 04, `data/r73-ps4-venue-listing.json`, probe `r73_ps4_venue_listing.py`, claim
`research-r73-ps4-venue-listing`. Executed: one run (seconds; joins existing JSONs, opens no archive or DB).

## R74 — Developer observations reconciled (prompt item 4: roadmap, Teams / Players / Equipment / Misc) — handoff [06](06-developer-observations-reconciled.md)

Plain-language: the developer's ten observations are answered with one reading per row; two needed measurements — the NHL 11 / 12
sock `_1_` file is the horizontal mirror of `_0_` (one kit, two files), and the PS3 NHL 12 / 13 kit members are `.rpsgl` (name
grammar transfers, bytes never); `seat*zone*` is reclassified `colour`; the rest are counting rules or readings, adopted.

Changed paths (new): handoff 06, `data/r74-nhl11-12-sock-pairs.json`, probe `r74_nhl11_12_sock_pairs.py`, claim
`research-r74-developer-observations`; dated notes on 09-21-pm handoffs 09 / 10 / 11 / 12 / 14 / 15 / 19. Executed: an inline sock
comparison on 24 then all pairs, the probe file (identical counts), the PS3 member listing (inline), the R51 value-space read (inline).

**League blockers (prompt item 4, first clause):** the pm handoff 13 disposition stands — the two remaining League blockers (CHL
participants; NHL 08 / 09 settings source) are code-resident and carry deferred cards L-CHL-1 / L-SET-09; no new static handle was found
to reopen them, so no package was spent on them.

## Final checkpoint — 23 September 2026, 05:30 MDT (system clock)

**State.** Six handoffs published with claims (`research-r69` … `research-r74`, all `ready`), atlas, queue rows 0–7 and three user
cards. The morning campaign's handoffs 02 / 03 / 04, queue, atlas, STATUS and cards carry dated corrections; the pm 2026-09-21 handoffs
09 / 10 / 11 / 12 / 14 / 15 / 17 / 19 and queue row 17 carry dated notes; no raw evidence was altered anywhere.

**Evidence and limitations.** Everything is researcher-verified static evidence, nothing game-verified: H1 is uncontradicted by 359
strictly located levels but 320 levels stay unresolved and only DXT1 / 3 / 5 on four titles were scored; the EA-MP3 start is supported
by a junction test with controls, not by listening; the gen7 EALayer3 frame grammar is not identified; the venue listing is a join, not a
runtime trace; the sock mirror relation is measured on block means, not texel-exact. Claim timestamps for R70 / R71 / R73 are
approximate (see the header).

**Developer queue:** rows 0–7 of [DEVELOPER-QUEUE.md](DEVELOPER-QUEUE.md). **User cards:** A-MP3-1 rev. 2, T-VEN-1 rev. 2, K-SOCK-1.

**Cleanup (verified).** Session scratchpad empty (smoke JSONs, run logs, the r71 audio directory and two note-stamping scripts removed);
`__pycache__` directories created by this lane's imports under the research probe directories (2026-09-18 … 2026-09-22-pm) removed;
`%TEMP%` holds no `r28_` / `r72_` copies; no background process of this lane is running (r70 / r71 / r72 all wrote their final JSONs;
other lanes' Python processes were not touched); `docs/research/` and `local/` remain untracked; nothing staged, committed or pushed;
the pre-existing dirty working tree is as found.

**One exact next task (roadmap order, Teams 2.6 / 2.7 → Players 3.4):** measure the PS3 NHL 12 / 13 kit triple join *through the
EB-archive listing* (jersey / pant / sock `.rpsgl` members of `cacherender.big` / `nocacherender.big`) against the `stockteamjerseys`
rows of the PS3 anchors, so the developer's kit resolver can cover those two titles by member presence — same triple contract, `.rpsgl`
extension, no extraction, exact residuals; then continue with Teams 2.7 gen7 audio consumers where the pm handoff 08 left them.

---

# Continuation — 23 September 2026, 14:50 → 15:10 MDT (under the coordinator review of R69–R74)

Brief: [research-20260922-pm-review.md](../../validation/research-20260922-pm-review.md). Same baseline (`8d60a6d`, dirty tree
preserved, `docs/research/` and `local/` untracked); no launches, no source writes, no staging / commits / pushes.

## R75 — Correction checkpoint (review items 1–4) — handoff [07](07-correction-checkpoint-r69-r74-review.md)

Published first, without rerunning probes: the R70 transition table (187 previously decisive levels not located, not 235; offsets
identical 694 / 694), the R71 wording ("not supported by this junction test"; onset = |x| > 1 LSB; listening = audible plausibility) and
mutation table per applicability with the three count checks named apart, the R74 sock classes kept separate. Dated notes on handoffs
02 / 03 / 06, queue rows 1 / 2 / 4, cards A-MP3-1 / K-SOCK-1, claim result fields; claim `research-r75-corrections-r69-r74`.

## R76 — PS3 NHL 12 / 13 archive kit join, reconciled with receipts 03 / 04 — handoff [08](08-ps3-nhl12-13-archive-kit-join.md)

Every developer count reproduced from the anchors and the archives' front tables; one wording discrepancy (members are `chunkzip`, not
`chunklzx`); spare pools equal under the developer's per-team rule (255 / 22, 208 / 1), the global rule published apart; fixture contract
with member identities and stored-byte hashes. Executed: probe once + one inline per-team recount. Claim `research-r76-ps3-archive-kit-join`.

## R77 — Gen7 EALayer3 v1 grammar established — handoff [09](09-gen7-ealayer3-v1-grammar-established.md)

The maintained public decoder's layout (vgmstream `mpeg_custom_utils_ealayer3.c`, read from the public repository; summarised, not
copied) was tested and corrected on the 48 r72 vectors: EA-frame tiling 48 / 48, granule counts 48 / 48 (first block +2 granules and one
`EE` PCM frame with 47 samples *after* its granule data — the summary's order failed on the data), rebuilt frames with a bit reservoir
decode with ffmpeg 48 / 48 at granules × 576. Diagnostic decode only; start handling open (Q-R77-1, card A-EAL3-1); no encoder. Executed:
four smoke runs, one full run (≈ 6 min); temporary audio removed. Supersedes handoff 05 (dated note). Claim `research-r77-gen7-ealayer3-v1-grammar`.

## R78 — Gen7 arena horn / chant bank join — handoff [10](10-gen7-arena-horn-chant-bank-join.md)

Ten trees: `goalhorn` / `periodhorn` → `<id:03>_goal/period.abk` for the NHL-club ids with exact residuals (11–14 non-NHL values per title
without a loose bank; 0 / 100 / 194 unreferenced); `chantaudioid` partial candidate; `cityaudioid` / `arenaaudioid` negative. Executed
once. Claim `research-r78-gen7-arena-sfx-bank-join`.

**Brief items 3 / 4:** no further mip package (no independent method for the unresolved levels was available without new evidence;
the write gate stays intact); the League blockers stand as deferred cards; Teams 2.6 / 2.7 advanced by R76 / R77 / R78.

## Final checkpoint — 23 September 2026, 15:10 MDT

**State.** Ten handoffs, claims `research-r69` … `r78` (all `ready`), queue rows 0–10, cards A-MP3-1 rev. 2, T-VEN-1 rev. 2, K-SOCK-1,
A-EAL3-1. Dated notes on handoffs 02 / 03 / 05 / 06 of this campaign; no raw evidence altered.

**Checks actually run this continuation.** r76 (once + per-team recount), r77 (four smoke + one full, ffmpeg decodes), r78 (once); the
R75 numbers were read from the published JSONs. No listening, no launches, no writes to sources.

**Limitations.** Nothing game-verified; the EALayer3 decode is a decoder-acceptance result, not heard; the start rule and the non-NHL
horn ids are open; PS3 12 / 13 audio and NHL 09 banks not covered by R78; H1 unchanged (candidate); candidate A unchanged (diagnostic);
venue discovery unchanged (read-only).

**Retained dependencies.** Public reference read for R77 (vgmstream `mpeg_custom_utils_ealayer3.c`, 2026-09-23) — cited, not vendored;
ffmpeg 8.1.1; numpy; the trees on `X:` / `H:`; the committed anchors.

**Cleanup (verified at closeout).** Scratchpad empty (r77 audio directory, smoke JSONs, run logs and the two note scripts removed);
`__pycache__` created under the research probe directories removed; `%TEMP%` holds no `r77_` / `r72_` / `r28_` copies; no background
process of this lane running; nothing staged, committed or pushed.

**One exact next task.** Run the R77 parse on the gen7 **announcer** banks (`ann_eng`, PS3 24 kHz / X360 `0x09` excluded) and the
`music_samples` / `.abk` sfx members of codec `0x05` to state the codec class's coverage beyond pbp; then, for Q-R78-1, list `sfx.abk` /
the archives for the non-NHL horn ids so the Arena page can name their source or refuse exactly.

---

# Continuation — 24 September 2026, 04:20 → 04:35 MDT (under the coordinator review of R75–R78)

Brief: [research-r75-r78-review-20260923.md](../../validation/research-r75-r78-review-20260923.md). Same baseline; no launches, no
source writes, no staging / commits / pushes.

## R79 — Correction checkpoint (review items 1–3) — handoff [11](11-correction-checkpoint-r75-r78-review.md)

Published first: R77 applicability narrowed to the six measured pbp sets, the PCM frame reconciled to the second EA-frame of block 0,
units stated, the public reference pinned (revision `4b6a02dd…`, SHA-256 `0bc1e0e9…`, fetched with curl and hashed) and the NHL
field-order difference stated as measured, not as an upstream fault; R78 city / arena ids restated as no established binding with the
coincidences reported; the developer-facing R76 `chunkzip` claim. Dated notes on handoffs 09 / 10, queue rows 9 / 10, claims r77 / r78.
Claims `research-r79-corrections-r75-r78`, `research-r76-chunkzip-correction-for-developer`.

## R80 — Gen7 codec-0x05 survey — handoff [12](12-gen7-codec05-survey.md)

Sixteen trees × (announcer, unmeasured pbp, music, three `.abk` banks): EALayer3 v1 established per tuple — PS3 announcer 24 kHz mono
32 / 32, PS3 pbp 09 / 10 / 12 12 / 12, music 44.1 kHz stereo 24 / 24 (stereo side info + rebuild added
to the R77 probe), PS3 `.abk` raw streams 24 / 28 (the entry size counts the 8 SNR bytes — found on the first run, corrected,
rerun); X360 announcer / pbp 11–Legacy `0x09`, X360 09 / 10 and all X360 `.abk` EA-XMA. Malformed inputs (truncation, flag byte, PCM count)
refused on every codec-0x05 class. Executed: one smoke run, two full runs (≈ 10 min each). Claim `research-r80-gen7-codec05-survey`.

## R81 — Gen7 unresolved horn sources — handoff [13](13-gen7-unresolved-horn-sources.md)

The residual ids are absent in every inspected source; `horns.abk` is empty; `aemshorn.csi` names a segment-built horn interface
(candidate); NHL 09 X360 horn banks are archive-only. Executed three times (guards). Claim `research-r81-gen7-horn-sources`.

**Brief item 4:** no further package this session; the League blockers stand as cards; Teams 2.7 advanced by R80 / R81.

## Final checkpoint — 24 September 2026, 04:35 MDT

**State.** Thirteen handoffs, claims `research-r69` … `r81` + the R76 developer-facing correction (all `ready`), queue rows 0–13, cards
A-MP3-1 rev. 2, T-VEN-1 rev. 2, K-SOCK-1, A-EAL3-1, H-HORN-1. No raw evidence altered; dated notes on handoffs 09 / 10.

**Checks actually run.** curl + sha256 of the pinned reference; r80 smoke + two full runs (ffmpeg decodes per class); r81 three runs.
R79 numbers were read from the published JSONs / files. No listening, no launches, no writes to sources.

**Limitations.** Nothing game-verified; every decode is decoder acceptance; the start rule (Q-R77-1) and the residual horn consumer
(Q-R78-1) are open; PS3 12 / 13 archive audio and archived music banks (PS3 09 / 10 / 12 / 13) are not surveyed; `0x09` and EA-XMA
have no decoder route (Q-R80-1).

**Retained dependencies.** The pinned public reference (cited, hashed, not vendored); ffmpeg 8.1.1; numpy; the trees on `X:`; the
committed anchors.

**Cleanup (verified at closeout).** Scratchpad empty (reference copy, r80 audio directory, smoke JSON, run log, the doc scripts removed);
`__pycache__` under the research probe directories removed; no background process of this lane running; nothing staged, committed or
pushed.

**One exact next task.** Survey the archive-only gen7 audio (PS3 12 / 13 speech + music + sfx members of `audioboot.big` / `nocache.big`,
PS3 09 / 10 music) with the R80 probe through the archive listers, so the codec-0x05 coverage table has no "not surveyed" cell left on
gen7; then the `0x09` class identification (Q-R80-1) as the last gen7 speech blocker.

> **Note 2026-09-24:** continued in [overnight-2026-09-24/STATUS.md](../overnight-2026-09-24/STATUS.md) (R0, R82 archive coverage + `ABKC` record grammar, R83 EA-Speex, R84 EA-XMA preview).
