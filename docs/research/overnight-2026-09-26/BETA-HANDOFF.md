# Beta handoff (research side) — October 1, 2026 target

> **Superseded 28 Sep 2026 04:41:** current authority is [../overnight-2026-09-28/BETA-HANDOFF.md](../overnight-2026-09-28/BETA-HANDOFF.md); corrections in [checkpoint 00](../overnight-2026-09-28/00-correction-checkpoint.md) (B4 / B6 removed by user decision, B13 wording, build pointer). This file is kept as the 27 Sep record.

Research lane, research-owned. Started 27 September 2026 05:00 MDT (system clock); updated at each completed package (see the log at the
end). Inputs: developer LEDGER "Refresh 26 Sep — current authority", developer receipts 28–31 (developer reports, not accepted results),
the research queue (`overnight-2026-09-25/DEVELOPER-QUEUE.md` C1–C18) and this campaign's handoffs. **Not** a release assessment: no
percentages, no "releasable" statement; research-ready, developer-tested, coordinator-reviewed, desktop-observed and game-accepted remain
distinct. Any scope change below is a **decision for the user**, never an adopted cut.

Classes: **defect** (current behaviour contradicts measured evidence) · **unimplemented** (operation absent; contract exists or is named) ·
**unproven mapping** (data readable, meaning / selection not established) · **unavailable source** (needed input not on this machine or not
authorised) · **manual acceptance** (only a user desktop / listening / game observation decides).

## Critical dependencies for October 1 (research view)

1. **Coordinator acceptance of research contracts that unblock developer work already in scope** — C18 (grouped schedule generation) is
   the largest remaining League 1.2 operation with a complete static contract.
2. **User decisions that cannot be made autonomously** — whether creation / cloning / deletion of leagues and teams is in the beta (needs a runtime experiment, no static contract can replace it); gen7 rules via exe patching (B6). *(Corrected 05:21: the XMA toolchain authorisation was given by the user on 2026-09-27 — no longer a pending decision.)*
   cloning / deletion of leagues and teams is in the beta (needs a runtime experiment, no static contract can replace it).
3. **Manual acceptance cards** — every preview / write is developer-verified only; the review build (`E:\nhlms-review-build-2026-09-27`)
   contains receipts through 28 only.

## Blocker rows

