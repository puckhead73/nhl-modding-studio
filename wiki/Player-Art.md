# Player Art

**Assets › Player Art** shows, for every player, what art the installed game actually ships for them, the portrait and the head model, and lets you point a player at a different shipped portrait, head or (on PS4) hair style. It reads the extracted install; it writes only database fields.

## The gate

The Assets pages need an install folder and extracted art. Without one: "No install folder for this project" with a hint to choose a game source folder under Project Overview. With a folder but no extracted art: "No extracted assets found at the selected source", naming the rendering and front-end folders it expected and reminding you that "An ISO storage folder does not contain extracted assets." The **Game source folders** panel follows so you can fix the link in place.

## The list

Filter chips with live counts: **All**, **Portrait** (a front-end portrait ships for this player), **Own head** (a head model authored for this player alone) and **Exportable** (the default: has art, and a name and birth date to build an export filename from). Search by name or export name. Columns **Player**, **Portrait** (✓ or —), **Head** (✓ for an own head, "shared" for a pool or generic head, —) and **Export name**, the slug built as `lastname_firstname_dd_mm_yyyy`, or "no birthdate".

## Portrait & head containment

With nothing selected, a read-only card summarises the whole roster: own heads (head id equals art id) with and without files, pool heads, how the stored "has portrait" flag compares with the files on disk, and shipped files no row references. Residual lines name players with an own head but no head file, an own head but no portrait, or a portrait flag but no file. The method badge says "gen7 file-name rules", "PS4 portrait rule · heads not measured" or "not measured".

## A player

The card shows the portrait (uncropped, on a checkerboard), pills for the head class and texture count, and notes where a shared head is not exported under the player's name or where a missing birth date prevents an export name.

**Containment** gives the art id, head id and class, whether the portrait and head files are present, and the stored flag. Then the reassign rows:

- **Reassign portrait (<id>) to**, **Reassign head (<id>) to**, and on PS4 NHL 17 to 21 **Reassign hair style (…) to**: enter an id and press **Check**. The plan lists refusals in red, "ships: <file>" evidence in green and notes in grey, with a disclosure of the exact source it checked.
- **Apply** is enabled only after an allowed check for the same id. "<Portrait> set to <id> — staged as one edit; review it under Pending changes, undo it, or Save." Apply stages one ordinary database field edit and writes no files; a stale plan is refused.

**Head Mesh** (own heads only) renders the head with **Textured** and **Wireframe** toggles, the face textures and a submesh list to toggle parts. **Asset details** lists the portrait and head ids with their field names, the bio record and the resolved files.

How ids map to files per title (the portrait id and head id fields, bespoke heads where the two are equal, pool heads above a floor) is documented on [Player Heads, Portraits and Appearance Binding](Player-Heads-Portraits-and-Appearance-Binding).

Last reviewed: 2026-10-20
