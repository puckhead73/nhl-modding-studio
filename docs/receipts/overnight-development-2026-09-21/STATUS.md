# Developer lane status — overnight 2026-09-21

Campaign: the user's 21 September overnight developer instruction (P0 colour race → P1 foundation
audit → P2 actionable League work → P3 Teams → P4 Players → P5 Equipment → P6 Miscellaneous).
Codex is architect/reviewer; a separate research session works ahead and owns `docs/research/**`,
`local/runs/overnight-research-*/**` and `research-*` claims — nothing there is edited here.

Baseline revision `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (branch `feat/phase5-asset-tooling`,
started 04:27 MDT); the dirty working tree (676 entries at start, the 18–20 September campaigns and
their corrections, all uncommitted) is preserved untouched except for the paths named in each
receipt. No staging, commits, pushes, resets, branch changes or clean operations. No game/emulator
launches; no writes to installs, images, archives, saves, anchors or profiles; private fixtures under
`E:\nhl-studio-fixtures` are read-only inputs; mutation paths are exercised on disposable copies only.

Every checkpoint here is **developer-verified, not Codex-accepted, not desktop-exercised, not
game-verified**. Visual/listening/in-game review is deferred to the user (`MORNING-REVIEW.md`).
NHL21-E1 stays accepted and closed. Projects/Builds are not implemented.

Ownership: this session is the sole application-code writer (claims `dev-*` under
`local/coordination/overnight-2026-09-21/claims/`). Research claims of the 20 September campaign are
all `ready`; no active research claim was found at start (latest: `research-r20-gen7-speech-events`,
rev. 4, 2026-09-20 18:10; the R22/R23 review of 21 September asks research to correct F1/F2 there).
No `docs/research/overnight-2026-09-21/` existed at start; checked at each package boundary.

## Checkpoints

- **P0 — R3 colour settlement generation** — delivered 04:36, developer-verified:
  [receipt 01](01-r3-colour-settlement.md). The late-`onSettled` re-arm is fixed by binding the
  duplicate guard to (write, authoritative generation); 6 deferred-summary cases (3 discriminate
  against R2), 47 focused / 156 full frontend / tsc clean, logs retained.

- **P1 — foundation audit** — delivered 06:55, developer-verified: [receipt 02](02-p1-foundation-audit.md).
  Seven foundations read; 7 defects fixed (null key in the dictionary rebuild; BIG4 `.bh` decoded word
  unchecked + unbounded header words; replan count bound; XPR2 DXT pitch byte / height clamp; PS3 DXT DDS
  always-full-chain; mirrored-edit rollback left a redoable half-edit); 17 regressions incl. an 82-member
  real XPR2 descriptor oracle; aggregate of the six crates green after one diagnosed test update (logs retained).

- **P2-A — BIG4 ordered reflow + NHL 07 boot.big cascade** — delivered 06:05: [receipt 03](03-p2a-big4-reflow-and-nhl07-cascade.md).
  Reflow reproduces the research placement oracle 16/16 on the six real archives; real NHL 10 X360 + PS3 routes;
  NHL 07 `data0.big` + `boot.big` four-file cascade on the real install copy; per-member placement carried from review to apply.
- **P2-A — chunklzx decoder promoted** — delivered 06:12: [receipt 04](04-p2a-chunklzx-decoder-promoted.md).
  50/50 research vectors byte-exact on the real archives; read paths decode; write paths refuse with the encoder reason.
- **P2-E — commentary catalogue** — delivered 06:22: [receipt 05](05-p2e-commentary-catalogue.md).
  New `tdb-speech` crate (research model reproduced on 10 loose trees), domain catalogue, API, Leagues → commentary page;
  trace figures exact through the API on NHL 15 / Legacy.
- **P3 — team re-identification (2.1)** — delivered 06:40: [receipt 06](06-p3-team-reidentification.md).
  Index refused with counts; art-id swap as one multi-table undoable step (roster-mirrored); bare art-id edits refused
  at the write layer; Team page Identity card. Contract = research handoff 10 (proposal).
- **P3 — team name calls (2.7)** — delivered 07:00: [receipt 07](07-p3-team-name-calls.md).
  Read-only `team_ID` binding on the Team page; research counts 207 / 211 (NHL 15) and 206 / 212 (Legacy) reproduced on both platforms' anchors.
- **P3 / 2.5 — chunklzx store route** — delivered 07:25: [receipt 08](08-p3-chunklzx-store-route.md).
  Native store encoder = research sizes exactly (16 synthetic + 50 real members); stager/BIGF reader/EB member route write `chunklzx` members back in the same container; loader acceptance unestablished (card R30-LZX-1).
- **P2-C / P3 — group labels + static glyph gate** — delivered 07:55: [receipt 09](09-p3-group-labels-and-glyph-gate.md).
  PS4 groups 77 / 78 named from their leagues (handoff 08); `defs/fonts/glyph-coverage.toml` + per-character verdict in the dictionary previews, C1 refused (handoff 09).
- **P4 — player recorded names (3.5)** — delivered 08:20: [receipt 10](10-p4-player-name-calls.md).
  Player page card: `playernames` rows by name text → pbp `player_ID`; bio audio id → announcer `playerid`; research 5 311 / 4 464 exact on NHL 15.
- **P2-B — texture resize (RW4xb2 / RW4ps3 / GNF)** — delivered 08:20: [receipt 11](11-p2b-gen7-ps4-texture-resize.md).
  Descriptor rules of handoffs 11 / 15; X360 rule reproduced on 18 984 / 18 984 real Legacy rasters, PS3 chain rule on 26 054 + 20 245 with exactly the research's cubes; two read-side defects fixed; runtime cards open.
- **P5 — equipment file check (gen7)** — delivered 08:32: [receipt 12](12-p5-equipment-file-check.md).
  Player → Equipment card; NHL 11 grammar; goalie-stick column fixed; 3 296 + 3 099 assigned ids all with files on Legacy / NHL 11.
- **P2-D — NHL 07 membership** — delivered 08:48: [receipt 13](13-p2d-nhl07-membership.md).
  Stock-row target through the selector, 2006-07 conference / division labels, All-Star guard; the old refusal is gone.
- **P2-E — context listing (CTX0 v4.5)** — delivered 08:50: [receipt 14](14-p2e-context-listing.md).
  Parsers (v4.5 + v3.1) record-for-record vs the research censuses on 7 + 3 sets; Contexts card on the commentary page covers NHL 11–Legacy.

## Active package

- None — campaign closed 09:05: [receipt 15](15-closeout.md) (aggregate 788 / 0 / 106, frontend 175, all censuses green). Next: Codex review of receipts 01–14, the morning cards, then the continuation task in receipt 15 §8.

## Claims (this lane)

- `local/coordination/overnight-2026-09-21/claims/dev-r3-colour-settlement.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p1-foundation-audit.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p2a-big4-reflow-nhl07-cascade.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p2a-chunklzx-promotion.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p2e-commentary-catalogue.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p3-team-reidentification.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p3-team-name-calls.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p3-chunklzx-store-route.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p3-group-labels-glyph-gate.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p4-player-name-calls.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p2b-texture-resize.json` — complete (GNF census outcome recorded at closeout).
- `local/coordination/overnight-2026-09-21/claims/dev-p5-equipment-file-check.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p2d-nhl07-membership.json` — complete.
- `local/coordination/overnight-2026-09-21/claims/dev-p2e-context-listing.json` — complete.

## Continuation instruction

If this session ends mid-package: read this file, the latest numbered receipt and the active claim;
rerun the "commands" table of the latest receipt; continue from "Active package".
- **R1 correction — resize representability (F1) + swap completion ownership (F2)** — delivered 21 Sep (evening), developer-verified:
  [receipt 16](16-r1-resize-representability-and-swap-ownership.md). Both review findings reproduced red first
  (GNF 16 400 → 16, RW4xb2 8 196 → 4; a project-7 swap failure on project 8's card); writers refuse unrepresentable
  requests before building and re-read their output; the card owns completions by immutable dispatch context.
  tdb-rx2 / studio-domain / studio-api aggregate 621 / 0 / 80; frontend 26 files / 182; real-container halvings rerun.
