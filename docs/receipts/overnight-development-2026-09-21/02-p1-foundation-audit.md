# Receipt 02 — Priority 1: foundation audit of the 20 September deliveries

**Developer-verified; not Codex-accepted; not a Codex acceptance of anything; nothing game-verified.**
Branch `feat/phase5-asset-tooling`, baseline `8d60a6d`. Nothing staged, committed, pushed, launched or
written live; private fixtures under `E:\nhl-studio-fixtures` read only; mutation paths exercised on
synthetic data, committed anchors and temp copies. Claim: `dev-p1-foundation-audit`.

## 1. Plain-language summary

Seven foundations delivered overnight on 20 September were read against their contracts and probed
with malformed inputs and failure cases. Five real defects were found and fixed: the dictionary
rebuild turned a *missing* key into an *empty* key; the old-archive index writer would accept a
sidecar length word that did not describe the bytes it placed, and could be made to allocate gigabytes
from a corrupt header; the Xbox 360 texture resize wrote a wrong pitch byte for DXT textures narrower
than 128 pixels and silently mis-encoded heights that are not multiples of 8; the PS3 DXT logo writer
always emitted a full mip chain (a shape those logos do not ship); and — the most consequential — when
the roster half of a mirrored database edit was refused, the main-database half was "undone" onto the
redo stack, so a later Redo could re-apply half an edit. All are fixed with regressions that fail
against the old code. The package re-plan, the team-logo family table and the whole-group /
private-copy scope logic held up; they gained bounds checks and regressions only.

## 2. Findings and fixes, per foundation

