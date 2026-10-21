# Equipment and Goalie Gear

**Evidence:** measured: file-name containment of every stored equipment value on the Xbox 360 NHL 08 to Legacy and PS3 NHL 08 to Legacy trees or archive listings; typed vault joins on PS4 NHL 15 to 21 including the genuine NHL 20 and the NHL 19 patch; create-player thumbnail joins are candidates. Which item the game draws is open everywhere.

## Gen7 file grammar

Equipment files on Xbox 360 (`.rx2`) and PS3 (`.rpsgl`) share the pattern `<part>_<n1>[_<n2>[_<n3>]][_<kind>]`, with texture kinds `dm` diffuse, `tm` tint or team-colour mask, `nm` normal, `sm` specular, `cm` colour, plus `am`, `csm` and `rm` whose meanings the sources do not give, and no suffix for a mesh. `fe<part>` is the menu copy.

| Family | Id position | Notes |
|---|---|---|
| glove | n1 | n2 0 to 2 are three tint maps |
| helmet (with chinstrap, jaw protector, padding, visor) | n1 = helmet brand (visor: visor type) | |
| skate, player stick | `texlib_<id>` = brand | generic meshes |
| goalie stick, goalie pads | n1 | |
| blocker, trapper | **n2** (n1 is the mesh style 0 or 1) | |
| goalie helmet (mask) | n1 = shell 0 to 7, **n2 = paint id** | |
| jersey, pant, sock | n2 = team art id | see [Team Identity, Logos and Kits](Team-Identity-Logos-and-Kits) |
| puck | n1 | |

Through NHL 11 gloves, sticks, skates, pads, blockers and trappers are texture-keyed (`texlib_<id>` over generic meshes); from NHL 12 they are mesh-position keyed. **No registration table exists**: an id is valid by its files existing, and because meshes are shared per part, a new id needs textures only (a static claim; runtime open). Every stored database value has files, and the shipped catalogue is 1.5 to 4 times larger than the rosters use: on Legacy, 14 of 56 glove ids, 10 of 19 helmet brands, 8 of 33 skate brands, 19 of 75 sticks, 29 of 70 goalie pads and 173 of 280 mask paints are worn. Complete file sets per id on Legacy: gloves seven files (53 of 53 complete), goalie pads (57 of 67; ten lack the third tint map), blocker and trapper in both styles (49 of 69 and 57 of 67), goalie sticks (38 of 38), mask paints for shell 0 (280 of 280; other shells partial). PS3 and Xbox 360 per-id sets are equal (Legacy 851 of 853).

## Goalie gear chain by title

| Item | NHL 08, 09 | NHL 10 | NHL 11 | NHL 12 to Legacy | PS4 15 to 21 |
|---|---|---|---|---|---|
| mask shell | `masktype` → shells 0 to 2 (NHL 09 also 100 to 102) | shells 0 to 5 (99 has none) | n1 | n1 | `masktype` = `GoalieHelmetBrand` 0 to 7 (8 from NHL 19); **99** on four or five goalies of 19 to 21 is not a brand |
| mask paint | `maskcolor` (`maskpattern` always 0) | `maskid` → n2 | n2 | n2 | the `(maskid, masktype)` **pair** names one texture pack |
| team mask slots | | `exhibitionteams.goaliemask0..5_{starter,backup}`, every value a paint id | slots 0 to 6 | slots 0 to 6 | 0 to 6 (15 and 16); 0 to 8 declared on 17 to 21 with 7 and 8 always 0 |
| pads, blocker, trapper | `texlib_<id>` | `texlib` | `texlib` | id position; every blocker and trapper id exists in both styles | `GoalieBrands`; blocker, trapper and stick packs are left or right per brand |
| goalie stick | n1 | n1 | n1 | n1 | brand plus handedness (16 to 21) |

From NHL 10 the team slots are a second mask input; how a goalie's own mask and the team slot combine is open, and no column stores the glove side on either generation. The Studio's **Team goalie mask paints** card lists the slots and files, and its goalie type codes are a best guess ([Uniforms and Nameplates](Uniforms-and-Nameplates)).

## PS4 catalogues

Every skater and goalie field joins a typed vault enum by **type name** ([PS4 Attrib/VLT Vault](PS4-Attrib-VLT-Vault)): sticks, tape styles, gloves, helmets, visors, skates, goalie masks, goalie sticks and the shared goalie brands enum for blocker, trapper and pads. Mask texture packs number 506 (NHL 15) to 991 (NHL 21), at most one per (paint, brand) pair, each with a per-design colour map `goaliehelmet_<brand>_<design>_cm` and three brand-shared maps; geometry is keyed by brand only, with throat-protector records per brand. Rows that name no pack exist (NHL 16: ten; NHL 17: five; NHL 19 to 21: the type-99 goalies), and the two prospect teams' slot values 248 and 249 have no record on NHL 16 to 21.

The Creation Zone CSVs of NHL 17 to 21 list skater gear with id equal to render id, a `$` dictionary-key name and a thumbnail of the same number (`creationzone/equipment/<family>/<family><id>`): helmets 12 to 18 rows, gloves 17 to 44, skates 9 to 123, sticks 30 to 192, visors 6 or 7. Every stored skater value joins except **helmet 99** (1 row on NHL 17, 90 on NHL 21); the lists differ per title except visors, so never borrow another title's. NHL 15 and 16 have the catalogue but no lists; their thumbnails stay unjoined. The goalie mask list (`goaliemaskinfo.csv`) uses an ordinal asset id on NHL 17 and 18 and `brand × 1000 + paint` from NHL 19 (38 to 53 rows), with thumbnails decoding by that rule on 426 to 453 masks.

## Create-player thumbnails on gen7

The front-end ships picker thumbnails per family (`fe/ion/artassets/createplayer/<family>/<family>{N}.big`): on Legacy 106 sticks, 83 gloves, 38 helmets, 58 skates, 92 pads, 93 blockers, 91 trappers, 54 goalie sticks, 88 masks, 421 heads and 3 visors, growing from NHL 08's 14, 9, 6, 9, 19, 18, 18 and 39. The pickers are stable from NHL 10 to Legacy. Thumbnail-to-id joins are candidates for pads and blockers, plausible for trappers and not supported for gloves; Xbox 360 NHL 15 and Legacy ship byte-identical front-end vault files yet different picker counts, so where the per-title lists live is open.

## What Studio offers

The Hockey Shop authors slot textures and installs them at an equipment id, which on gen7 means creating or overwriting the id's files; the player's Equipment tab changes which id is worn and checks the id's files or catalogue record ([Hockey Shop](Hockey-Shop), [Players](Players)).

Last reviewed: 2026-10-20
