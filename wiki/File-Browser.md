# File Browser

**Assets › File Browser** walks the extracted install tree file by file. It exports anything you pick, meshes as glTF and portraits and textures as images, keeping the game's own folder layout, and it can write an edited image back into a portrait or texture in place. It needs an install folder; it does not need the "extracted" gate because it browses whatever loose tree is there.

## Browsing

A breadcrumb from **install** through each folder; **↵ ..** goes up. Rows have a checkbox, an icon or thumbnail (**Show thumbnails** applies to replaceable files), the name, a kind pill (**mesh → glb**, **portrait → png** or **texture**), **Replace…** on portraits and textures, and the size. **Select everything in this folder** ticks every row.

## Export

Checkboxes: **Meshes → .glb**, **Portraits → .png**, **Textures → .png** ("RenderWare texlibs and PS4 GNF maps decode to .png"), **Crop portraits to the subject** (off) and **Copy unrecognized files as-is** (off: include files the exporter cannot decode, or fall back to a raw copy when a decode fails). **Export N selected…** asks for a folder. The **Last export** card reports meshes, portraits, textures and copies, plus skipped files.

## Replace…

The loose-file replace dialog, which the Teams, Leagues and League Imagery pages use as well:

1. Pick the edited image (png, jpg, bmp or tga).
2. **Replace texture** shows **Current** and **New** with their sizes.
   - Sizes match: "A one-time .bak of the original is kept before the first write."
   - Size mismatch on a container that cannot be resized: "this <container> must be replaced at its exact size (W×H)."
   - Resizable container: "It will be re-encoded to W×H on write", with a note per container type. Portrait and league-logo packages resize on Xbox 360 NHL 12 to Legacy (XPR2 at 64 or 256 wide) and PS3 NHL 12 to Legacy (DDS); RW4 packages of NHL 07 to 11 are same-size only. Texlibs resize on Xbox 360 NHL 09 to Legacy and PS3 for DXT rasters of 128 texels or more, single-level only on Xbox 360; tiny, linear, cube, uncompressed and NHL 07 and 08 rasters are refused. PS4 GNF recomputes its header; cube, BC6H and volume textures are refused. Every note ends "Whether the game accepts a resized … is a runtime question."
3. A **pre-write check** runs first ("Checking what this write would do…"); nothing is written until it answers, and it may refuse ("This write would be refused: …") or disclose a partial replacement, for example a mipped raster whose lower levels are kept.
4. **Write texture** writes the file in place, keeping a one-time `.bak`. The result card names the file, container, size and backup, and reminds you: "Project changes still need deployment; loose-file edits may need repacking before the game reads them."

Texture formats and their constraints are documented on [Textures](Textures); the archives and repacking question is covered in [Archives and Compression](Archives-and-Compression).

Last reviewed: 2026-10-20