| # | Foundation | Read | Defects found → fixed | Regressions added | Scope actually verified |
|---|---|---|---|---|---|
| 1 | Canonical dictionary rebuild (`tdb-loc::rebuild`, `growth` planner) | serializer, tree builder, planner refusals, accounting | **D1** a row with a null *key* pointer was re-emitted as an empty key (`unwrap_or_default`) — a silent semantic change; keys are now `Option` end to end and null stays null | `tests/rebuild.rs` +5: null key preserved (tree kept and forced), changed bytes between plan/apply refused (with and without CRC repair), CRC-corrupt input refused at planning, key pointer into the tree / past the pool = parse error, record count past the u16 words refused | the 25 committed anchors (existing no-op / glyph / sharing sweeps still pass: 12/12); the private 175-file sweep **not rerun** (no byte path of a non-null-key file changed) |
| 2 | BIG4 + `.bh` writer (`tdb-archive-big::big4`) | bind, plan, stage, verify, synth | **D2** `.bh` word 8 (decoded length) was taken from the caller unchecked: a raw member with a non-zero word or a RefPack member with the wrong word was placed as requested (the domain computes it correctly, the engine did not enforce the measured contract). **D3** a corrupt entry-count / index-size header word led to `Vec::with_capacity(count)` (capacity overflow) or a multi-GB allocation before any check | `big4::tests` +4: malformed header words refused before allocation (count, index size, sidecar count, unterminated name); decoded word must be the RefPack declared length or 0 (identity no-op still precedes the check); two adjacent fit-in-slot edits stay in their own slots (grow into own slack + shrink, 129 refused with the slack named); a previously appended archive binds, its re-pointed entry's slot runs to EOF, a second append works, dead bytes stay, a drifted source is refused at staging | synthetic pairs only (10/10); the real BIG4 sweep logs of 20 Sep are unchanged evidence (the added checks refuse, never re-place) |
| 3 | Span-preserving re-plan (`tdb-archive-big::replan`) | parse, follower placement, assembly, verification | none in the write path; **D4** count word unbounded before `with_capacity` | `replan::tests` +2: malformed packages refuse (count, span past end, overlap, data_offset inside the table, truncations, bad magic); replacing the **first** member moves every follower consistently for stride 0 / 64 across six lengths | synthetic X-C packages (7/7); the 129-package identity sweep (`every_gen7_package_replans_identically…`) rerun on the private fixtures: passes |
| 4 | XPR2 / DDS resize (`tdb-rx2::xpr::replace_resized`, `studio-domain::assets` DDS paths) | both resize paths, the parser's own descriptor model, the PS3 DDS writers; **census of 82 real XPR2 members and 99 real PS3 DDS members** in the private fixtures (developer probe, read-only) | **D5** DXT resize left DW7's second byte at its stored value (wrong for widths < 128 — the C6 `leaguelogossmall` 64-wide case wrote a stale pitch that only our own parser ignores) and clamped heights not divisible by 8 into a wrong descriptor; **D6** the PS3 DXT DDS writer regenerated a *full mip chain* with the mip flags set for every DXT DDS — the NHL 12–Legacy `leaguelogossmall` DXT5 logos ship one level (`mips == 0`, body = base only; measured on 6 fixture members), so every same-size replace of them changed the member's shape (the C1 sweep compared pixels only). Fix: one measured rule for both XPR2 paths — `DW7 = 0x80|w/128, (2·w)&0xFF` (format-independent; 82 members at 32/64/128/256 wide + the three texlib ground truths at 256/512/1024 agree), `DW9 = (h/8-1)<<16 | 0xE000 | (w-1)`, widths limited to the measured powers of two 32…1024, heights to multiples of 8 ≤ 2048, else `ResizeUnmeasured`; the DDS writer keeps the shipped 128-byte header verbatim apart from height/width/linear-size/mip-count and preserves the shipped level structure (base only ↔ full chain for the new size ↔ declared count clipped) | `league_logo_packages.rs` +2 (+1 `--ignored`): **XPR2 resize oracle** — for all 82 real members a same-dims `replace_resized` reproduces the stored header byte-for-byte and a resize to each of the six measured shapes yields the DW7/DW9 words a real member of that shape carries (DXT5 member checked, incl. the 64-wide byte `0x80`); unmeasured shapes refuse; **PS3 DXT DDS mip structure** — base-only source stays base-only with flags / reserved / pixel-format / caps verbatim, full-chain source keeps a full chain; the real-fixture dimension-change test now asserts the resized `leaguelogossmall` member's structure | private fixture sweeps rerun: `league_logo_packages` all 6 `--ignored` cases pass (incl. the C6 Apt sweep and dimension change); `tdb-rx2` default 112 passed. **Missing fixture:** the `legacy/playerheadssmall_p1512.xpr2` portrait is absent from the fixture root, so the five portrait-XPR2 `--ignored` cases (incl. `resize_to_same_dims_preserves_header`) **fail loudly as designed** (missing-fixture, not a regression) — the 82-member oracle covers the same header identity claim on the logo/imagery/team-logo members instead |
| 5 | Public multi-cell edits and atomic rollback / mirroring (`set_cells_int`, workspace mirror) | session `set_cells_int` / `_rows`, the workspace mirror pattern at every coordinated op | **D7 (pre-existing, every mirrored op)** when the roster mirror failed, the canonical half was rolled back with `undo()`, which pushes it onto the session's **redo** stack while the workspace redo stack is untouched; with an earlier real undo on the workspace stack a Redo then re-applied the half-written op as if it were the earlier one (reproduced: `(1, 1)` phantom depths). Fix: `EditSession::abort_last()` (pop, no redo push) + `Workspace::abort_canonical()` (abort + clear the workspace redo stack, since the session's transaction already cleared its own) used at all 13 `Some(Err)` rollback sites, the three divergence sites and `roll_back_both`; the real `undo()` stays only in `Workspace::undo`. Also: `set_cells_int` now refuses an empty write set and a field named twice in one op (the API refused empty; the domain silently applied duplicates last-wins) | `tests/workspace.rs` +2: refused mirror leaves nothing redoable and history untouched — asserted through `set_cells_int`, `set_field`, `set_cells_int_rows` and `set_captaincy` with a prior undone op on the stack (discriminates: fails `(1, 1)` against the old rollback), then a good mirrored op undoes/redoes on both files; empty / duplicate refusals and one-step undo. **Fault injection seam** added (`EditSession::inject_fault`, `Workspace::inject_roster_fault`, `#[doc(hidden)]`, off by default): the coherence and referential gates are delta gates, so no stock pair refuses an unrelated mirrored edit — the seam is the deterministic way to reach the rollback path | NHL 14 PS3 nhlng + ROSTER anchors (temp copies), 12/12 |
| 6 | Team-logo family selection (`studio-domain::team_logos`) | family table vs the census manifest (id ranges per family, 27 sources) | no mismatch in the table; one disclosure gap: the closed `teamlogosreflection` family (30 members, art ids 0–29 on every 13–Legacy source) was offered for every club, so a club with art id ≥ 30 got a loose path, an archive route and an existence answer only when a root was given. Now the family carries its measured ceiling: beyond it `exists = Some(false)`, no archive route, a note naming the range | `tests/team_logos.rs` +1: inside vs beyond the ceiling on Legacy X360; team-sized families untouched; the census test (`--ignored`) still passes | Legacy X360 anchor + the census manifest |
| 7 | Whole-group / private-copy scope and undo (`Workspace::asset_member_*`) | preview / apply / conflict / undo / rebind; the LIFO invariants | none — the ownership rule, in-place supersede and LIFO restore are consistent | `tests/asset_member_undo.rs` +1: whole-group `l100` → private `l100` supersedes → private `l5` accepted → undo `l5` (a held review is refused as stale) → undo `l100` restores the group and a private `l5` is refused again → undo the group frees `l5` | synthetic PS4 archive over the NHL 15 anchor, 5/5 |

