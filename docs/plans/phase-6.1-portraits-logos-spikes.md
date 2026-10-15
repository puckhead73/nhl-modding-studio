# Phase 6.1 — Portraits & Logos: discovery-spike briefs

> **Status:** 📋 drafted 2026-07-26. Gates the 6.1 pipeline work in
> [phase-6-asset-tools.md](phase-6-asset-tools.md#phase-61--portraits--logos-first--2d-no-blender-dependency).
> Each spike lands a verified finding in `docs/audits/` with provenance + confidence (CLAUDE.md rule 3)
> before any pipeline code depends on it.

## Grounding: what already exists (verified against the tree, 2026-07-26)

The **read / decode / export** side is substantially built in `crates/studio-domain/src/assets.rs` and
`crates/tdb-rx2` — the spikes below must **not** re-derive it:

- **Portrait id resolution.** `ubPc.lOnr` = portrait art id; **`kOtt.lOnr` = team-logo art id** (same
  raw id, different table) ([assets.rs:116-118](../../crates/studio-domain/src/assets.rs#L116)).
- **Portrait path resolution per (title, platform)** ([assets.rs:355-372](../../crates/studio-domain/src/assets.rs#L355)):
  - X360/PS3 — `fe/ion/artassets/…/p{id}.big` (EA BIGF → RefPack → Xbox-tiled `XPR2`).
  - PS4 **NHL 15** — flat PNG `assets/portraits/p{id}.png`.
  - PS4 **NHL 16+** — tiled BC7 `kettle_assets/playerportraits/p{id}.gnf`.
- **Decode is magic-dispatched** — `decode_portrait` handles all of the above by sniffing the file
  ([assets.rs:1093](../../crates/studio-domain/src/assets.rs#L1093)); `DecodedImage` → PNG
  (`to_png`/`to_png_fast`), `cropped_to_opaque`, and **DDS encode** via `tdb_archive_codec::encode_dds`
  already exist ([assets.rs:985-1004](../../crates/studio-domain/src/assets.rs#L985), :1221).
- **Export flows** ship: `export_player_assets` ([:1335](../../crates/studio-domain/src/assets.rs#L1335)),
  `export_team_outfits` ([:2321](../../crates/studio-domain/src/assets.rs#L2321)) with team-id→slug raster
  renaming ([:2182-2187](../../crates/studio-domain/src/assets.rs#L2182)), and `browse_asset_dir`.
- **X360 texture container** read/round-trip (`texlib_<id>.rx2`: tiled, byte-swapped DXT1/3/5) is
  documented + implemented in [tdb-rx2/src/texlib.rs](../../crates/tdb-rx2/src/texlib.rs); GNF decode in
  `gnf.rs`; XPR2 in `xpr.rs`.

**⇒ The real 6.1 gaps are (a) the ENCODE/REPLACE (import) direction per target, (b) a searchable
portrait/logo index and match-review workflow that reuses the established universal slug, and (c)
the per-game logo icon generator.** The spikes below scope exactly those.

---

## Spike P-S1 — Import/replace round-trip per (title, platform) ⟵ gates the converter + `portrait_import`

**Question.** For each in-scope portrait/logo target, can we take an arbitrary source image (PNG/DDS)
and write it back **in the game's native on-disk form** such that the game loads it correctly — i.e. is
**encode the exact inverse of the decode we already have**, container and all?

**Why it's non-trivial (the decode side hides these).** Reading is magic-dispatched and forgiving;
writing must reproduce the *exact* container the title expects:

- **X360/PS3** — re-tile + 16-bit-pair byte-swap the DXT payload (inverse of `texlib::untile_dxt`),
  wrap in `XPR2`, **RefPack-compress**, and rebuild the **BIGF** archive preserving the sibling
  Scaleform entries `p{id}.big` carries (~6 entries — we replace one raster, keep the rest). Mip chain +
  the preserved `0xE000` width-word flag bits ([texlib.rs:43-45](../../crates/tdb-rx2/src/texlib.rs#L43))
  must survive.
- **PS4 NHL 15** — flat PNG; near-trivial, but confirm colour space / channel order / expected
  dimensions.
- **PS4 NHL 16+** — encode **tiled BC7 GNF** (inverse of `gnf.rs` decode) — the highest-risk encoder;
  verify tiling + header against a real re-import.
- **Dimensions/format policy** — does each title tolerate a resized/reformatted portrait, or must we
  match the vanilla raster's exact WxH + pixel format? Decide resample-to-target vs reject.

**Method.** For each target, take a **vanilla** portrait/logo already on disk → decode → re-encode with
the new writer → **byte-compare to the original** (the headline round-trip proof) → then a **modified**
raster → install into a copy of the anchor tree → **in-game load check**. Team logos exercise the same
chain via `kOtt.lOnr` (reuse the outfit raster-naming at [assets.rs:2182](../../crates/studio-domain/src/assets.rs#L2182)).

**Output (per target, in `docs/audits/portrait-logo-import.md`).**
- A **support row** {title, platform, container, codec, exact-vs-resample policy} with confidence.
- The **byte-identical re-encode proof** for at least the proven titles first (X360/PS3 Legacy), PS4
  handled per its own row.
- A pinned **fallthrough policy**: a target whose encoder is unproven is **listed read/export-only and
  import-disabled**, never written with a guessed container.

**Gates.** The on-import converter, `portrait_import` / `logo_import`, and which (title, platform) cells
flip to import-supported (proven-first).

---

## Spike P-S2 — Portrait source index + match review ⟵ gates `portrait_search`

**Settled input contract.** Player-specific source assets use
`lastname_firstname_dd_mm_yyyy`. `tdb-core::dev_key` owns the implemented normalization and validation;
this spike must not re-derive or generalize it.

**Grounding.** The EA and historical sides already derive the bridge key, Player Assets exports it,
and the headmesh library recognizes it. PS4's native `first_last` asset name remains a separate target
adapter concern. In-game portraits are keyed by numeric `lOnr`, so the new work maps an incoming
universal slug to that resolved player identity.

**Method.** Index representative source portrait sets using `is_well_formed_player_slug`; resolve exact
keys through the existing EA/canonical identity bridge; measure missing, malformed, duplicate and
unresolved cases; design the explicit review queue for those cases.

**Output (in `docs/audits/portrait-slug-grammar.md`).**
- A searchable source-asset index keyed by the implemented universal slug.
- Coverage for exact, missing, malformed, duplicate and unresolved keys.
- An ambiguity/manual-resolution workflow with persistent mappings to canonical player identities.
- Integration tests proving portrait import and existing Player Assets/headmesh exports resolve the
  same player key.

**Gates.** `portrait_search`, the match-list ranking UX (high→low, "review all slug matches", manual
browse), and the subject-name resolution shown in the UI.

---

## Sequencing within 6.1

P-S2 (matcher) and P-S1 (importer) are **independent** and can run in parallel. Ship order once both
land: index/search + matcher UI (needs P-S2) can surface *before* import is enabled per target (needs
P-S1) — i.e. users can find/preview matches while write-back is still proven-first gated. The **logo
icon generator** depends on P-S1's encode chain (it emits the same containers) and follows it.

## Deferrals / non-goals for these spikes

Not in P-S1/P-S2: the 3D mesh/Blender work (Phase 6.2+); appearance enums (6.3); any portrait format
whose **in-game load** can't be verified this round (list read-only, revisit). No format specifics
asserted as in-game-true without the round-trip + load proof (CLAUDE.md rules 3–4).
