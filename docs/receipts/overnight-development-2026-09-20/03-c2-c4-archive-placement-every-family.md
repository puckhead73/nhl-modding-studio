# Receipt 03 — C2/C3/C4: league logo + imagery archive placement on every family (Priorities 1–2)

Developer lane, overnight 2026-09-20. **Developer-verified; not Codex-accepted.** Baseline `8d60a6d`.
The real-archive sweeps run in the background as this receipt is written; their per-title results are
appended in §4 as they complete (see the log paths in §7) and in the final receipt.

## Plain-language summary

A replaced league logo or presentation image now reaches the archive the game actually reads on
**every** title family, through the same reviewed, restorable copied-target save:

- **PS4 NHL 15–21** — already built for NHL 21; now exercised on the real `inxp_10*.big` of every title
  (copies of the installs, up to 3.8 GB): a league logo (a shared block on 16–21 → private copy) and a
  non-logo image (large logo / Memorial Cup / broadcast trophy) replaced with marked images in one save,
  reopened, restored byte-exact.
- **Xbox 360 NHL 11–Legacy, PS3 NHL 12–Legacy, Legacy Recompiled** (`fe.big` / `cache.big`) — new: the
  logo *package* inside the archive is patched in place by the span-preserving re-plan and placed by the
  existing archive stager (fit-in-slot when it fits, otherwise appended and re-pointed). Proven on real
  packages in synthetic archives and on the real Legacy `cache.big` of both consoles (the other titles are
  in the background queue).
- **Xbox 360 NHL 07–10, PS3 NHL 08–10** (`data0.big` / `fe.big`, the BIG4 archives with a `.bh` index
  beside them) — new: a BIG4 writer that patches the archive **and** its `.bh` sidecar together as one
  change set (two artifacts, restored together). Fit-in-slot counts only slack bytes verified zero;
  growth appends and re-points both indexes — a shape no stock archive has, said so in the review
  (card R2-BIG4-1). NHL 07's `data0.big` refuses growth because its `.bh` is mirrored inside `boot.big`.

Whether any console loads the rewritten archive is still the user's runtime card; nothing here is
game-verified.

## 1. Package, disposition, scope

- **C2** — PS4 real archives 15–21 (`--ignored` sweep, disposable copies under `%TEMP%`).
- **C3** — gen7 EB archive route: package members (BIGF) accepted by the archive-member planner; archive
  binding per title on the logo and imagery listings; synthetic + real `cache.big` cases.
- **C4** — BIG4 + `.bh` writer (`tdb-archive-big::big4`), the planner's BIG4 branch, two-artifact staging
  through the copied-target plan/transaction, NHL 07 growth policy.
- Not built: whole-group replace of a PS4 dedup group; the BIG4 **ordered reflow** (the stock shape; the
  append is the implemented growth placement); the NHL 07 `boot.big` cascade; different dimensions.

