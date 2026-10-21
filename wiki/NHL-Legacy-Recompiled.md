# NHL Legacy Recompiled

**Evidence:** measured on one registered Recompiled install (a June 2026 build), by static read-only comparison with the Xbox 360 NHL Legacy files: whole-archive identity on 20 shared paths, sampled member identity per asset class, and the game's own log text. Nothing was run; which copy of an asset the recompiled program loads is open.

The NHL Legacy Recompilation runs the Xbox 360 release of NHL Legacy on a PC. For modding purposes its game data **is** the Xbox 360 data: the archives are the same bytes. What differs is where the game keeps saves and caches, and that nothing is shipped loose.

## What is identical

| Fact | Label |
|---|---|
| The install's `game/` folder shares 20 paths with the Xbox 360 Legacy tree: the 13 top-level `.big` archives (`audioboot`, `audioboot2`, `boot`, `cache`, `cacheaudio`, `cacheboot`, `cachemovie`, `cacheoverlay`, `cacherender`, `data0`, `nocache`, `nocacherender`, `renderboot`), the four `gamedata/` archives, `default.xex`, `options.viv` and `nxeart`. All 20 are byte-identical. | measured, this install |
| The main database is the stored member `db/nhlng.db` inside `cacheboot.big`, byte-equal to the Xbox 360 and PS3 Legacy database. The dictionaries (8 locales) and auxiliary databases sit beside it in the same archive. | measured |
| All 44 602 paths of the Xbox 360 Legacy extracted tree match exactly one member path in the Recompiled archives. | measured as a path match; bytes compared on samples per class, and on all 67 venue packages |
| The Xbox 360 tree's loose folders (`rendering/`, `db/`, `fe/`, `audio/`) have no counterpart: the Recompilation keeps everything inside its archives. | measured |

Because the archive bytes are the same, every codec, framing and decoder fact measured on Xbox 360 Legacy archive members ([Archives and Compression](Archives-and-Compression)) applies to the Recompilation's members. Studio treats NHL Legacy and the Recompilation as one title on two runtimes, so database and dictionary edits port between them. Byte identity proves the data is the same; it does not say which copy the program loads or whether it accepts a changed archive.

## Where things live

| Item | Convention | Label |
|---|---|---|
| Game data | `<install>/game/` with the 13 archives and `gamedata/`. Studio accepts either the install folder or its `game/` subfolder as the root. | measured |
| `_compiled` | A **user-supplied convention** for a loose extraction beside the game data (`_compiled/{db,fe,rendering,…}`). Absent in the inspected install; seen once elsewhere as a user-made extraction. The game is not shown to produce or require it. | measured as present or absent; convention only |
| Runtime drives | The game's logs mount `game:` on the install's `game` folder and resolve `cache:` to a user-data folder under Documents (`Documents/nhllegacy/cache`). | measured from log text |
| User-data folder | Saves, `Headers/`, a `cache/guest/` folder (version marker, ad cache, highlights) and `cache/shaders/`. A copied install still reads and writes this same folder. | measured |
| Saves | `Documents/nhllegacy/<profile id>/454109EC/00000001/<TYPE timestamp>/<TYPE timestamp>` plus a 328-byte header sidecar under `…/454109EC/Headers/`. The payloads are the raw inner saves (a ROSTER begins `RosterFile`, same size and layout as the Xbox 360 Legacy roster); see [Save Containers for Xbox 360 and PS3](Save-Containers-Xbox-360-and-PS3). DYNASTY saves carry container version 5 and are read-only. | measured |

## Which archive holds what

| Asset class | Member path | Archive | Wrapper |
|---|---|---|---|
| Databases, dictionaries | `db/*.db`, `fe/loc/*.db` | `cacheboot.big` | stored |
| Sound banks (`.abk`) | `audio/sfx/*` | `nocache.big` (206), `audioboot2.big` (10), `audioboot.big` (1) | stored |
| Streams (`.ast`) | | `nocache.big` (7), `cacheaudio.big` (7) | |
| Speech and context | `audio/speech/*`, `audio/context/*` | `audioboot.big` (9), `nocache.big` (21) | 20 chunklzx, 10 stored |
| Large and small portraits | `fe/ion/artassets/playerheads[small]/<bucket>/p<N>.big` | `nocache.big` / `cache.big` | stored BIGF packages |
| Head bundles and head textures | `rendering/player/player_<N>.rx2`, `texlib_<N>.rx2` | `cacherender.big` | chunklzx |
| Venue packages, jumbotron, ice surface, banners | `rendering/stadium/*`, `jumbotron/*`, `icesurface/*`, `banner/*` | `cacherender.big` (two ice layers in `renderboot.big`) | chunklzx |
| Equipment (nine slot folders) | `rendering/<slot>/*` | textures in `cacherender.big`; shared meshes in `renderboot.big` | chunklzx |
| Team crests, league and calendar logos | `fe/ion/artassets/teamlogos/t<artid>.big` and siblings | `cache.big` | stored |
| Rink logos | `rendering/logo/texlib_<artid>_0_0.rx2` | `cacherender.big` | chunklzx |
| Jerseys and nameplates; pants; socks | `rendering/jersey/*`, `pant/*`, `sock/*` | `nocacherender.big` and `cacherender.big`, shared meshes in `renderboot.big` | chunklzx |

No duplicate paths exist within or across the archives. Two `gamedata/` archives are not listed by Studio: `gps.big` (a classic BIG4 with 15 settings-like entries) and `faceposer.big` (an EB variant naming 904 `faceposer_<N>.cba` files whose row layout is open); neither holds portrait, head or arena art by name.

## The loose-override question

The game's logs record *failed* file opens only. During play it requests individual loose asset paths under `cache:\` (`cache:\db\nhlng.db`, `cache:\audio\sfx\sfx.ast`, `cache:\rendering\…\*.rx2`, `cache:\fe\ion\artassets\…`), plus `cache:\<archive>.big` and `update:\patch.big`; no `game:\` path other than the executable appears.

| Question | Status |
|---|---|
| Does a loose file at the `cache:` path load, and does it beat the archive member? | open |
| Is any loose path under the game folder (including `_compiled`) ever read? Successful opens are not logged, so absence is not proof. | open |
| Do loose files survive the runtime's own cache writes? | open |
| Lookup order between loose, cache and archive | withdrawn: failed-open logs cannot show order |

Consequence for Studio: loose equipment installs on the Recompilation are refused, and board, arena, kit and nameplate writes are withheld, until an in-game test with an unmistakable visible replacement settles the question. The package installer, by contrast, places Legacy loose textures under `_compiled/` for a Recompiled recipient on synthetic fixtures; the two positions are recorded side by side, not reconciled.

## What Studio reads today

Working on the Recompilation: opening and editing the main database after extraction; sound banks and streams through an archive-member route; portrait and head reassignment with an existence check by member path; the team and league logo table and its replacement through `cache.big`. Routes that expect loose files (speech catalogue, portrait and head previews, equipment previews, kits, arena and venue art, the team header logo) have read contracts written but not yet implemented, and show "not measured" or a placeholder.

## Open

Loader precedence; acceptance of a modified archive (a `chunklzx` re-wrap is needed for `rendering/` members); save compatibility and roster limits beyond the sidecar layout; other Recompiled builds and their `_compiled` conventions; the `faceposer.big` row layout.

Last reviewed: 2026-10-20
