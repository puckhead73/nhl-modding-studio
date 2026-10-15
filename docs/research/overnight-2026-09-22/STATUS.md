# Research lane status — 2026-09-22 (extended continuation)

Campaign brief: the fresh-session prompt in
[research-20260921-pm-delivery-review.md](../../validation/research-20260921-pm-delivery-review.md) — F1–F3 corrections as the first
published checkpoint, then Xbox packed-mip addresses / per-level boundaries, then EA-MP3 framing, then the arena / assets roadmap in
order. Codex coordinates and reviews; the developer lane owns application code. Baseline: branch `feat/phase5-asset-tooling`, HEAD
`8d60a6d`; `docs/research/` and `local/` untracked; the dirty working tree preserved; nothing of the developer's touched. No launches,
no writes to any source tree, no staging, no commits, no pushes. Visual and listening reviews are deferred to the user.

Startup read (06:55–07:05 MDT): repository instructions, the delivery review, the pm campaign's 00-ATLAS / STATUS / DEVELOPER-QUEUE,
handoffs 03 / 17 / 18, the r60 / r42 probes, the shared claims directory and the developer lane's 2026-09-22 STATUS (read only).

## R65 — Correction checkpoint F1–F3 (07:00–07:15 MDT) — handoff [01](01-correction-checkpoint-f1-f3.md)

Plain-language: the three corrections are published before any new work. The Xbox slot-extent conclusions are withdrawn to what the
measurement supports (a lower bound taken inside an already-bounded slot — no proof that a resize fits, no allocation rule, nothing about
where a mip level sits); the EA-MP3 "headers were removed" explanation becomes a candidate and the user-facing text becomes "not
supported yet, the tested conversion failed"; the PS4 texture census's coverage wording is reconciled (all seven titles, NHL 15 **and**
NHL 19 multi-stream, first-stream qualification only, shipped maxima are observations, BC6H agreement is size-only). Every measurement,
table and JSON stays exactly as published.

Changed paths — pm campaign (research-owned, dated notes only): handoffs 03 (note + reproduction footer), 17, 18 (notes), DEVELOPER-QUEUE
rows 14 / 17 / 18, 00-ATLAS (header note + rows 03 / 17 / 18), STATUS (header note). New (this campaign): 00-ATLAS, STATUS,
DEVELOPER-QUEUE, handoff 01, claim `research-r65-corrections-f1-f3`. Executed: no probes (documentation package). Not executed: deletions,
writes to sources, launches, commits.

**Correction checkpoint published.** Continuing to the Xbox per-level layout contract without a review stop, per the prompt.

## R67 — EA-MP3 sized-frame grammar and decode (prompt priority 2; 08:20–09:10 MDT) — handoff [03](03-ea-mp3-sized-frame-grammar-and-decode.md)

Plain-language: the NHL 15–17 speech format is not missing any headers — each block is a run of length-prefixed complete MP3 frames.
Reading the lengths, every block of all 24 test sounds parses exactly, and the frames decode end to end with two separate MP3 decoders
that agree within one unit. The declared duration is the frame total minus a short trim at the end. The one open detail is what the
first record's 1 152 raw PCM samples are for (most likely a stand-in for the decoder's unusable first frame; not proven — 24 ms). Preview
is therefore possible on NHL 15–17; replacement is still not (no encoder, and the bit reservoir runs across blocks). Listening stays
with the user. The last-generation EALayer3 class was **not** examined and is not assumed to be the same.

Changed paths (new): handoff 03, `data/r67-ea-mp3-sized-frames.json`, probe `r67_ea_mp3_sized_frames.py`, claim
`research-r67-ea-mp3-sized-frames`; atlas row 03 + Q-R28-1 update + Q-R67-1; queue row 1; pm handoff 18 (resolved note). Executed: r67
smoke (3 vectors), one `--keep` vector for the lag / continuity scratch analysis (`x8_pcm_lag.py`, scratchpad), the final 24-vector run;
temporary audio removed by the probe. Not executed: listening, encoders, writes. **Background:** r66 (X360 level addresses) on NHL 13.

## R68 — PS4 venue directory by vault level name (prompt priority 3, Teams 2.5; 09:15–09:40 MDT) — handoff [04](04-ps4-venue-directory-by-level-name.md)

Plain-language: the PS4 arena folder is named by the vault level record already tied to the arena row — its debug name is the folder
name on every title, so the Arena page can find a venue in two hops with no guessing. Closes Q-R64-1.

