# Receipt 12 — R3 / Teams 2.5: the arena's shipped assets, joined where measured and listed everywhere else

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Real Xbox 360 / PS3 trees and committed
anchors read **only**; every test tree is a disposable temp directory that removes itself. Nothing
staged, committed, pushed, launched or written live. Claim: `dev-r3-arena-assets-card`. Dispatch:
item 5 of the coordinator review of 2026-09-22 (Teams: arena). Research consumed (unedited):
2026-09-21 pm handoffs [19](../../research/overnight-2026-09-21-pm/19-gen7-arena-venue-asset-join.md)
(R62) and [21](../../research/overnight-2026-09-21-pm/21-ps4-arena-venue-asset-inventory.md) (R64).

## 1. Plain-language summary

The Arena page could describe an arena's database fields (receipt 05) but said nothing about the
files that draw it. Now it does, in exactly the terms the research measured and no more.

On the Xbox 360 / PS3 titles the arena's art id is literally the name of its venue model and, from
NHL 12 on, its jumbotron texture. Studio resolves both for every arena and lists the venues the game
ships that no arena row uses (spare venues). Two things people would expect on the arena — the
centre-ice logo and the rafter banners — are keyed by the **team**, not the arena, so the card
resolves them through the arena's home teams and says that is what it did; on NHL 09–11, whose team
table has no home-arena link at all, it says no tenant can be resolved. The ice bounce map is shown
as a **candidate** (the field's value names a file on every row, but so would two other fields), and
the remaining ice layers and the named ice bases are listed as shipped files with no established
binding. On PS4 the venues are named folders selected through a vault file Studio does not read yet,
so the card lists the folders and the shared numbered ice families and claims no binding at all.

Every family says which of these classes it is in, every resolved file says where its key came from
and whether it was found, and the card writes nothing.

On the real trees: every arena row of NHL 09 / 11 / 13 / 15 / Legacy (Xbox 360) and Legacy (PS3)
names an existing venue model; every row of 13 / 15 / Legacy names an existing jumbotron; the spare
counts equal the research's residuals exactly; the candidate bounce map resolves on every row of the
titles that carry the column; the team-keyed families resolve for 87–93 % of tenanted arenas, the
same "most but not all" the research measured for team art ids.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/arena_assets.rs` | **new** — `ArenaAssetContract` / `arena_asset_contract(game_id, platform)`, `Binding` (established / via home team / candidate / no established binding), `AssetFamily` / `AssetFile` with present / missing / unknown evidence, `arena_assets` (gen7 families, PS4 venue directories + `common/` families), `arena_asset_inputs` (the row's `arenaartid` / `bouncemap`, tenants via `kOtt.buDa` with the link's presence recorded, the residual id sets). 7 unit cases. |
| `crates/studio-domain/src/session.rs`, `lib.rs` | `Session::arena_asset_inputs`; module registration. |
| `crates/studio-domain/tests/arena_asset_join.rs` | **new** — six real trees against R62's per-title residuals (`data/r62-gen7-arena-asset-join.json`). |
| `crates/studio-api/src/{dto,convert,registry,commands,lib,bindings}.rs` | `ArenaAssetsDto` / `AssetFamilyDto` / `AssetFileDto`; `Registry::arena_assets` (contract from the project's title and platform, tree as evidence, platform-disagreement note); the `arena_assets` command; export roots 133 → **136**. |
| `crates/studio-api/tests/arena_assets.rs` | **new** — the seam on the Legacy, NHL 09 and NHL 21 anchors with synthetic trees. |
| `app/src/bindings/{ArenaAssetsDto,AssetFamilyDto,AssetFileDto}.ts` | **generated** (`--write`; `--check` in sync). |
| `app/src/components/ArenaAssetsCard.tsx` (+ `.test.tsx`), `app/src/pages/ArenaDetail.tsx`, `app/src/lib/client.ts` | the card under the venue-parameters card; the command typing. |

## 3. Evidence

### 3a. Real trees — `cargo test -p studio-domain --test arena_asset_join -- --ignored`
([r3-arena-asset-join.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-arena-asset-join.log))

| Tree | Arena rows | Venue model resolved | Jumbotron resolved | Spare venues (R62) | Spare jumbotrons (R62) | Bounce map present | Tenanted arenas: logo / banners |
|---|---|---|---|---|---|---|---|
| NHL 09 X360 | 188 | **188** | no `jumbotron/` (as measured) | 4 (4) | — | no `bouncemap` column | no home-arena link on this title |
| NHL 11 X360 | 242 | **242** | no `jumbotron/` | 10 (10) | — | no column | no home-arena link |
| NHL 13 X360 | 174 | **174** | **174** | 10 (10) | 12 (12) | 174 / 174 | 174: 152 / 171 |
| NHL 15 X360 | 178 | **178** | **178** | 12 (12) | 14 (14) | 178 / 178 | 178: 156 / 170 |
| Legacy X360 | 187 | **187** | **187** | 12 (12) | 14 (14) | 187 / 187 | 187: 165 / 177 |
| Legacy PS3 (`.rpsgl`) | 187 | **187** | **187** | 12 (12) | 14 (14) | 187 / 187 | 187: 165 / 177 |

The spare-id lists themselves (`[66, 254, 393, 511, 514, 600–605, 999]` on Legacy …) are in the log.
One expectation I typed by hand — 12 spare jumbotrons on NHL 15 — was wrong; the research JSON says
14 and the join found 14. The test now reads its expectations from the JSON's residuals rather than
from a transcription.

### 3b. Unit and seam cases

Domain (7): the gen7 join resolves venue / jumbotron / logo-through-team / bounce map and lists the
spare and the named ice bases; missing files and an absent `jumbotron/` are reported with reasons;
an unreadable directory is *unknown*; NHL 11 has no jumbotron; PS3 resolves `.rpsgl` and lists every
container of an id; a row without `arenaartid` names nothing; no tree → the same families with
unknown evidence; NHL 07 / 08 / no title → nothing joined; PS4 lists directories and `common/`
families as a candidate with no file resolved; strict stem parsing.

Seam (3): Legacy — venue present with an install-relative path, the spare `999` listed, the logo
through the home team, banners missing with "absent", candidate and unbound families labelled;
NHL 09 — no jumbotron and "no home-arena link" on the team-keyed families; NHL 21 PS4 — vault
reference, three directories, `common_ice` as a candidate, no `established` family anywhere, an
unknown arena id is `None`.

Card (4): binding badges per family, file evidence with the key's source and the miss reason, the
spare list on demand, a candidate never shown as established; the PS4 directory listing with the
vault reference named; a late answer for another arena never painted; the null and error states.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib arena_assets` | **7 passed** |
| `cargo test -p studio-domain --test arena_asset_join -- --ignored` | **1 passed** (six trees, 6.7 s) |
| `cargo test -p studio-api --test arena_assets` | **3 passed** |
| `export-bindings --write` / `--check`; `bindings::tests::live_bindings_match_the_dto_contract` | 3 written, in sync, **136 roots**; drift test passed |
| `npx tsc --noEmit` | clean |
| `npx vitest run src/components/ArenaAssetsCard.test.tsx` | **4 passed** |
| package check bundle — [r3-arena-assets-checks.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-arena-assets-checks.log) | domain 7, seam 3, `arena_venue` 2 (+1 ignored), `team_kits` 3, bindings 25 — all passed; `npx vitest run` **29 files, 195 passed** (was 28 / 191); `tsc` clean |

## 5. Coverage effect (Phase 2 matrix 2.5)

| Row | Before | After |
|---|---|---|
| Venue model / jumbotron (gen7 `rendering/*`) | not joined to the arena row | **read-only join: impl+tested, dev** — `arenaartid` → `stadium/stadium_<id>_container_*` on 100 % of rows of six real trees (09 / 11 / 13 / 15 / Legacy X360, Legacy PS3), `jumbotron/texlib_<id>` on 12+; spare venues listed per title, equal to R62's residuals |
| Centre-ice logo / banners | listed under the team's outfit surface only | shown on the arena **through the home team** with that provenance stated; on NHL 09–11 the absence of the home-arena link is stated |
| Bounce map / other ice layers | — | **candidate** (bounce map, resolving on 100 % of rows of 13 / 15 / Legacy) and **no established binding** (centre-line, line art, lightmap, hotspots, named ice bases) — labelled, never promoted |
| PS4 rink / ice binding | blocked — research-parsed, not implemented | **listing: impl+tested, dev** — venue directories and the `stadiums/common/` families shown as shipped evidence under a *vault reference Studio does not read*; `arenaartid` labelled a candidate. The vault reader itself stays open |
| Replacing any of these | — | unchanged: the jumbotron / logo / banner / bounce-map textures are ordinary `RW4xb2` / `RW4ps3` / GNF members already covered by the Replace… route (with receipt 11's disclosures); the venue model is a mesh container and is not offered |

## 6. Not done / limitations

- No PS4 vault reader: the PS4 half is an inventory with the selector named, not a join. That is
  the research lane's open Q (R64 §3) and a developer package once the vault's typed data has a
  Studio reader.
- NHL 09 / 11 carry no team → arena link, so the team-keyed families cannot be resolved there;
  NHL 10 / 12 / 14 were not run in the real-tree case (the contract is the same; unclaimed until run).
- Two families overlap with the team surfaces (logo, banners); the card names the team it resolved
  through rather than duplicating the team's editors.
- Desktop review card **V-32**.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `wire_arena_assets.py` | deleted at this checkpoint (the scratchpad is empty) |
| Temp roots `nhlms-arena-*`, `nhlms-arenaapi-*` | removed on drop; verified absent. Six `nhlms-*` roots from **earlier campaigns** (`nhlms-league-campaign-all-*`, `nhlms-logos-gen7-*` ×3, `nhlms-p3-op-*` ×2) remain in `%TEMP%`; they are not owned by this session's manifests and were left in place, recorded here |
| Background processes | none |
| Evidence logs `r3-arena-asset-join.log`, `r3-arena-assets-checks.log` | retained |

## 8. Next action

Continue Phase 2: kits are read-only (receipt 10), audio remains format-blocked (2.7), so the next
actionable Teams step is the gen7 **team logo package sweep** for different-dimension replacement
(2.4 "team packages not swept") or, if the research queue's Players inputs rank higher, Phase 3 —
the gen7 player portrait / head containment card (research handoffs 11 and 20).
