# Receipt 10 — Final closeout, developer lane, overnight 2026-09-20

**Developer-verified; not Codex-accepted; nothing game-verified.** Baseline `8d60a6d`, branch
`feat/phase5-asset-tooling`. Nothing was staged, committed, pushed, deployed or claimed as accepted.
Closed 09:10 MDT with no background job or orphan process left running.

> **Correction (OVERNIGHT-DEV-20260920-R1, [record](../../validation/overnight-dev-20260920-r1-corrections.md)):**
> the R1 review did not accept the campaign in full. Two P1 defects were found and are fixed
> (whole-group scope conflicts, F1; colour drafts migrating between teams, F2). Statements of this
> receipt that are corrected there: §2/§9 "PS4 vault slug binding unresolved" (the typed links are
> research-parsed — 2026-09-19 handoff 07, 2026-09-20 handoff 11 — not a Studio reader/writer or a
> runtime selector); §9 "chunklzx decoder promoted … the boot-write probes crashed on raw
> substitution" (the framing is research-pinned, Studio's decoder is not corrected to it, and the
> substitution claim has no recorded evidence — loader tolerance is card R2-LZX-1); "research 07 §1"
> and "research 06 rev. 2 / 11" resolve to the 2026-09-20 handoffs 07 and 11 and the 2026-09-19
> handoff 06 rev. 2 (two campaigns have a handoff 07). §8's scratchpad logs are now durable:
> [evidence manifest](../../validation/evidence/overnight-development-2026-09-20/MANIFEST.md); the
> scratchpad edit scripts are deleted. Phase 2 cells: [provisional matrix](../../validation/phase-2-coverage-matrix-2026-09-20.md).

## 1. Plain-language summary