Changed paths (new): handoff 04, `data/r68-ps4-venue-dir-by-level-name.json`, probe `r68_ps4_venue_dir_by_level_name.py`, claim
`research-r68-ps4-venue-dir`; atlas row 04 + Q-R64-1 closed; queue row 2. Executed: one smoke run (NHL 21 / 15) + one full run (archive
listings only). **Background:** r66 (X360 level addresses) on NHL Legacy.

## R66 — X360 `RW4xb2` per-level addresses from content (prompt priority 1; 07:15–10:20 MDT, run in the background across the other packages) — handoff [02](02-rw4xb2-per-level-addresses-from-content.md)

Plain-language: instead of measuring how much room a texture takes, this package finds each smaller mip copy by content — the right
position matches the shrunken parent 10–30 × better than anywhere else, so a misplaced level is detectable. On all four titles the found
positions follow one rule (own tile per level while the smaller side exceeds 16 texels, then one shared tail tile with the console's
texel-offset placement), confirmed for every level down to 4 × 4 blocks; the last three tiny levels cannot be told apart by this test
and stay unverified. Yesterday's extent probe turns out to have read every slot 4 KiB early (file-table offsets are header-relative),
which is recorded as a defect note in handoff 17 and in handoff 01.

Changed paths (new): handoff 02, `data/r66-rw4xb2-level-addresses.json`, `data/r66-contract-check.json`, probes
`r66_rw4xb2_level_addresses.py` / `r66_contract_check.py`, claim `research-r66-rw4xb2-level-addresses`; atlas row 02 + Q-R65-1 answered
+ Q-R66-1; queue row 3; pm handoff 17 (defect note); handoff 01 (addendum); USER-CARDS (new: A-MP3-1, A-ICE-1, T-VEN-1). Executed: two
40-file smoke runs, seven scratch analyses, the four-title run (≈ 11 min), the contract check. Not executed: writes, launches, Studio
binaries.

## Final checkpoint — 22 September 2026, 10:30 MDT

**State.** Four handoffs published with claims (`research-r65` … `research-r68`, all `ready`), atlas rows, queue rows and user cards:
01 corrections F1–F3 (+ the r60 read-offset defect), 02 X360 per-level contract H1 (priority 1), 03 EA-MP3 sized-frame grammar and
decode (priority 2), 04 PS4 venue directory by vault level name (priority 3, closes Q-R64-1). The pm campaign's handoffs 03 / 17 / 18,
queue rows 14 / 17 / 18, atlas and STATUS carry dated supersession notes; no raw evidence was altered.

**Evidence and limitations.** Everything is researcher-verified static evidence, nothing game-verified: H1 holds on content-located
levels ≥ 4 × 4 blocks (462 decisive levels, 30 shapes, 4 titles) and is silent on the three smallest levels, 16-wide strips, DXN / CTX1 /
`8_8_8_8` and cubes; the EA-MP3 decode is verified by two decoders' sample counts and hashes on 24 vectors, not by listening, and the
role of the 1 152 embedded PCM samples is open (Q-R67-1); gen7 EALayer3 was not opened and is not assumed related; the venue-name
binding is an exact join on all seven titles with one runtime card (T-VEN-1). The gen7 ice-layer selector cannot be settled by data
(card A-ICE-1). The developer's active R2 arithmetic package was not touched; the H1 contract and its acceptance test supplement it.

**Developer queue:** rows 0–3 of [DEVELOPER-QUEUE.md](DEVELOPER-QUEUE.md).

**Cleanup.** Session scratchpad empty (smoke JSONs, `x1`–`x8` scratch scripts, logs, temporary audio and `__pycache__` removed);
no `__pycache__` under the campaign's probes; no background process of this lane is running (r66 and r68 wrote their final JSONs);
other lanes' Python processes were never touched; `docs/research/` and `local/` remain untracked; nothing staged, committed or pushed;
the pre-existing dirty working tree is as found.

**One exact next task:** open the gen7 EALayer3 v1 class on its own vectors (PS3 / X360 NHL 11–Legacy `pbp` speech banks, 8 per title):
parse the block prefixes as bounded structures the way r67 did for `0x0b` — test the sized-frame reading first, then the granule-header
readings — decode with two implementations where any decodes, and publish the grammar or the precise failure; do not assume the
`0x0b` grammar transfers.


## Correction pointer — 22 September 2026, pm

The coordinator's [delivery review of R66–R68](../../validation/research-20260922-delivery-review.md) is applied by [research R69](../overnight-2026-09-22-pm/01-correction-checkpoint-f1-f3-r66-r68.md) (pm campaign, handoff 01): H1 restated as a candidate, the EA-MP3 contract bounded to a diagnostic preview, the venue counts made exact with the shared-id residual, the re-pointing proposal withdrawn, R60 marked invalid / superseded. This campaign's handoffs carry dated correction notes; no evidence was altered.