Not reopened: the R1 F1 whole-group conflict corrections (preserved; the new case exercises them).

## 3. Changed identities (SHA-256 prefixes)

| Path | Hash | Change |
|---|---|---|
| `crates/tdb-loc/src/rebuild.rs` | `66a5046ca8c775a0` | null keys preserved (Option end to end); verification message |
| `crates/tdb-loc/tests/rebuild.rs` | `38559e413b954622` | +5 malformed-input cases |
| `crates/tdb-archive-big/src/big4.rs` | `b4c954d66373c82e` | header bounds before allocation; `.bh` decoded-word consistency (`DecodedWordInconsistent`, `HeaderShape`); +4 cases; synthetic raw members now carry word 0 |
| `crates/tdb-archive-big/src/replan.rs` | `6e0d52f69eec502d` | count bound; +2 cases |
| `crates/tdb-rx2/src/xpr.rs` | `5a3248b3eab0502e` | one measured geometry rule for both resize paths; measured width set; refusal text |
| `crates/tdb-archive-codec/src/dds.rs` | `9d6c7f61c0345aa1` | `DdsFormat::linear_size` public |
| `crates/studio-domain/src/assets.rs` | `68d7a731cc635013` | `PortraitSlot::Dds` carries header + mips; level structure preserved; dead helper removed; docs |
| `crates/studio-domain/src/session.rs` | `72eec884eb9e7214` | `abort_last`; `inject_fault` seam; `set_cells_int` empty/duplicate refusals |
| `crates/studio-domain/src/workspace.rs` | `e5c26a5099643fac` | `abort_canonical`; every failure rollback routed through it; `inject_roster_fault` |
| `crates/studio-domain/src/team_logos.rs` | `1ffd7dfe8b556d93` | `Family::max_id`; reflection ceiling 29 |
| `crates/studio-domain/tests/league_logo_packages.rs` | `9640306e2f44dc72` | XPR2 resize oracle (`--ignored`), DDS mip-structure case, resized-DDS assertions, refusal texts |
| `crates/studio-domain/tests/workspace.rs` | `50210abae056eff1` | +2 cases |
| `crates/studio-domain/tests/team_logos.rs` | `df7ea6f7215fb6f3` | +1 case |
| `crates/studio-domain/tests/asset_member_undo.rs` | `d96886d166e8008b` | +1 case |

No DTO changed (bindings check expected unchanged; run in §4). No `defs/`, anchors, fixtures or installs touched.

## 4. Checks executed (developer)

| Command | Result | Log |
|---|---|---|
| `cargo test -p tdb-loc --test rebuild` | 12 passed, 1 ignored (private sweep) | in aggregate |
| `cargo test -p tdb-archive-big --lib big4` / `replan` | 10 / 7 passed | in aggregate |
| `NHL_STUDIO_FIXTURES=… cargo test -p tdb-rx2 --lib xpr -- --include-ignored` | 5 passed; **5 failed = missing fixture** `legacy/playerheadssmall_p1512.xpr2` (absent from the root; fails loudly by the C0-FX1 contract) | [p1-xpr-fixture-missing.log](../../validation/evidence/overnight-development-2026-09-21/logs/p1-xpr-fixture-missing.log) |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-domain --test league_logo_packages -- --ignored` | 6 passed (Apt sweep 141, dimension change, XPR2 resize oracle 82 members, replan identity, gen7 + PS4 round trips) | [p1-logo-fixture-sweeps.log](../../validation/evidence/overnight-development-2026-09-21/logs/p1-logo-fixture-sweeps.log) |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-domain --test team_logos -- --ignored` | 1 passed (census) | in the same log |
| `cargo test -p studio-domain --test workspace` / `asset_member_undo` / `team_logos` | 12 / 5 / 3 passed | in aggregate |
| discrimination runs (old code restored temporarily, then reverted from a scratch copy): R3 colour (3 fail), mirror rollback (`(1, 1)` phantom depths) | as stated | not retained (scratch copies deleted) |
| `cargo test -p tdb-loc -p tdb-archive-big -p tdb-archive-codec -p tdb-rx2 -p studio-domain -p studio-api` (default mode, one run) | see §4a | [p1-aggregate-default.log](../../validation/evidence/overnight-development-2026-09-21/logs/p1-aggregate-default.log) |
| `cargo run -p studio-api --bin export-bindings -- --check` | see §4a | [p1-bindings-check.log](../../validation/evidence/overnight-development-2026-09-21/logs/p1-bindings-check.log) |

