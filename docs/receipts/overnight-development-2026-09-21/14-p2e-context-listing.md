# Receipt 14 — Priority 2-E (Leagues 1.5 / Teams 2.7): the commentary context listing (CTX0 v4.5 and v3.1)

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
baseline `8d60a6d`. Real Xbox 360 / PS3 trees read only. Claim: `dev-p2e-context-listing`.
Research consumed: 2026-09-21 handoff 01 (`research-r24-ctx0-structure`) §1–2 with its
`r24-ctx0-records.json` as the record-for-record oracle, handoff 16 (`research-r41-ctx0-v31`) §1–2
with `r41-ctx0-v31-records.json`, and the contract wording of handoff 03 (rev. 2b).

## 1. Plain-language summary

The commentary's "when does which line get said" rules live in a context file the research lane
decoded this month by following its pointers. Studio now reads it the same way: every context
names the game trigger that starts its evaluation, the conditions it tests (named compiled
expressions — what they compute is not decoded), and what it does — speak a speech event or raise
another trigger. The reader reproduces the research's census record for record on all seven
supported v4.5 sets (Xbox 360 NHL 13 / 14 / 15 / Legacy, PS3 14 / 15 / Legacy; 3 371–4 183
records each, 0 refusals), PS3 equals Xbox 360 on 14 / 15 / Legacy; the older v3.1 layout of NHL
11 / 12 is read too (2 833 / 2 833 / 3 339 records equal to the research's second census, PS3 =
Xbox 360 on NHL 11), so the Leagues → Commentary page's new searchable Contexts card covers NHL
11–Legacy. PS3 NHL 13 (archive-only) says so instead of listing anything. Which context the engine picks when several
triggers fire, the priority words and the run-once groups are stated as runtime, never as facts.

## 2. What users can now do

- Leagues → Commentary → **Contexts**: search contexts by name / trigger / condition / event,
  expand one to its conditions and item parameters; the totals in the header.

## 3. Implementation

