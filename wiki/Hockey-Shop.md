# Hockey Shop

The **Hockey Shop** section is a library for equipment textures. "Browse the equipment the install ships for each slot and preview it in 3D. The store on the right is a library of textures you can build up and install into the game. Editing a player's gear happens on their Equipment tab — the shop reads and authors the render files, never the database."

It needs a project with an install tree: "Open a root-folder project (not a single database file) so the shop can scan the game's rendering folder for equipment."

## Layout

- **Category**: **Skater Equipment** (slots **Stick**, **Gloves**, **Helmet**, **Skates**) or **Goalie Equipment** (**Goalie Stick**, **Blocker**, **Trapper**, **Pads**, **Goalie Helmet**).
- **Left**: the game's items for the slot, by id and model name, searchable. Click one to preview it.
- **Centre**: the 3D preview with **Textured** and **Wireframe** toggles and a **Pieces** list, then the item's textures. When a store entry is selected, the centre shows the slot's model wearing that texture instead.
- **Right**: the **Store** for this slot.

## The store

- **Sync game** copies a texture for every shipped item in the slot into your store, credited to the game ("Extracted N game item(s) into the store.").
- **Add PNG…** imports your own texture; the **Add to store** dialog asks for a **Name** and an **Author**, both required, and notes "It's saved to the store only — installing it into the game is a separate step."
- Click an entry to preview it. Entries already installed in the game are tinted green. ✕ removes an entry from the store immediately, without confirmation.
- **Install…** opens the install dialog.

The store lives in Studio's application data, not in the project, so it is shared across projects of the same game.

## Installing a texture

**Install “<name>” — <slot>**: choose the **equipment id** to install at (the range depends on the slot, up to 127, 255 or 511; it defaults to the entry's original id). The live plan shows the file naming rule, the mode (**overwrite**, replacing an existing item, or **create**, cloning a donor id's files), the target file and texture index, and any warnings (⚠) or blockers (✕). **Install to game** is enabled only with no blockers and applies only if nothing changed since the plan you reviewed. It writes render files only, requests a backup and never touches the database.

To put the item on a player, set that slot on the player's Equipment tab ([Players](Players)). The equipment id grammar per title is documented on [Equipment and Goalie Gear](Equipment-and-Goalie-Gear).

Last reviewed: 2026-10-20
