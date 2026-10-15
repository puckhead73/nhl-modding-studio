# Receipt 15 — R3 / Equipment 4.x: the component set an equipment id must carry, checked per player and censused per family

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** Branch
`feat/phase5-asset-tooling`, HEAD `8d60a6d` (unchanged). Real Xbox 360 / PS3 trees read **only**; the
unit case's temp directory removes itself. Nothing staged, committed, pushed, launched or written
live. Claim: `dev-r3-equipment-component-sets`. Dispatch: item 5 of the coordinator review of
2026-09-22 (Equipment). Research consumed (unedited): 2026-09-21 pm handoff
[10](../../research/overnight-2026-09-21-pm/10-gen7-equipment-component-sets.md) (R52 §1–3).

## 1. Plain-language summary

The equipment card on a player's page (receipt 12, 21 Sep) could say that an assigned glove or
pad id *has files*. The research has since measured what a complete id looks like — for gloves
seven textures (three maps plus a diffuse and three team-tint maps), for pads six, for a blocker
or trapper the set twice over (once per mesh style), for a goalie stick two, for a mask paint its
shell-0 colour map — and which shipped ids fall short of that (typically the third tint map). The
meshes are shared per style, so an id needs no mesh of its own.

The card now says, per slot, whether the assigned id carries the family's full set and, if not,
exactly which files it lacks, separately for the in-game files and the front-end copy. On the
titles whose families are one texture library per id (sticks, skates, helmets everywhere; every
family on NHL 11 / 12) it checks for that one container. Where nothing is measured (NHL 07–10,
PS4) it says so instead of inventing a set. An incomplete set is information, never a refusal —
the game ships those ids.

A family-wide census reproduces the research: on Legacy (both consoles) 53 of 53 gloves, 57 of 67
pads, 50 of 68 blockers, 58 of 66 trappers, 38 of 38 goalie sticks and 280 of 280 mask paints carry
the full set, with the research's incomplete-id lists matched id for id; NHL 13 and 15 show the same
shape. The census counted one id fewer than the research on blocker and trapper, and one mask paint
on NHL 13 without a shell-0 file; both are recorded as observations, not smoothed over.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/equipment_components.rs` | **new** — `SetContract` / `set_contract(slot, game_id, platform)` (measured / single container / not measured), `expected_stems` (R52 §1 majority sets incl. the `fe*` copies), `check_components` (per-group present / missing), `family_census`. 4 unit cases. |
| `crates/studio-domain/src/equipment_slots.rs` | `EquipmentFileCheck` carries `set_contract` + `component_groups`; `check_player_equipment_files` takes the title / platform (they never change which files are listed). |
| `crates/studio-domain/src/lib.rs` | module registration. |
| `crates/studio-domain/tests/equipment_component_census.rs` | **new** — the census on four real trees against R52 §1 and its JSON. |
| `crates/studio-api/src/{dto,registry,bindings}.rs` | `EquipmentFileCheckDto` + `set_contract` / `set_rule` / `component_groups`; `ComponentGroupDto`; export roots 139 → **140**. |
| `app/src/bindings/{EquipmentFileCheckDto,ComponentGroupDto}.ts` | **generated** (`--write`; `--check` in sync). |
| `app/src/components/EquipmentFilesCard.tsx` (+ `.test.tsx`) | a set pill per group ("in-game set complete" / "front-end set missing feglove_27_0_dm"), "set not measured on this title" for ids with files on unmeasured titles. |

## 3. Evidence

### 3a. Census — `cargo test -p studio-domain --test equipment_component_census -- --ignored`
([r3-equipment-component-census.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-equipment-component-census.log))

| Tree | gloves | pads | blocker | trapper | goalie stick | mask paint (shell 0) |
|---|---|---|---|---|---|---|
| Legacy X360 | 53 / **53** | 67 / **57** (R52: 57; the 10 incomplete ids identical) | 68 / **50** (R52: 69 ids, 50 with every majority component; the 18 incomplete ids identical after id 0) | 66 / **58** (R52: 67 / 58; 8 incomplete ids identical after id 0) | 38 / **38** | 280 / **280** |
| Legacy PS3 (`.rpsgl`) | 53 / 53 | 67 / 57 | 68 / 50 | 66 / 58 | 38 / 38 | 280 / 280 |
| NHL 13 X360 | 29 / 29 | 50 / 40 | 51 / 37 | 49 / 41 | 24 / 24 | 249 / **248** (paint 260 ships shells 1–7 only) |
| NHL 15 X360 | 47 / 47 | 64 / 54 | 65 / 51 | 63 / 55 | 33 / 33 | 256 / 256 |

Every incomplete id on 13 / 15 lacks only a component the research names as optional (the third
tint map, a normal map, a front-end diffuse) — or, once, the mask shell-0 file. Front-end copies:
8 glove ids lack `feglove_<id>_0_dm` on every tree (R52: the same 8), 10 pad ids on Legacy.

### 3b. Reading differences recorded (observation O-9)

- R52 counts `blocker_0_am` / `trapper_0_am` as "id 0, `am` only". Studio's blocker / trapper
  matchers key the id at the second numeric position (`blocker_<style>_<id>_…`), so a single-number
  stem is a style-level file and not an equipment id: 68 / 66 ids instead of 69 / 67. The complete
  counts and every other incomplete id agree.
- NHL 13 mask paint 260 has `goaliehelmet_{1..7}_260_cm` but no shell-0 file; R52's "identical in
  shape" holds for every other id.

### 3c. Seam regression on real trees — `cargo test -p studio-api --test equipment_files -- --ignored`
([r3-equipment-files-seam-after-sets.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-equipment-files-seam-after-sets.log))

**1 passed** (112 s): Legacy 805 players sampled, **3 296 / 3 296** assigned ids with files; NHL 11 759 sampled, **3 099 / 3 099** — the 21 Sep containment figures unchanged with the set fields added to every row; catalogue sizes identical to receipt 12.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib equipment` (slots + components) | **24 passed** |
| `cargo test -p studio-domain --test equipment_component_census -- --ignored` | **1 passed** (four trees) |
| `export-bindings --write` / `--check`; bindings drift test | 2 written, in sync, **140 roots**; passed |
| `npx tsc --noEmit` | clean |
| `npx vitest run src/components/EquipmentFilesCard.test.tsx` | **3 passed** (was 2) |
| seam regression (§3c) — `cargo test -p studio-api --test equipment_files -- --ignored` | **1 passed** (Legacy 3 296 / 3 296, NHL 11 3 099 / 3 099) |

