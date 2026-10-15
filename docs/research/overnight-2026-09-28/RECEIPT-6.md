# Research receipt 6 (consolidated) — continuation after receipt 5, 28 September 2026

Research lane. Prompt: [`research-20260928-after-receipt5.md`](../../plans/research-20260928-after-receipt5.md). Run from about 13:10 to
13:40 MDT (system clock). Current beta authority: [BETA-HANDOFF.md](BETA-HANDOFF.md). Resumable log: [STATUS.md](STATUS.md). Queue
C37–C38.
- No staging, commits, pushes, publication, app / game / emulator launches, or listening.
- No installed source, anchor or fixture was written.
- No app, definition, binding, developer-test or coordinator-record edits.
- Every newly delivered contract awaits its applicable independent review.

## Summary

- **Corrections first.** Research fixed four overstatements in receipt 5:
  - the Recompiled comparison counted matching file names, and compared bytes only for a sample and for whole archives;
  - the "every arena file has an archive copy" claim did not hold everywhere: PS3 NHL 11–13 are archive-only, and the Xbox NHL 10
    archives were never checked;
  - a board-art rule that allowed writing only one copy is withdrawn;
  - the NHL 11–13 style labels are recorded as a closed coordinator decision.
- **PS3 board-ad replacement contract.** A same-size replacement of a PS3 board-ad picture, including all its smaller copies, was
  tested on throw-away copies. It changed only that picture's bytes and restored exactly. The contract is narrowed to the two formats
  PS3 boards actually use.
- **Create-player equipment pictures.** No data table links gear numbers to these pictures. For pads and blockers on NHL 12 →
  Legacy, however, picture N shows the same design as render texture N. Studio can show those pictures as labelled candidates.

## 1. Packages and outputs (`docs/research/overnight-2026-09-28/`)

| Package | Output | Result | Queue |
|---|---|---|---|
| 1 — corrections | [09](09-after-receipt5-corrections.md) | K9 R138 narrowed; K10 R137 topology table; K11 §5 loose-only allowance withdrawn; K12 S3 closed (coordinator); K13 R123r2 replay recorded; K14 receipts 55–62 consumption. Dated notes in 03 / 04 / RECEIPT-5 and C17 / C32 / C35; BETA-HANDOFF refreshed | notes |
| 2 — board writer | [10](10-ps3-board-raster-writer-contract.md) | PS3 same-size contract: validated edge (+ 14 / 14 synthetic cases); census of 6 PS3 trees; 4 full-chain disposable round trips; DXT1 / DXT3 admitted, DXT5 not; pair / archive-only / unmeasured identities and failure invariants | C37 |
| 3 — gen7 thumbnails | [11](11-gen7-createplayer-thumbnails.md) | no data edge; numbered-file candidate with a picture-content check (pads, blocker; trapper plausible); masks only 88 generic; NHL 10 not joined; two unread Studio thumbnail prefixes name no file | C38 |
| 4 — boundary check | this receipt | no new developer observation; receipt 63 (PS4 heads) raises no research question; trapper check added to 11 | — |

## 2. Corrected claims

- **R138:**
  - 44 602 unique **path** matches, plus 4 shared-file exceptions;
  - **52** sampled byte comparisons;
  - R128's archive-to-archive identity, which is separate evidence and says nothing about the X360 loose override files.
- **R137:**
  - measured loose / archive pairs: X360 NHL 11–15 / Legacy, PS3 NHL 14 / 15 / Legacy;
  - archive-only: PS3 NHL 11–13 and Recompiled;
  - **X360 NHL 10 is unmeasured**, because its BIG4 archives were never compared.
- **03 §5:** "loose-only write allowed with disclosure" is withdrawn. A measured pair shares one review and recovery scope, or refuses.
- **S3:** a coordinator decision to keep the values numeric. It is not a user decision and not a blocker.
- **Developer delivery** runs through receipt 63, not 50–54 as receipt 5 said. It is not blanket-reviewed.

## 3. Developer-consumable contracts (priority order)

1. **C37 — PS3 board-ad replacement** (after coordinator review):
   - scope: DXT1 / DXT3, same size, the whole plain chain;
   - the validated edge;
   - pairs as one review and recovery scope; archive-only sources through the member route; X360 NHL 10 and DXT5 refused;
   - the stored archive length changes, so the archive stager must be used.
