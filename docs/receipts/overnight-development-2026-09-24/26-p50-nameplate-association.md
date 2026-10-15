# Receipt 26 (P2.6-N) — each kit row names its nameplate sheet, on the six measured trees

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p50-nameplate-association`. Clock: 03:28 → 03:35 (real clock, MDT,
26 Sep 2026). Direction: the refreshed ledger's row "2.6 kit name keys (R101) — nameplates keyed
like jerseys … ready, low impact", as corrected by R103.

## 1. Plain-language summary

The Uniforms card now shows, for each kit, the nameplate sheet the game ships under the same three
numbers as the jersey (`name_<style>_<art id>_<variant>_cm`), and whether that file is present. The
research matched this naming for every kit row on six game trees (Xbox 360 / PS3 NHL Legacy and
NHL 15, PS3 NHL 12 and 13). It is shown as a naming association, not as what the game actually
loads. It does not change whether a kit is called complete. On other titles nothing is looked up,
and the card's notes say why.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/kit_files.rs` | `nameplate_measured(title, platform)` (the six R103 trees only; Recompiled and others excluded), `nameplate_stem`; `KitView.nameplate` (looked up under `jersey/`, loose or render-archive member, through the same listing as the kit); `team_kits(…, nameplates)`; a note either way; unit test (association beside the kit, missing nameplate leaves the kit complete, `name_*` never becomes a spare, unmeasured title not looked up) |
| `crates/studio-api/src/{registry,dto,convert}.rs`; `app/src/bindings/KitDto.ts` (generated) | `KitDto.nameplate` |
| `app/src/components/TeamKitsCard.tsx` + test | a "nameplate" line under the selected kit: stem, status, "naming association (R103) · not part of completeness" |
| `crates/studio-domain/tests/kit_selector_join.rs` | real-tree assertion: every row's nameplate present on the six R103 trees; none looked up on the other six |

## 3. Evidence

[`p50-nameplate.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p50-nameplate.log):

- domain lib `kit_files` 16 / 16 (incl. the new case);
- `kit_selector_join` with `--include-ignored`, real trees, read-only: nameplates looked up and **present on
  659 / 659 (X360 15), 703 / 703 (X360 Legacy), 659 / 659 (PS3 15), 703 / 703 (PS3 Legacy), 676 / 676 (PS3 12,
  render archives), 684 / 684 (PS3 13, render archives)** = R103's counts; not looked up on X360 11 / 12 / 13 / 14
  and PS3 11 / 14; every kit verdict unchanged (all rows complete, R56);
- `export-bindings --write` (1 written) then `--check` in sync; `studio-api` lib 31 / 31, `team_kits` 3,
  `ps3_kit_member_route` 1 (+1 ignored), `archive_member_preview` 1 (+1 ignored);
- `tsc` clean; vitest 40 files / 247 tests.

## 4. Limits

- Naming association only (R103): which nameplate the game dresses is unobserved. `variant` and
  `fevariant` are identical columns, so they cannot be told apart.
- The other gen7 titles (X360 11–14, PS3 11 / 14) are not looked up: R103 did not measure them.
- Rink logos are not keyed (R103: positions 2–3 remain candidates).
- No replace route change: nameplate sheets were already reachable through the Uniforms outfit list.
