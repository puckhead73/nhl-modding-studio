# Receipt 6 independent review — 28 September 2026

Disposition: corrections and narrowed PS3 format evidence are useful; C37 writer authorization remains withheld pending the validator correction below. Continue the research session. Developer's receipt-65 correction queue remains unchanged.

## Independent checks and findings

Read receipt 6, handoffs 10/11, current BETA-HANDOFF, the R142 validator/synthetic source and R143d qualitative results. Executed the extracted R142 in-memory validator and synthetic functions with equivalent standard-library constants, without importing its install-reading dependencies. All 14 original vectors returned their expected outcomes. The old helper was not rerun. Four added probes were admitted:

1. **Overlapping data can escape detection.** The algorithm compares only neighboring sorted spans. An enclosing span A, followed by a short contained span B, then selected span C inside A but beyond B, leaves C unmarked. Reproduction from `synth(a_off=0x80,b_off=0)`: replace chunk 5 with data `(offset=0x40,size=0x10,type=0x10030)` and give data chunk 6 size `0x180`. Selected chunk 4 occupies `[0x80,0xa8)` within chunk 6's `[0,0x180)`, but its board edge is accepted. This violates the no-overlap contract and could permit modification of bytes owned by another entry.
2. **DXT5 contract and helper disagree.** `synth(fmt=0x83f3,a_size=80)` is accepted. The published writer contract explicitly excludes it. A broad census parser may identify DXT5, but writer admission must be a separate tested refusal; currently this helper is described as the validated edge.
3. **Invalid dimensions/counts are normalized.** `synth(a_size=24)` with descriptor width zero, and `synth(a_size=32)` with mip count zero, both pass. The helper uses a minimum of one pixel/level. No measured zero-value encoding was established in the handoff. Require explicit positive dimensions/counts for writer admission, or document a measured exception before admitting it. These are defensive boundary findings, not evidence that installed boards contain them.

Results and probe source hash: [research-20260928-receipt6-evidence.json](research-20260928-receipt6-evidence.json). No installed file was read or written by this replay. No scratch copies or processes remain. No full census, four real-file round trips, independent texture decoding, visual sheet inspection, archive staging or transaction test was rerun in this review. Those remain research-reported evidence.

## What the handoff now establishes, within its evidence level

The PS3 same-size DXT1/DXT3 full-chain scope, paired versus archive-only topology, changing compressed member length, and requirement for archive staging are coherent implementation inputs once admission is corrected. Solid-block round trips establish placement, not Studio encoder quality or recovery safety. Paired output transaction tests and independent decoding of actual Studio encoder output remain developer obligations. No X360 authorization follows.

C38 remains candidate presentation evidence. The 14 clear samples comprise Legacy pads/blockers and NHL 12 pads; they are research visual judgments, not verified stored-value selection semantics across every intervening title or PS3. Equal ID sets do not prove identical pictures. Deleted sheets were not independently reviewed. Preserve reproducible source identities and a compact review set for the successor. A missing mask picture must say only that no picture is available unless a separate source establishes team-mask status; absence alone does not prove it. The two unused thumbnail prefixes are latent implementation observations, not current user-facing failures.

## Reconcile status without losing remaining work

Research BETA-HANDOFF still names r59 and says receipts 61/62 are not bundled. Receipt 65 reports r64 inclusion; the coordinator independently verified r64 and ran 13 focused tests. Original F1 is closed within its tested in-memory recovery scope. The separate cache/source revalidation and temporary-file ownership findings remain with development. See [receipt-65 review](development-20260928-receipt65-review.md).

Replace generic “remaining Recompiled read routes” with named operations and actual current consumers; delivered public DB editing and bank routes must not be listed as wholly absent. Runtime, save compatibility and limits remain unestablished. Preserve applicable old manual cards while mapping them to r64, including V-94/V-95, rather than silently closing them or requiring obsolete binaries.

Next executable prompt: [research continuation](../plans/research-20260928-after-receipt6.md). No app/defs/research-owned file edits, staging, publication or launches occurred in this coordinator review.
