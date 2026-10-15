# Research receipt 5 (consolidated) — overnight 28 September 2026, after receipt 4

Research lane. Started 04:37, receipt published 06:47 MDT (system clock). Prompt:
[`research-20260928-fresh-overnight-after-receipt4.md`](../../plans/research-20260928-fresh-overnight-after-receipt4.md). Current beta
authority: [BETA-HANDOFF.md](BETA-HANDOFF.md). Resumable log: [STATUS.md](STATUS.md). Queue: `../overnight-2026-09-25/DEVELOPER-QUEUE.md`
C29–C35.

- Nothing was staged, committed, pushed, published or launched.
- No app, game or emulator was started, and nothing was listened to.
- No installed source was written.
- The only decoder run was the installed pinned XMA helper, fed synthetic RIFF wrappers of shipped bytes.
- A `ready` claim below means delivered for review. It is not coordinator or game acceptance.

## Summary

Research corrected its own published statements first, then delivered contracts the developer could use straight away:
- goalie mask pairs and skater-gear catalogues with pictures and the games' own names on PS4;
- the remaining Xbox 360 sound shapes;
- where Legacy Recompiled keeps every file Studio needs.

The developer consumed most of it overnight (receipts 50–54), still awaiting review. Research also produced an exact byte-level contract
for replacing an arena's board-ad pictures at the same size, and a corrected schedule reference validator. What remains is mostly in-game,
listening or coordinator decisions, plus implementation of the board-art writer and the Recompiled read routes.

*(Corrections 28 Sep 13:21, [09](09-after-receipt5-corrections.md): "every file Studio needs" = 44 602 unique **path** matches + 52 sampled byte comparisons + R128 whole-archive identity (K9); "loose ↔ archive twins identical everywhere" holds for the measured pairs only, with PS3 NHL 11-13 archive-only and X360 NHL 10 unmeasured (K10); the §5 loose-only allowance is withdrawn (K11); S3 is a closed coordinator decision (K12); developer delivery is through receipt 62, not 50-54 (K14).)*

## 1. Handoffs (all in `docs/research/overnight-2026-09-28/`)

| # | Package | Result | Queue |
|---|---|---|---|
| [00](00-correction-checkpoint.md) | 1 — corrections | K1–K8: B4 / B6 removed (user decisions); "nothing static remains" withdrawn; B13 wording; bounded "no checksum"; R128 "every file" → 20 shared paths; stock-mode caveat; delivered-vs-residual table; residual next questions | — |
| [01](01-r123-reference-revision-2.md) | 2 — R123 revision 2 | host rule by value (Q11 family), `balance` transcription, stored key vs position (R2 / R3, Q13), R11 generated-vs-kept, R9-retained; 27 / 27 + 29 / 29 vectors; **stored key = position on 144 / 144 stock tables (first measured)**, 0 kept rows under default participants; Austrian phase facts | C29 |
| [02](02-ps4-goalie-mask-pair-contract.md) | 3 — G3 | ≤ 1 pack per (`maskid`, `masktype`) on all 7 vaults; all FileRefs present; classes per source incl. genuine NHL 20 and patches; team slots have no brand; Creation Zone list + thumbnails (NHL 19+: assetid = style × 1000 + pattern) | C30 |
| [02b](02b-ps4-skater-gear-catalogue.md) | 3 — skater gear | NHL 17–21 helmet / gloves / skates / stick / visor → CZ row → name-matched enum → thumbnail; every skater value joins except helmet 99; NHL 15 / 16 enum only | C31 |
| [03](03-board-art-replacement-boundaries.md) | 4 — board art | same-size operation contract (§5); X360 NHL 10–15 / Legacy + PS3 NHL 11–15 / Legacy (§8–9); PS3 `83f1/2/3` = plain DXT1/3/5 via descriptor u32 @ +0; loose ↔ archive twins identical everywhere; clean disposable-copy round trip | C35 |
| [04](04-recompiled-archive-routes.md) | 5 — Recompiled | 44 602 / 44 606 X360 Legacy loose files = one archive member each (4 = shared `gamedata`); 52 / 52 samples identical; route per class; Studio loose routes find nothing; six sidecars match `content_header.rs` | C32 |
| [05](05-xma-residual-inbank-shapes.md) | 5 — residual in-bank XMA | intro + loop; 4 / 6-ch chains; **false refusal of (0, `0x7FFF`) packets** on NHL 10 → Legacy | C33 |
| [06](06-ast-xma-streams-11-13-14.md) | 6 — `.ast` streams | every stream record exact on NHL 10–15 / Legacy (§1, §4) | C34 |
| [07](07-nhl07-08-generic-heads.md) | 6 — NHL 07 / 08 heads | file-less heads = `headid == artid` generic players; smaller card P34-G08 | — |
| [08](08-cz-name-keys-dictionary.md) | 6 — Creation Zone names | every `$` key of the PS4 mask / skater-gear lists resolves in the title's own `eng_us.db` (exact or case-insensitive hash); only two placeholder words unresolved | C36 |

## 2. Developer-consumable items (priority order; status per developer receipts, not accepted)

1. **Board-art writer** (B13) behind handoff 03 §5.
   - PS3 venue previews first: receipt 49 says "not measured", and handoff 03 §1 now measures them.
   - Then the writer: loose + archive twin planned or refused; stored / chunkzip first.
