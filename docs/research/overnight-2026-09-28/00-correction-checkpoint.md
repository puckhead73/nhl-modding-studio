# Checkpoint 00 (28 Sep) — corrections to research authority after receipt 4, and what the developer has already consumed

Research lane, 28 September 2026, published 04:40 MDT (system clock). Prompt
[`research-20260928-fresh-overnight-after-receipt4.md`](../../plans/research-20260928-fresh-overnight-after-receipt4.md) package 1.
Claim `research-q1-correction-checkpoint` (`local/coordination/overnight-2026-09-28/claims/`). Inputs: the
[beta scope decisions](../../plans/beta-20261001-scope-decisions.md), the
[receipt 45 review](../../validation/development-20260928-receipt45-review.md), developer receipts 32–45 and LEDGER (developer reports,
not accepted results), the empty user results file `docs/plans/user-verification-2026-09-27-results.md`, and research receipt 4 with its
handoffs. Nothing was re-measured for this checkpoint. R121 determinism and the settled decoder and logo censuses were **not** rerun.

**This campaign.** New handoffs go in `docs/research/overnight-2026-09-28/`. The current research beta authority moves to
[`BETA-HANDOFF.md`](BETA-HANDOFF.md) here, and the 27 Sep file carries a dated pointer. The single developer queue stays at
[`../overnight-2026-09-25/DEVELOPER-QUEUE.md`](../overnight-2026-09-25/DEVELOPER-QUEUE.md). Earlier handoffs and raw data are unchanged
except for dated notes.

## Plain-language summary

The user has decided that league and team creation, cloning and deletion, and patching the game program, are outside the October 1
beta. Those two rows leave the research blocker list. Most of what research handed over on 27 September is now implemented by the
developer (reports awaiting review): style labels, Xbox NHL 09 and PS3 archived music, old-title logo resizing, portrait and head
reassignment, goalie mask paints, schedule preservation and inferred group labels. Several research statements were too strong and are
corrected below: the board-art "binding undecoded" wording, a "no checksum" negative, "every file identical" for Recompiled, and "all
shipped schedules pass". Research also withdraws the closing claim of receipt 4 that nothing static was left to do. §4 lists, for each
residual, the question that would settle it and whether static work can.

## 1. Corrections (dated 28 Sep; the original text stays in place with a pointer)

