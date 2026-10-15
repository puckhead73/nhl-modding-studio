# Developer lane status — R4 continuation, 2026-09-23

Campaign: the fresh extended continuation of the
[coordinator review of receipts 10–17](../../validation/overnight-development-20260922-receipts10-17-review.md)
(22 September 2026): close F1 / F2 first with permanent regressions, then title-text keys on the
song and arena cards, then kit inspection parity, then the wider roadmap, without routine review
stops. Codex is architect / reviewer; the research lane owns `docs/research/**` and `research-*`
claims — nothing there is edited here. The accepted kit correction (receipt 10 of 22 Sep) was not
reopened.

**Starting state, recorded rather than assumed:** HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051`
(branch `feat/phase5-asset-tooling`), working tree dirty from the 18–22 September campaigns and
preserved. No staging, commits, pushes, resets, branch changes or clean operations; no game or
emulator launches; no writes to installs, images, archives, anchors or saves; real trees on `X:`
and the committed anchors are read-only inputs; mutation is exercised in memory or on disposable
copies. NHL 21 E1 stays accepted and closed. Projects / Builds untouched.

Every checkpoint is **developer-verified, not Codex-accepted, not desktop-exercised, not
game-verified**. Claims: `dev-r4-*` under `local/coordination/overnight-2026-09-23/claims/`.

## Checkpoints

- **01 — F1 / F2 correction checkpoint (Xbox 360 write boundary and audit claims)** — delivered,
  developer-verified: [receipt 01](01-r4-xbox-write-boundary-correction.md). Changed-size writes
  into mipped Xbox 360 rasters refuse before any mutation; same-size writes are disclosed partial
  replacements with an applicability gate and a byte-write mask, shown before Apply; the audit is
  restated with a uniform classifier over 15 families / 8 titles; handoff 17 removed as support;
  debug-blue reduced to an attributed note; receipt 11, the Phase 1 matrix and the 22 Sep cards
  carry dated corrections.
- **02 — Song / arena title keys through the project's own dictionary** — delivered:
  [receipt 02](02-r4-title-text-keys.md). Seam test across attach / detach / re-attach / project
  switch; unresolved keys stay keys with an explicit state; replies stamped.
- **03 — PS3 NHL 12 / 13 kits from the render archives** — delivered:
  [receipt 03](03-r4-ps3-archive-kits.md). 676 / 684 rows resolve on the real trees from the
  archives' file tables; members not decoded there.
- **04 — Archive-member texture preview (Teams 2.6, dispatch item 4)** — delivered:
  [receipt 04](04-r4-archive-member-preview.md). The archived kit components preview through a
  read-only archive-member route (front tables + one member); the real NHL 13 jersey member decodes
  at 1024²; no reassignment, no archive write.
- **05 — R4 review F1: no partial replacement without its current disclosure** — delivered:
  [receipt 05](05-r4-review-f1-disclosure-gate.md). Write waits for a current successful plan for
  the exact target and source; pending / failed plans block with Retry; re-read on every open;
  the stale-plan write refuses at the backend.
- **06 — Teams 2.5 PS4 venue listing under research R73** — delivered:
  [receipt 06](06-r4-ps4-venue-listing.md). Every vault level carrying the arena's art id listed
  with folder status, texpack keys and rink id; ambiguities named, never resolved silently; shared
  content once; no `arenaartid` reassignment (the R68 proposal is not implemented).
- **R74 reconciliation consumed** (research 2026-09-22 pm handoff 06): the NHL 11 / 12 sock 0 / 1
  files are a mirrored pair of one kit — the kit contract text and the re-pointing reason say so
  (unit case updated); the seat-zone reclassification to `colour` matches the developer override,
  which is kept (the card already renders them as colours).

## Checks (closeout regression)

[r4-closeout-regression.log](../../validation/evidence/overnight-development-2026-09-23/logs/r4-closeout-regression.log):

| Suite | Result |
|---|---|
| `cargo test -p tdb-rx2 --lib` | 120 passed, 0 failed, 26 ignored (private-fixture cases) |
| `cargo test -p studio-domain --lib` | third section: **219 passed, 0 failed, 2 ignored** (was 216) |
| `cargo test -p studio-api` (lib + every test target) | second section: **34 targets: 126 passed, 0 failed, 33 ignored**; third section (after packages 05 / 06): **34 targets: 125 passed, 0 failed, 34 ignored**; the export-root count moved 142 → 148 (texture replace plan, song genre count, dictionary stamp, three PS4 venue-listing roots) |
| `export-bindings --check` | in sync — 148 roots |
| `npx vitest run` (full) | second section **31 files, 206 passed**; third section: **31 files, 209 passed** (TextureReplace 13, ArenaAssetsCard 4) |
| `npx tsc --noEmit` | clean |

The regression above ran before package 04; its additions were run on their own afterwards
(`archive_member_preview` default 1 passed, `studio-api --lib` 31 passed, `team_kits` 3 passed,
`kit_files` 15 passed, TeamKitsCard 5 passed, bindings in sync, tsc clean) and the whole
`studio-api` / vitest set was rerun at closeout (second section of the same log).

Real-tree / `--ignored` runs of this continuation, each failing loudly without its inputs:
`rw4xb2_mip_placement` (15 families, 832 s), `kit_selector_join` (12 trees, 75 s),
`archive_member_preview` (the real NHL 13 jersey member); `arena_assets` seam + domain on the
committed anchors for the venue listing. Logs under
`docs/validation/evidence/overnight-development-2026-09-23/logs/r4-*`. `zz_diag` was not invoked.

## Blockers (recorded, not campaign stoppages)

- The Xbox 360 packed-tail layout (research Q-R65-1; the R66 H1 candidate awaits the research
  lane's F1 restatement). Until then a resized image cannot be written into a mipped Xbox 360
  raster, and a same-size write is partial.
- **Corrected 24 Sep 2026:** the PS3 12 / 13 kit members are `chunkzip` (research R76); replacement is implemented through the archive-member route ([24 Sep receipt 03](../overnight-development-2026-09-24/03-t2-ps3-kit-member-replacement.md)). Original wording: writing a texture back into a PS3 12 / 13 render-archive member: no `chunklzx` encoder
  (R30-LZX-1 position unchanged); preview only.
- Which dictionary file / screens a PS4 title reads at runtime for song and horn strings.

## Deferred cards

Desktop: V-36 … V-44 in [MORNING-REVIEW.md](MORNING-REVIEW.md); V-31 revised in the 22 Sep file.
Runtime: R32-XB2-1 rev. 3 (single-level rasters only), R32-XB2-3 (same-size partial replacement
observed at distance, not predicted).

## Cleanup

| Artifact | Disposition |
|---|---|
| Scratchpad patch scripts (`patch_*.py`) and the `locprobe/` cargo project (with its `target/`) | deleted at closeout; scratchpad verified empty |
| Temp roots created by this continuation's tests (`nhlms-c1-loose-f1-boundary-*`, `nhlms-songloc-*`, `nhlms-kit-eb-*`, `nhlms-ebprev-*`) | removed on drop; verified absent |
| Background processes | none at closeout |
| Evidence logs `r4-*` (5 files; the closeout log carries three dated sections) | retained |
| Shared build cache, other lanes' files, game trees, anchors, private fixtures | untouched; nothing deleted by age |

## One exact next task

Next Teams operation in the parity order (2.7 audio): the **PS4 NHL 15–17 EA-MP3 diagnostic
preview** under research contract rev. 2 (2026-09-22 pm handoff 02 / queue row 1) — sized-frame
parse with the static checks (Σ declared block samples = `num_samples`, reservoir rule, nine frames
per full block), concatenated MPEG decode, candidate-A start, labelled **"diagnostic preview (start
rendered by candidate A; not game-verified)"**, kept distinct from any verified game audio;
replacement stays refused (no encoder, cross-block reservoir). Decoder dependency choice (a
pure-Rust Layer III decoder) is the first decision of that package. On the Xbox 360 side, consume
the research lane's R70 classifier only as a hypothesis check; resized writes into mipped rasters
stay refused.
