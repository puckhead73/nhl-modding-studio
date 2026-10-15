# Developer lane status — overnight 2026-09-22

Campaign: the user's extended unattended development instruction of 22 September 2026 (P0 close the
R2 defect → P1 complete the resize evidence → P2 actionable Phase 1 → P3 Teams → P4 Players /
Equipment / Miscellaneous). It **supersedes** the "stop for coordinator review" instruction in
`docs/validation/overnight-development-20260921-r1-followup.md` without erasing that review's
findings. Codex is architect/reviewer; the research lane owns `docs/research/**` and `research-*`
claims — nothing there is edited here.

**Starting state, recorded rather than assumed:** HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051`
(branch `feat/phase5-asset-tooling`), working tree dirty with **721 entries (187 modified, 534
untracked)** from the 18–21 September campaigns. Preserved except for the paths named in each
receipt. No staging, commits, pushes, resets, branch changes or clean operations; no game or
emulator launches; no writes to installs, images, archives, anchors or saves; real trees on `X:` and
`H:` are read-only inputs; mutation is exercised on disposable copies.

Every checkpoint is **developer-verified, not Codex-accepted, not desktop-exercised, not
game-verified**. Visual/runtime review is queued for the user. NHL 21 E1 stays accepted and closed.
Projects/Builds are not touched.

Ownership: this session is the sole application-code writer (claims `dev-*` under
`local/coordination/overnight-2026-09-22/claims/`).

## Checkpoints

- **P0 — R2: descriptor validation order** — delivered, developer-verified:
  [receipt 01](01-r2-descriptor-validation-order.md). The coordinator's `u32::MAX` DXT5 panic
  reproduced red, then fixed by moving every bound ahead of the dimension-dependent arithmetic;
  `try_tiled_buffer_size{,_texels}` added as checked forms; the `DW7` pitch `debug_assert` became a
  refusal. 118 `tdb-rx2` lib + 2 public-route tests pass. Receipt 16's boundary claim corrected with
  a dated note.

- **P1 — resize evidence: real GNF, fixtures, `DW8` join** — delivered, developer-verified:
  [receipt 02](02-p1-resize-evidence.md). Six SHA-256-pinned real NHL 21 / NHL 16 GNF members
  resize through the codec and the public route on disposable copies (the halve→regrow round trip
  reproduces the shipped header **byte-for-byte**); the private fixture root was restored by
  provenance, taking `tdb-rx2 --lib --include-ignored` from 118 passed / 26 loudly failing to
  **144 passed, 0 ignored**; `DW8` is now a checked join rather than a preserved unknown; GNF
  multi-stream files refuse with their own reason.

- **P2.1 — NHL 07 / 08 descriptor generation** — delivered, developer-verified:
  [receipt 03](03-p2-nhl07-08-descriptor-generation.md). The `DW0 = 3` gate is lifted on research
  R42 §2's measurement and reproduces 9 388 / 9 390 (NHL 07, 2 cube maps) and 9 744 / 9 744
  (NHL 08, 110 undecodable jersey libraries asserted) on the real trees; NHL 09's 243
  previously-refused descriptors now work; the NHL 07 **container** writer stays refused by its own
  layout reason. The `DW8` join holds on 22 401 + 16 981 + 13 836 descriptors across three trees.

- **P2.4 — the Contexts listing reaches PS4** — delivered, developer-verified:
  [receipt 04](04-p2-ps4-context-listing.md). A little-endian `0XTC` reader produces the **existing**
  `Ctx0` type, so the card and the view are shared; all seven PS4 titles reproduce the research
  census exactly through the registry seam. The condition-list layout the research left unspecified
  was measured (pointer array, 440 / 440) before use, and an overflow defect in the new reader was
  caught by its own test and fixed with checked reads.

- **P2.5 — arena venue parameters inventoried** — delivered, developer-verified:
  [receipt 05](05-p2-arena-venue-inventory.md). A developer-owned class map (159 fields, from R51)
  plus per-title presence and constancy **measured on the open database**; colour zones render as
  colours; unclassified fields are refused for editing with the reason. Four era signatures tested;
  NHL 21's 65 fields and 17 / 450 colour-carrying rows agree with R51 independently. Bindings
  regenerated and in sync.

- **P3.4 — the pool-head floor follows the title era** — delivered, developer-verified:
  [receipt 06](06-p3-pool-head-floor.md). Research R53 §2 named Studio's fixed 60 000 rule wrong for
  NHL 09–13; the floor is now measured from the open file (60 000 when it carries such ids, else
  10 000). Pool populations on five anchors match the research, two exactly. The portable
  shared-head signal is unchanged and now agrees with the id rule on 99 %+ of players.

- **P2.7 — the PS4 goal horn reads as a club** — delivered, developer-verified:
  [receipt 07](07-p2-goalhorn-catalogue-join.md). `goalhorns.csv` is read and joined by R50 §2's
  measured rule; NHL 21 (150 club horns / 3 Rangers gap / 11 sentinels) and NHL 17 (46 / 1 / 0)
  reproduce the research exactly with **0 unresolved arenas**. `periodhorn` is deliberately left a
  number — same id space, not joined by the research.

- **P2.6 — a team's kits joined to their textures** — delivered, developer-verified:
  [receipt 08](08-p2-kit-selector-join.md). `(style, team art id, variant)` → jersey / pant / sock
  resolves **627 / 627, 659 / 659, 703 / 703** rows on the real NHL 11 / 15 / Legacy trees, matching
  R56 exactly, with spare pools of 97 / 54 / 228. A wrong first reading of the sock template was
  caught by the real-tree test (0 of 627) and corrected: there are **two** measured sock forms and
  the resolver picks by inspecting the tree. Domain + evidence; the Uniforms surface is next.

## Consumed research (unedited, 2026-09-21 pm lane)

| Handoff | Used for |
|---|---|
| 02 (R42 / R47) §1 | `DW8` = file-table index of the paired pixel-data entry → a checked refusal |
| 02 §2–3 | NHL 07 / 08 `DW0 = 3` generation lift; the 110 undecodable NHL 08 jersey libraries |
| 02 §4–6 | field bounds confirming the R1 constants |
| 03 (R43) §2 | GNF multi-stream files (NHL 15 / 19) → a dedicated refusal reason |
| 15 (R58) | the PS4 `0XTC` context walk → the Contexts card on NHL 15–21 |
| 09 (R51) | the arena venue-field inventory → `defs/arena/venue-fields.toml` and the Arena card |
| 11 (R53) §1–2 | the gen7 pool-head id ranges → the era-measured pool floor |
| 08 (R50) §1–2 | the PS4 goal-horn catalogue → the arena card's horn name |
| 14 (R56) §1–2 | the gen7 kit selector → the jersey / pant / sock join and the spare pool |

- **Closeout** — [receipt 09](09-closeout.md): eight packages, the integration runs, the deferred
  cards, the cleanup manifest and the review areas. HEAD unchanged; nothing staged.

## R3 — fresh-session extended dispatch (coordinator review of 2026-09-22)

Dispatch: [overnight-development-20260922-review.md](../../validation/overnight-development-20260922-review.md).
Same rules; HEAD still `8d60a6d`; nothing staged, committed, pushed, launched or written live.
Every checkpoint below is developer-verified, not Codex-accepted.

- **R3.1 — F1 kit completeness corrected + the read-only Uniforms card + ten-tree coverage** —
  delivered, developer-verified: [receipt 10](10-r3-kit-completeness-correction-and-uniforms.md).
  The contract comes from the title / platform, every required component carries present / missing /
  unknown evidence, one rule judges rows and spare candidates, unreadable folders are unknown. The
  coordinator's reproduction is a permanent red / green at the resolver, the registry seam and the
  card. 100 % of rows on X360 NHL 11 / 12 / 13 / 14 / 15 / Legacy and PS3 NHL 11 / 14 / 15 / Legacy.
  Receipts 02 / 05 / 08 / 09 corrected with dated notes; observations for research published
  ([OBSERVATIONS-FOR-RESEARCH.md](OBSERVATIONS-FOR-RESEARCH.md), dispatch item 3).
- **R3.2 — Xbox 360 mip-layout audit** — delivered, developer-verified:
  [receipt 11](11-r3-xbox360-mip-layout-audit.md) and
  [the audit record](../../validation/xbox360-mip-layout-audit-2026-09-22.md). Content-based
  placement check on 1 100 real rasters: the sequential model holds for every level above 64 texels
  and is contradicted for 19–34 % of packed-tail levels; Studio's chain allocation exceeds every
  shipped slot. The same-size Xbox route now rewrites only the verified levels and keeps the shipped
  tail (disclosed); the changed-size route discloses its unestablished tail. No mipped write expanded.

- **R3.3 — Teams 2.5: the arena's shipped assets** — delivered, developer-verified:
  [receipt 12](12-r3-arena-assets-card.md). Gen7 venue model / jumbotron by `arenaartid` on 100 % of
  rows of six real trees with the spare venues equal to R62's residuals; centre-ice logo / banners
  through the home team (or "no home-arena link" on 09–11); bounce map a candidate; the other ice
  layers unbound; PS4 venue directories and `common/` families listed under a vault reference Studio
  does not read. Read-only card on the Arena page; nothing written.

- **R3.4 — Teams 2.4: the team-logo dimension sweep** — delivered, developer-verified:
  [receipt 13](13-r3-team-logo-dimension-sweep.md). 168 gen7 packages of the private fixture set:
  07–11 refuse by name (same-size package forms), 12–Legacy resize where the quad is the texture's
  own size with every invariant held, calendar / crop families refuse as display crops; 0
  unexpected outcomes. No production code changed.

- **R3.5 — Players 3.4: portrait / head containment** — delivered, developer-verified:
  [receipt 14](14-r3-player-containment-card.md). The `hasportrait` flag shown as stored and
  cross-tabulated against the shipped files (never derived, never written), bespoke / pool presence,
  "shipped without asset" named, the unreferenced pools counted; R53 reproduced to the unit on
  X360 09 / 13 / Legacy and PS3 Legacy; PS4 portraits by art id with heads unmeasured. Summary on the
  Player assets index pane, a line on each player's page.

- **R3.6 — Equipment 4.x: component sets** — delivered, developer-verified:
  [receipt 15](15-r3-equipment-component-sets.md). The player's equipment card says whether an
  assigned id carries the family's majority set (R52) and what it lacks, per in-game / front-end
  group; single-container families and unmeasured titles stated; the family census reproduces R52
  on Legacy (both consoles) with two recorded reading differences and the same shape on 13 / 15.
  No writer.

- **R3.7 — Misc 5.4 / Teams 2.7 PS4: the song catalogue listing** — delivered, developer-verified:
  [receipt 16](16-r3-song-catalogue.md). The NHL 17–21 catalogue listed on the Team page as
  "selectable songs", with the two measured negatives stated (no join from the team's song cells,
  no bank binding); real installs reproduce R54's row counts; NHL 15 / 16 and gen7 refuse by contract.

- **R3 closeout** — [receipt 17](17-r3-closeout.md): seven packages after the correction, the
  closeout regression, the deferred cards (V-30 … V-35), the observations file, the cleanup
  manifest and the review areas. HEAD unchanged; nothing staged.

## Exact next action

Consume the research lane's Xbox 360 packed-tail contract (Q-R65-1) when it lands: replace the
sequential tail model in the texlib writers with the established per-level addresses, extend the
placement audit to assert every tail level, and only then lift receipt 11's tail disclosures. Until
it lands, the next independent package is the PS4 dictionary resolution of the `$TXT_SONG_*` /
`$TXT_*` keys on the song and arena cards through the existing team-text route.