2. **Recompiled read routes** (B14; handoff 04 §6): stored members first (db, loc, banks). Fix the "by lineage, loose" wording in
   `gen7_sfx_banks.rs`.
3. **PS3 NHL 08–14 archive-member reassignment check** (B10; handoff 17 §9): the remaining gap for a delivered operation.
4. **Consumed overnight, awaiting review:**
   - G3 mask pairs (51) and pictures (54);
   - skater gear (52);
   - in-bank XMA (50);
   - `.ast` streams (53).

   Receipt 53's NHL 10 / 15 / Legacy loop and surround shapes are now swept (06 §4).
5. **Creation Zone names** (handoff 08): look up by `loc_hash`, fill the `%1s %2s` mask template.
6. **R123 production safeguards:** already implemented (48). Revision 2 now matches them.

## 3. Sources and executed checks

- **Sources:**
  - X360 NHL 07–15 / Legacy and PS3 NHL 08–15 / Legacy trees on `X:`;
  - the PS4 installs CUSA00561 … CUSA18182 on `H:` (vaults: `boot.big` `gccollections` only);
  - private fixtures: genuine NHL 20 CUSA15439, NHL 19 patch 56, NHL 15 patch 00;
  - the registered Recompiled install on `H:`.

  Hashes are in each handoff and data file. `anchors/ps4/nhl20` was used only as a recorded alias of the NHL 19 base (same bytes).
- **Probes** (`local/runs/overnight-research-2026-09-28/probes/`): r123r2, r135 / r135a, r136, r137 / r137b / r137c, r138, r139 / b / c,
  r140, r141 (+ `ps4_member.py`, `q1_pointer_notes.py`).
  - Each was executed as described in its handoff.
  - Two probe runs had defects that were fixed and rerun: R137's first PS3 pass used the X360 data type, and R139b's label check used
    the wrong prefix. Neither faulty output was published.
- **Not run:** Studio tests or builds (research does not own them), listening, runtime, R121 determinism, and the settled logo and decoder
  censuses.

## 4. Corrections and withdrawals this run

- **00 K1–K8:** as in §1.
- **Handoff 20 §10** (developer observation 49, confirmed on both platforms): slot words must lie inside the `0xEF0001` block (288- and
  240-byte blocks exist); there are **361** packages, not 360.
- **R123:** revision 1's R3 "holds" was unmeasured, because it used position as the key. Now measured true (01 §3).
- **Receipt 42's wording** "PS4 NHL 16+ have 9 slots": NHL 15 / 16 declare 7 slots (02 §4). This is an observation for the developer; no
  display defect is claimed.

## 5. Residual blockers (operation / targets → owner → smallest next action)

| Operation / targets | Owner | Smallest next action | Kind |
|---|---|---|---|
| Austrian phased schedules, PS4 18–21 | user → coordinator | card P1-A (01 §4) | runtime |
| Board-art writes: loose vs archive precedence, chunklzx re-wrap, X360 packed tail, appearance | developer (writer) · user (cards) | copy-precedence card; R30-LZX-1 | runtime + implementation |
| Recompiled loader precedence / save acceptance | user | runtime card on a copy | runtime |
| Goalie slot vs personal mask; helmet / mask 99; stick handedness | user | G-MASK-1 | runtime |
| NHL 07 / 08 generic faces | user | P34-G08 (07 §3) | runtime |
| Surround speaker order; all audio quality | user | listening cards | listening |
| S3 style labels; NHL 08–11 division names | coordinator; excluded (disassembly) | decision; none | decision / excluded |

## 6. User-only observations

These are cards, not questions: P1-A, P34-1, P34-G08, G-MASK-1, the board-art copy-precedence card, and listening A-XMA-2 → 4 on a
bundle that includes receipts 44–54.

## 7. Cleanup and retained artifacts

- **Removed:**
  - package scratch `local/runs/overnight-research-2026-09-28/scratch`: logs, intermediate JSON, and two disposable 10–11 MB package
    copies;
  - the venue unwrap temp folder;
  - the session scratchpad.

  Each deletion was checked for exact resolved containment and reparse points first.
- **Bytecode:** removed the 9 `.pyc` files this session wrote, and this campaign's new `__pycache__`. The older caches in other probe
  folders are untouched.
- **Kept:**
  - probes;
  - published data (`docs/research/overnight-2026-09-28/data/`, largest 0.9 MB);
  - claims `research-q1` … `q6c`, all `ready`.
- Nothing is running. E: had 16 GB free at the end (20 GB at the start; the developer lane also writes there).

## 8. Next research task

Join the gen7 create-player equipment thumbnails (`fe/ion/artassets/createplayer/{pads,blocker,trapper,goaliestick,mask,helmet,gloves,
skates,sticks}/<family>{N}.big`, 18 Sep handoff 30) to the database values and file ids of handoff 18 / C20. Do it per title on X360
NHL 10–Legacy, and look for a typed or data-level index (front-end tables, `equipatrib.bin`, `fe` package contents) before any number
coincidence. Publish per family: joined, unjoined, candidate. Any number-only coincidence stays labelled as a candidate, as handoff 02
§5 did for NHL 15 / 16.
