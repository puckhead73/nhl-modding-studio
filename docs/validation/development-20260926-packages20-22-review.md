# Packages 20–22 focused coordinator review — 26 September

## Disposition and checks

Continue the same developer session. The content-driven encoding defect is corrected for the tested known layouts; the title-scoped equipment behavior passes focused checks. Package 22 has a reproduced bank/stream identity refusal gap. Unknown-layout classification also needs tightening before receipt 20's general refusal claim is closed.

Independent runs: `studio-domain --test encoding_contract --test equipment_scope`: 8 passed, 1 private-fixture test ignored; `Gen7SfxBankCard.test.tsx`: 4 passed. A separate coordinator regression on disposable copies failed as below. No full-suite, private NHL 20, decoder sweep, desktop, listening or runtime rerun. No independent audit of all public-contract citations in this pass. No application changes or live writes; temporary probe removed after evidence recorded.

## F1 — P2: mismatched AST identity does not prevent preview

`gen7_sfx_banks.rs::ast_facts` reports the sibling AST id mismatch, but `preview_entry` never validates that identity. `frame_stream` starts at the record's payload offset, so it does not inspect the file's S10S header/id. The read-only listing can warn and still return a decoded result under the bank's record identity.

Reproduction: copy PS3 NHL 15 `audio/sfx/playercalls.abk` and `playercalls.ast` into an owned temporary root; flip only AST byte 7 (id word); call `bank_tree`, then `preview_entry(..., record=0)`. The tree reports `ast_id_matches=Some(false)`. Preview nevertheless succeeds with **33,207 samples and 66,458 WAV bytes**. The assertion that preview must refuse fails. Source files were read only; the copied root was removed by its test guard.

Require a valid stream header and bank-matching id before framing/decoding any stream record. Apply the same validation to the bytes/file handle actually consumed, not just a cached listing. Mark affected entries non-previewable with a useful reason; direct domain/API requests must refuse too. Inline ram records need no unrelated sibling AST and should keep working.

Add malformed/missing/mismatched AST and post-listing replacement cases, unchanged-pair success and ram-only independence. Do not fix this by disabling the whole bank when valid ram entries remain. Cover the direct route as well as UI affordances before extending to archived members.

## F2 — P2: byte order alone is not a recognized PS4 layout

`encoding.rs::schema_family` returns `Some(Ps4)` immediately for every little-endian database, before checking even the BIO table or relevant descriptors. Similarly the gen7 modern branch is presence of a field named `draftyear`. This is stable under cell edits, but broader than the claim that unknown layouts become raw/read-only. A future or altered little-endian schema is automatically assigned weight/date/cap-hit semantics.

Use a bounded semantic layout signature: check the tables, field identities, widths/ranges/types and other discriminators needed by the encoded operations, allowing harmless capacity/content changes. Keep shared families where measured equivalence warrants them. Do not require entire stock-byte identity, and do not return to population inference. Add unknown/altered little-endian and contradictory-descriptor negatives plus accepted modified-roster/reopen cases. Unsupported title/platform combinations must not silently count as agreement. Audit bulk/import seams so any semantic writer uses the resolved contract; intentional raw low-level routes should be explicitly distinguished.

This is a code-inspection finding; no new synthetic unknown-layout test was executed in this review. The existing known-family tests all passed.

## Next prompt — correction checkpoint, then extended continuation

Read this review and the original 24 September overnight developer brief. Close F1 and F2 first with focused regressions and a plain-language correction receipt. Preserve the successful known-layout offset fixes, equipment raw-id round trips and diagnostic-only labels. Regenerate bindings only if the API changes. Publish the checkpoint and continue automatically under the existing extended-work authorization.

Next implement archive-backed PS3 NHL 09/10/12/13 bank listing and supported diagnostic preview using each title's own ABKC/AST members and R98/R100. Validate member identities, bank/stream association, wrapper boundaries and source ownership; distinguish structural coverage from selected decoded vectors. Do not extend Xbox decode without a documented decoder decision. Preserve loop/multichannel refusals until their separate implementation contracts are satisfied.

Correct arena chant wording using R97 and the newest coordinator research review: numbered loader assets and club-name associations are measured; selection is untraced. No silence/fallback or definitive runtime selector claims. Add arena-to-bank navigation where the source is established, retaining ambiguous/no-source readings rather than inventing a binding.

Then continue ready League/Team operations in priority order. Consult current research claims before declaring a blocker; use the original fallback queue if one operation is blocked. Equipment expansion remains a fallback, not a reason to skip ready League/Team integration. Keep diagnostic decoding separate from replacement; Xbox mipped resize, team creation and re-key remain under their existing limits.

No routine review stops, launches, live/source writes, staging, commits or pushes. Application-source changes inside Studio are authorized. Preserve unrelated dirty files, shared caches and other lanes' processes. Exclude zz_diag; clean owned temporary material after durable evidence is preserved. Finish with exact tests, coverage, remaining limits, morning cards and one next action.

## Roadmap

Known-layout player encoding and title-scoped equipment labels have passed focused review. League/Team sound-bank inspection and diagnostic preview are implemented with a source-association defect to close. Archive-backed coverage and arena navigation are the next useful end-to-end steps. No runtime acceptance or overall phase closure is claimed.
