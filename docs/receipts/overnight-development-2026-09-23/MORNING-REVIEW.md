# Morning review — developer lane, R4 continuation of 2026-09-23

Everything below is developer-verified only (cargo / jsdom evidence). Nothing was launched, nothing
was written to a live install, and **no card here claims the game accepted anything**. The cards
of the 19–22 September campaigns remain valid except where a revision is named here (V-31 and
R32-XB2-1 are revised in the 22 September file). NHL 21 E1 stays closed.

## Runtime review (a console / emulator run — the only way these close)

| Card | Title | What to do | Expected | Failure means | Restore | Receipt |
|---|---|---|---|---|---|---|
| **R32-XB2-1 rev. 3** | NHL Legacy Xbox 360 (copied install) | see the 22 Sep file: the changed-size step applies to single-level rasters only now; a mipped raster refuses before any write | — | — | — | 01 |
| **R32-XB2-3** (new) | Any Xbox 360 title (copied install) | Replace… a same-size jersey texture (a mipped raster), repack / install by your own step, view the jersey close up and at gameplay distance | close up: the new image (base and large levels). At distance: **whatever the shipped packed-tail levels render** — the previous image's content is what the file carries there; Studio does not predict what the console shows | a wrong base at close range = a Studio defect; the old image at distance = the disclosed limitation, not a defect | the `.bak` beside the file | 01 |

## Desktop review (no game)

| Card | Title | What to do | Expected | Restore | Receipt |
|---|---|---|---|---|---|
| V-36 | Any Xbox 360 title (root-folder project open), a **mipped** jersey raster | Replace… with a different-size image | before Write: a red "This write would be refused" note naming the declared level count and "Nothing was written"; the Write button disabled; the file untouched | — | 01 |
| V-37 | Same raster | Replace… with a same-size image | before Write: an amber "Partial replacement" note (base + N large levels rewritten; M packed-tail levels keep the shipped bytes; "not a full-image replacement"); after Write: the same note on the result card; `.bak` beside the file | the `.bak` | 01 |
| V-38 | NHL 17–21 PS4 project with the extracted install attached, **no dictionary attached** | Team → Selectable songs | every title shown as its `$TXT_SONG_…` key in amber monospace; the line "No dictionary is attached to this project"; genre pills show their keys | — | 02 |
| V-39 | The same, after **Attach dictionary** with the title's own `eng_us.db` | the same card | titles read as text ("Alien Dance"), genres as text ("Rock"), the line names `eng_us` and the row count; a title the dictionary lacks stays as its key with the tooltip "not in this project's attached dictionary"; the filter matches the resolved text. Detach → keys again | — | 02 |
| V-40 | NHL 17–21 PS4 project, Arena → Venue parameters, goal horn row | with / without the dictionary attached | the horn reads as the club's text (green) when the key resolves; as the `$NHLTeamName_…` key (amber) otherwise, tooltip naming the state | — | 02 |
| V-41 | PS3 NHL 12 or NHL 13 project with the `USRDIR` attached as the install root | Team → Uniforms → Kit textures | directories read "N members in cacherender.big + nocacherender.big"; every component of a stock kit is **present** with the member path (`cacherender.big:rendering/jersey/…rpsgl`); NHL 12 lists both sock files; the note says the members are listed from the file table | — | 03 |
| V-42 | The same | The same card, wait for the thumbnails | each archived component shows a decoded thumbnail (a `big` marker only while decoding, or with the decode error in its tooltip); the tooltip names the archive member it was decoded from; nothing offers a write | — | 04 |
| V-43 | Any Xbox 360 title, a mipped jersey raster | Replace… with a same-size image; watch the dialog before pressing anything | "Checking what this write would do…" first, with Write **disabled**; then the amber partial-replacement note and Write enabled. Cancel, pick the same image again: the check runs again (not cached). Break the check (e.g. detach the tree mid-dialog) → "The pre-write check failed…", Write disabled, **Retry check** and Cancel available | — | 05 |
| V-44 | NHL 15–21 PS4 project with the extracted install attached, Arena page → Arena assets | Honda Center (art id 0) and any other arena | Honda Center: "Venue by vault level name · several levels share this id — none chosen", both `anaheim` and `common_ice_int` listed with folder present / missing and their texpack keys; another arena: "one level, one folder" with its `stadiums/<name>` and file count; "Shared / unreferenced content: common …" once; "Re-pointing is not offered: … cascade …"; no re-point control anywhere. NHL 15: Carolina Arena reads "level without a shipped folder" | — | 06 |

## Notes for the user

- The Xbox 360 same-size replacement is now honestly a **partial** one: the small "packed tail"
  levels keep whatever EA shipped, because nobody has established where the console keeps them.
  Until the research lane settles that layout, a resized image cannot be written into a mipped
  Xbox 360 raster at all — Studio refuses instead of guessing.
- Song titles, genres and the goal-horn name now read as text only through the dictionary you
  attach to that project; nothing is borrowed from another title, and an unresolved key is shown as
  the key on purpose.
- PS3 NHL 12 / 13 kits are read from the two render archives: presence by name, thumbnails decoded
  from the members on demand. Nothing can be written back into those archives (no encoder for
  their compression), so the card stays read-only there.
