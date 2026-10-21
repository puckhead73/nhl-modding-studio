# Asset Export

**Assets › Export** writes player art out of the game as ordinary files, named by player, and builds the appearance catalogue the [Appearance Studio](Appearance-Studio) reads. It is export only; nothing in the game changes. It shares the Assets gate described under [Player Art](Player-Art).

## All head meshes: appearance catalog

"Every head, hairstyle and facial-hair mesh — players, generics, referees, coaches and trainers — with a catalog.json the Appearance Studio reads to mix any combination." The checkbox **Head and hair textures (.dds)** (on by default) includes the player, create-a-player and staff maps, exporting the base mip without recompressing. **Build appearance catalog…** asks for a folder and writes:

- `heads/` with one GLB per mesh, named by base, class (head, hair, facial hair) and game;
- `textures/` with one `.dds` per map;
- `catalog.json`.

The result shows counts of heads, hair, facial hair and textures, the total size, and any skipped or failed items.

## Export player assets

"Portraits, head meshes and head textures, named by the player they belong to."

**Which players**: **Everything exportable** (every player with art and a name to file it under), **Players with a portrait** (the portrait library) or **Players with their own head model** (the head-mesh library).

**What to write**: **Portraits (.png)**, **Head meshes (.glb)** (geometry and UVs), **Head textures (.dds)** (one file per map), **Original game files alongside** ("Archives the untouched source bytes, so later compatibility work never has to re-extract"), and **Crop portraits to the subject** (off by default: "EA ships portraits on a mostly-empty 512×512 canvas. Leave off to keep them exactly as shipped.").

**Export N players…** asks for a folder and writes `portraits/<slug>.png`, `heads/<slug>.glb`, `textures/<slug>_<map>.dds` and, with originals enabled, the source files beside them. The slug is `lastname_firstname_dd_mm_yyyy`. Generic placeholder portraits and shared heads are skipped, as are players without a birth date; the helper line counts them.

The **Last export** card reports portraits, meshes, textures and original files written, the size, placeholders skipped, and any failures.

Last reviewed: 2026-10-20
