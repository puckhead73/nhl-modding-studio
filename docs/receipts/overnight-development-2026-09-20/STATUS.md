# Developer lane status — overnight 2026-09-20

Campaign: continued Phase 1 development beyond the OVERNIGHT-DEV-R1 stop point (user-authorized overnight
run, 20 September 2026). Baseline revision `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (branch
`feat/phase5-asset-tooling`); the dirty working tree (649 entries at start, incl. the whole 19 September
campaign and its R1 corrections) is preserved untouched except for the paths named in each receipt.
No staging, commits, pushes, resets or clean operations this lane. No game/emulator launches, no writes to
installs, images, anchors or profiles; private fixtures under `E:\nhl-studio-fixtures` are read-only inputs.

Checkpoints here are **developer-verified, not Codex-accepted**. Visual/manual review is deferred to the user
(`MORNING-REVIEW.md`). NHL21-E1 stays accepted and closed.

**R1 (20 September, afternoon):** the campaign was reviewed and **not accepted in full**
([review](../../validation/overnight-development-2026-09-20-review-r1.md)); the correction package
[OVERNIGHT-DEV-20260920-R1](../../validation/overnight-dev-20260920-r1-corrections.md) fixed F1/F2,
reconciled the stale blockers (F3), moved the retained logs to
[durable evidence](../../validation/evidence/overnight-development-2026-09-20/MANIFEST.md) and opened
the [provisional Phase 2 matrix](../../validation/phase-2-coverage-matrix-2026-09-20.md). Receipts 06
and 10 carry correction notes; the others are left as written.

Ownership: this session is the sole application-code writer. Research (claim `research-r8-jersey-triple-contract`,
Phase 2 jersey joins) owns `docs/research/**` and `local/runs/overnight-research-*/**`; nothing there is edited here.

## Checkpoints

- **C0 — R1 verification and foundation audit** — delivered 04:50, developer-verified: [receipt 01](01-r1-verification-and-foundation-audit.md). R1 holds (3 + 4 + 8 cases incl. the real NHL 21 member); one D6 defect found (packages rebuilt in a non-shipped layout) and fixed in C1; R1 scratch removed.
- **C1 — loose-route closure** — delivered 04:55, developer-verified: [receipt 02](02-c1-loose-route-closure.md). Span-preserving package writer (`tdb-archive-big::replan`), loose dialog ownership, changed-pixel sweeps on 77 + 34 logo files and 64 + 22 non-logo imagery files (new private fixtures). Research 02 §2 tail correction recorded (16 opaque bytes, not zero).

- **C2/C3/C4 — archive placement on every family** — delivered 06:20, developer-verified: [receipt 03](03-c2-c4-archive-placement-every-family.md). Real PS4 15–19 archives passed (20/21 running); gen7 EB route (`fe.big`/`cache.big`, real Legacy X360 + PS3 passed, 7 more queued); BIG4 + `.bh` writer for NHL 07–10 (synthetic pairs with real packages; real archives queued; NHL 07 = sidecar-cascade refusal).
- **C5 — localization canonical rebuild (LOC-W3 C / O-G)** — delivered 06:40, developer-verified: [receipt 04](04-c5-localization-canonical-rebuild.md). Tree builder in the measured format; EA stream-length rule measured (`bits/8 + 1`); 25 + 106 dictionaries repack byte-identical to the slack; explicit rebuild in the panel, route and save.

- **C6 — different-dimension logos/imagery** — delivered 06:35, developer-verified: [receipt 05](05-c6-different-dimension-logos.md). Apt geometry measured on all 141 packages (six-vertex quad + member-0 size words); resize on Xbox 12–Legacy XPR2 (64/256 wide) and PS3 12–Legacy DDS with the quad rewritten; RW4 (07–11), RW4ps3 (08–11), GNF (PS4 16–21) and the display-crop families refused with reasons.

- **C7 — remaining League closure + blocker record** — delivered 07:20, developer-verified: [receipt 06](06-c7-league-closure-and-blockers.md). Whole-group scope for shared PS4 blocks (explicit); 99 PS4 locale members extracted and swept; NHL 15 patch DB groups tested; Priority-5 blockers recorded precisely. Matrix and MORNING-REVIEW updated.

- **C8 — Phase 2 opening: Team identity (2.1) names + colours** — delivered 07:30, developer-verified: [receipt 07](07-c8-team-identity-colours.md). Split name cells on PS4 16–21; Colours section (2 triples on 11–15/Legacy/PS4 15, 6 triples on PS4 16, 9 packed words on PS4 17–21 — shapes and the packed byte order measured on the anchors); one-operation multi-cell write (`set_cells_int`) with one undo; colour control with swatch + hex. PS4 NHL 20/21 real-archive sweep results recorded (7/7 passed).

- **C9 — Phase 2.4: Team logo families** — delivered 07:58, developer-verified: [receipt 08](08-c9-team-logo-families.md). Measured census of every `teamlogos*` family on all 27 gen7/PS4 sources (new private fixtures, 285 members); per-title family table proven against the census; Team logos card (loose + archive replace); 284/285 members replaced through the loose route (the one refusal is the un-offered Xbox One DDS twin). gen7 EB real-archive sweep results recorded (9/9 passed). 2.2 measured as already Editable on every title where the tables exist (surface probe; read matrix) — no package needed.

- **C10 — BIG4 binder fix + PS4 team-text parity** — delivered 08:20, developer-verified: [receipt 09](09-c10-big4-binder-and-ps4-team-text.md). The BIG4 real sweep exposed a binder defect on PS3 NHL 10 (declared index size overruns the first payload; measured on all seven stock archives; fixed + unit-tested; real rerun queued). Team text (2.3) proven on every PS4 English dictionary (14, base + patch) and the NHL 15 anchor. 2.2 parity measured (surface probe): Editable everywhere the tables exist. BIG4 sweep results so far recorded (Xbox 10/09, PS3 09, NHL 07 refusals passed).

## Active package

- **None — campaign closed 09:10 MDT.** [Receipt 10 (final closeout)](10-final-closeout.md): plain summary, packages and gaps, coverage pointers, aggregate results (studio-domain 386 / studio-api 107 / frontend 136 passed; real-archive sweeps PS4 7/7, gen7 EB 9/9, BIG4 7/7, Legacy team logos 2/2), Codex-first review areas, cards, cleanup, the exact continuation task. No background job or orphan process remains.

## Claims (this lane)

- `local/coordination/overnight-2026-09-20/claims/dev-c0-r1-verification.json` — complete.
- `local/coordination/overnight-2026-09-20/claims/dev-c1-loose-route.json` — complete.
- `local/coordination/overnight-2026-09-20/claims/dev-c2-ps4-real-archives.json` — complete (sweeps finishing in the background).
- `local/coordination/overnight-2026-09-20/claims/dev-c3-c4-gen7-archive-placement.json` — complete (real-archive queues in the background).
- `local/coordination/overnight-2026-09-20/claims/dev-c5-loc-rebuild.json` — complete.
- `local/coordination/overnight-2026-09-20/claims/dev-c6-dimension-change.json` — complete.
- `local/coordination/overnight-2026-09-20/claims/dev-c7-league-closure.json` — complete.
- `local/coordination/overnight-2026-09-20/claims/dev-c8-team-identity.json` — complete.
- `local/coordination/overnight-2026-09-20/claims/dev-c9-team-logo-families.json` — complete.
- `local/coordination/overnight-2026-09-20/claims/dev-c10-big4-binder-ps4-team-text.json` — complete.

## Continuation instruction

If this session ends mid-package: read this file, the latest numbered receipt and the active claim; rerun the
"runnable commands" of the latest receipt; continue from the "Active package" / "Next queue entry".
