# Receipt 05 — Phase 1.4 league logos; Phase 1.5 audio disposition

> **R2 correction (18 September, F3):** 1.4 has one real Legacy XPR2 replacement round trip; RW4 (Xbox 09–11), PS3 DDS and PS4 GNF league-package replaces are unexercised or unimplemented, archive/disc placement of a replaced logo is not built, and non-logo league imagery is not inventoried. 1.5's finding supports "no separately named league bank found on the Xbox titles", not absence of league-related audio (generic banks unopened; PS3/PS4 unknown), and closes nothing. See the [Phase 1 coverage matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md) §1.4/§1.5 (packages P1-E/F/G/H).

Overnight developer lane, 18 September 2026. Consumes research handoffs P1.4-X
(`14-phase1-league-logo-assets.md`, `data/xbox360-league-logo-map.json`) and A-X
(`15-audio-banks-xbox360.md`, `data/xbox360-assets-audio.csv`). Status: **implementation
complete / automated verified / user review pending** (1.4); **evidence-backed absence recorded**
(1.5). Nothing launched; the only real-file writes were to temp copies.

## Plain-language summary

**Logos (1.4).** A league's logo is not addressed by its code: each game maps the code to a
family-wide *art index* (a small table in the Xbox executable; a database column on PS4) and the
frontend loads four package files for that index (`leaguelogos`, `…big`, `…small`, `…wide`).
Studio now knows those tables for NHL 09 → Legacy, shows on the League page which index a league
uses, how well that is established (traced on the Xbox executable, by reference on PS3, native on
PS4, inferred for NHL 07/08), which other leagues share the same picture (the NHL shield also
serves Winter Classic, national squads, Rest Of World and HUT on Legacy), and lists the four
package files with thumbnails and a Replace action through the same export → edit → reimport flow
the team tools use. The one codec gap this exposed — the logo packages are uncompressed 8_8_8_8
textures, which the replace path refused — is closed; a real Norway package on the X: Legacy tree
was decoded (64×64 / 256×256) and round-tripped through a temp copy.

**Audio (1.5).** Research searched every `audio/**` member of every archive of the ten Xbox titles:
no league-keyed audio exists. League-related presentation sounds, if any, sit inside the generic
frontend/SFX banks. Recorded as evidence-backed absence per Xbox title; the PS3 twins carry the
same archive families (by identity, not re-searched); PS4 is unknown. No Studio operation is
claimed or built for 1.5.

## Implementation (1.4)

- `defs/leagues/logos/{nhl09,nhl10,nhl11,nhl12,nhl13,nhl14,nhl15,nhllegacy}.toml`: code → logo
  index, witness = the title's Xbox executable table (VA recorded). NHL 07/08: no table found;
  index = code by count (`inferred`).
- `crates/studio-domain/src/league_logos.rs`: `LogoTable` loading (`defs/leagues/logos`),
  `league_logo(catalog, tables, game, platform, code, install_root)` → art index + evidence +
  shared codes + variants (`fe/ion/artassets/<family>/l<idx>.big`; PS4
  `nhl/ui/kettle_assets/leaguelogos/size{64,256,512}/l<idx>[_fr].gnf`) with loose-file existence.
- `LeagueCatalog` now lists enum-only codes (CHL, United States, …) with zero members so they can
  be browsed, given a logo and chosen as a membership target.
- `assets::replace_portrait_texture`: uncompressed 8_8_8_8 XPR2 slots are now replaced (RGBA →
  big-endian `[A,R,G,B]` linear → tiled via `tdb-rx2`'s existing encoder; DXT 4×4 guard applies
  to DXT only).
- `studio-api`: `Registry::league_logo`, command `league_logo`, `LeagueLogoDto` (95 export roots).
- App: `LeagueLogoCard` on the League page (thumbnails via `texture_preview`, Replace via
  `useTextureReplace` → `import_texture` with backup).

## Per-target disposition

| Target | Index source | Packages | Replace |
|---|---|---|---|
| NHL 09–15, Legacy — Xbox 360 | **traced** (executable table) | loose under the X: extraction trees; disc ships them inside `cache.big`/`fe.big`/`data0.big` | XPR2 8_8_8_8 / DXT5 on 12–Legacy through the portrait chain (real Legacy round-trip); RW4 on 09–11 **not verified** (texlib path exists, not exercised on these members) |
| NHL 09–15, Legacy — PS3; Recompiled | **by-reference** (same file names) | PS3 DDS packages (loose + `cache.big`) | DDS path exists (portrait chain); not exercised on a league package |
| NHL 15–21 PS4 | **native** (`stockleagues.artid`) | `kettle_assets/leaguelogos/size*/l<idx>.gnf` inside `inxp_10.big`; extracted copies addressed | GNF same-size replace exists; not exercised on a league file |
| NHL 07/08 | **inferred** (index = code) | tiny sets (07: l0,l1; 08: l0–l5 + lea) | as 09–11 (RW4), unverified |

Not claimed: which screen draws which family; archive-vs-loose precedence at runtime (the disc
route would need a BIGF repack + a same-length RefPack stream or an archive re-plan — the range
patch route of receipt 02 does not apply to a logo whose RefPack length changes); adding a new
logo index (executable change).

## Checks

| Check | Result |
|---|---|
| `cargo test -p studio-domain --test league_logos` | 2 passed, 1 ignored |
| `… --test league_logos real_tree -- --ignored` with `NHL_STUDIO_XBOX_TREE=X:\…\NHL Legacy\NHL Legacy` (read-only on the tree) | ok — four Norway packages exist; `leaguelogos` decodes 64×64, `leaguelogosbig` 256×256; replace round-trip on a temp copy → 935-byte package, decodes back to the written colour; the tree file's mtime unchanged |
| `cargo test -p studio-domain --lib assets` | 55 passed (DXT/DDS/GNF paths unchanged) |
| `cargo test -p studio-domain --test leagues --test membership --test schedules` | 11 + 4 + 2 passed |
| `tsc --noEmit` | clean |

## Changed paths

New: `defs/leagues/logos/*.toml` (8), `crates/studio-domain/src/league_logos.rs`,
`crates/studio-domain/tests/league_logos.rs`, `app/src/components/LeagueLogoCard.tsx`,
bindings `LeagueLogoDto.ts`, `LeagueLogoVariantDto.ts`. Modified:
`crates/studio-domain/src/{lib.rs,leagues.rs,session.rs,assets.rs}`,
`crates/studio-api/src/{dto.rs,registry.rs,commands.rs,lib.rs,bindings.rs}`,
`app/src/lib/client.ts`, `app/src/pages/LeagueDetail.tsx`.