### 4a. Aggregate result

Run 1 (`p1-aggregate-default-run1-one-failure.log`, retained complete): **23 test binaries passed; 1 failed** —
`studio-domain --lib` 175 passed / 1 failed: `assets::tests::ps3_dds_portrait_write_roundtrip_synthetic`,
which built a synthetic *single-level* DDS source and asserted the old "always a full chain" rule that
D6 corrected. Diagnosed, not rerun blindly: the test now builds the source the way EA portraits ship
(full chain) and asserts the chain is preserved; `cargo test -p studio-domain --lib` rerun: **176
passed, 0 failed, 2 ignored** (`p1-studio-domain-lib-rerun.log`). Per binary (run 1): `tdb-loc`
lib 7 / dictionaries / growth / locale_coverage / private / rebuild 12 / retext; `tdb-archive-big` lib
75 (+ `ps4_real_archives` 4 ignored); `tdb-archive-codec` 45 + 4 (8 ignored); `tdb-rx2` 112 (26
ignored); `studio-domain` 176 lib + every integration binary (asset_member_undo 5, league_logo_packages
3, team_logos 3, workspace 12, …); `studio-api` 31 lib + 22 integration binaries incl.
`big4_asset_member_route` 1 (6 ignored), `asset_member_route` 5, `copied_target` 15, `registry` 7.
Ignored = fixture-gated, never counted as passing. Bindings: "Bindings are in sync" (`p1-bindings-check.log`).

Not run: `cargo fmt --check` / `clippy -D warnings` (repo-wide baseline defects, C0.2 — not repaired here); the full workspace `cargo test --all`; the `--ignored` archive sweeps of 20 Sep (`ps4_real_archives`, gen7 EB, BIG4 real pairs — the engine changes add refusals and change no accepted byte); any launch. Ignored cases are ignored, never passing.

## 5. Omissions / open

- D5/D6 change bytes of files the 20 September real sweeps wrote (XPR2 headers at widths < 128 in DXT only; every PS3 DXT DDS member). The retained sweep logs of 20 Sep remain valid for what they asserted (decoded pixels); they did not assert the shapes corrected here.
- The DW7 rule's DXT1 / DXT3 width behaviour is covered by the texlib ground truths (512 DXT1, 256 / 1024 DXT3), not by a resized package written and decoded through the package path.
- The fault-injection seam is `#[doc(hidden)]` production code; Codex may prefer a `cfg(test)`-gated variant (it cannot be `cfg(test)` today because the integration tests live in a separate crate).
- Runtime acceptance of every corrected shape is unchanged: user cards (MORNING-REVIEW).

## 6. Cleanup

Scratchpad: `xpr2_dw_census.py` (the read-only census probe, 4 KB — retained until closeout as the source of the DW7/DDS tables above, then deleted; its numbers are recorded in §2 and in the oracle test's expectation table), `big4_audit.py` (edit script, deleted), scratch copies of `ColorField.tsx` / `workspace.rs` (deleted). No temp directories left (`nhlms-logos-*` / `nhlms-ws-*` are removed by the tests). No processes.

## 7. Next

Priority 2 — actionable League work: A (archive/package completion: BIG4 ordered reflow against the research oracle), then B–E; the research lane has meanwhile published the gen7 commentary read-only contract rev. 2 (2026-09-21 handoff 03, claim `research-r24-ctx0-structure`, `ready`) for E.