Overnight, the developer lane finished the Phase 1 (Leagues) work that had a contract and opened
Phase 2 (Teams). On the League side: the loose logo replace was proven with visibly different images
on every real package and member (and its writer was corrected so packages keep their shipped
layout); logos and imagery can now be placed inside the packed archives of **every** family — PS4
15–21 (7/7 real archives), Xbox 360 / PS3 11–Legacy (9/9 real archives) and the old BIG4 + `.bh`
pairs of NHL 08–10 (6/6 real pairs, NHL 07 refused by policy) — with the copied-target save and a
byte-exact restore; the dictionary got the canonical rebuild that lets new characters be written;
league logos can be resized on the Xbox 12–Legacy and PS3 12–Legacy families; the PS4 locales (99
dictionaries) and NHL 15's patch database are covered. On the Team side: the split names and the
team colours are editable on every title that stores them (the packed colour word's byte order was
measured, not assumed), every logo family a title ships for a club is listed with preview / replace /
archive placement (the family table measured on all 27 sources and proven on the real Legacy
archives), and the team's localized names were proven on the PS4 dictionaries. One real defect was
found and fixed by the sweeps (PS3 NHL 10's archive header overstates its index). What remains is
either research-blocked (audio formats, gen7 `rendering/*` compression, PS4 vault slugs, the jersey
contract the research lane holds) or a runtime question for the user's cards.

## 2. Completed packages and remaining gaps

| Package | Receipt | Result |
|---|---|---|
| C0 R1 verification + foundation audit | 01 | R1 holds; D6 layout defect found → fixed in C1 |
| C1 loose-route closure (changed pixels, non-logo imagery, span-preserving writer) | 02 | delivered |
| C2/C3/C4 archive placement on every family (PS4, gen7 EB, BIG4 + `.bh`) | 03 (+05/08/09 for the sweep results) | delivered; real sweeps **7/7, 9/9, 7/7** |
| C5 localization canonical rebuild (LOC-W3 C / O-G) | 04 | delivered |
| C6 different-dimension logos (XPR2 64/256, PS3 DDS) | 05 | delivered; RW4 / RW4ps3 / GNF blocked with reasons |
| C7 whole-group scope, PS4 locales, NHL 15 patch DB, Priority-5 blocker record | 06 | delivered |
| C8 Phase 2.1 team identity: names + colours, one-op multi-cell write | 07 | delivered |
| C9 Phase 2.4 team logo families (census, card, 285-member sweep, real Legacy placement) | 08 | delivered |
| C10 BIG4 binder fix (PS3 NHL 10), Phase 2.3 PS4 team-text parity, 2.2 parity measured | 09 | delivered |

**Remaining gaps (exact):** receipt 06 §6 (Priority 5: NHL 07 league identity, row creation, inferred
groups, tournaments, gen7/PS3 rules, schedule formats, audio); receipt 05 (RW4 / RW4ps3 / GNF
resize descriptors); receipt 03 (BIG4 ordered reflow not built, NHL 07 `boot.big` cascade, disc images
same-length only); Phase 2: `artid` / `index` re-identification (cascade contract, research 07 §1),
2.5 arena/ice (gen7 `rendering/*` members are chunklzx — decoder unpinned, no encoder; PS4 vault slug
binding unresolved, research 06 rev. 2 / 11), 2.6 kits (research claim `r8-jersey-triple-contract`
active — not entered), 2.7 audio (format blocked). Which screen consumes which family / colour / name
is research unknown everywhere → user cards.

## 3. Operation-by-title coverage (current, developer level)

Phase 1 cells: `docs/validation/phase-1-coverage-matrix-2026-09-18.md` (C1–C7 cells) — every
preview / replace / placement / dictionary cell is impl+tested or blocked with the codec or contract
named; no cell is game-verified. Phase 2 cells (until Codex opens a Phase 2 matrix): receipt 07 §6
(names / colours per title; `artid`/`index` not impl), receipt 08 §6 (logo families: list + loose
replace impl+tested on every title; archive placement impl+tested on the real Legacy `cache.big`
both consoles, by identity elsewhere; NHL 07 blocked by policy; resize as Phase 1), receipt 09 §1
(2.2 surfaces Editable everywhere the tables exist; NotApplicable on 07–10 for picks/jerseys, 07 for
team equipment) and §3 (2.3 team text on gen7 + every PS4 English dictionary).

## 4. Aggregate test results (default suites, this tree, 08:07–09:08)

| Crate / app | Result |
|---|---|
| `tdb-archive-big` | 69 passed, 1 ignored (+ `ps4_real_archives` 4 ignored) |
| `tdb-archive-codec` | 45 + 4 passed, 8 ignored |
| `tdb-loc` | 33 passed, 10 ignored (locale sweeps fixture-gated) |
| `tdb-rx2` | 112 passed, 26 ignored (private fixtures, C0-FX1 contract) |
| `studio-domain` | **386 passed, 0 failed, 20 ignored** (rerun after the `asset_member_undo.rs` signature fix, §6) |
| `studio-api` | **107 passed, 0 failed, 21 ignored** |
| frontend (`vitest`) | **19 files, 136 tests passed**; `tsc --noEmit` clean |
| bindings | `export-bindings --check` in sync (119 roots, 227 files); `bindings::tests` 25 passed |

Fixture-gated (`--ignored`) cases executed tonight and passing: loose route gen7 77 + PS4 34
(C1), imagery 64 + 22 (C1), team logos 284 + asserted refusal (C9), Apt/re-plan sweeps 94/47 (C6),
PS4 locales 99 (C7), NHL 15 patch DB (C7), team-logo census 27 sources (C9), PS4 team text 14
dictionaries (C10), synthetic BIG4 pairs + NHL 07 refusals (C4/C10), real archives PS4 7/7, gen7 EB
9/9, BIG4 7/7 (incl. the PS3 NHL 10 rerun), Legacy team logos 2/2.

**Omissions (not run):** the full workspace `cargo test --all` (only the six touched crates ran);
`cargo fmt --check` / `clippy -D warnings` (repo-wide baseline defects, C0.2 — not repaired here;
only the four new frontend files and two new Rust files were formatted); `zz_diag`; the mount-gated
Xbox 360 probes in `studio-domain::assets` (they return early — pre-existing silent skips, recorded
in C0.2 §5.4); no headmesh work (external stream); no game or emulator launch. Ignored tests are
counted as ignored, never as passing.

## 5. Areas for Codex review first

1. **`tdb-archive-big::replan` and the loose writer switch** (receipt 02) — every logo/imagery
   write now goes through the span-preserving re-plan; the 16-byte opaque tail correction to
   research 02 §2.
2. **BIG4 + `.bh` writer and the binder correction** (receipts 03, 09) — append-and-retarget is a
   shape no stock archive exhibits; `data_start` from the parsed directory (PS3 NHL 10).
3. **Canonical dictionary rebuild** (receipt 04) — serializer C, the `bits/8 + 1` stream-length
   rule (corrects research B7), class A/B order rules, explicit opt-in.
4. **`set_cells_int` as a public command** (receipt 07) — one op / one undo for colour triples;
   the packed byte order proof.
5. **Team logo family table** (receipt 08) — the offered/not-offered rule (team-sized counts) and
   the two families of NHL 15 (`playnow/teamlogos`; the un-offered `capilano` DDS twin).
6. **Whole-group scope** (receipt 06) — the sibling-row rewrite in `stage.rs`.
7. Whether the Phase 2 matrix should be opened now (receipts 07–09 carry the first cells).

## 6. Defects found overnight (fixed in the tree)

- D6 packages rebuilt in a non-shipped layout (C0 → C1).
- BIG4 append broke offset order — recorded, not refused (C4).
- NHL 07 `.bh` cascade refusal keyed by title (C4).
- PS3 NHL 10 `fe.big` declared index size overruns the first payload line (C10).
- `studio-domain/tests/asset_member_undo.rs` not updated for C7's `whole_group` parameter — caught
  by the closeout aggregate, fixed (`9aec99711f5d483e`), 3 passed.
- The team page's Misc-fields list would have shown `_g`/`_b` colour components twice (C8 follow-up,
  `app/src/pages/TeamDetail.tsx` `ca64acbc9bde887a`).

