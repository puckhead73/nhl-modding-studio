# Frostbite implementation review and roadmap — 14 October 2026

## Findings and implementation decisions

NHL 22–24 PS4 need a container-backed asset workspace independent of a TDB session.
The current project registry assumes a database; feeding Frostbite through BIG extraction
would hide the actual dependency graph and force a nonexistent roster prerequisite.
Start with a read-only browser reachable during onboarding. Keep installed files immutable.
Preserve layer, superbundle, bundle, asset kind, manifest order, SHA-1, resource metadata,
and absolute CAS location. Do not flatten duplicate assets into one filename.

Claude's [structural report](../formats/frostbite-ps4-file-structure.md) and probes establish
the PS4 layout from installed bytes. The new Rust implementation uses those measurements
for bounds-checked DbObject, TOC, manifest and CAS framing readers. Research scripts are
useful oracles, but not production parsers: unchecked lengths, unbounded LEB128, swallowed
bundle errors and empty-manifest rejection need correction in the production path.

### References inspected

| Reference | Findings | Studio decision |
|---|---|---|
| [FMT releases](https://github.com/FMTDev/FMT.Releases) | Release/distribution repository; lists NHL 22–24 PS4 at 50%, links older supported releases; advertises multiple console platforms and separate core/profile/EBX/compiler packages | Useful behavioral comparison, not a complete NHL source implementation or proof of runtime support. Record exact version/profile for future comparisons. |
| [FrostyToolsuite](https://github.com/FrostyToolsuite/FrostyToolsuite), commit `ee2a587af394fc784b309bb7990dab051065330f` | .NET 8 rewrite; README says early development and no functional UI. SDK separates filesystem, loaders, CAS codecs, RIFF/legacy EBX, type SDK and mod compilation. CLI can load, create/update mods and produce ModData. GPL-3.0 source. | Use as format/architecture reference; no source vendoring into MIT Studio. A PC ModData launch workflow is not evidence of PS4 deployment. Full compatibility requires game profiles and types, not just archive access. |
| `Reference/FIFA Editor Tool v2.1.2.zip` | Binary distribution, native image/Assimp dependencies and FIFA21–23/FC24–26 SDK DLLs; no NHL SDK listed | Not an NHL engine or source library. Useful eventual UX/export comparator; no automatic execution or bundling. |
| `Reference/FIFA Mod Manager v2.1.2.zip` | Companion binary distribution | Mod ordering/packaging reference; FIFA package acceptance is not PS4 NHL deployment verification. |
| `Reference/TextureViewingScript.rar` | Python source and cached scan results. NHL 26/27 PS5-specific TOC records, stored CAS reader, RES scanning, GUID resolution, BCn decode, PS5 swizzle, DDS export. README distinguishes direct resolution from adjacency inference. | Keep heuristic results explicitly labeled. Never unpickle supplied cache files. Port only verified format behavior into separately qualified PS5 profiles. Source has no established reuse licence in the supplied archive. |
| `Reference/NHL27TextureViewerCpp.7z` | Supplied archive contains executable, caches and README; README describes C++ sources/build commands but those sources are absent | Cannot review or reproduce absent implementation. Documented UI/export behavior is a target, not verified code. Size-only cache validation is insufficient when CAS files commonly have equal lengths. |
| `Reference/NHL_Legacy_Jersey Preview Generator.zip` | Blender add-on plus scene: `JERSEY` material, color/specular/normal texture suffix matching, `NORMAL_TYPE` selection, scene-controlled rendering, texlib folder traversal | Good future `blender-job` adapter. Requires explicit input/output args, pinned scene/settings, noninteractive invocation, deterministic filenames, job cancellation and render receipts. Deferred as requested. |

The PS5 script's format differs materially: 16-byte chunk records with a separate magic
and location word, versus PS4's 12-byte records. Its NHL 27 Data TOCs can be bare while
Patch TOCs remain wrapped. Its observed stored-only CAS assumption must not replace the
PS4 multi-codec reader. Its adjacency fallback cannot establish asset identity. None of
these supplied claims qualifies NHL 25 or any unexamined PS5 build.

Frosty's Manifest2019 loader identifies header +0x20 as name data, +0x2c as flags,
+0x30/+0x34 as encoded-word/tree-word counts and +0x38 as the Huffman table. Bundle
name words are bit offsets, not hashes. This is a testable explanation for Claude's
unresolved name identity and patch matching. The reader implements an independent
bounded table traversal and will be checked on the actual NHL files. Decoded names alone
do not prove runtime overlay precedence or TOC signature acceptance.

Codec 0x11 is identified as Oodle Kraken by Frosty's codec model, consistent with the
research bytes. A decoder integration must verify exact output lengths, malformed-stream
handling and representative real payloads. Do not redistribute a vendor DLL merely because
a reference tool can load it. Keep unsupported compressed assets listable/raw-exportable.

## Ordered roadmap and acceptance criteria

1. **Container foundation and onboarding (in progress).** Native bounded readers; metadata
   inspection; decoded bundle names; layer-aware paginated browser; manifest asset details;
   raw and stored-block decoded export outside the install. Reject unknown profiles and
   malformed spans. Audit all three installs and NHL 24 Patch against research counts,
   including five empty bundles. Add synthetic truncation/path/size tests.
2. **Persistent asset catalogue.** Transactional project-local index with content fingerprints,
   schema/profile version, incremental rebuild, cancellation/progress, search by asset path,
   kind, resource type and GUID; preserve all occurrences and reverse shared-span references.
   Page queries must not rescan every bundle. Workspace reopen must not require a roster.
3. **Complete payload reads.** Optional explicit codec provider with tested ABI and bounded
   output; RIFF EBX chunk/fixup/type metadata; resource/chunk joins; resolve chunkfilecollectors.
   Locate roster/localization from decoded evidence, then attach existing TDB/localization
   editors only after schema/CRC validation. Treat encrypted initfs as opaque until established.
4. **Asset parity.** Texture RES + chunk decoding (validate GNF versus platform-specific data),
   full mip/slice exports, texture previews; mesh/skeleton/material dependency graph; sound-bank
   previews via existing codecs; typed EBX editor with references. No inferred associations
   silently become writable mappings.
5. **Write/deploy qualification.** Minimal copied-install stored-block replacement; hash/index
   updates; decoded-name patch matching, base fallback/deletions; signature behavior; shared
   spans and asset dependencies; runtime launch/edit/reload/restore receipts on each title.
   Only then implement project change sets, conflict resolution and mod distribution.
6. **Pre-Frostbite parity.** Reconcile the Oct 13 review against current code first: several
   defects may already be fixed in uncommitted work. Prioritize copied-archive deployment UI,
   correct target selection, full-mip texture replacement, save trailer CRC and real NHL 20
   provenance; then vault-backed kits/equipment/arenas and audio. Reuse existing transaction
   and preview workflows. Runtime acceptance remains a separate evidence level.
7. **PS5 and optional Blender rendering.** Add separate measured NHL 25/26/27 profiles using
   local fixtures, then integrate the jersey renderer through the existing job crate.

Full Frosty parity spans editors, type SDKs, dependency tracking, codecs, compilers and
runtime deployment. It is not achievable just by implementing TOC/CAS extraction. The
milestones above expose useful functionality early without claiming unverified write support.

## Validation record

The first implementation is present in `crates/tdb-frostbite`, the standalone Tauri commands
in `studio-api/src/frostbite.rs`, and `/frostbite` in the app. Selecting NHL 22, 23 or 24 in
the project picker opens it. It does not create a database project or write into an install.
The browser exposes Data/Patch, decoded bundle-name search, manifest asset metadata,
TOC-level chunks, paginated lists, raw export and stored-block decoded export. Output uses
a temporary file and no-clobber publication outside the source install.

| Install tree | Bundles | Manifest asset occurrences | TOC chunks | Empty bundles |
|---|---:|---:|---:|---:|
| NHL 22 Data | 43,214 | 903,631 | 132,571 | 1 |
| NHL 23 Data | 52,619 | 1,090,837 | 150,330 | 2 |
| NHL 24 Data + Patch | 149,902 | 2,525,959 | 190,594 | 2 |

All **245,735** bundle manifests and their span counts parsed successfully. All bundle
spans and TOC chunk references were checked against CAS lengths. Counts include duplicate
asset occurrences, not distinct content. Empty manifests are accepted. NHL 24 Patch's
declared but absent `ui.toc` is reported, not silently replaced with Data. See the
[audit outputs](../research/frostbite-2026-10-14/).

**New measured finding:** compressed names decode successfully on all examined TOCs.
Within each examined TOC the names are unique. On NHL 24, **13,457 of 20,187** Patch bundle
names match names in the corresponding Data superbundle (4,412 contentlaunchsb; 7,671
contentsb; 4 globals; 467 globalsfull; 1 game; 1 movies; 901 worldssb). The remaining
6,730 do not match within that superbundle. This resolves the unstable numeric name-word
problem; it does not prove the runtime's full overlay/deletion/fallback rules.

Pre-Frostbite work completed after the reader/browser slice:

- Connected `CopiedTargetPanel` to the PS4 Project Overview, retaining its existing
  server-owned review/apply/restore workflow and archive-member options.
- Separated PS4 roster DATA trailers from the TDB buffer at `Project` source detection.
  Packaging restores the original trailer; database resizing is refused when a trailer
  exists. CRC operations now see only the declared TDB extent in the older roster path,
  including unsupplemented sessions. This also works with the existing Save Editor overlay.
- Recorded observed compression in the save matrix: NHL 15 chunkzip no longer appears as
  unsupported or mislabeled LZ4. Regenerated the matrix after successful fixture checks.
- Verified that the already-present full-mip GNF replacement implementation passes its
  synthetic and real-fixture tests. It did not need to be reimplemented.

Validation: 9 native reader tests (including malformed/truncated inputs and a synthetic
install-to-export flow), 13 browser/copied-target UI tests, 8 GNF tests, 3 PS4 session tests,
the full save matrix, 7 in-place-save tests, 2 save-identification tests and 2 overlay tests.
The matrix now explicitly checks trailer preservation and recomputed PS4 inner CRCs after
edits. TypeScript compilation and the studio-api Rust check pass. Existing unrelated Rust
warnings remain. This is automated/structural evidence, not console/emulator runtime proof;
the new browser has not had a desktop visual walkthrough.

### Remaining work / exact limits

Milestone 1 delivers the measured PS4 structural reader and a useful initial browser;
it is **not complete payload decoding or Frosty parity**. Compressed 0x11 payloads can be
exported raw but need a decoder before typed viewing. Encrypted initfs remains opaque.
PS5 profiles, typed EBX, texture/mesh/audio editors, Frostbite roster editing and deployment
are not enabled. The standalone browser does not yet persist a full project/catalogue.
It reparses a selected TOC for requests; all-install asset-path search, caching,
cancellation, content fingerprinting and progress are milestone 2. Raw exports are CAS
framed bytes, not usable image/mesh files. Unknown codecs/profiles fail explicitly.

The next highest-value work is persistent asset indexing plus an explicitly configured
codec provider, then decoding the chunkfilecollector assets to locate legacy roster and
localization data. No signature acceptance, mod-package compatibility or PS5 support level
has been promoted. Existing uncommitted changes are retained; no game files were rewritten.

### Additional reference details for the next implementer

The Python PS5 texture scanner is a discovery tool, not a complete manifest-based reader.
It filters raw CAS offsets by powers-of-two dimensions, supported format enum values
54/55/60/61/63/66/67 and plausible mip fields. It reads dimensions at RES +22/+24, mip count
at +30 and the chunk ID at +40. Names come from nearby plausible strings; those associations
must remain advisory. GUID-table ordering needs an explicit normalization contract rather
than indiscriminate byte reversal. PS5 location decoding must preserve the independent
record magic/root and packed location; a patch index may reference Data bytes.

Its swizzle module documents AMD SW_4K_S layouts: 16-byte BC elements use 16×16-block
tiles, while BC1 uses 32×16-block tiles. The BC1 derivation includes image-continuity scoring;
that deserves independent pixel comparisons and mip/slice fixtures before write support.
Do not substitute the PS4 8×8 Morton tile implementation. The viewer's DDS export is
top-mip oriented; Studio parity should preserve the full mip/slice chain.

Frosty's RIFF reader separates EBXD payload, EFIX fixups/dependencies and EBXX auxiliary
information. Object decoding also relies on a type resolver and generated type SDK. A RIFF
chunk enumerator alone cannot deliver a safe typed editor. The native roadmap therefore
separates container parsing, type metadata, reference resolution and edits.

The Blender add-on expects material nodes `J_COLOR`, `J_SPEC`, `J_NORMAL`, switch
`NORMAL_TYPE`, and `JERSEY`/`JERSEY_RETRO` collections. Missing texture inputs can leave
the previous jersey's node image attached. A headless adapter must clear/reject missing
inputs per job, sort ambiguous glob matches, set every render parameter explicitly and
avoid depending on inherited scene state. Reuse the scene only after asset/licence provenance
is established; the supplied scene was not executed during this review.
