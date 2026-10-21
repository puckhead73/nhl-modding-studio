# Uniforms and Nameplates

The **Uniforms** tab of a team ([Teams](Teams)) shows the team's kits on a 3D model, lets you edit the jersey registry, replace the kit's textures, paint helmet and glove zones, and lay out the name, number and patches on the nameplate sheet. It needs the game's assets extracted to show textures; without an install tree the registry is still editable.

## Kit viewer

The left pane renders the selected kit on the shared player rig. Controls: **Head / equipment** (the top-overall player by default, or any roster player), **Textured** and **Wireframe** toggles, and a **Pieces** list with **All** and **None**; clicking a piece name zooms to it. Left-drag rotates, middle-drag moves, scroll zooms. Where no 3D kit is registered the textures are still listed below with the note that their link to a kit has not been verified.

On PS4 a checkbox **Show ambiguous filename matches** reveals files that may belong to several teams, with the warning to inspect a texture before replacing it.

## Kit details

- **Kit** dropdown: every kit found for the team, named like "Reebok / Modern · Home · default (style 1 · var 0)", plus unregistered "Other textures".
- **+ Add New Jersey** clones the selected kit into a new registry row. The dialog takes the style, team art id and variant that make the new identity, shows the new asset id, and optionally clones the kit's texture files onto the new id (needs an install tree; refused when the id lengths differ because the containers name files in fixed-length slots). The row is staged as a pending, undoable edit.
- Jersey metadata: **Asset ID** (read-only); **Style** and the art id, variant and front-end variant (locked, they are the asset id); editable **Index**, **Helmet Logo**, **Light Color** (home dark or away light), **Loc String** (Home, Away, Alt, the 2006-07 variants, Vintage) and the flags **Helmet #s**, **Tie / Laces**, **Is Default**, **Is Lockable**.
- **In-game jersey select preview**: the thumbnail the game shows in its jersey picker, with **Replace…** where the kit has its own, or a note that the kit falls back to the shared custom art, which cannot be overwritten from here.

## Kit assignments

A measured view of which texture files each registry row resolves to: a badge for how the title's selector works ("measured · two sock files per kit", "no selector table", "not measured"), the selected row's identity and flags, and each component as present, missing or unknown with **Replace…** or **Replace in archive…**. Spare kits no row points at are listed separately. Re-pointing a row at another kit is not offered; the reason is stated.

## Parts: Jersey, Helmet, Gloves, Pants, Socks

Choosing a part frames the 3D view on it and lists its textures. Each texture card has **Replace…** ("Preview and write an edited image into this texture (keeps a .bak)"); click a texture to zoom. In a project, replacements are staged for deployment.

- **Helmet** adds **Team goalie mask paints**: the starter's and backup's paint numbers per slot, with file counts. "Which uniform uses which slot is not established."
- **Gloves** shows trapper and blocker instead when the previewed player is a goalie; **Pants** shows pads.
- The **equipment editor** (helmet and gloves) edits **Equipment Info** (type, brand, era, variant) and **Zone Colors**: up to nine named zones (gloves: main, middle, piping, cuff, wrist, thumb; helmet: main, strap, logo) as RGB values written as one undoable step. Goalie type codes are a best guess (see [Known Limitations](Known-Limitations)).

## Nameplate editor

**Edit Jersey Layout** opens the editor for the kit's name, number and patch sheet. Select a zone by clicking it or from the list, drag to move, drag the edge handles to reshape, nudge with the arrow keys (Shift for 10 px) and resize with + and −. Set each zone's type: **Name**, **Number (back)**, **Sleeve — right**, **Sleeve — left**, **Captain patch (C/A)**, **Stanley Cup patch** or **Ignore**.

- **3D preview** renders the name and number live through the kit's font map on the jersey model as you move zones, with a **Preview player** picker. Without a font map the preview shows zones only. "Reference alignment and the scale of letters and numbers are approximate — verify in-game."
- **Baseline** can be curved (arch or diagonal) by dragging three magenta dots. This is experimental and preview-only: **Save does not write a curved baseline back to the texture.**
- **Save to texture** bakes the zones onto a fresh sheet, runs the pre-write check, and **Write texture** writes it with a one-time `.bak`.

The jersey font layout behind the preview is a measured atlas of the font rasters inside each kit's texture library; see [Team Identity, Logos and Kits](Team-Identity-Logos-and-Kits) and [Textures](Textures).

Last reviewed: 2026-10-20
