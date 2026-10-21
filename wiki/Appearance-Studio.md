# Appearance Studio

**Assets › Appearance** browses a built appearance catalogue: every head, hairstyle and facial-hair mesh a game ships, for players and staff alike, with a 3D preview and the textures that belong to each. In the current beta it is a browser; its header describes it as the base for a future mix-and-match editor. It needs no project install, only a catalogue folder built by [Asset Export](Asset-Export).

## Loading a catalogue

**Choose catalog folder…** picks "a built appearance-catalog folder (contains catalog.json)". The last folder is remembered. Without one: "Build one first under Assets → Export → “Build appearance catalog”." The header shows counts of heads, hair and facial hair and the game the catalogue came from.

## Browsing

The **Catalog** column lists assets with a colour dot for the class (head, hair, facial hair), the name, and the origin for anything that is not a player. Search by name; filter by **Type** (All, Head, Hair, Facial hair) and **Source** (All plus every origin found: player, generic, retired, coach, referee, trainer and so on). The first match is selected automatically.

The detail card shows pills for origin, class, submesh count and texture count, the head-mesh viewer restricted to this asset's submeshes (**Textured**, **Wireframe**, a submesh list with **All** and **None**), a texture strip, and **Details** (mesh file, submeshes, source).

Nothing is written. Head-mesh conversion between games is being completed outside Studio; see [Known Limitations](Known-Limitations).

Last reviewed: 2026-10-20
