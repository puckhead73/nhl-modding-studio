# N19-H2a review and H2b constrained-fit dispatch

Decision: accept H1R writer corrections for the fingerprinted offline donor workflow. Accept H2a as useful structural/anatomy reconnaissance, not proof of animation compatibility. Authorize **N19-H2b: correct the evidence claims, complete protection masks, align, and produce a conservative offline Mogilny fit in one package**. Do not wait for full facial reverse engineering to prepare a reviewable fit. No runtime/deployment approval is implied.

## Review evidence and limits

- Reviewed the profile-gated adapter, writer checks, anatomy-mask code, source-transform probe, and cutaway image.
- Independently ran `cargo test -p tdb-rx2 mcd_write::tests --lib`: **31 passed**, zero failed or ignored.
- Eye-mask seeds currently use each eyeball AABB's half-diagonal times 1.35. Lip seeds are every boundary component except the largest, with a two-ring transition. These are useful candidate masks; those rules alone do not establish complete anatomical coverage.
- The cutaway shows why retaining the original shell matters: oral surfaces extend behind the lip seam, and the same head mesh extends into the torso. An explicit interior mask and source-coverage mask are needed before wrapping.
- Source handling in `mcd_anatomy.rs` scans rotation strings and pairs them with primitive order; `source_overlay` currently emits measurements, not actual landmark-overlay images. That is enough for this file's equal-rotation reconnaissance, not a fitting transform implementation.
- Other suite totals and detailed channel counts are Claude-reported; not independently rerun or structurally decoded during this review.

## Correct the report before fitting

1. Replace the magnitude-census verdict in code/output/receipt. Small sampled values are **consistent with** deltas; they cannot exclude quantized, compressed, local-space, transformed, sparse, or indirect absolute targets. Undecoded payload framing prevents declaring that no absolute targets exist or that the tail AABB is the only derived field. Retain unknown blocks verbatim for the first offline candidate as an experimental preservation policy, not as proven compatibility.
2. Say “no identified blink/jaw channel in the decoded names; animation source unresolved.” Do not claim absence of animation data anywhere in the container while payloads remain undecoded, or rule out external/runtime bindings.
3. Keep normal/tangent preservation only for the small position-only writer diagnostic. H3 still requires recomputed native normals/tangents for the final shaped surface. A baked textured/shaded preview can use regenerated attributes before native shading write support exists, but must identify that difference.
4. Distinguish exact coincidence from the probe's micrometre-rounded grouping. Record pair distances and grouping tolerance. One-ring mean height is a proposed upper/lower classification, not proof of lip mechanics; equal/near-equal or geometrically unclear pairs stay ambiguous and frozen.

## H2b implementation

### A. Complete conservative masks

1. Keep current protected IDs as a minimum. Inspect and explicitly add **all oral-cavity/inner-lip surfaces**, not just boundaries and two rings. Trace connectivity on the validated donor mesh, inspect labeled cutaways, and record cavity IDs and uncertain regions. Do not seed a traversal that floods into the outer face without a verified stopping boundary.
2. Verify each candidate “oral” boundary loop by location and connectivity; document IDs/counts/bounds. Boundary size alone is not anatomical classification. Freeze ambiguous groups whole; never weld coincident vertices, bridge the opening, or smooth across the upper/lower seam.
3. Inspect both upper/lower eyelid folds, inner sockets, and their transitions. Expand the AABB-derived seed if any pocket surface lies outside it. Keep both eyeballs and lashes fixed. Provide close-up cutaways with a legend and a clear distinction between hard-frozen regions and deformation falloff.
4. Freeze torso, shoulders, attachment seam, and every donor region lacking a trustworthy corresponding source surface. The donor's source-unmatched chest must not project onto Mogilny's chin. Build an explicit **editable outer-face mask**, not simply the complement of protected vertices. Treat hair/scalp coverage separately and preserve native companions.
5. Save masks with donor hash, vertex IDs/count/order, method and manually verified overrides. Assert all masks are in range and hard-protected/editable sets do not overlap. Freeze uncertainty rather than resolving it through a guessed nearest surface.

### B. Align the reference and quantify the compromise

1. Parse scene/node/mesh links and accumulated transforms correctly, retaining head/eye identity independently of the `None` material. Apply each node transform once. Preserve the original source geometry/UVs for texture baking; any fitting-only repair is a separate artifact.
2. Use corresponding eyes, mouth landmarks, nose, chin and ears within the overlapping face surface to establish a rigid/similarity alignment. Record selected landmarks, transform matrix, units, and residuals. Do not fit overall donor bounds that include torso/scalp. Do not independently move donor eyes or oral cavity to chase source landmarks.
3. Produce actual source/donor front, profile, three-quarter, and mouth/eye close-up overlays. Show residual errors in protected regions separately from the free outer face. These images supply the likeness-cost assessment that H2a did not produce.

### C. Produce one conservative Path B candidate

1. Reuse the existing donor-morph/Blender job machinery where appropriate; verify its actual behavior and extend the input contract for this donor's masks. Do not invent a second general head pipeline. A deterministic offline implementation is sufficient; no UI is needed.
2. Fit only the approved exterior vertices using region-constrained correspondence and a recorded displacement cap. Prevent projection onto eyes, oral interior, opposite lip, torso, or source regions without correspondence. Use a smooth falloff outside hard-frozen regions and reapply hard constraints after each fitting/smoothing step.
3. Preserve donor topology/order/indices/UVs and hard-frozen positions bit-for-bit. No new rig, weight transfer, source-topology substitution, mouthbag nudge, or eyeball nudge. Start every iteration from the fingerprinted pristine donor; do not admit arbitrary prior outputs as vanilla profiles.
4. Reject inverted or collapsed triangles introduced by the fit and inspect self-intersection/inner-surface penetration in eyes, lips, and cavity. Separate pre-existing donor degeneracies/contact from new defects; do not demand a positive neutral lip gap if the original mouth is closed. Report motion/displacement at mask boundaries and large outliers.
5. Export the candidate with source/donor/mask fingerprints and vertex-ID mapping, plus unchanged-attribute/protected-position assertions. The current native writer may emit a clearly labeled **position-only offline diagnostic** with opaque data preserved and bounds refreshed. It is not ready for final lighting or animation claims.
6. For visual review, regenerate preview normals/tangents with correct donor seam/winding behavior. Provide neutral solid-shaded comparison and, if straightforward with existing tools, a draft source-colour bake onto donor UVs. Texture work must not delay the first geometric comparison; no GNF/mip qualification is required in H2b.

## Acceptance and next decision

Deliver a reproducible command, corrected evidence report, masks/landmarks, one fitted candidate, front/profile/three-quarter and oral/eye cutaways, and quantitative checks. Include before/after/source comparisons at the same camera/scale. Report whether the free-face likeness improves and what donor eye/mouth features remain by design.

**Accepted H2b evidence is offline shape and preservation only.** Blink/jaw function, opaque-data compatibility and actual file loading remain unresolved runtime gates. If masks leave too little editable surface for an adequate likeness, show that result rather than relaxing protected anatomy automatically. Only stop before a candidate if a concrete geometry/identity/coverage defect makes even a conservative fit invalid; document it with an image and exact affected region.

Final native normals/tangents, texture bake/GNF mip chain, and runtime A/B test remain the following packages. Return a receipt of at most 100 lines with exact paths, commands/results and full hashes. Keep proprietary assets in private scratch. No commits/pushes, live deployment, support-matrix edits, or unrelated cleanup.