Final hashes of files reformatted after their receipts: `crates/studio-domain/src/team_logos.rs`
`bdba58456cc78900`, `tests/team_logos.rs` `7f9d9e27ff6fe565`, `app/src/components/ColorField.tsx`
`73c0194ce8a7dd9e`, `ColorField.test.tsx` `4ecc1cba75de4389`, `TeamLogoCard.tsx` `a5add71771893952`,
`TeamLogoCard.test.tsx` `63fcc763c268bfd8`.

## 7. Batched user review cards

`MORNING-REVIEW.md`: desktop V-6 … V-11; runtime R2-EB-2 (PS4 15–21), R2-EB-1 (Legacy), R2-BIG4-1
(NHL 08–10, PS3 10 now runnable), LOC-W3 R-C, R6-DIM-1, T-1 (colours on screen), T-2 (split names),
T-3 (which screen draws which logo family). The 19 September cards remain valid. NHL21-E1 stays
closed.

## 8. Cleanup and retained dependencies

- Removed: every copied test exe and runner script, the C6 probe temp roots under `H:\TEMP`
  (`nhlms-logos-apt/dims-*`, six), the throwaway probe tests (`zz_*`), `__pycache__`.
- Retained in the session scratchpad (disposable, 343 KB): the `c*_*.py` edit scripts, `ed.py`,
  and `logs/` (the five real-archive sweep logs + the two aggregate logs — the evidence behind §4
  and receipts 03/08/09; copy them into `docs/validation/` if Codex wants them durable).
- Retained private fixtures (regenerable by the committed extractors under
  `local/runs/overnight-development-2026-09-20/`): `imagery/` (C1), `loc-ps4/` (C7, 250 MB),
  `teamlogos/` (C9, 21 MB) under `E:\nhl-studio-fixtures`.
- Pre-existing, not touched: `H:\TEMP\nhlms-league-campaign-all-97012`, `nhlms-logos-gen7-*` (19 Sep),
  `nhlms-p3-op-*`; the untracked `--print` CSV at the repo root (18 Sep); the 649 unrelated dirty
  entries (now 667 with tonight's files).
- Disk at close: E: 24 GB free, H: 32 GB free — the multi-GB archive copies need `%TEMP%` on H:.

## 9. Exact continuation task

If Codex accepts receipts 07–09: open the **Phase 2 coverage matrix** from their §6/§1 tables, then
the next developer package is **2.4/2.5 rendering-side team art on gen7** — but only once one of
two contracts exists: (a) the `chunklzx` decoder promoted (its oracle test is mount-gated and the
public pass-through still reports unsupported) **and** an encoder or a loader-accepted substitute
codec (the boot-write probes crashed on raw substitution), or (b) the PS4 vault slug → `ge_texture`
binding for centre-ice / rink art. Until then the actionable Phase 2 work is the **`artid` / `index`
re-identification** with a refuse-or-cascade contract accepted from research 07 §1, and, when the
research lane releases `r8-jersey-triple-contract`, **2.6 kits** on top of the existing jersey
tables and the C9 family pattern. Runtime cards T-1 … T-3 and the Phase 1 cards are the user's.

## 10. Runnable commands (whole campaign)

```sh
cargo test -p tdb-archive-big -p tdb-archive-codec -p tdb-loc -p tdb-rx2 -p studio-domain -p studio-api
cargo run -p studio-api --bin export-bindings -- --check
cd app && npx tsc --noEmit -p . && npx vitest run
# fixture-gated sweeps: see the "Runnable commands" of receipts 02–09 (NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures)
```
