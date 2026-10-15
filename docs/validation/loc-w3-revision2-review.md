# LOC-W3 revision 2 coordinator review

18 September 2026. Continue the same research session for a bounded document correction. Developer R3 remains the active development priority; this does not dispatch LOC-W3 implementation or reopen NHL21-E1.

## Disposition and evidence

The operation split, anomalous-order insertion refusal, shared-pointer support, separate appended-text/new-key runtime cards, and placement limitations are useful and retained. The contract still needs the corrections below before implementation. No new extraction or experiment is required.

Coordinator inspected handoff 20 and its census JSON: 95 records; five no-descent records have duplicate hashes; xbox360-nhl07-anchor and ps3-nhl07-anchor both have zero null text pointers. This is a document/data consistency review, not an independent rerun of the census or decoding measurements.

## Required corrections

1. **Ordering:** class A means nondecreasing unsigned hashes (no descents), not strictly ascending. Existing equal-hash runs remain intact. Class B has at least one descent. Keep the separate new-key collision policy; do not reject a valid existing collision merely because class A permits equality. Update B2, class definitions, output invariants and tests together.

2. **Preserving serializer:** append after the physical end of the original pool, before the relocated trailing checksum, not after the last used string byte. Preserve the entire original pool including slack. Explicitly account for changed offsets, sizes and checksums. No-op byte identity belongs to P1.

3. **Canonical rebuild:** remove the universal empty byte-difference requirement in C1. A serializer that removes slack or repacks strings cannot promise byte identity. Require decoded row-by-row equality for a no-op semantic rebuild, preserved opaque metadata, and independently accounted structural differences: pool layout/length, discarded specifically identified slack or unreachable string storage, affected pointers, affected sizes and CRCs, and tree bytes only when rebuilding the tree. Do not permit opaque-byte loss under a generic pool exception. List these rules for shipped and Studio-touched inputs. C2 must compare to the requested result, including inserted rows and edited texts, rather than requiring an unchanged row set for every operation. Validate changed ranges against the transformation, not against a list produced solely by the serializer under test.

4. **Baseline-relative validation and regressions:** unchanged key/hash/text invariants exclude exactly the requested insertions and edits and the explicitly selected current sharers. T5 must allow the edited pointer and any associated size/CRC updates instead of requiring the whole record block to remain identical. T7's this-row and all-sharers cases must start independently from the same shared baseline: after splitting A away from B, B is no longer a sharer of A. T8 must account for downstream pointer/size/CRC changes during canonical rebuilding. T9's null-count increase applies only to a previously non-null row or a new null row; an already-null edit is idempotent. Scope existing-key refusal to insertion, not existing-key text edits.

5. **Census wording:** replace the every-title null-pointer claim in B5 and the revision notes with the measured exception for both NHL 07 anchor records. State counts and coverage from the JSON without turning sampled coverage into a universal title/locale assertion. Retain support for null pointers regardless of whether a particular input contains one.

## Next Claude prompt

Apply these five corrections to research handoff 20 and its test matrix, updating the atlas/status only where needed. Preserve the accepted operation scope and runtime gates. Reuse the existing 95-file census; do not launch games, extract more inputs, modify application code, or create a build cache. Check the finished document for contradictions among supported operations, invariants and expected tests. Provide a short plain-language receipt followed by the exact corrected sections and stop for coordinator review. Do not stage, commit or push. Clean only obsolete owned scratch with verified containment; keep specimens only for a named active dependency.

## Roadmap

Phase 1 remains active across targets. LOC-W3 is a Phase 1.3 implementation contract awaiting this correction, not implemented growth support or runtime acceptance. Accepted codec/logo/group research remains available after developer R3. Phase 2 expansion remains queued; Projects/Builds remain the final stretch feature. NHL21-E1 remains accepted and closed.