2. **C38 — create-player thumbnails:**
   - fix the goalie-stick and skate prefixes;
   - show a labelled candidate picture for pads / blocker / trapper (NHL 12 → Legacy, PS3 15 / Legacy), and at lower confidence
     for sticks / gloves / helmet / skates;
   - masks only when the file exists; nothing for NHL 10 or the visor.

## 4. Measured vs inferred

| Statement | Class |
|---|---|
| Validated-edge refusals | synthetic, measured 14 / 14 |
| Census counts, formats, one NHL 11 refusal | measured on every PS3 venue package |
| Full-chain replace / read-back / restore | measured on 4 disposable copies (2 formats × 2 topologies) |
| Native chunkzip framing reproduced; length change | measured 4 / 4 members |
| Encoder quality of `compress_bc` | **not run by research** |
| Game display and copy precedence | unobserved |
| Thumbnail N ↔ render N | visual judgement on 14 clear samples (+ 2 plausible trapper), with set containment; **not** a data edge |
| Stored value N → thumbnail N | numbered-file candidate (value sets fit at several shifts) |
| `ASSET_ID` source | native code or a hashed vault: unknown |

## 5. Checks actually run

- **R142** (`d7ee2d48…`), one run of about 3 min:
  - 14 synthetic cases;
  - census of 6 trees (370 packages);
  - 4 round trips with Pillow BCn level decode;
  - zlib-9 re-wrap.
- **Inline:** the NHL 11 refusal detail.
- **R143** (`fda9bc69…`) and **R143b** (`2c4da45a…`): 9 sources each (X360 10–Legacy, PS3 15 / Legacy).
- **R143c** (`596928a9…`): once on Legacy pads. It failed as a method, and that is recorded.
- **R143d / e sheets:** Legacy pads (2 sheets), blocker, trapper; NHL 12 pads; NHL 10 pads (not discriminable).
- **Failures kept:**
  - R143e's first NHL 10 run: an RW4 thumbnail with no RefPack member;
  - a non-ASCII byte literal in the patched R143c, fixed;
  - one shell quoting error in an inline check, rerun as a script.

## 6. Residual October 1 blockers (owners, smallest action)

| Blocker | Owner | Smallest action |
|---|---|---|
| Board-ad replacement (B13) | coordinator → developer | review handoff 10; then a PS3 writer behind C37 (X360 stays gated: packed tail, R30-LZX-1, NHL 10 topology) |
| Austrian phased schedules (B1) | user → coordinator | card P1-A |
| User acceptance of delivered previews / writes (B9) | user | r59 cards (receipt 60 §7) |
| Goalie mask slot vs personal mask (B11) | user | card G-MASK-1 |
| Recompiled beyond reads (B14) | developer · user | remaining read routes; runtime card |
| NHL 07 / 08 generic heads (B10) | user | card P34-G08 |
| Hockey Shop F1 correction review | coordinator | review receipt 61 |
| Release: holder name, artwork replacement before external release | user / developer | not research |

## 7. Cleanup and retention

- **Removed:** R142's `scratch-q8`. Before removing it, research checked that it was empty, sat at the expected resolved path inside
  `local/runs/overnight-research-2026-09-28`, and was not a reparse point. Four disposable package copies were deleted by the probe
  after their restore check.
- **Deleted:** the R143 contact sheets and the trial JSON (session scratchpad), and the session scratchpad's helper scripts after
  publication.
- **Bytecode:** none was created this run (checked).
- **Kept:**
  - probes R142, R143, R143a–e;
  - data files r142, r143, r143b, r143d.
- **Not touched:** shared caches, user bundles, the toolchain, fixtures, other lanes' H:\TEMP directories.
- **Disk:** E: has about 11 GB free.
- **Claims:** q7, q8 and q9 are `ready`.

## 8. Next research task

Resolve the gen7 create-player `ASSET_ID` source statically **only if** a gen7 Attrib vault schema can be established from the
research's PS4 vault reader. Test it on X360 Legacy `attribdb/feattribdb.{vlt,bin}`: look for arrays equal to the per-title thumbnail id
lists (e.g. Legacy pads: 1–51, 53–72, 74–84, 100–109). If the vault layout does not parse, publish that negative and move to the
trapper / glove content checks on NHL 13 → 15 with the R143d method.
