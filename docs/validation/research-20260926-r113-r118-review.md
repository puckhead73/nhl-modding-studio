# Coordinator review — R113–R118

Continue the same research session. Review scope: receipt 2, handoffs 07/08/11/12, and Studio's rating classification and encoding call sites. No installed-file census, desktop flow or runtime test was independently rerun. The correction checkpoint addresses the prior three reporting findings. New research remains researcher-verified; the generator remains a proposal.

## Corrections and developer routing

1. **R118 reports distributions, not identical changes in every cell.** Replace “nothing moves”, “everything moves up by 14” and exact-gap wording with the measured medians and spread. Every shared field having median +14 is meaningful continuity evidence; quartiles 11/14/14 already show that individual changes differ. Preserve the inferred status and identify the source of the previously accepted endpoint observations without requiring new game runs.
2. **Goalie consistency is a supported developer finding, with a corrected example.** `attributes.rs::NON_RATING_SUFFIXES` excludes fighting/style/paddledown/potential but not consistency. `rating_offset` classifies the 0..63 consistency fields as ratings, and `view.rs` substitutes the title contract's six-bit offset. Thus NHL 08's reported raw 0..2 follows the +50 path, not +36; NHL 11–Legacy follow +36. Route this to the developer for a focused classification investigation and regression coverage across display, edit, import and derived calculations. A stock value census alone does not establish the legal write domain or the labels for the three values. Do not assign meanings, clamp to 0..2, or silently alter a fitted goalie Overall formula without a separate justification. Keep floppiness unresolved per title.
3. **Generator arithmetic needs one clarification.** For a symmetric per-pair matrix, total games is the upper-triangle sum, or half the full off-diagonal sum; not the sum of all entries. Require zero diagonal, nonnegative integer counts and symmetry. Keep source anomalies distinct from generated validity. An Austrian round-tagged final block is measured; its runtime phase semantics remain unverified as the body already states.

The current group data and validator can support further design; they do not authorize a generator implementation on their own. Tournament interpretation must retain candidate labels wherever a selector or placeholder meaning is inferred. Ledger reconciliation correctly distinguishes stock save correspondence from edited-input behavior.

## Prompt for Claude — continued extended research

Read applicable repository instructions, this review, current claims, the current developer queue and latest developer receipts. Publish a dated correction checkpoint for the three items above, reconciling handoffs, receipt and queue. Preserve original evidence. Continue without routine review stops while usage allows.

Then perform the proposed Players 3.1 cross-title identity audit, with these constraints:

- Match players using independent identifiers and record confidence, collisions and unmatched rows. Do not use a field under investigation as an unquestioned match key. For a birth-date investigation, use an independent match or show the circularity explicitly. Separate same-title cross-platform comparisons from adjacent-season comparisons.
- Examine weight, height, handedness, position and nationality/birthplace separately. Report field metadata, raw and decoded distributions, source hashes and per-title changes. Retain legitimate roster edits and outliers; player continuity is supporting evidence, not proof of a unit or enum mapping. Do not infer a code label from a number's frequency alone.
- Include the genuine NHL 20 source and relevant known patch variants. Reuse proven identical DBs with explicit provenance rather than pretending to rerun independent evidence. Bound conclusions on missing titles or fixtures.
- For weight and height, compare metadata and the stable encoding contract; do not revive population-based runtime decoding. For categorical fields, seek title-owned label tables or consumer references before proposing label changes.
- Publish a compact developer discrepancy table: existing behavior, observed evidence, confidence, exact affected titles and the smallest regression or further check. Research must not modify application code or silently repair fixtures.

After that, return to actionable League/Team gaps and new developer observations. Prioritize any ready audio-source or schedule-contract question that blocks a concrete package. If none can progress without user observations or a genuinely new method, continue Players, Equipment and Miscellaneous research in roadmap order. Do not reopen settled questions merely to fill the queue, and do not promote read-only discovery into writer support. Projects/Builds remain deferred.

Claim each bounded package before starting. If blocked, record the missing evidence and next discriminating step, then move to independent work. Keep visual/listening/runtime cards for the user and preserve accepted NHL 21 E1. No launches, live writes, application/defs/bindings changes, staging, commits or pushes. Preserve other lanes' work and shared caches. Use actual clock timestamps and validated temporary-sibling publication. Clean owned regenerable scratch after its dependency ends.

Each receipt starts with a plain-language summary, followed by exact coverage, measured versus inferred results, residuals, reproduction commands, developer handoff and cleanup. End the extended run with a consolidated receipt, current queue and one exact next task.
