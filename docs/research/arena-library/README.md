# Arena library research

This research inventories the locally available Studio game library and prepares an expandable arena library for the active NHL 24 foundation. It does not install arenas or claim that cross-engine presentation works in-game.

The census accounts for **33 catalog targets: 32 available sources and NHL 25 unavailable**. It records **7,618,035 asset occurrences**, **1,970 venue/shared-environment groups**, and **5,926 database arena rows**. These are deliberately different counts. There are **1,363 bounded payload specimens**, with one explicit unsupported NHL 27 screen-widget fixup variant. The catalog is an inventory and evidence base; unresolved database bindings and runtime behavior remain visible research gates.

- [Arena and filename catalog](arena-reference-catalog.md): source-year/platform variants, database names where joined, and complete observed filename lists per venue asset group.
- [Format, dependency, and presentation findings](findings.md): what the evidence establishes and what remains unresolved.
- [City-intro filename reference](city-intro-reference.md): exact scenic-video, sidecar and surviving logo-resource names by source year.
- [NHL 24 implementation roadmap](../../plans/nhl24-arena-library-roadmap.md): work packages, interfaces, conversion strategy, installation composition, and acceptance gates.
- [Machine-readable catalog](data/catalog.json): source coverage, errors, variants, and database joins.
- [Reader provenance](data/tool-provenance.json): identities of the research readers and historical mapping input.
- [Source build metadata](data/source-builds.json): observed title/version metadata where available, with unresolved versions explicit.

## Reading the evidence

Each source directory contains `summary.json`, `database.json`, a compressed `assets.jsonl.gz` index, `venues.json`, `movies.json`, and one `*.files.txt` per arena/environment token. Where specimens were decoded, `payload-evidence.json` records their hashes, headers, EBX imports, video streams, and errors. The catalog links every arena's filename list; the asset index provides its archive or TOC/CAS location.

An asset occurrence is a file/member or a bundle-manifest entry. One asset can occur in several bundles and layers. A venue token is an asset grouping, **not** proof of a distinct physical building. Database arena rows can alias a shared scene. Shared ice and environment groups are retained rather than counted as additional real-world arenas.

The research retains game year, platform, source path, layer occurrence, and source hash evidence. The numeric year for NHL Legacy denotes its NHL 16-era provenance; it is not a release-date assertion. Recompiled has no invented numbered-title year. NHL 25 is explicitly unavailable.

Database records come from the repository anchors and are identified by hash. They are not silently represented as the effective installed database. Ignite joins reuse the historical vault-level mapping and are labeled accordingly; the census verifies the corresponding archive paths exist today. Frostbite suffix numbers are never promoted to verified arena database keys.

## Reproduction

From the repository root, with Python 3.12 and the already available research readers:

```powershell
python scripts/arena_library/census.py
python scripts/arena_library/probe.py
python scripts/arena_library/finalize.py
python scripts/arena_library/summarize.py
python -m unittest discover -s scripts/arena_library -p "test_*.py"
```

`census.py --only nhl26,nhl27` restricts the scan. `--render-only` rebuilds the catalog from completed source summaries. Source paths are explicit in `source_list()`; adjust them when installations move. The scanner reads ISO and archive indexes without extracting whole games. Derived loose rendering trees and previous sample dumps are excluded from the installed Gen7 archive census.

The existing local `tdb.py`, `schema_dump.py`, `xiso_list.py`, and `ps4_leaguelogos.py` readers are prerequisites, not newly bundled dependencies. Frostbite readers live under `scripts/nhl27`. CAS specimen decoding uses the already installed UEViewer Oodle library when available; absence or unsupported codecs are recorded rather than bypassed. `ffprobe` supplies the sampled video stream evidence. No decoder DLLs or game payloads are included in these documents.

The compressed indexes contain filenames, identifiers, locations, and metadata—not textures, meshes, or videos. `finalize.py` verifies each compressed index against its uncompressed content before removing the generated plain index and checks catalog/file-list consistency.
