# Game Settings

**Game Database › Game Settings** shows the catalogue of mode-setup settings the game stores: the rules a new league or franchise starts from, their ranges, defaults and the screens that use them. On most titles it is read-only. On NHL 15 PlayStation 4, and on NHL 16 to 21 through their CSV members, a stored default can be changed one cell at a time.

The page is hidden in the Save Editor.

## Where the catalogue comes from

The source line says which: "Read from the open database's five catalogue tables" (NHL 15 PS4) or "Read from N setting_csv members of …" (NHL 16 to 21). Pills say whether labels come via the dictionary or are shown as keys, and whether stored defaults are editable ("one cell at a time", or "pending until a copied-target save" for CSV members). On titles with no catalogue the page says "No settings catalogue for this target" with the reason; see [Schema Drift Across Titles](Schema-Drift-Across-Titles) for which titles have one.

## The table

Filter between **league & competition settings** and **all N settings**, and search by setting, role or id. Columns: id, setting, role, range, global default, screens and whether the setting is in the stored-default contract. Pills flag null cells, settings stored twice (equal or differing) and excluded rows.

## The detail

Select a row to see its identity, code range, size, the **Global stored default**, the **Stored defaults per screen** (with enabled and visible flags; a screen name taken from another title's numbering is marked "lineage"), the **Choice codes** with their labels where a label array is bound, and any stock anomalies, which are preserved rather than corrected.

### Edit a stored default

For eligible rows: choose the **cell** (the global default or one screen), enter the **new code** (from a labelled list where one exists), **Preview**, then **Apply** or **Discard**. On NHL 15 PS4 the edit is a database cell and applies immediately as a pending edit. On NHL 16 to 21 "the edit rewrites one cell of a CSV text member inside the archive; it stays pending until the next copied-target save re-compresses and places the member" (see [Project Overview and Building](Project-Overview-and-Building)).

Whether a game screen offers a stored default, and which copy it reads, has not been established; the catalogue is data the game ships, and the page says so in its evidence notes (**show evidence notes**).

Last reviewed: 2026-10-20