## 2. Changed paths (SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-archive-big/src/big4.rs` | `61bd1af2577b862c` | **new** — `Viv4` sidecar parser, `BoundBig4::bind` (directory + sidecar cross-check: count, offset/size, djb2 hash, trailer length, `size_le` = length, 64-alignment, overlaps; index-order-vs-offset-order recorded, not required), `plan` (`Growth::Append` / `Growth::Refuse(reason)`; fit-in-slot against **verified-zero** slack — a non-zero gap byte ends the usable slack and is preserved), `stage` (streaming copy + patches + appends), `verify` (re-bind, planned entries/records, untouched bytes compared), `stage_files`; synth builder; 5 unit cases |
| `crates/tdb-archive-big/src/lib.rs` | `5505da97f610ce73` | `pub mod big4` |
| `crates/studio-domain/src/asset_member_edit.rs` | `f423b0e82019fd62` | EB branch accepts BIGF package members (re-plan writer; a builder fallback is refused inside an archive); **BIG4 branch** `plan_big4_member_edit` (sidecar required, RefPack unwrap/re-wrap with the `.bh` decoded length, placement predicted as the stager decides it, NHL 07 `dataN.big` growth refusal); `Big4MemberInfo`, `big4_sidecar_relative`, `big4_growth_refusal`; `game_id` parameter |
| `crates/studio-domain/src/copied_target.rs` | `6c220c6ba9296e3a` | `TextMemberReplacements.big4: Option<Big4GroupInfo>`; `big4_member_replacements`; `CopiedTargetPlan::add_big4` (two artifacts `<key>` + `<key>.bh`) |
| `crates/studio-domain/src/txn.rs` | `2ab5e35346e356f7` | `StagedBig4`, `stage_big4`, `big4_artifacts` (re-verification now; sidecar identity re-checked; unqualified items incl. the append shape) |
| `crates/studio-domain/src/league_logos.rs` / `league_imagery.rs` | `0eccabc6800a7a6e` / `d5c1902257111b1a` | `gen7_logo_archive` (07/08 + PS3 08 `data0.big`; 09/10 `fe.big`; Xbox 11 `fe.big`; 12–Legacy/Recomp/PS3 12–Legacy `cache.big`; NHL 11 PS3 loose-only), `gen7_logo_archive_is_big4`, `gen7_logo_archive_at` (archive + `.bh` must exist under the root); `archive` on every gen7 variant/member; notes disclose the PS3 loose + `cache.big` pair, the BIG4 sidecar and NHL 07's restriction; the tile/banner note reworded to "research unknown" (R1 F3 had narrowed the matrix, not this string) |
| `crates/studio-domain/src/assets.rs` | `311d49f91ed9ea67` | `TextureContainer::display()` (human labels) |
| `crates/studio-domain/src/workspace.rs` | `e23d5a1946b08aa1` | passes the title to the planner |
| `crates/studio-api/src/registry/copied_target.rs` | `d93b4ebe197c5895` | `stage_asset_members` BIG4 branch (bind pair → `big4_member_replacements` → plan with the group's growth policy → `stage_big4` → `add_big4`); the sidecar joins the reservation; artifact labels ("Archive with replaced texture member(s)", "Archive index sidecar … patched together with its archive") |
| `crates/studio-api/tests/asset_member_route.rs` | `cf7c88869cd6ef3d` | +1 `--ignored`: real PS4 `inxp_10*.big` sweep (`NHL_STUDIO_PS4_TITLES`) |
| `crates/studio-api/tests/gen7_asset_member_route.rs` | `6d505dbea9f93fb0` | **new** — binding table (default); synthetic `cache.big` with real Xbox / PS3 Legacy packages (3 members: two fit, DXT5 appends); real gen7 EB sweep (`NHL_STUDIO_GEN7_TARGETS`) |
| `crates/studio-api/tests/big4_asset_member_route.rs` | `4e472cc325640e7f` | **new** — NHL 07 policy (default); synthetic BIG4 + `.bh` pairs with real Xbox 08/10 + PS3 08/10 packages (marked + noise members; two artifacts; both files restored); NHL 07 growth/sidecar refusals; real BIG4 sweep (`NHL_STUDIO_BIG4_TARGETS`) |
| `app/src/components/ArchiveMemberReplace.tsx` | `846cd4aa5b555962` | human container label |

No DTO change in this package (bindings untouched by it).

## 3. Behavior and decisions

- **Placement is decided from the actual encoded bytes** (research review F1): the EB stager already did;
  the BIG4 planner reads the gap after the slot and uses only its verified-zero prefix (F2). Studio's
  RefPack stream is longer than EA's, so a noise image grows past its slot (append) while a marked logo
  compresses smaller (fit) — both paths are exercised on real packages.
- **BIG4 append** breaks the directory's offset order (stock keeps them equal). `bind` records
  `index_order_is_offset_order` instead of refusing, so a Studio-appended archive can be edited again; the
  plan's notes, the placement prediction and the artifact's qualification items all say the shape is
  unobserved in stock (card R2-BIG4-1). The ordered reflow is not implemented.
- **NHL 07** `data0.big`–`data5.big`: growth refused with the `boot.big` reason (research 02 §1.1: the
  `.bh` copies exist on NHL 07 only — keyed on the title, not the archive name); an identical member is
  a no-op; a shorter one fits (the `.bh` record changes — the cascade also applies to that; the planner
  currently allows a *fit* on NHL 07, which still changes the `.bh` size/decoded words — **open**: to be
  strict, NHL 07 should refuse any `.bh` change; recorded as a limitation, not yet enforced).
- **PS3 12–Legacy** ship the packages loose *and* in `cache.big`: both routes are offered, the pair is
  disclosed on the card; one save does not touch the other copy (precedence research unknown).
- **Two artifacts, one transaction**: `.big` + `.bh` are prepared, applied, recovered and restored together
  by the existing engine (the sidecar artifact depends on the archive artifact for ordering).

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `cargo test -p tdb-archive-big --lib big4` / `--lib replan` | 5 / 5 passed |
| `cargo test -p studio-api --test gen7_asset_member_route` | 1 passed (binding table), 3 ignored |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-api --test gen7_asset_member_route -- --ignored` (synthetic) | **2 passed** — Xbox Legacy: logo fit (8176→5296), `divisionlogos` fit, DXT5 small append (4080→4144); PS3 Legacy: fit / fit / append (3888→5616); installed packages stock-shaped, pixels marked, restore exact |
| real gen7 EB (`every_real_gen7_eb_archive…`, xbox-nhllegacy + ps3-nhllegacy, 312 / 308 MB copies) | **1 passed** (2 targets; 240 s / 235 s; fit-in-slot on both members; restore byte-exact) — before the growth member was added; the full 9-target queue with the growth member is in the background (§7) |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-api --test big4_asset_member_route -- --include-ignored` | **6 passed** — Xbox 08 `data0.big`, Xbox 10 `fe.big`, PS3 08 `data0.big` (raw inner: all fit), PS3 10 `fe.big` (inner RefPack: small appends); both files restored; NHL 07: growth refused with the boot.big reason, a tampered `.bh` refuses at parse, a missing `.bh` refuses and drops the archive route from the listing |
| real PS4 (`every_real_ps4_inxp10_archive…`) NHL 15 (337 MB) | **1 passed** in 219 s: logo fit (48717→41117), trophy fit; restore exact |
| real PS4 NHL 16 / 17 / 18 / 19 (292 MB / 851 MB / 1.77 GB / 2.99 GB) | **passed** (198 s / 620 s / 1 200 s / 2 088 s): logo + `large_league_logo` private copies (both shared with their `_fr` twins), siblings byte-identical, two table rows + archive-size word changed, installed members decode to the marked pixels in native chunkzip framing, restore byte-exact |
| real PS4 NHL 20 / 21 | in progress (background) |
| `cargo test -p studio-domain --test league_logos --test league_imagery --test asset_member_undo`; `-p studio-api --test asset_member_route --test loose_texture_route` (default) | all passed |
| `cd app && npx tsc --noEmit -p . && npx vitest run src/components/ArchiveMemberReplace.test.tsx src/components/LeagueImageryCard.test.tsx` | clean / 12 passed |

**Performance note (limitation, not a defect of correctness):** the copied-target save of a multi-GB
archive is slow — NHL 19's 3 GB took 35 minutes (bind hashes every entry, stage streams, verify re-reads,
apply re-verifies). A user saving into NHL 20's 3.76 GB `inxp_10.big` should expect ~40 minutes. Not
optimised in this campaign.

