# Receipt 06 (R4) — Teams 2.5 PS4: the venue listing by vault level name, under research R73

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Committed PS4 anchors read **only**; the
tests' trees are disposable temp directories. Nothing staged, committed, pushed, launched or written
live. Dispatch: [the R4 focused coordinator review](../../validation/development-20260923-r4-review.md),
"next": the **R73 listing contract**, not the superseded R68 re-pointing proposal. Research consumed
(unedited): 2026-09-22 pm handoff [04](../../research/overnight-2026-09-22-pm/04-ps4-venue-listing-contract.md)
(R73) and its correction checkpoint 01 (F3). Claim: `dev-r4-ps4-venue-listing`. Read-only; no
`arenaartid` reassignment, no write contract.

## 1. Plain-language summary

On PlayStation 4 an arena's venue folder is not named by any arena column; the research showed it
is reached through a vault "level" record whose id equals the arena's art id. Studio does not read
the vault, so the research's join was turned into a developer-owned table
(`defs/arena/ps4-venue-levels.json`, provenance inside) and the Arena assets card now lists, for
the open arena, **every** level record carrying its art id — each with its folder (checked live in
the attached tree: present, missing, or "no folder shipped"), its centre-logo / centre-line /
ice-line texture-pack keys resolved into the measured sets, its rink id and its own `ArenaID`
attribute (which is not a second key) — and classes the row the way the research does: one level
and one folder; several levels sharing the id (every candidate shown, **none chosen**, the reason
stated); a level without a shipped folder; no level; or a title outside the table. Folders no
level names are listed once as shared content. Because one level record binds the folder, three
texture packs, a rink id, crowd placement and NIS groups at once, no re-point of `arenaartid` is
offered anywhere on the card, and the card says why.

On the NHL 21 anchor Honda Center (art id 0) lists `anaheim` and `common_ice_int` as ambiguous,
the other arenas resolve to one level, and on NHL 15 Carolina Arena resolves to a level the
research found no folder for — the research's own residuals, reproduced through the seam.

## 2. Files changed

| Path | Change |
|---|---|
| `defs/arena/ps4-venue-levels.json` | **new** — per title (NHL 15–21): the level records arena rows reach (`id`, `name`, `dir`, `ArenaID`, `RinkId`, texpack keys), the texpack misses and set sizes, typed-reference counts, shared / unreferenced directories, level names without a folder, the shared-id map; derived from R73's JSON (script inline, once) |
| `crates/studio-domain/src/arena_assets.rs` | `Ps4VenueLevels` (+ `load`), `Ps4VenueLevel`, `Ps4VenueTitle`, `Ps4TexpackRef`, `FolderStatus`, `Ps4VenueCandidate`, `Ps4VenueClass`, `Ps4VenueListing`, `ps4_venue_listing`; `arena_assets_with_levels`; `ArenaAssetsView::ps4_venue`; the PS4 directory note reworded (the join runs through the level record). Unit case `ps4_arena_art_id_lists_every_vault_level_candidate_and_never_chooses` |
| `crates/studio-api/src/{registry,commands,dto,convert,bindings}.rs` | `Registry::arena_assets_with_defs` (the command resolves `defs/`); `Ps4VenueListingDto` / `Ps4VenueCandidateDto` / `Ps4TexpackRefDto`; `ArenaAssetsDto::ps4_venue` (export roots 145 → 148) |
| `crates/studio-api/tests/arena_assets.rs` | **new** `ps4_arenas_list_their_vault_level_candidates_from_the_r73_table_and_never_choose` |
| `app/src/bindings/{ArenaAssetsDto,Ps4VenueListingDto,Ps4VenueCandidateDto,Ps4TexpackRefDto}.ts` | **generated** |
| `app/src/components/ArenaAssetsCard.tsx` (+ `.test.tsx`) | the "Venue by vault level name" section: class, reason, candidates with folder / texpacks / rink, shared content, dependencies, the no-re-point line |

## 3. What is asserted

| Case | Result |
|---|---|
| NHL 21 anchor, Honda Center (art id 0), tree with `anaheim` + `common_ice_int` | class `ambiguous`; both candidates listed, both `present` (1 file); reason names both and "none is chosen"; `reassignment_supported = false`, reason says "cascade" |
| NHL 21, an ordinary arena | `unambiguous`, one candidate, `stadiums/<name>`, `present` / `missing` by the tree; `unknown` with no tree; `unmeasured` without the defs root |
| NHL 15, Carolina Arena (art id 5) | `level_without_folder`, candidate `no_folder_shipped` |
| Shared content | `common`, `unused` listed once with their counts; `anaheim` never there |
| Texpack keys | resolved `in_set`; the two research misses (1001 on NHL 17, 100 on NHL 20 / 21) are recorded in the table and, since no arena row reaches those levels, exercised on a synthetic table (`in_set = false`, "1001 outside the set" in the dependency line) |
| gen7 | no PS4 listing (`null`) |
| Card | class badge, reason, per-candidate folder state and "no texpack with this key", shared content, "Re-pointing is not offered", no re-point / reassign button |

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib arena_assets` | **8 passed** (was 7) |
| `cargo test -p studio-api --test arena_assets` | **4 passed** (was 3) |
| `cargo test -p studio-api --lib` | 31 passed (export roots 148) |
| `export-bindings --write` / `--check` | 4 written, in sync |
| `npx vitest run src/components/ArenaAssetsCard.test.tsx` | **4 passed** |
| `npx tsc --noEmit` | clean |

## 5. Coverage effect (Phase 2 matrix 2.5, PS4 15–21)

| Row | Before | After |
|---|---|---|
| 2.5 PS4 venue binding | venue directories listed as shipped evidence under a vault reference Studio does not read | **read-only listing by vault level name: impl+tested, dev** on the committed anchors against the R73 table — every candidate level with folder status, texpack keys, rink id; ambiguities named and never resolved silently; shared content once; no reassignment (the R68 write proposal is not implemented) |

## 6. Not done / limitations

- The level table is research-derived, not read from the vault by Studio; it covers the levels the
  measured arena rows reach on the anchors (32 / 38 / 58 / 34 / 34 / 34 / 34 records) and carries
  the residuals as text. An arena art id outside the table reads "no level record carries this id",
  which is the table's limit, not a vault reading. A vault reader remains the way to make this
  live.
- Whether the console draws the listed folder for the arena is not claimed (card T-VEN-1 unchanged).
- The real extracted PS4 trees were not swept for this package; folder presence is checked live on
  whatever tree is attached.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `patch_venue_*.py` | deleted at closeout |
| Temp roots `nhlms-arena-ps4-levels-*`, `nhlms-arenaapi-ps4-levels-*` | removed on drop; verified absent |
| Background processes | none |

## 8. Next action

The roadmap's next Teams operation (STATUS §next).
