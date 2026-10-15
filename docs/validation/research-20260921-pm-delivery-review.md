# Research PM delivery review — 2026-09-22

Focused review of DEVELOPER-QUEUE, handoffs 03/17/18 and the r60/r61 probe implementations. No source census or audio decode independently rerun; no application changes, launches or game-data writes. Other handoffs are not independently accepted by this review.

## F1 — P1 contract correction: non-zero extent is not allocation or mip-layout proof

R60 computes the last non-zero byte inside a slice already bounded by the file-table slot. Slots are reported as 8 KiB-aligned. Consequently, `slot_lt_extent4k == 0` is a consequence of the measurement boundaries and alignment, not independent evidence that the texture's addressed mip content fits. Valid texture bytes can be zero. A maximum of observed non-zero extents remains a lower bound on addressed bytes, not an upper bound or a sufficient allocation formula.

Withdraw the statements that this validates the resize rule, that keeping the slot never truncates, that the maximum tail-page observation is what to allocate, and that matching aggregate extent establishes exact single-level layout. An allocation-size oracle records allocations; it cannot establish sub-level positions. Preserve the useful slot-size and non-zero-extent distributions with accurate labels. Distinguish model disagreement with observed non-zero extent from rejection of a complete layout model: a larger prediction may include valid zero bytes or padding.

Before enabling or qualifying changed mipped output, require an independent per-level address/layout contract and test that distinguishes misplaced mips. This applies to shipped shapes too; presence in a size oracle alone is insufficient. Audit the existing developer route rather than declaring it broken from these measurements. Do not disable independently established layouts merely because this new inference is invalid.

## F2 — P2 contract correction: MP3 reconstruction is a hypothesis

R61 establishes the sampled prefixes/header candidates, declared sample counts, failure of a conventional frame walk, and failure/short output of the attempted ffmpeg decode. `10368 = 9 * 1152` and one conventionally walkable frame do not independently parse the remaining eight frames or prove their headers were stripped. The probe does not yet identify each granule, its side information and payload boundaries.

Change the summary, refusal text and queue to say that EA-MP3 framing is not yet supported by Studio and the tested conversion failed. Retain header reconstruction as a candidate investigation. Do not promise the same routine solves gen7 EALayer3 v1: establish the variant relationship and independently test each class. A decoder exit code alone is insufficient; verify declared duration/sample count, errors, channel/rate, and an independent reference where available. User listening remains deferred.

## F3 — coverage reconciliation

Handoff 03's all-seven-title table is valuable recorded coverage, but its reproduction footer still says the full run is in progress and only NHL 20 is done. Queue row 14 still singles out only NHL 15 multi-stream files although NHL 19 now has 5342. Reconcile the final state throughout queue, atlas, STATUS and claims. Distinguish first-stream rule checks from validating every stream in multi-stream files. These files must preserve all streams or refuse; first-stream success is not whole-file resize qualification. Shipped maxima are observations, not runtime limits. BC6H byte-size agreement is not itself decoded-pixel/encoder correctness.

## Research continuation prompt — fresh session

Start a fresh extended research session. Read repository instructions, this review, the PM atlas/queue/STATUS and current coordination claims. Apply F1–F3 as the first published checkpoint, with dated supersession notes and unchanged raw evidence. Do not touch developer-owned application files or matrices. Publish a developer-facing correction claim immediately, then continue without stopping for routine review or user input.

First priority: independently establish Xbox packed-mip addresses and per-level boundaries. Reuse established format implementations/specifications where available, recording provenance and applicability. Use diagnostic level patterns or an independent decoder to distinguish correct placement from equal allocation size. Keep unsupported shapes explicit. Coordinate with the active developer so this work supplements its R2 arithmetic fix and real-source tests without duplicating owned investigations.

Second: investigate EA-MP3 framing without assuming missing headers are the answer. Parse bounded structures, compare independent decoder implementations where available, validate all 24 vectors, then separately evaluate gen7 EALayer3 variants. Record failed hypotheses without elevating them to universal exclusions. No new audio writer without an encoding/alias/container contract.

Third: progress the unresolved PS4 venue vault-reference chain, gen7 ice-layer selector, kit components/selectors, player portrait/head relationships, equipment and overlay/movie formats in roadmap order, favoring contracts that unblock an existing feature across titles. Reuse completed inventories; do not repeatedly census settled fields. On research/runtime-only blockers, publish a precise deferred card and move to independent work. Do not infer a consumer from filenames or a shared number alone.

Continue the existing extended research rules: source files read-only, no launches/live writes/staging/commits/pushes; explicit lane ownership; runnable probes and exact source coverage; claim coordination; checkpoint between packages; manifest-owned cleanup only; no shared-cache deletion; all listening/visual reviews deferred to the user. Preserve NHL 21 E1 acceptance. Finish when usage requires with plain-language summary, developer queue, evidence/limitations, cleanup and an exact next task.

## Addendum for the already-running developer

Continue the extended development brief and finish the R2 arithmetic correction first. Read this review before consuming PM handoff 17/18. Do not use r60 non-zero extents or their maxima as proof of allocation sufficiency or mip placement; audit affected resize claims and require independent layout evidence. Keep the proven operations and record unsupported cases rather than stopping the entire campaign. For EA-MP3 expose the measured catalogue information and a truthful unsupported-preview reason; do not implement header synthesis as an established contract. For NHL 15/19 multi-stream GNF, preserve all streams or refuse explicitly. Proceed with other ready roadmap packages while research resolves these dependencies.
