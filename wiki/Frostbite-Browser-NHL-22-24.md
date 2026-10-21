# Frostbite Browser (NHL 22 to 24)

NHL 22, 23 and 24 on PlayStation 4 use EA's Frostbite engine rather than the TDB databases and `.big` archives of the earlier titles. Studio cannot edit them. Choosing one of them in the project wizard opens a standalone, read-only **Frostbite assets · NHL 22–24 PS4** browser that lists the game's bundles and chunks and exports raw or stored assets. It works without a project or a roster database, and nothing it does touches the game.

## Opening a game

Enter or **Browse** to an extracted PS4 game folder that contains `Data/layout.toc`, then **Open assets**. The reader supports the PS4 layout only. Warnings after opening may say that a superbundle's table of contents is missing, or "Patch base does not match Data head; do not combine these installs". A note is always shown: "Read-only catalogue: roster editing and deployment are not enabled. Data and Patch are shown separately."

## Browsing

- **Content layer**: **Data** and, where present, **Patch**.
- **Superbundle**: the superbundles of that layer.
- **Entry kind**: **Bundles** (search decoded bundle names) or **TOC chunks** (search chunk GUIDs).

In bundle mode, picking a bundle lists its assets with kind, logical size, type, content hash and where the stored bytes sit, 100 per page. In chunk mode each chunk card shows its GUID and location.

## Exporting

- **Export raw** saves the stored span with its framing preserved (default name `asset-N.casblocks`).
- **Decode stored blocks** saves the decoded payload (default name `asset-N.bin`), which works only for uncompressed blocks; compressed data reports "codec … requires a decoder; raw export remains available". Most PS4 payloads are compressed.

Exports must be saved outside the source install.

## Not supported

Roster editing, deployment, decoding compressed blocks, combining the Data and Patch layers, texture or 3D previews, and any layout other than the PS4 one. See [Frostbite: NHL 22 to 24](Frostbite-NHL-22-24) for what is known about the file structure.

Last reviewed: 2026-10-20
