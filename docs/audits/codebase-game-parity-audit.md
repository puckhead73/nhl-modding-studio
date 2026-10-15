# Codebase game-parity audit

**Audit date:** 2026-08-01  
**Scope:** the 17 catalog entries (NHL 07–21, NHL Legacy, and Legacy Recompiled), their 27 committed
main-database anchors, save/container handling, database editing surfaces, and asset tooling.

## Executive result

The database viewer/editor has the right foundation for cross-title parity: TDB parsing is endian-aware,
metadata is loaded from each game, optional capabilities are probed from the loaded schema, and curated
player/team profiles resolve fields by metadata name rather than assuming one modern field id. A new
matrix regression test now proves that every committed main-database anchor can build skater, goalie,
and team views and that every curated field displayed by those views targets a table, record, and field
which actually exist in that anchor.

That is **database-field presentation parity**, not complete product parity. Full editing parity is not
yet established for every feature. Asset authoring remains generation-specific and incomplete, several
top-level asset/build screens are honest placeholders, NHL 10/11 v1 and PS4 save writes remain
experimental pending in-game validation, and some import mappings/codecs remain deferred. The UI must
not advertise these as fully supported until their gates below are satisfied.

## Ground-truth matrix

| Database family | Catalog entries | Committed main DB anchors | Current result |
|---|---|---:|---|
| Gen6 big-endian | NHL 07 | PS3 research copy + Xbox 360 | Profile/view addressing passes; Xbox gen6 roster write tests pass |
| Early Gen7 big-endian | NHL 08–10 | PS3 + Xbox 360 | Profile/view addressing passes; NHL 08/09 PS3 onboarding restored in this audit |
| Gen7 big-endian | NHL 11–15, NHL Legacy | PS3 + Xbox 360 | Profile/view addressing passes; v2/v4 save paths tested; v1 remains experimental |
| Recompiled Gen7 | Legacy Recompiled | Uses the NHL Legacy Xbox-derived schema | Discovery/assets have recomp-specific paths; no independent DB anchor by design |
| Gen8 little-endian | NHL 15–21 | PS4 | Profile/view addressing passes; raw DB and save repack tests pass; console acceptance remains unverified |

The 27-anchor test is `every_anchor_exposes_only_addressable_profile_fields` in
`crates/studio-domain/tests/field_profile.rs`. It auto-discovers anchors, so a newly committed main DB
joins the gate without another hand-maintained title list. The explicit count assertion makes an
accidental anchor removal visible.

## Database presentation and editing

### Verified

- `tdb-core` parses/writes big- and little-endian TDB v6 databases and has byte-round-trip coverage.
- `tdb-defs` loads per-game metadata including name, range, update flag, and foreign-key metadata.
- `Capabilities::probe` gates optional player fields, styles, equipment companions, arena links, and
  rating vocab coverage from the loaded DB rather than the selected title.
- Player/team curated profiles omit absent schema fields and resolve renamed fields by metadata name.
- Ratings use schema-sensitive scales for Gen6, Gen7, and PS4, with inverse write tests.
- Entity transactions cover team moves, trades, extra memberships, jersey/captaincy operations,
  draft-pick ownership, roster status, and companion/relationship repair.
- The full raw-field disclosure views remain available, but `FieldView.editable` honors metadata
  `update=False`; absence is not converted into a fake field.

### Remaining parity gates

- Run an edit/save/reload mutation matrix against **every** anchor, not only representative anchors.
  The new test proves addresses are valid but deliberately does not mutate read-only ground truth.
- Establish expected tab-level capabilities (lines, strategies, contracts, draft picks, jerseys,
  equipment, career stats) and hide tabs whose backing tables are absent. Today most domain readers
  return empty data safely, but the tab bar itself is largely unconditional.
- Complete enum/semantic confirmation for fields still intentionally marked `unverified`.
- Finish import mapping for enum-coded identity, birthplace, style, and currently dropped biases.
- Validate create/position-flip companion creation on each materially different schema family.

## Save/container parity

| Path | State |
|---|---|
| Raw TDB | Byte-identical no-op round trip and validated edits covered |
| Gen6 roster | Read/write covered by tests |
| PS3/Xbox v2/v4 inner container | Extract/repack and edited payload covered |
| NHL 10/11 v1 | Code-complete but experimental: `tag_1c` is not reversed; in-game acceptance required |
| PS4 emulator save | Framing/repack covered for fixtures; on-console/emulator acceptance required |
| Xbox STFS outer package | Not implemented; current anchors expose inner save payloads |
| Multi-part dynasty/season | Not a supported general write path |

UI copy and compatibility documentation should consistently distinguish structural round-trip support
from verified in-game support.

## Asset parity

The asset layer contains real production code, but it is not yet a uniform 17-game editor.

- Xbox-derived formats: BIG/EB extraction, RefPack/chunkzip, XPR2/DDS, RX2 texture libraries, mesh,
  skeleton, OBJ/GLB preview, texture replacement, outfit export, and head substitution are implemented.
- PS4 formats: EB extraction, GNF texture decode, MCD mesh decode, and PS4-specific asset-root/name
  discovery are implemented and fixture-tested.
- Remaining: ChunkLZX extraction, unsupported DDS/GNF/XPR format variants, fully generalized asset
  replacement/install on every title, portrait/logo UI parity, equipment authoring parity, audio,
  build/snapshot management, and the bundled Blender conversion pipeline.
- Player portrait and team logo headers still use placeholders; the primary Assets and Builds routes
  also disclose that those surfaces are not built.

Asset support therefore needs a separate capability model keyed by observed archive/resource formats,
parallel to database `Capabilities`. Title selection alone is insufficient because extracted layouts
and individual resources vary within a game/platform.

## Defects corrected during this audit

1. NHL 08 and NHL 09 had verified PS3 anchors but were omitted from the frontend platform catalog.
2. Their PS3 product ids were absent from discovery, preventing install/save resolution.
3. PS3 save matching treated an empty regional product id as a universal prefix. Region matching now
   ignores empty ids, preventing unrelated saves from being classified as the selected game.
4. Cross-title profile tests sampled four anchors. The new matrix gate covers all 27 main DB anchors.

## Required release definition

The executable review, mutation, container, asset, and in-game verification program is defined in
[Full-game QA and editability plan](../plans/full-game-qa-editability.md).

Use three separate statuses per game/platform rather than one broad “supported” flag:

1. **Database view/edit:** matrix view-address gate + representative mutation/save/reload gate.
2. **Save write:** structural repack gate + in-game/emulator acceptance gate.
3. **Assets:** per resource family (portraits, logos, jerseys, equipment, heads, audio), each with
   discover/preview/export/replace/install gates.

No title should receive a blanket full-parity status while any required resource family is a
placeholder or its save path is experimental. That keeps “supported” from doing the sort of heroic
double duty normally reserved for fourth-line grinders.
