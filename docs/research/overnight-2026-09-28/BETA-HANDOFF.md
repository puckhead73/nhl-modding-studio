# Beta handoff (research side) — October 1, 2026 target — current authority from 28 September

Research lane, research-owned. Created 28 September 2026 04:41 MDT; **refreshed 06:39** after developer receipts 46–54, **at 15:20 after receipts 61–65 and the receipt-6 / receipt-65 reviews ([12](12-reconcile-after-receipt6.md))**, and **again 13:21 after receipts 55–62 and the coordinator's receipt-5 / receipt-60 reviews** ([09](09-after-receipt5-corrections.md)) (system clock). Log at the end.
**Supersedes** [`../overnight-2026-09-26/BETA-HANDOFF.md`](../overnight-2026-09-26/BETA-HANDOFF.md) (27 Sep, kept with a pointer). Corrections
that led to this version: [00](00-correction-checkpoint.md) and handoff 20 §10.

**Not** a release assessment: no percentages and no "releasable" statement. The following stay distinct levels: research-ready,
developer-tested, coordinator-reviewed, desktop-observed and game-accepted. Developer receipts are developer reports, not accepted results.

**Settled scope (user, 27 Sep):**
- League and team creation, cloning and deletion are outside the beta.
- Executable patching is final-release scope.
- The XMA helper ships in the release archive.
- MIT is the licence for Studio-owned code (holder name still unrecorded, receipt 46).
- Projects / Builds remain deferred.
- NHL 21 E1 stays accepted.
- Internal / team builds may keep the existing artwork; it is replaced before external release (user, 28 Sep). Licence, artwork and helper packaging are developer / user work, not research.
- **S3 closed (coordinator, 28 Sep):** NHL 11–13 companion style values stay numeric pending an independently supported mapping.

**Build (15:20):** the **r64** review bundle holds developer receipts through 64 (receipt 65), `distribution: internal-review`. Original Hockey Shop F1 is closed for its tested in-memory recovery scope. F2 (head-catalogue cache revalidation at Apply) and temporary-file ownership are **active developer corrections**. *Earlier build line:* the r59 review bundle contains developer receipts through 59 (receipt 60). Receipts 61 (Hockey Shop rollback F1) and 62 (Recompiled public edits) are not in a bundle. The receipt-60 review found F1 and reviewed only bounded parts of 46–60.

Classes:
- **unimplemented** — the operation is absent;
- **unproven mapping** — readable, but meaning or selection is not established;
- **manual acceptance** — only a user desktop, listening or game observation decides;
- **decision** — owned by the coordinator or user.

## Blocker rows (by affected operation)