| # | User-visible operation | Targets | Evidence level now | Missing | Class | Lane | Smallest next action | Depends on | Static research can help? |
|---|---|---|---|---|---|---|---|---|---|
| B1 | Generate a grouped league schedule (Swiss, Finnish, Austrian, NHL 07 Swedish) or from a user games-per-pair table | X360 / PS3 NHL 07, 10–Legacy; PS4 15–21 (league tables) | research contract + reference validator (C18); Studio: round robin only (dev) | implementation of partition / matrix / phases generation with Q / R refusals | unimplemented | coordinator → developer | coordinator accepts or amends [16](16-schedule-generation-contract.md); developer implements behind the existing held-review apply | coordinator review | done (static); runtime meaning of the Austrian `round = 1` tag stays open (card P1-A) |
| B2 | Style labels on the Lines page / roster cell agree with the player page | X360 / PS3 NHL 07–10 (S1 / S2); NHL 11–13 missing labels (S3) | measured (split, agreement, dictionaries) + source reading; not desktop-observed | per-title scoping of `player-style.toml` and `PLAYER_STYLE_LABELS`; S3 decision | defect (S1 / S2), decision (S3) | developer; coordinator for S3 | regression per [15](15-native-label-consumers.md) §4 | — | done; NHL 10 labels stay inferred without an in-game check |
| B3 | Nationality labels | NHL 08–12 unlabelled non-national-team codes | developer-reported delivered (receipt 28); residual codes 83 / 90 on NHL 08 / 09 are birthplace candidates | none for beta (numbers shown by design) | — | — | optional in-game check (Kopitar / Vas on NHL 08) | — | exhausted statically |
| B4 | League / team creation, cloning, deletion | all | contract missing; gen7 tables packed; PS4 spare rows without an "active" representation | table-growth tolerance + whole-graph reference contract | unimplemented + unproven (runtime) | user decision → research / runtime | **decision for the user**: in or out of the beta; if in, one runtime experiment on a copy (grown table) | runtime observation | only after the experiment |
| B5 | Art-id re-key / archive rename | gen7 archives | contract missing (six readiness layers; same-length outer rename designable only) | writer contract covering every layer and both copies | unimplemented | research → developer | none before beta without a new discriminator; swap-between-existing remains | — | limited |
| B6 | gen7 rules / season settings | X360 / PS3 NHL 10–Legacy | compiled in `default.xex` | a data-file copy or accepted exe patching | unavailable source | user decision | **decision for the user**: exe patching in or out | — | no |
| B7 | Conference / division names NHL 08–11 | X360 / PS3 08–11 | raw numbers shown; name lists recovered from the NHL 10 / 11 executables (handoff 26); conference = position fits; **division binding neither positional nor reversed** | the consumer code that picks a division name | unproven mapping | developer (neutral member-list labels) · research only with code tracing | label divisions by member abbreviations; conference by position (inferred) | — | only with disassembly |
| B8 | XMA previews (Xbox 360 banks / SNS) | X360 NHL 09–Legacy | **helper built** (developer receipt 32: pinned LGPL FFmpeg 8.1.3, 83 / 83 R84 vectors bit-identical); SNS listing wiring = developer's next step; NHL 09: full research sweep 471 / 471 streams, 1 983 / 1 985 in-bank exact (handoff 24) | SNS / `.ast` / NHL 09 admission in Studio; listening | unimplemented (in progress) + manual acceptance | developer | admit NHL 09 with the count rule only (C26); helper PCM-identity rerun on a sample | — | done for NHL 09 decoder evidence |
| B9 | All previews and writes accepted by the user | all | developer-verified only | desktop / listening / game observation | manual acceptance | user | cards in the developer MORNING-REVIEW and research cards (L-LOGO-1, K-NAME-1, T-CHANT-1 rev. 3, A-SPX-1 rev. 3, A-AST-1, A-LOOP-1, T-ARTID-1, H-HORN-1 rev. 2, P1-A / P1B-3) | a build with the later receipts | no |
| B10 | Reassign a player's portrait / head to another shipped asset | X360 07–Legacy, PS3 08–Legacy (08–14 from archives, handoff 17 §9–10), PS4 15–21; Recompiled = X360 Legacy bytes (handoff 21) | listing implemented (dev); reassignment conditions measured (C19); no writer | selection by the game (runtime) + a writer | unimplemented + unproven mapping | developer (writer) · user (card P34-1) | card P34-1 on a copy; writer behind the C19 refusal list | P34-1 if the coordinator requires selection evidence first | done statically; NHL 07 / 08 heads without files unexplained |
| B11 | Goalie gear shows what each number is and validates assignments | X360 / PS3 08–Legacy (PS3 08–14 from archives, handoff 18 §8), PS4 15–21 incl. genuine NHL 20 | database parity done (dev, receipt 31); value → file / catalogue candidates measured (C20) | display / validity per C20; which item the game draws | unimplemented (labels) + unproven mapping (selection) | developer · user (card G-MASK-1) | G1 (NHL 08 / 09 `maskcolor`), G2 (team slots), G3 (PS4 pair) | — | done statically for goalie gear; skater gear next |
| B12 | Preview PS3 NHL 09 / 10 / 12 / 13 menu music | PS3 NHL 09, 10, 12, 13 | research contract established (R82, 3 vectors each); Studio lists and refuses the tuple | add the four `music_samples` tuples to `ealayer3::MEASURED` with the R82 regression | unimplemented (contract ready) | developer | A1 in [19](19-audio-admission-residuals.md) | — | done |
| B13 | Replace an arena's board ads (gen7) | X360 NHL 10 / 13 / 15 / Legacy measured; PS3 15 / Legacy packages identical in structure | art per arena venue package; **material → raster decoded** (handoff 20 §6: slot 0 at `0xEF0001` +256); board-ad rasters nameable | in-place same-size write (inputs measured, handoff 20 §8: DW8 pairing, no checksum) + restore; in-game check | unimplemented (contract missing for the write) | developer (listing now; write contract) · user (card) | list / preview an arena's board-ad rasters by material name; then design the in-place write | — | done for identification |
| B14 | Every Legacy Recompiled target claim | Legacy Recompiled (registered install) | **database / game-file identity measured**: 20 / 20 files = X360 Legacy; archived `db/nhlng.db` = the X360 Legacy anchor (handoff 21) | loader precedence, save / header sidecar, runtime | manual acceptance + unimplemented (per-feature admissions) | developer (cite identity; decide admissions) · user (runtime) | developer receipts may state Recompiled database coverage by measured identity for this install | — | done (bytes); runtime not static |
| B15 | Give a team / league logo a new size on NHL 07–11 | X360 NHL 07–11 and PS3 NHL 08–11 measured (NHL 07: wide / league families are crops; some payload chunks unresolved) | refused by name today (receipt 13); Apt rule and ARGB8 descriptor rule measured (C27) | ARGB8 RW4xb2 resize + 08–11 package route + re-wrap | unimplemented (contract input ready) | developer · user (card) | implement per [25](25-old-logo-package-resize-inputs.md) §4 | — | done for X360; PS3 `RW4ps3` package form not measured |

## Log

- 05:00 — created after packages 0 (checkpoint 00d), 2 (handoff 15) and 1 (handoff 16).
- 05:08 — B10 added (package 3, handoff 17).
- 05:17 — B11 added (package 4, handoff 18).
- 05:20 — B12 added (package 5, handoff 19).
- 05:21 — B8 and dependency 2 corrected: the user approved the XMA toolchain (developer plan of 27 Sep); helper build active.
- 05:27 — B13 added (package 6, handoff 20).
- 05:34 — B14 added (Recompiled source identity, handoff 21). B7 recorded as needing the executable consumer trace (decompressed images from 18 Sep no longer present; local->name binding untraced); B13's third look (parameter chunks) found no direct raster index.
- 05:53 — B8 updated: helper built (receipt 32); NHL 09 XMA sweep (handoff 24). Scorebug team assets negative (handoff 23); animation fields census (handoff 22).
- 06:05 — B15 added (handoff 25, logo resize inputs on X360 NHL 08–11).
- 06:16 — B7 narrowed (handoff 26).
- 06:24 — B10 / B11 coverage updated (PS3 archive-held titles measured).
- 06:27 — B13 upgraded: material → raster decoded (handoff 20 §6).
