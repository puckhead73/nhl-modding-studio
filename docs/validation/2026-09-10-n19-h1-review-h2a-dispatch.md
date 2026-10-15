# N19-H1 review and H2a dispatch

Decision: accept the exact-donor offline no-op/forehead-patch demonstration and H0R codec regression work. H1's broader fail-closed ownership/writer claim remains conditional on the corrections below. Authorize H1R and read-only H2a anatomy/deformation investigation. Runtime baseline/precedence remain open; neither blocks this offline package. Path B remains the intended fitting route.

## Independently reviewed

- Read `mcd_write.rs`, `pack_mcd.rs`, `mcd_edit_proof.rs`, the post-fix manifest, and the deployment specification.
- Staged no-op SHA256 matches `4DA8F5D5C9EAA64F3AAA38B7E9E4CB0B0AEA74B91A1884FF863BF0CC61243195`; diagnostic matches `E19190EAFAA9A381097218186A309DE310CBFB3279CAB070520A275CA833E322`.
- Ran `cargo test -p tdb-rx2 mcd_write::tests --lib`: 15 passed.
- Ran `cargo test -p tdb-archive-codec padded_chunkzip --lib`: 3 passed, including arbitrary compressed-length alignment and truncation tests.
- Other suite totals, the database census, zero-weight measurement, and SKTN interpretation remain Claude-reported findings, not independently reproduced in this review.

## Corrections to conclusions

1. **Index ownership is improved, not fully established.** `McdContainer::parse` still selects the first unused XDNI with the requested count, then the first used match. It neither rejects multiple equal-count candidates nor validates the selected index payload's type, extent, and vertex range. The decoy test only rules out a different-count decoy. Count equality alone does not prove a shared buffer.
2. **Position encoding is discarded.** `Attr` retains byte size but not component type/count; every `p0` is then decoded and written as three f32 values. A syntactically accepted `3f16` position can therefore be read/written incorrectly. Empty component tokens can panic in `split_at(1)`. Non-finite replacement coordinates are also currently accepted.
3. **An exact profile is not enforced.** The proof and adapter accept arbitrary parseable donors, despite measured forehead coordinates and ownership assumptions being Eichel-specific. Same vertex count cannot prove source vertex order or protected-region identity. Restrict the experimental invocation to a checked donor/profile rather than implying general NHL19 support.
4. **Zero weights narrow the evidence, not the animation location.** They do not prove the named skeleton is unused or that all animation must reside in the opaque region. External binding, runtime-generated streams, and indirect relationships remain possible. Retaining zero bytes is not yet evidence of retaining a working facial rig.
5. **Uniqueness has a bounded meaning.** Unique head IDs across seven inspected DB revisions support no shared `pPaD` in those revisions. They do not alone prove that no other player/variant/runtime reference can consume the same filename. State the finding at the measured scope.
6. **Revision equivalence is limited.** Matching geometry supports offline fitting; differing skeletal name ordering cannot be declared runtime-safe before binding semantics are known.

## N19-H1R — Bounded writer corrections

1. Preserve VFMT component type/count and admit only the exact supported position encoding (`3f32`) for editing. Reject unsupported/empty/malformed tokens with errors, not panics. Check field extents within stride, declared stream slots, equal vertex counts for split streams, and finite replacement positions.
2. Reject missing/truncated required declaration words instead of substituting zero/one defaults. Avoid unchecked arithmetic and duplicate/ambiguous position or submesh identities. Keep the work bounded to the measured donor layout.
3. Resolve XDNI identity from stronger native relationships if available. Otherwise explicitly pin verified buffer offsets/type/count/ranges to the exact donor profile and reject deviations or ambiguous matches. Do not describe first-count matching as declaration-only ownership. Validate payload extent, encoding and every triangle index; explicitly document opaque non-triangle groups.
4. Add synthetic regressions for equal-count decoys/ambiguous sharing, truncated selected indices, out-of-range indices, unsupported position types, malformed VFMT tokens, split-count mismatch, and NaN/Inf patches. Ensure failures happen before output artifacts are installed.
5. Gate the experimental fit/proof entry point with the verified donor fingerprint and measured layout/profile. A patch91 variant may be admitted only under its own explicit profile; reject arbitrary same-count inputs. Carry source fingerprint, vertex IDs/order, and protected masks with later fit outputs; an array length check alone cannot establish those invariants.
6. Rerun targeted tests and exact-donor no-op/diagnostic proof. Preserve expected hashes if the accepted fixture's output is unchanged. Keep position-only diagnostics clearly separate from the final fitting packer, which still needs regenerated normals/tangents.

## N19-H2a — Establish anatomy and deformation before wrapping

1. Produce a reproducible read-only report of SKTN count, names/order, offsets and references for both revisions. Reconfirm raw t7/t8 bytes against structurally selected head streams and document precisely what zero entries establish.
2. Inspect the ~480k–1007k region: identify declared tables, channel names, target vertex references/counts, delta versus absolute-position encodings, and any dependencies outside the container. Do not assume an XDNI block is triangles because it has indices. Preserve raw unknown spans.
3. Trace at least one eyelid and one mouth/jaw-related deformation end to end if possible. Report referenced vertex sets, displacement units/frame, and whether replaying a channel is supported by evidence. If the binding is external, locate it read-only from the exact install/archive identity. If not resolved, label it unknown rather than generating speculative animation.
4. Export inspection geometry using validated index ownership. Show donor eye sockets/eyeballs/lashes and oral opening/inner lips/cavity from front, profile, and cutaway views. Locate the mouthbag or native equivalent; do not assume it is a separately named submesh.
5. Build explicit donor vertex-ID masks for eyelid pockets, upper/lower lip seam and transition regions, oral interior, and neck. Derive them from inspected topology and any proven channel binding. Keep coincident upper/lower lip vertices distinct. Mask previews and counts are required before fitting.
6. Apply source scene transforms once and prepare Mogilny/Eichel landmark overlays. Report the likeness cost of frozen eye and mouth structures. This package permits alignment previews and masks, not a full shrinkwrap or relaxing protected anatomy.
7. Deliver a decision on Path B's deformation compatibility: which bytes remain verbatim, which absolute targets/bounds require updates when base positions move, and what remains blocked until runtime. Offline fitting previews may proceed in a later package even if animation verification needs the operator; do not claim blink/jaw correctness without evidence.

## Deployment specification corrections

Remove the truncated-file probe from the default experiment. It introduces a crash/error signal that is less specific than a valid asset A/B test. A no-op control rendering normally does not prove it loaded. A trace showing a logical asset request does not establish which physical loose/archive payload supplied it. Therefore no visible change plus a generic request trace remains inconclusive.

Use a cold game restart between control/diagnostic runs, the same demonstrably 3D head scene, front AND profile screenshots, installed-file hashes, and physical-file/archive-entry load evidence where available. Do not broadly delete shader caches or saves. Restore the exact pre-test path state: if absent, remove only the created test file and verify absence; if present, restore its recorded original bytes/hash. Verify the staged donor separately.

Replace “offset reflow” with qualified fit-in-slot or append-and-retarget planning; whole-archive reflow is not the default. Preserve uncertainty about archive precedence and revision-dependent skeletal data. No live deployment or game launch is included in H1R/H2a.

## Receipt

At most 100 lines plus linked reports/images: exact changed paths, targeted commands/results, full hashes, corrected ownership/encoding checks, anatomy masks and deformation findings, remaining runtime gates. No commits/pushes, capability promotion, full topology substitution, weight transfer, or unrelated dirty-tree cleanup. Stop for architect review when H1R/H2a artifacts are ready.
