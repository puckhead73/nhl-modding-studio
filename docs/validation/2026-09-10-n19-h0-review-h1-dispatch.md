# N19-H0 review and H1 dispatch — 2026-09-10

Decision: accept corrected offline fixture identification; **H0 partial, not gate passed**. Authorize H0 evidence closure and offline H1. Runtime baseline and precedence remain open. Path B, including protected eye pockets, oral cavity, and lip separation, remains the accepted method.

## Review evidence

- Read the current codec diff, `eb_find.rs`, `mcd_probe.rs`, and saved archive manifest.
- Independently hashed the three decoded native scratch meshes: each is 1,043,168 bytes; base and patch93 agree, patch91 differs. Full hashes are in the updated plan.
- Independently compared the extracted 3,129,504-byte mesh's thirds with those three files: exact matches in base/patch91/patch93 order. The extracted artifact must not be used as a donor. This removes the supposed three-VIEW fitting requirement.
- Independently compared revisions: 4,569 differing bytes, all within zero-based offsets 203972–211545 inclusive. Did not independently decode the reported 141-name table or prove its index semantics.
- The alignment correction addresses the reported residue problem. However, the existing synthetic padded fixture builder explicitly pads compressed data to a multiple of eight; existing passing tests do not regress arbitrary compressed-length alignment.
- `logs/eb_find_eichel.tsv` still records pre-fix `TruncatedHeader` errors for seven entries. Corrected decoded artifacts exist, but the cited successful manifest needs regeneration.
- The receipt lists three changed files while its preamble says two. Correct the receipt using exact paths. Mtime alone cannot prove unrelated content was untouched; record available baseline/status/diff evidence without claiming historical proof that was not captured.
- Codec/archive/rx2 test totals are Claude-reported; Codex did not rerun those suites in this review.

## N19-H0R — Small evidence and codec closure

1. Add a synthetic multi-chunk padded chunkzip regression with actual compressed lengths not divisible by eight, including multiple alignment residues and byte-exact decoded payload assertions. It must fail against the prior stepping algorithm and pass with the fix. Place padding between chunks, outside each declared compressed payload; do not repeat the old fixture's masking behavior.
2. Cover truncated padding/header/payload and alignment arithmetic boundaries. Keep checked arithmetic or document the established range invariants. Restore the padded-decoder doc comment to its function; the inserted helper currently captures that comment. Do not broaden into LZX changes without separate fixture evidence.
3. Rerun targeted codec and archive tests and retain actual commands/results. Regenerate a post-fix all-match manifest and per-entry hashes without overwriting the old failure evidence. Improve the probe to surface partial scan/decode failure through an explicit summary and nonzero status; successful exit currently does not prove every entry decoded.
4. Treat `+0x0C`/`+0x10` as opaque fields. Cross-revision stability supports a hypothesis but does not prove their hash function or absence of other integrity requirements. Preserve them for same-identity replacement; qualify the writer experimentally before asserting checksums need no work.
5. Complete read-only database extraction and Eichel binding/shared-reference inspection in private scratch; this is within the authorized investigation. Enumerate database revisions first, rather than assuming the base `inxp_00.big` entry is authoritative. Record candidate DB identities and ambiguity if runtime precedence is unresolved.
6. Name the three actual changed paths in the corrected receipt: `crates/tdb-archive-codec/src/lib.rs`, `crates/tdb-archive-big/examples/eb_find.rs`, `crates/tdb-rx2/examples/mcd_probe.rs`. Preserve all pre-existing changes and keep new edits bounded.

## N19-H1 — Authorized offline work

1. Use `4DA8F5D5C9EAA64F3AAA38B7E9E4CB0B0AEA74B91A1884FF863BF0CC61243195` as the exact offline donor profile. Use the patch91 revision as a mismatch/structure comparison fixture; do not interchange revision-specific tables.
2. Build the bounded structural inspection and native patch representation in `tdb-rx2`, connected to `tdb-headmesh::pack_mcd` as appropriate. Explicitly reject this known concatenation artifact and trailing unexplained containers. No general RSF serializer is required.
3. Resolve mesh→format→stream→index ownership. Investigate the 56-versus-100 eye triangle result; equal vertex counts alone prove neither shared topology nor the correct buffer. Report declared index ranges, primitive counts, degenerates, and any intentional sharing independently of the preview heuristic.
4. Establish what the facial-name table indexes: skeletal joints, morph channels, or another binding. Trace references and revisions before treating names as a vertex anatomy map. Derive protected eye/lip regions from verified bindings plus inspected geometry. No guessed name-to-vertex mapping.
5. Prove byte-identical no-op serialization AND a small same-count positional patch in a known exterior cheek/forehead region. A no-op implemented as returning the original bytes does not alone prove the editor. Retain indices, order, UVs, weights, facial tables, eyes, mouth, and opaque regions; update only established position/bounds fields for the diagnostic.
6. Deliver an allowlisted byte-span diff, independently checked patch coordinates, reparse output, protected-position and unchanged-data assertions, and source/output hashes. Add synthetic malformed/ownership/count cases. Keep game files and private fixtures outside the repository.
7. Prepare a deployment experiment specification, but no live file drop, game launch, or archive modification is necessary to complete this offline package. Do not let runtime waiting block the above work.

## Runtime experiment requirements

A random marker file is not a decisive loose-file override test: the game must request the exact asset, and there must be observable evidence of which payload loaded. Prepare a valid, visibly changed native asset at the exact candidate path, with baseline scene, runtime/version, file-open trace if available, expected positive observation, cleanup and hash restoration. A negative result without evidence that Eichel's asset was requested does not disprove overrides. No effect can also reflect caching or a wrong scene/path.

Keep the archive-replacement route as the planned delivery fallback. Base and patch93 agreement supports an offline donor choice but does not prove runtime archive precedence. The user/operator can run the prepared experiment once the valid diagnostic and restoration procedure are reviewable; do not request permission for a speculative marker drop before preparing them.

## Receipt and stop point

Return at most 100 lines covering H0R/H1 results, exact changed paths, targeted checks, full artifact hashes, structural ownership evidence, and unresolved runtime/binding facts. Prepare concrete offline artifacts; no commit, push, live deployment, or support-matrix edits. Stop for architect review when the offline package is complete.
