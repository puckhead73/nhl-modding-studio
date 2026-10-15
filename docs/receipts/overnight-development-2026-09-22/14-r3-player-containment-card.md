# Receipt 14 — R3 / Players 3.4: portrait and head containment, the `hasportrait` flag as stored, and the shipped pools

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Real Xbox 360 / PS3 trees and committed
anchors read **only**; every test tree is a disposable temp directory that removes itself. Nothing
staged, committed, pushed, launched or written live. Claim: `dev-r3-player-containment-card`.
Dispatch: item 5 of the coordinator review of 2026-09-22 (Players). Research consumed (unedited):
2026-09-21 pm handoffs [11](../../research/overnight-2026-09-21-pm/11-gen7-player-portrait-head-containment.md)
(R53 §1–2, §4) and [20](../../research/overnight-2026-09-21-pm/20-ps4-player-portrait-containment.md) (R63).

## 1. Plain-language summary

The Player assets pages could show a player's portrait and head when the files were there, but
they could not say, for the roster as a whole, what the install ships and what it does not — and
they said nothing about the database's own "has portrait" switch, which the research measured as
disagreeing with the files on the later titles (93 players on NHL 13 and 127 on Legacy are flagged
*no* while a portrait file exists; 2 Legacy players are flagged *yes* without one).

Now the Player assets list opens on a containment summary, and each player's page carries a
containment line. Both show three things kept deliberately apart: the **ids** the row carries, the
**flag** exactly as stored (never worked out from a file, never written from one), and the **files**
the attached install actually ships — present, missing, or unknown when no tree is attached. Players
whose own head or portrait is missing are named as "shipped without asset" (NHL 09's Nedved,
Andersson, Marha, Zherdev …), and players whose flag and file disagree carry a note that says the
flag is shown as stored, not corrected. The summary also counts the shipped portraits and heads that
no roster row references — the pool a future reassignment could draw from, listed here, never
written.

On the four real trees checked the numbers reproduce the research to the unit: bespoke counts and
their file presence, the pool population, the four-way flag-versus-file cross-tab, the unused pools
and the named residuals. The PS4 titles resolve portraits by art id (`.gnf`, `.png` on NHL 15) and
leave heads unmeasured, as the research did; NHL 07 / 08 are reported as not measured.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/player_containment.rs` | **new** — `ContainmentContract` / `containment_contract(game_id, platform)` (gen7 head prefixes per title, PS4, unmeasured), `bio_rows` (ids + `hasportrait` by metadata name), `player_containment` (portrait buckets, head directory, bespoke = head id equals art id, the per-file pool floor of receipt 06, the flag × file cross-tab, unused pools, bounded residual name lists, per-player notes). 4 unit cases. |
| `crates/studio-domain/src/session.rs`, `lib.rs` | `Session::player_bio_rows`; module registration. |
| `crates/studio-domain/tests/player_containment_join.rs` | **new** — four real trees against R53 §1. |
| `crates/studio-api/src/{dto,convert,registry,commands,lib,bindings}.rs` | `PlayerContainmentViewDto` / `ContainmentSummaryDto` / `PlayerContainmentDto`; `Registry::player_containment`; the command; export roots 136 → **139**. |
| `crates/studio-api/tests/player_containment.rs` | **new** — the seam on the Legacy, NHL 09, NHL 21 and NHL 08 anchors with synthetic trees. |
| `app/src/bindings/{PlayerContainmentViewDto,ContainmentSummaryDto,PlayerContainmentDto}.ts` | **generated** (`--write`; `--check` in sync). |
| `app/src/components/PlayerContainmentCard.tsx` (+ `.test.tsx`) | the card in summary and per-player modes; one shared query keyed on project + install root + platform. |
| `app/src/pages/PlayerAssets.tsx`, `app/src/pages/PlayerAssetDetail.tsx`, `app/src/routes.tsx`, `app/src/lib/client.ts` | `PlayerAssetsIndex` (the summary above the pick prompt), the per-player line under the identity card, the route, the command typing. |

## 3. Evidence

### 3a. Real trees — `cargo test -p studio-domain --test player_containment_join -- --ignored`
([r3-player-containment-join.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-player-containment-join.log))

| Tree | Players | Bespoke (head file / portrait file) | Non-bespoke (R53 "other") | Pool heads shipped | `hasportrait` × file (1·1 / 1·0 / 0·1 / 0·0) | Unused shipped portraits / heads | Residuals named |
|---|---|---|---|---|---|---|---|
| NHL 09 X360 | 3 968 | 256 (**251** / **251**) | 3 712 | 3 711 / 3 711 | 2 705 / 0 / 1 / 1 262 | **214** / **129** | heads: Nedved, Andersson, Häller, Marha, Bäumle; portraits: Zherdev, Andersson, Häller, Marha, Bäumle |
| NHL 13 X360 | 5 412 | 268 (268 / **267**) | 5 144 | 5 142 / 5 142 | 4 298 / 0 / **93** / 1 021 | **2 433** / **375** | portrait: Grigorenko |
| Legacy X360 | 5 632 | 246 (246 / 246) | 5 386 (5 385 pool + 1) | 5 385 / 5 385 | 4 376 / **2** / **127** / 1 127 | **4 724** / **457** | flagged without file: Gergo Nagy, Maurizio Colella |
| Legacy PS3 (`.rpsgl`) | 5 632 | 246 (246 / 246) | 5 386 | 5 385 / 5 385 | 4 376 / 2 / 127 / 1 127 | 4 724 / 457 | the same |

Every figure equals R53 §1. Two readings I had to correct against the research while writing the
test: the `playerheads/silhouettes/` sub-directory holds the pool heads' stand-ins, not portraits
(counting it gave 2 924 shipped / 218 unused on NHL 09 instead of 2 920 / 214), and R53's "other"
class is every non-bespoke row, which Studio splits into *pool* (in the measured range) and *other*
(head id 0 or below the floor: one row on NHL 09 / 13, one on Legacy).

### 3b. Unit, seam and card cases

Domain (4): the gen7 cross-tab with a missing bespoke head and a flagged pool player without a
file; NHL 09's `head_` prefix and an unattached rendering tree (heads unknown, portraits read);
PS4's `.gnf` / NHL 15 `.png` portraits with heads unmeasured; an unmeasured title and a missing
flag column. Seam (2): Legacy — no file evidence without a tree, then a bespoke player reads
"shipped without asset" and a flagged pool player's note, the flag identical with and without the
tree; NHL 09's head files, NHL 21 PS4 portraits only, NHL 08 unmeasured. Card (4): the cross-tab as
data and the residual names, a player row's ids / flag / evidence / note and the not-found state,
the unmeasured and error states, the query keyed on the install root.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib player_containment` | **4 passed** |
| `cargo test -p studio-domain --test player_containment_join -- --ignored` | **1 passed** (four trees, 3.8 s) |
| `cargo test -p studio-api --test player_containment` | **2 passed** |
| `export-bindings --write` / `--check`; bindings drift test | 3 written, in sync, **139 roots**; 25 passed |
| `npx tsc --noEmit` | clean |
| `npx vitest run` (full) — [r3-player-containment-checks.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-player-containment-checks.log) | **30 files, 199 passed** (was 29 / 195) |