| # | Where | Was | Corrected to |
|---|---|---|---|
| K1 | BETA-HANDOFF B4, B6; dependency 2 | B4 creation / cloning / deletion and B6 executable patching as beta blockers "for the user to decide" | **Removed from the beta blockers** by the user's decisions (beta scope §1 + clarification, §2). Kept as later scope: B4 → post-beta lifecycle work (static reference contract + runtime table-growth experiment, research preserved), B6 → final-release executable-patching backlog. Existing league / team edits and data-file settings stay in scope. |
| K2 | RECEIPT-4 next task (§ "one exact next task") | "no static item remains open without either a runtime observation, a user decision or disassembly-grade work" | **Withdrawn.** Finishing a package list does not exhaust static work. §4 lists each residual with its next discriminating question, method and class. |
| K3 | BETA-HANDOFF B13; RECEIPT-4 rows 6 and "remaining blockers"; queue C22 first sentence | "material → raster undecoded / binding undecoded" | The **data-level material → raster edge is decoded** (handoff 20 §6–§7: u32 at +256 / +288 of the `0xEF0001` chunk before each `0xEF0005` material names a `0x20003` descriptor chunk; §8: pixels in the DW8-addressed `0x10031` entry). The **complete writer contract is still absent**: which copy loads (loose vs `cacherender.big`), archive re-wrap / recovery, mip and packed-tail rules, shared or overlapping payload spans, unknown integrity fields, backup / restore. The edge also does not prove runtime selection. |
| K4 | Handoff 20 §8 row "Checksums"; C22 note "no checksum found" | "no CRC-32 / Adler-32 … appears" read as "no checksum" | **Bounded negative:** on three X360 Legacy packages, CRC-32 and Adler-32 of the data region and of the whole file are not any u32 header word in 0 .. file-table offset. Untested: other algorithms, other ranges (per entry, per chunk), fields outside the header, and PS3. Unknown header words are preserved, never recomputed or cleared. |
| K5 | Handoff 21 summary ("every file, byte for byte"); queue C23 / B14 "20 / 20 files" | whole-install identity shorthand | **20 shared paths** under the Recompiled `game\` are byte-identical to the same paths of the X360 Legacy tree. There is **one Recompiled-only file**, `default - Copy.xex`, with the same bytes as `default.xex`. The X360 tree's loose extracted folders (`rendering\`, `db\`, …) have **no** Recompiled counterpart. Archive identity is a byte fact. It is **not** a loose-route availability fact and does not establish loader precedence. |
| K6 | Handoff 16 / queue C18 / RECEIPT-4 ("144 shipped tables, 0 failures") | read as "every stock table satisfies the strict invariants" | Stock mode **relaxes** R5 / R9 / R10 to reported warnings (and derives `odd_home` from the stock rows) and **derives** each table's pair quotas from the same rows it checks. The replay proves reproducibility and internal consistency only, not that strict generated-output invariants hold on stock data. The revised reference is Package 2 (handoff 01 of this campaign). The coordinator's replay result (27 / 27, 144 tables, JSON-equal) stays the recorded result for revision 1. |
| K7 | BETA-HANDOFF B8 / B12 | "SNS / NHL 09 admission = developer's next step"; "A1 not yet admitted" | Developer-reported delivered: receipts 33 (SNS / `.ast` XMA of R92 shapes), 36 (PS3 NHL 09 / 10 / 12 / 13 music; X360 NHL 09 banks with the count rule only, two short records refused by name). Decoder success is **not** listening acceptance. |
| K8 | BETA-HANDOFF header "review build … receipts through 28" | stale build pointer | Current review bundle `E:\nhlms-review-build-r43-2026-09-27\NHL Modding Studio rc-20260927-r43\nhl-studio.exe` contains receipts **through 42**. Receipt 44 (labels) and receipt 45 §3 (Hockey Shop NHL 12) are **not** in it. NHL 21 E1 league identity remains user-accepted for its established scope. No other user observation is recorded (results file empty). |

00d's corrections stand unchanged: the `GEN6_COUNTRIES` scope for NHL 07–09, the NHL 10 and NHL 11 / 12 Japan cases (receipt 28), the
England / Great Britain wording, D2 as a width fact, D3 as a placeholder investigation, R121's deterministic ranking, and the C15
disposition.

## 2. Consumption reconciliation — delivered vs residual (developer reports, not accepted)

| Row | Delivered (receipt) | Residual (owner) |
|---|---|---|
| C17 / B2 style labels | S1 / S2 fixed: per-title roster cell and Lines rail; NHL 07 roster cell and NHL 10 show numbers with a note (35) | S3 (NHL 11–13 companion labels by continuity) = coordinator decision; S4 = observation; the NHL 10 table stays **inferred** (card) |
| C18 / B1 schedules | pair quotas (34); retained-row preservation and kept-date collisions (41, accepted bounded by the coordinator); Austrian tagged rows refused (34) | single-phase user partition + exact-source stock convenience **dispatched** to the developer (after-receipt-45 §3); **R123 reference repair = research Package 2**; Austrian phased generation **withheld**; runtime meaning of `round = 1` (card P1-A) |
| C19 / B10 portrait / head | reassignment on X360 NHL 09 → Legacy, PS3 NHL 15 / Legacy, PS4 portraits (38) | refused by name: PS4 heads (vault records), PS3 NHL 08–14 (archive-held), Recompiled (archive-held), NHL 07 / 08; selection = card P34-1 |
| C20 / B11 goalie gear | G1 `maskcolor` note + regression (38), file positions incl. the NHL 12 pads fix (40), G2 team paints (42) | **G3 PS4 (`maskid`, `masktype`) pair = research Package 3**; mask shell `masktype` file position ambiguous (40); slot vs personal mask precedence = runtime (G-MASK-1) |
| C21 / C26 / B8 / B12 audio | SNS / `.ast` XMA of R92 shapes (33); PS3 archived music and X360 NHL 09 banks (36); helper in the archive (43) | two-segment in-bank records, looping stream entries, > 2 channels, NHL 11 / 13 / 14 `.ast` shapes, Recompiled audio source route (research Package 5); listening (user) |
| C22 / B13 board art | read-only gallery **dispatched** to the developer (after-receipt-45 §4) | writer contract (research Package 4); precedence and in-game result (runtime) |
| C23 / B14 Recompiled | cited as database identity for this install (receipts 38 / 45) | archive-member source route per operation (research Package 5); saves / headers sidecar; loader precedence (runtime) |
| C24 animation fields | no developer report | numeric edits only; nothing to consume beyond widths |
| C25 scorebug | negative; nothing to build | none |
| C27 / B15 old logo resize | X360 NHL 07 → 11 and PS3 NHL 08 → 11 package forms, with refusals (37) | crops, `bar.big`, multi-mip, unswizzled and untagged NHL 07 payloads stay refused; in-game card |
| C28 / B7 group names | neutral division labels by member clubs on 08–11; conferences "(inferred)" on NHL 10 / 11; NHL 07 header defect fixed (44, not in r43) | division names need the executable consumer (disassembly: out of beta); NHL 08 / 09 conference lists not read |
| C15 / C16 identity and ratings | receipts 27 / 28 (00d) | goalie `consistency` 0 / 1 / 2 meanings (runtime); nationality 83 / 90 (optional card) |

## 3. Build and acceptance pointer

Current review bundle: r43 (receipts ≤ 42). Archive `E:\nhlms-release-candidate-20260927\NHL Modding Studio rc-20260927-r43.zip` passed
the coordinator's verifier and 13 negative cases (package integrity only). Accepted by the coordinator in bounded scope: receipt 41
retained-row preservation; R123 as input to single-phase partition-to-matrix and exact-source single-phase stock convenience, with
corrections. Nothing else from receipts 28–45 or research receipt 4 is accepted in full. NHL 21 E1 is accepted for its scope.

## 4. Residuals — next discriminating question, method and class

| Residual | Next question | Method | Class |
|---|---|---|---|
| R123 odd-host / index / collision | Does the reference reject unknown rules, incomplete or foreign host maps, and generated-vs-retained collisions, and model stored key ≠ position? | reference repair + failing / valid vectors (Package 2) | static — this run |
| Austrian phases | Exact rows, tags, source identities and participant relations of the `round = 1` block | read-only facts from R113 / R114 data (Package 2) | static facts; phase semantics runtime (P1-A) |
| G3 PS4 mask pairs | Which (`maskid`, `masktype`) pairs resolve uniquely, ambiguously or not at all in each source's vault, and what typed links exist to models / textures / thumbnails? | vault joins per source incl. genuine NHL 20 (Package 3) | static — this run |
| Skater equipment thumbnails | Is there a typed catalogue → thumbnail link for one skater-equipment family? | vault / dictionary reading (Package 3 tail) | static — this run |
| Board-art writer | Which bytes may change in a same-size, same-format raster replacement, and do the container invariants survive a round trip on a copy? | extend R127g to measured families; disposable-copy round trip (Package 4) | static; precedence and appearance runtime |
| Recompiled route | Which archive member serves each Studio operation on the Recompiled install, and what do the resolvers do there? | install layout + resolver reading, member hashes (Package 5) | static; loader precedence runtime |
| XMA two-segment / other shapes | Where do segments join, and do framing and length rules hold for the smallest samples? | helper decode of minimal samples (Package 5) | static — decoder evidence only |
| PS4 head reassignment | Which vault record a head id resolves to, and whether a target exists | vault `SkaterNameAttribute` join (R124) | static (fallback) |
| NHL 07 / 08 heads without files | Are these rows defaulted by a shared head, or do they name files elsewhere? | archive member names vs roster ids | static (fallback) |
| PS3 NHL 08–14 archive reassignment | none (conditions measured, handoff 17 §9) | developer implementation | implementation |
| S3 style labels (NHL 11–13) | — | coordinator decision | decision |
| NHL 10 style labels, NHL 08–11 division names | — | in-game card / executable consumer trace | runtime / disassembly (excluded) |
| Art-id re-key / archive rename | full writer across the six readiness layers | not before beta without a new discriminator | static but large (parked) |
| Goalie `consistency` meanings; nationality 83 / 90 | — | in-game observation | runtime only |