| Component | Change |
|---|---|
| `crates/tdb-speech/src/ctx0.rs` (new) | `parse_ctx0_v31(bytes, speech_moir, ctx_moir)` (handoff 16 §1): 19-word records, inline trigger / condition / item names, one item per record typed by the MOIR name tables (`speech` / `context` / `other-module`), parameters as trigger slots / immediates / expressions (script id + arguments); the undecoded header words recorded raw (the research's `zeros_ok` holds on 2 446 of 2 833 NHL 11 records; the rest carry a w13 pointer it did not decode — not followed). `parse_ctx0(bytes, speech_ns, ctx_ns)`: the header tables (A-table, T1 / T2 / T3 identities, pools), per record the trigger (pool 2 offset), the name, items (fixed part 36 B PS3 / 28 B X360; namespace-typed `speech` / `context`; parameter kinds 1 immediate / 2 call block with immediate arguments), conditions (pool-1 name, blob index, arguments), extra words, the Legacy slot-2 block; every research check is a refusal. The context id is checked against the record's T3 id (the prose's "== index" is contradicted by the research's own census — 0–1 records per set — and not enforced). v3.1 (4-byte magic, NHL 11 / 12) refused by name. |
| `crates/studio-domain/src/speech_catalogue.rs` | `context_catalogue(game, platform, root, query, limit)` → `ContextCatalogueView` (coverage: X360 11–Legacy, PS3 11 / 14 / 15 / Legacy; v3.1 detected by magic and read with the MOIR tables; PS3 12 / 13 refuse with the reason); condition text `name(args)`, parameter text (`N` / `blob N(args)`), notes with the no-trigger count. |
| `crates/studio-api` (`dto.rs`, `registry.rs`, `commands.rs`, `lib.rs`, `bindings.rs`) | `ContextCatalogueDto` / `ContextRecordDto` / `ContextItemDto`; `Registry::context_catalogue`; command; bindings; root-count guard 126. |
| `app/src/components/ContextCatalogueCard.tsx` (+ test), `pages/SpeechCatalogue.tsx`, `lib/client.ts` | the card under the commentary catalogue. |

## 4. Regression evidence (developer)

| Case | Asserts |
|---|---|
| `tdb-speech/tests/ctx0_records.rs::every_supported_set_equals_the_research_census` (`--ignored`, 7 real sets + 3 refused, 4 s) | speech / context namespaces from the EVT0 files equal the research's; per set the record count, pool sizes, item / speech / context / condition / no-trigger totals equal the JSON census; **record for record**: name, trigger, item names / kinds / event ids / parameter counts, condition names / blobs / argument counts, the Legacy slot-2 blocks; PS3 = X360 on 14 / 15 / Legacy (structural equality); NHL 11 (both platforms) and 12 refuse by version |
| `tdb-speech/tests/ctx0_records.rs::v31_sets_equal_the_research_census` (`--ignored`, X360 NHL 11 / 12 + PS3 NHL 11) | records 2 833 / 2 833 / 3 339; triggers 2 699 / 2 699 / 3 200, no-trigger 134 / 134 / 139; items typed speech 2 796 / 2 796 / 3 302, raised 3 / 3 / 3, other-module 34 / 34 / 34; conditions 11 702 / 11 702 / 13 099; parameters 3 043 / 3 043 / 3 345; the research's 400-record samples record for record (names, triggers, item names / parameter counts, condition names / script ids / argument counts); PS3 = X360 on NHL 11 |
| `studio-api/tests/speech_catalogue.rs::context_listing_reproduces_the_research_worked_example_on_nhl15` (`--ignored`) | totals 3 866 / 153 / 1 980 / 4 007 + 515; record 1 and record 52 as the research's worked example (trigger, the three conditions, the items and the immediate parameter); the query narrows and caps; the no-trigger note (51); **NHL 11** lists 2 833 records / 2 796 + 3 items / 11 702 conditions / 134 without a trigger through the v3.1 reader, other-module items typed, slot parameters as text |
| `ContextCatalogueCard.test.tsx` (2 jsdom cases) | args incl. the query; trigger / speaks / raises / condition-count wording; expansion; refusal text; nothing without an install root |

Runs: [p2e-ctx0-records.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2e-ctx0-records.log)
(1 passed — the first run failed on **my** refusal-text expectation for NHL 11, diagnosed:
the 4-byte-magic header was reported as an unknown platform tag; refused by version name until
the v3.1 reader landed), [p2e-ctx0-v31-records.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2e-ctx0-v31-records.log)
(1 passed — two earlier runs failed on **my** header-constant assumptions for v3.1 (words 6–10 /
13 vary on 387 NHL 11 records the research recorded as `zeros_ok = false`); relaxed to raw),
[p2e-context-listing-api.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2e-context-listing-api.log)
(1 passed); card test 2 passed; `npx tsc` clean.

## 5. Coverage (Phase 1 matrix 1.5 / Phase 2 matrix 2.7, "context listing")

| X360 11 / 12 | X360 13–15 / Legacy | PS3 11 | PS3 12 / 13 | PS3 14 / 15 / Legacy | 07–10 / PS4 | Recomp |
|---|---|---|---|---|---|---|
| **impl+tested, dev (v3.1), census-equal** | **impl+tested, dev, record-for-record vs the research** | impl+tested, dev (v3.1) | refused (archive-only) | impl+tested, dev | refused with reason | by identity |

## 6. Open / omissions

- v3.1: the w13 pointer on 387 NHL 11 records and the script id → `.gas` binding are not
  decoded (as the research states); group refs are not shown.
- Crowd contexts (`crowds_contexts.bin`, the same parser per the research) not listed in the UI.
- The listing does not join a context's speech item to the sentence catalogue (a click-through
  would be a follow-up; both listings are read-only).

## 7. Cleanup

Scratchpad `ctx_domain.rs` deleted. No temp roots, no processes.
