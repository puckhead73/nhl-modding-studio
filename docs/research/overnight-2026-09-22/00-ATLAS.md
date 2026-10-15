# Research atlas — 2026-09-22 (extended continuation campaign, started 07:00 MDT / 13:00 UTC)

Research lane, continuation of [overnight-2026-09-21-pm](../overnight-2026-09-21-pm/00-ATLAS.md) under the coordinator's
[delivery review of 2026-09-22](../../validation/research-20260921-pm-delivery-review.md) and its fresh-session prompt: publish the
F1–F3 corrections first, then (1) independently establish the Xbox packed-mip addresses and per-level boundaries, (2) investigate EA-MP3
framing without assuming missing headers, (3) progress the arena / assets roadmap in order. Everything here is **research input** for the
developer, researcher-verified and pending Codex review; nothing is game-verified; no game or emulator launched; nothing under `X:`, `H:`,
`anchors/`, application code, `defs/`, bindings, developer matrices or receipts modified.

> **Corrected 2026-09-22 pm** by [research R69](../overnight-2026-09-22-pm/01-correction-checkpoint-f1-f3-r66-r68.md) (coordinator review F1–F3 of R66–R68): handoff 02's H1 is a candidate with partial static support (totals hold under the R66 predicate only); handoff 03 is a diagnostic-preview contract with the first 24 ms unresolved; handoff 04's counts are exact in R69 (NHL 16 / 17: none missing; ids shared by several names on 16–21) and its re-pointing proposal is withdrawn; Q-R64-1 closed for naming only (runtime selection = Q-R69-1). Successors: R70 / R71 / R73 in the pm campaign.

Status: [STATUS.md](STATUS.md). Developer consumption queue: [DEVELOPER-QUEUE.md](DEVELOPER-QUEUE.md). Claims:
`local/coordination/overnight-2026-09-21/claims/research-r6[5-9]*.json` / `research-r7*.json` (the shared directory the developer lane
reads). Probes (Python 3.12, read-only): `local/runs/overnight-research-2026-09-22/probes/`. Data: `data/`.

Lane ownership this campaign: `docs/research/overnight-2026-09-22/**`, `local/runs/overnight-research-2026-09-22/**`, the `research-r65+`
claims, and dated correction notes inside this lane's own earlier campaigns. Developer-owned application files, matrices and receipts are
not touched.

## 1. Handoffs

| # | Handoff | Roadmap step | One-line result |
|---|---|---|---|
| 01 | [Correction checkpoint F1–F3](01-correction-checkpoint-f1-f3.md) | review | R60's slot-extent conclusions withdrawn (the extent is a lower bound measured inside the slot: no truncation-safety proof, no allocation rule, no level placement; only "under" counts are model evidence); R61's header-removal explanation and its EALayer3 transfer restated as hypotheses with new user-facing text; R43 coverage reconciled (seven titles, NHL 15 + 19 multi-stream, first-stream qualification only, maxima are observations, BC6H size-only). Raw evidence unchanged. |
| 02 | [`RW4xb2` per-level addresses from content](02-rw4xb2-per-level-addresses-from-content.md) | 1.3 / 2.5 / 4.x X360 | Levels located by a tiled content search with a decisive-score criterion (NHL 09 / 10 / 13 / Legacy, 142 DXT1 / 3 / 5 descriptors, 30 shapes): **H1** — own `align32` tile per level while `min(w,h) > 16` texels, then one packed tail tile with the Xenos texel-offset placement — agrees on **386 / 387** own-tile and **76 / 78** packed decisive levels ≥ 4 × 4 blocks (exceptions: flat score-0 textures); `DW12` = level-1 tile base 141 / 142; packed offsets identical across titles. **Not established:** levels ≤ 8 × 8 texels, 16-wide strips, DXN / CTX1 / `8_8_8_8` / cubes. r60's 4 KiB read-offset defect recorded. |
| 03 | [EA-MP3 sized-frame grammar + decode](03-ea-mp3-sized-frame-grammar-and-decode.md) | 1.5 / 2.7 / 3.5 PS4 (15–17) | Codec `0x0b` blocks are chains of **sized frames** (`u16le size │ 0x8000`, optional `u32 pcm_samples` + s16le PCM on the first frame only, then a complete MPEG-1 Layer III frame): tiles **24 / 24** vectors, 9 frames per full block, per-frame bitrate, bit reservoir crossing blocks; concatenated frames decode with ffmpeg `mp3` and `mp3float` (≤ 1 LSB apart) to frames × 1 152, `num_samples` = that minus a last-block trim. r61's "headers removed" withdrawn. Open: the 1 152 embedded PCM samples (Q-R67-1). No encoder; gen7 EALayer3 untested. |
| 04 | [PS4 venue directory by level name](04-ps4-venue-directory-by-level-name.md) | 2.5 PS4 | `ge_level.DebugName` = `stadiums/<dir>/` on **32 / 33 … 79 / 79** levels (NHL 15–21); every venue directory is named by a level; unnamed dirs are `common*` / `unused` / `vancouvertest`; one level per title ships no folder. Chain `arenaartid → ge_level.ID → DebugName → dir` — **closes Q-R64-1**. |

## 2. Questions

- **Q-R65-1** — the Xbox 360 per-level address / layout contract: where each mip level of a mipped `RW4xb2` texture begins inside its
  slot, established by content rather than by allocation size, with a test that distinguishes a *misplaced* level from an *equally sized*
  one. Opened by F1; **answered by handoff 02 down to 4 × 4-block levels** (H1); the three smallest levels, 16-wide strips and the
  untested formats stay open as **Q-R66-1**.
- **Q-R28-1 (audio classes)** — EA-MP3 (`0x0b`) storage form **identified** (handoff 03: sized frames; preview decodes); the relationship
  to gen7 EALayer3 v1 is untested; EA-XMA, codec `0x09` and every encoder stay open.
- **Q-R67-1** — the role of the 1 152 s16le PCM samples carried by the first EA-MP3 frame of every sample: replacement for decoded frame 0
  (favoured on 18 / 24 vectors by junction continuity; consistent with the block sample count) or a pre-roll; neither established. 24 ms
  at the start of a clip.

- **Q-R64-1** — **closed for directory naming only** by handoff 04 (vault level name = venue directory, observed); the runtime-selection half is open as **Q-R69-1** (R69, 2026-09-22 pm).
- **Q-R43-1 / Q-R44-1 / Q-R60-1 / Q-R62-1 / Q-R9-1 / Q-R40-1 / Q-R48-2 / Q-R36-1** — carried forward from the pm campaign
  atlas unchanged except where F1–F3 restates them.