## 5. Coverage effect (Phase 4 — [phase-4-coverage-matrix-2026-09-22.md](../../validation/phase-4-coverage-matrix-2026-09-22.md))

| Row | Before (receipt 12, 21 Sep) | After |
|---|---|---|
| Existing-id check: "has files" | impl+tested (Legacy + NHL 11 containment 100 %) | **"has the family's set, missing: …": impl+tested, dev** on X360 13 / 15 / Legacy and PS3 Legacy; single-container check on sticks / skates / helmets and on NHL 11 / 12; not-measured stated on 07–10 and PS4 |
| Family census (reassignment target set) | — | impl+tested, dev — the complete ids per family are the ids a reassignment could point at; the incomplete ids are valid as shipped with their gaps named |
| New-id registration (place the majority set) | not impl | not impl — the set to place is now known per family (R52 §2); no writer, no runtime card |

## 6. Not done / limitations

- No writer: reassignment stays an ordinary cell edit (the card shows the consequence), and
  registration of a new id is not built.
- The `n2` tint semantics (home / away / third) are R52's candidate, not shown as meaning.
- NHL 14 and the PS3 13 / 15 trees were not run through the census (same contract; unclaimed).
- Desktop review card **V-34**.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `wire_components.py` | deleted at this checkpoint (the scratchpad is empty) |
| Temp root `nhlms-equipset-<pid>` | removed by the unit case; verified absent |
| Background processes | none |
| Evidence logs `r3-equipment-component-census.log`, `r3-equipment-files-seam-after-sets.log` | retained |

## 8. Next action

Phase 5 — Miscellaneous presentation: the PS4 overlay / scorebug inventory (research handoff 16)
and the PS4 song catalogue listing (handoff 12), as read-only pages; or the closeout if usage
requires.
