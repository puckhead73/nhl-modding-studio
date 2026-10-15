# N19-H2b review and H2c likeness refinement

Decision: accept H2b as a reproducible conservative offline baseline. It is not yet the selected likeness for final shading/texture packaging. Authorize **one bounded H2c refinement of non-anatomical visibility exclusions**, with comparable renders and a draft colour bake. Keep the H2b candidate and all anatomical protections intact as the control. Do not expand into new topology or rigging work.

## Reviewed evidence

- Independently verified candidate SHA256 `152703AC9964DE7F51A9EEA6446132E72FC4822A231AD666AFF04D0FF9AFF561`.
- Ran `cargo test -p tdb-headmesh nhl19_fit::tests --lib`: 5 passed, zero failed/ignored.
- Inspected donor/fitted/source three-quarter renders. The fitted shape remains visually close to the donor; the source is visibly faceted while the other two are smoothly shaded. This limits a direct likeness judgment.
- Reviewed the visibility/mask and fit-quality code. The orientation check compares before/after triangle normals; it is not a self-intersection test. Five current tests do not directly regress the reported smoothing/falloff defect.
- Other counts and broad suite totals remain Claude-reported. H2b has no native shading, animation or runtime validation.

## H2c scope

### 1. Separate protection from uncertain visibility

- Preserve the exact H2b anatomical vertex-ID sets: eye sockets/lids, cavity/inner lips and seam groups, oral transitions, shell/attachment regions, and their original positions. Record their hashes independently from heuristic masks. Do not reduce them or nudge eyes/cavity.
- Partition every previously frozen vertex by reason. The reported 461 covered vertices frozen only by the normal/+Z rule are **candidates for inspection**, not automatic new editable vertices. Reliable source correspondence includes anatomy, surface orientation and coverage; distance <=6 mm alone does not establish it.
- Replace the front-only visibility rule with a conservative multi-direction exterior-surface assessment appropriate to the face, supported by connectivity and region limits. An occluded vertex from one viewpoint may be an outer cheek/ear; visibility from any viewpoint alone also does not establish that it is safe exterior tissue.
- Retain torso, shoulders, mouth interior, inner sockets, ambiguous seams, and source-uncovered regions regardless of apparent ray visibility. Show newly admitted IDs highlighted in front/profile/cutaway, with counts and reasons. Exclude uncertain vertices instead of stretching correspondence rules to increase the editable count.

### 2. Make a controlled comparison

- Start from the same pristine donor and hold alignment, correspondence limits, displacement cap, anatomical masks, and fitting parameters fixed for the first visibility-only A/B comparison. Changing alignment at the same time would confound the result.
- Produce **one refined candidate**. If it remains heavily damped, optionally produce one additional candidate using a measured spatial/geodesic falloff instead of uniform graph-ring distance. Keep the same exact hard-frozen set and gate smoothing by the same influence. Do not shrink protection to force full influence. Two refined candidates maximum in this package.
- Preserve source/donor/transform/mask hashes, vertex IDs/order, UVs and connectivity. Export hard-frozen bitwise checks and unchanged-native-attribute checks. Keep native artifacts position-only diagnostics with stale shading explicitly labeled.
- Report fit residuals before/after over the **same fixed anatomical sample/correspondence evaluation set** for baseline and refined fits, not only whichever vertices each method selected. Split editable exterior, protected regions and unsupported coverage. Show median/RMS/95th/max, no-match counts, influence distribution, moved-vertex counts and protected-boundary displacement. Do not equate more editable vertices with better likeness.

### 3. Regression and geometric quality

- Add a focused synthetic fitting regression for the smoothing bug: hard-frozen positions stay exact, smoothing respects low influence next to the boundary, and the prior ungated implementation would violate the asserted bound. Also cover rejecting oral/interior and source-uncovered vertices while admitting an exterior side-facing surface the old +Z rule incorrectly excluded.
- Keep normal-change/degenerate checks, but name them accurately. Add localized triangle intersection/penetration checks around the lips, cavity, eyes and newly admitted transition regions, excluding intended shared topology and recording pre-existing contacts. If an automated check is incomplete, record its scope and supplement with close-ups rather than claiming a general collision-free mesh.
- Clarify the alignment explanation: three non-collinear matched points can determine a proper 3D similarity; rank-two covariance alone does not make the problem unsolvable. The observed negative scale was an implementation/degeneracy-handling defect. The explicit oriented-frame method is acceptable; keep its evidence without teaching the wrong general limitation.

### 4. Fair visual comparison and draft texture

- Render donor, H2b baseline, refined fit, and aligned source at identical camera, scale and lighting with comparable normal treatment. Source triangle duplication must not create unintended per-triangle shading: retain authored normals or use a preview-only smoothing solution that preserves intentional seams. Do not weld/reindex native donor data.
- Provide front/profile/three-quarter and focused eye/mouth/cavity comparisons with consistent legends. Keep an untextured comparison to assess shape independently.
- Bake the supplied Mogilny colour image to the selected refined donor's unchanged UVs as a **draft preview texture**. Use region-constrained rays/cages, separate upper/lower lip correspondence, and island dilation. Preserve donor interior/eye appearance where source coverage is absent; report missing coverage and projection errors. No GNF encoding or final mip qualification is needed yet.
- Show the draft textured result against the textured aligned source. Texture must not conceal failed geometry checks. Do not change normal-map interpretation or claim game lighting from this preview.

## Exit and following work

Select the best baseline/refinement based on likeness plus preserved anatomy, not minimum distance alone. If neither refinement materially improves likeness without damaging constraints, report that limit and show the comparisons; do not keep iterating or relax the eye/lip protections silently.

Return a short receipt with exact changed paths, commands/checks, full artifact hashes, fixed-set metrics, newly admitted mask views and the recommended candidate. This package should produce visible progress and a concrete selection, not another inspection-only result. Native normals/tangents and final GNF texture work follow the selected fit. Runtime file loading and facial animation remain separate open gates.

No commits/pushes, game launch, live deployment, topology substitution, weight transfer, or unrelated cleanup. Private inputs/output remain in scratch. Preserve the H2b control artifacts.