| # | User-visible operation | Targets | Delivered (developer report) | Research this run | Residual | Class | Owner | Smallest next action |
|---|---|---|---|---|---|---|---|---|
| B1 | League schedule from groups / pair quotas | league tables X360 / PS3 07, 10–Legacy; PS4 15–21 | pair quotas (34), retained rows (41, coordinator-accepted bounded), single-phase grouped + exact-source stock groups with host-rule checks (48; selected suites passed in the receipt-60 review) | R123 revision 2 ([01](01-r123-reference-revision-2.md)), **replayed exactly by the coordinator** (27 + 29 vectors, 144 tables) | Austrian phased generation withheld: meaning of `round = 1`, participant selection, dates (developer obs 48) | manual acceptance / decision | user (card P1-A) → coordinator | card P1-A on a copied NHL 21 save |
| B2 | Style labels agree across surfaces | X360 / PS3 NHL 07–13 | S1 / S2 fixed (35) | S3 **closed** ([09](09-after-receipt5-corrections.md) K12) | NHL 11–13 companion values stay numeric by coordinator decision (not a defect, not a blocker); NHL 10 table inferred | manual acceptance (NHL 10 only) | user | NHL 10 style card |
| D1 | Developer corrections from the receipt-65 review | PS4 heads (and hair reuse); Hockey Shop writes | — | — | F2 warm-cache source revalidation at Apply; `WriteTxn::write` temp ownership | developer correction (not closed by normal head tests) | developer | the receipt-65 dispatch | *(1 Oct, [18](18-after-receipt8-corrections.md) K19: receipt 66 has bounded coordinator acceptance — F2 and temporary-name ownership closed in tested scope, hair offline accepted; earlier unreviewed work and cards stay open.)*
| B7 | Group names NHL 08–11 | X360 / PS3 08–11 | neutral division labels, "(inferred)" conferences (44) | — | division names need the executable consumer | unproven mapping (disassembly, excluded) | — | none in beta |
| B8 / B12 | XMA and archived-music previews | X360 NHL 09–Legacy, Recompiled; PS3 NHL 09 / 10 / 12 / 13 music | 33, 36; in-bank residuals (50); `.ast` NHL 11 / 13 / 14 (53) and NHL 10 / 15 / Legacy (56); **Recompiled banks from its own archives, 5 PCM-equal previews (59)** | [05](05-xma-residual-inbank-shapes.md), [06](06-ast-xma-streams-11-13-14.md) | listening; surround speaker order | manual acceptance | user | cards A-XMA-5 → 8 on r59 |
| B9 | Previews and writes accepted by the user | all | developer-verified only | — | desktop / listening / game observation | manual acceptance | user | on **r64**: V-82 → V-95 (V-94 PS4 heads, V-95 stale apply new), outstanding V-45 → V-81, A-XMA-2 → 8; game cards per [12](12-reconcile-after-receipt6.md) §4 |
| B10 | Point a player at another shipped portrait / head | X360 07–Legacy, PS3 08–Legacy, PS4 15–21, Recompiled | X360 09 → Legacy, PS3 15 / Legacy, PS4 portraits (38); PS3 09 → 14 and Recompiled via archive members (55); **PS4 heads 15–21 (63, independently integration-tested)** | NHL 07 / 08 file-less heads ([07](07-nhl07-08-generic-heads.md)) | F2 cache revalidation (developer correction, blocks hair reuse); NHL 07 / 08; drawn head in game | manual acceptance · developer correction | developer (F2) · user (V-94, P34-1, P34-G08) | F2 correction; card V-94 on r64 |
| B11 | Goalie / skater gear: what each number is | gen7 08–Legacy; PS4 15–21 incl. genuine NHL 20 | G1 (38), file positions (40), G2 (42), G3 (51, 54), PS4 skater gear (52), names (57) | [02](02-ps4-goalie-mask-pair-contract.md), [02b](02b-ps4-skater-gear-catalogue.md), [08](08-cz-name-keys-dictionary.md); gen7 pictures [11](11-gen7-createplayer-thumbnails.md) as corrected in [12](12-reconcile-after-receipt6.md) §3 and [13](13-gen7-feattribdb-negative-and-fallback.md) (no data edge; pads / blocker candidates on sampled titles; gloves not supported) | slot vs personal mask; handedness source; brand / helmet 99; `ASSET_ID` source (native, not in vaults) | manual acceptance · unproven mapping | user (G-MASK-1) · developer (C38 presentation) | C38 prefixes + labelled candidates |
| B13 | Replace an arena's board-ad art | X360 NHL 10–15 / Legacy; PS3 NHL 11–15 / Legacy | read-only gallery (49, 58); no writer | [03](03-board-art-replacement-boundaries.md) as corrected in [09](09-after-receipt5-corrections.md); PS3 contract [10](10-ps3-board-raster-writer-contract.md) **with the corrected admission validator (§8, R142b, 29 / 29 vectors)** | coordinator review of C37; developer transaction / encoder tests; X360 gates (packed tail, R30-LZX-1, NHL 10 topology); precedence; in-game result. Gallery or refusal ≠ completion | unimplemented · manual acceptance | coordinator → developer · user | coordinator review of handoff 10 §8 |
| B14 | Legacy Recompiled claims | Recompiled (registered install) | DB public edit / import / save / reopen (62); banks (59); archive-held reassignment (55); logo table + member replace | routes per operation ([12](12-reconcile-after-receipt6.md) §2); **speech read contract [16](16-recomp-speech-read-contract.md)** (chunklzx gap in the preview reader); loose-override premise [15](15-recomp-loose-override-premise.md): the runtime probes `cache:\` loose paths (user-data folder), precedence unknown, card R-RECOMP-LOOSE-1 | **named missing routes:** speech catalogue, speech-preview `game/` root, containment card (misleading source text), portrait / head / equipment previews, team header logo, league imagery row (`legacyrecomp` id split), kits, venue art; O12-1 Hockey Shop loose-only route admitted; runtime loading / saves / limits not inferred | unimplemented · manual acceptance | developer · user | the §2 table rows, highest first: containment disclosure and O12-1 | *(1 Oct: speech route, containment index and Hockey Shop guard delivered in receipt 67, reviewed with read-path robustness findings; read contracts added for arena [19](19-recomp-arena-read-contract.md), equipment [20](20-recomp-equipment-read-contract.md), team header logo [21](21-recomp-team-header-logo-read-contract.md), kits [22](22-recomp-kit-read-contract.md); card R-RECOMP-LOOSE-1 replaced by revision 2 (18 K20), pending a user backup / restore plan; developer frozen on r67.)*
| B15 | New-size logos on NHL 07–11 | X360 07–11, PS3 08–11 | package-form resize (37) | — | in-game result; crops / multi-mip refused | manual acceptance | user | logo card |

**Removed from the beta blockers** (user decisions, 27 Sep; later scope kept):
- **B4** league / team creation, cloning and deletion;
- **B6** executable patching.

**No beta residual:** **B3** nationality (optional card only).

**Parked:** **B5** art-id re-key.

## Log

- 04:41 — created from checkpoint 00 (package 1).
- 04:46 — B1: research reference revision 2 delivered ([01](01-r123-reference-revision-2.md)); Austrian phase semantics = card P1-A (static facts in 01 §4).
- 05:02 — B11: G3 contract delivered ([02](02-ps4-goalie-mask-pair-contract.md)); skater gear name keys / thumbnails for PS4 17–21 ([02b](02b-ps4-skater-gear-catalogue.md)) — new developer-consumable item for Equipment 4.1–4.4.
- 05:38 — B14: archive-member routes per operation ([04](04-recompiled-archive-routes.md)); B8 / B12: residual XMA in-bank shapes decode exactly ([05](05-xma-residual-inbank-shapes.md)) — incl. a false refusal of valid (0, 0x7FFF) packets on NHL 10 → Legacy.
- 06:02 — B13: same-size operation contract delivered ([03](03-board-art-replacement-boundaries.md)); open = copy precedence, chunklzx re-wrap, X360 packed tail, in-game result.
- 06:39 — refreshed after developer receipts 46–54 (G3 / skater gear / XMA / `.ast` consumed); B13 coverage widened (§8–9); B10 NHL 07 / 08 narrowed ([07](07-nhl07-08-generic-heads.md)).
- 06:47 — B11: Creation Zone names resolved ([08](08-cz-name-keys-dictionary.md)); consolidated receipt [RECEIPT-5](RECEIPT-5.md) published.
- 13:21 — after-receipt-5 prompt, package 1: refreshed after developer receipts 55–62 and the coordinator's receipt-5 / receipt-60 reviews ([09](09-after-receipt5-corrections.md)): B2 S3 closed (coordinator decision); B13 twins qualified, loose-only allowance withdrawn; B14 R138 narrowed to path matches + samples; delivered portions recorded in B8 / B10 / B11 / B13 / B14.
- 13:40 — after-receipt-5 run closed: [10](10-ps3-board-raster-writer-contract.md) PS3 board-raster contract (B13), [11](11-gen7-createplayer-thumbnails.md) gen7 create-player thumbnails (B11), consolidated [RECEIPT-6](RECEIPT-6.md).
- 15:21 — after-receipt-6 run closed: [12](12-reconcile-after-receipt6.md), [13](13-gen7-feattribdb-negative-and-fallback.md), handoff 10 §8, [RECEIPT-7](RECEIPT-7.md).
- 18:49 — after-receipt-7: [14](14-after-receipt7-corrections.md) corrections (C37 accepted for implementation; vault narrowed; process deviations); [15](15-recomp-loose-override-premise.md) O12-1 premise + card R-RECOMP-LOOSE-1; [16](16-recomp-speech-read-contract.md) Recompiled speech read contract (C39).
- 18:52 — [17](17-recomp-portrait-head-read-contract.md) portrait / head read contract (C40); consolidated [RECEIPT-8](RECEIPT-8.md).
- 01:54 (1 Oct) — after-receipt-8: [18](18-after-receipt8-corrections.md) corrections (15 withdrawn ordering, card revision 2 pending; 16 / 17 amendments); read contracts [19](19-recomp-arena-read-contract.md) arena, [20](20-recomp-equipment-read-contract.md) equipment, [21](21-recomp-team-header-logo-read-contract.md) team header logo, [22](22-recomp-kit-read-contract.md) kits (queue C41–C44); consolidated [RECEIPT-9](RECEIPT-9.md).