## 5. Coverage effect (Phase 3, Players — [phase-3-coverage-matrix-2026-09-22.md](../../validation/phase-3-coverage-matrix-2026-09-22.md))

| Row | Before | After |
|---|---|---|
| 3.4 portrait / head containment, gen7 (X360 / PS3 09–Legacy) | per-player file resolution only (`list_player_assets`); no roster-level statement; the flag not shown | **read-only cross-tab: impl+tested, dev** — reproduces R53 on X360 09 / 13 / Legacy and PS3 Legacy; NHL 09's `head_` naming; the flag as stored; "shipped without asset" named |
| 3.4 `hasportrait` | not displayed on the asset pages | displayed **as the DB flag**, cross-tabulated against the file, never derived or written (R53 §2) |
| 3.4 PS4 portraits (15–21) | resolved per player (`kettle_assets/playerportraits`, NHL 15 PNG) | + the roster-level cross-tab and the unreferenced pool (R63); heads stated as name-keyed vault data, not measured |
| 3.4 reassignment | — | the unused shipped portraits / heads are counted as the target set; **no writer**, no validation path yet |
| NHL 07 / 08 | — | reported as not measured |

## 6. Not done / limitations

- No reassignment and no flag editing here; the existing bio-field editors still edit `hasportrait`
  as a cell, and this card never touches it.
- Real-tree coverage is 09 / 13 / Legacy (X360) + Legacy (PS3); 10 / 11 / 12 / 14 / 15 use the same
  contract and are unclaimed until run. The PS4 real trees were not run (the seam covers the rule on
  a synthetic tree; R63's counts are not reproduced here).
- Desktop review card **V-33**.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `wire_containment.py` | deleted at this checkpoint (the scratchpad is empty) |
| Temp roots `nhlms-contain-*`, `nhlms-containapi-*` | removed on drop; verified absent |
| Background processes | none |
| Evidence logs `r3-player-containment-join.log`, `r3-player-containment-checks.log` | retained |

## 8. Next action

Phase 4 — Equipment: the gen7 component-set check of research handoff 10 (R52) on the existing
equipment file card ("has files" → "has the family's set, missing: …").