## 5. Per-target coverage after C2–C4 (matrix 1.4 "place the replaced package inside the packed archive")

| Family | Archive | Status |
|---|---|---|
| PS4 15 | `inxp10.big` (raw PNG members) | impl+tested on the **real** archive |
| PS4 16–19 | `inxp_10.big` / `inxp_10-010.big` (chunkzip GNF, shared spans → private copy) | impl+tested on the **real** archives |
| PS4 20–21 | same | impl+tested (21: real member in a synthetic archive, D8); real-archive runs in progress |
| X360 11 | `fe.big` (EB, BIGF packages raw) | impl+tested (synthetic with real packages by identity of the layout); real archive queued |
| X360 12–15 / Legacy, Recomp | `cache.big` | impl+tested on the **real** Legacy archive; 12–15 queued (same code path, real archives) |
| PS3 12–15 / Legacy | `cache.big` (+ loose copy disclosed) | impl+tested on the **real** Legacy archive; 12/15 queued; 13/14 by identity (no `cache.big` in the survey for them — **research unknown whether they ship one**) |
| X360 07/08, PS3 08 | `data0.big` + `.bh` | impl+tested (synthetic pairs with real packages); real 08 archives (2 GB) queued; NHL 07: same-length only (policy) |
| X360 09/10, PS3 09/10 | `fe.big` + `.bh` | impl+tested (synthetic); real 09/10 queued |
| Disc images | — | unchanged: same-length only |

## 6. Limitations / open

- NHL 07 `data0.big`: the policy refuses growth but still allows a fit that changes the `.bh` size/decoded
  words — strictly also a cascade into `boot.big`; tighten to "refuse any `.bh` change" (open, small).
- BIG4 ordered reflow not implemented; append is the only growth placement (disclosed).
- PS3 NHL 13/14 `cache.big` presence unmeasured (the research survey measured 12/15/Legacy).
- Whole-group PS4 replace not offered.
- Runtime acceptance of every placement: user cards (R2-BIG4-1, R2-EB-1, R3-IMG-2).

## 7. Temporary artifacts / processes

- Background sweeps (this session): `c2_run.sh` → `c2_real_ps4.log`; `c3_run.sh` → `c3_real_gen7.log`
  (queued behind C2); `c4_run.sh` → `c4_real_big4.log` (queued behind C3), all in the session scratchpad
  `H:\TEMP\claude\e--Repositories-nhl-modding-studio\6e40fb65-…\scratchpad\` with copies of the test
  binaries (`c2_asset_member_route.exe`, `c3_gen7_route.exe`, `c4_big4_route.exe`) so cargo rebuilds do
  not disturb them. Each test copies one archive into `%TEMP%\nhlms-*` and removes it by `Drop`. The
  scripts, logs and binary copies are disposable after the results are recorded in the final receipt.
- Scratchpad edit scripts `c2_real_ps4_test.rs`, `c3_*.rs`, `c4_*.py|rs` — disposable.

## 8. Runnable commands

```sh
cargo test -p tdb-archive-big --lib big4
cargo test -p studio-api --test gen7_asset_member_route --test big4_asset_member_route
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test big4_asset_member_route -- --include-ignored
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test gen7_asset_member_route -- --ignored     # synthetic + real Legacy cache.big (default targets)
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures NHL_STUDIO_PS4_TITLES=nhl15 cargo test -p studio-api --test asset_member_route every_real_ps4 -- --ignored --nocapture
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures NHL_STUDIO_BIG4_TARGETS=xbox-nhl10 cargo test -p studio-api --test big4_asset_member_route every_real_big4 -- --ignored --nocapture
```
