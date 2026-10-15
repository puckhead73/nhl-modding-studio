# Receipt 09 — Closeout of the 22 September 2026 extended developer campaign

**Everything in this campaign is developer-verified only. Nothing is Codex-accepted, nothing was
desktop-exercised, nothing is game-verified; no commit, staging, push, reset, branch change or clean
was made; no game or emulator was launched; no live install, image, archive, save, anchor or profile
was written.**

Branch `feat/phase5-asset-tooling`, HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` — **unchanged
from start to finish**. The working tree the campaign inherited (721 entries, 187 modified) is
preserved; it now holds 733 entries, the difference being this campaign's own new files. NHL 21 E1
stays accepted and closed. Projects / Builds untouched.

## 1. What users can now do

- **Resize PS4 textures with real evidence behind it.** Six shipped NHL 21 / NHL 16 textures now
  round-trip through the codec and through the **API route the Replace… dialog calls** (the dialog
  itself is not exercised by these tests) — and halving one then growing it back reproduces EA's own
  header byte-for-byte. Header reproduction says nothing about the pixels: a halve → regrow is a lossy
  downscale then upscale, and the original texels do not survive it. (Whether a console *loads* a
  resized texture is still a card for the user.) *(Wording corrected 22 Sep 2026 per the coordinator
  review.)*
- **Get a clear refusal instead of a crash or a wrong file.** An impossible size is refused with the
  value and the limit named, before anything is built; a two-texture PS4 file says "multi-stream"
  instead of blaming a size mismatch; NHL 07's container says its layout is the reason.
- **Read commentary contexts on PS4.** The Contexts card covered Xbox 360 / PS3 only; it now covers
  NHL 15 → 21 as well, with the same presentation.
- **Understand an arena's settings.** Every venue field is grouped and glossed, PS4 colour zones show
  as swatches, fields that never vary are folded away, and unexplained fields are marked read-only
  with the reason. On NHL 17–21 the goal horn reads as the club or generic horn it names.
- **A team's kit rows are joined to their textures in the backend** (implemented and tested; **not
  yet user-facing** — no API seam or UI existed at closeout, as §8 and receipt 08 §6 say). Each
  kit row resolves its jersey, pants and socks on the three titles measured, plus the spare kits
  shipped but unused. *(Corrected 22 Sep 2026 per the coordinator review: the original bullet read
  as if users could already see this; the user-facing surface is receipt 10's package.)*
- **Run the test suite that was silently empty.** The private fixture folder held none of the
  `tdb-rx2` fixtures, so 26 cases failed loudly for want of inputs. All six fixtures were restored
  by provenance; those 26 now pass.

## 2. Packages, with per-title / per-platform coverage

| # | Package | Coverage reached |
|---|---|---|
| [01](01-r2-descriptor-validation-order.md) | R2: dimension validation before size arithmetic | codec-wide (all `RW4xb2` titles) |
| [02](02-p1-resize-evidence.md) | Real GNF round trips, the fixture root, the `DW8` join | **PS4 NHL 21 + NHL 16** (real members, codec + public route); X360 Legacy / 11 / 09 (`DW8` join 22 401 + 16 981 + 13 836, 100 %) |
| [03](03-p2-nhl07-08-descriptor-generation.md) | The `DW0 = 3` generation lifted | **X360 NHL 07** (9 388 / 9 390, 2 cubes) and **NHL 08** (9 744 / 9 744, 110 undecodable asserted); NHL 09 gained 243 |
| [04](04-p2-ps4-context-listing.md) | Contexts listing on PS4 | **all seven PS4 titles**, census reproduced exactly |
| [05](05-p2-arena-venue-inventory.md) | Venue-parameter inventory | four era signatures: PS3 NHL 07, PS3 NHL 08, X360 Legacy, PS4 NHL 21 |
| [06](06-p3-pool-head-floor.md) | Pool-head floor by era | X360 NHL 09 / 11 / 13 / 15 / Legacy |
| [07](07-p2-goalhorn-catalogue-join.md) | PS4 goal-horn catalogue | **NHL 17 and NHL 21** real installs (46 and 150 club horns, 0 unresolved) |
| [08](08-p2-kit-selector-join.md) | Gen7 kit → texture join | **X360 NHL 11 / 15 / Legacy** (627 / 659 / 703, 100 %) |

## 3. Defects found and fixed

| Defect | Found by | Regression |
|---|---|---|
| `patch_descriptor_for_resize` panicked on `u32::MAX` (size arithmetic before the bound) | coordinator R2 review | `patch_descriptor_refuses_extreme_dimensions_without_overflowing` — the verbatim case, width-only / height-only extremes, all four formats, both branches, and the valid maxima |
| GNF multi-stream files refused with the wrong reason | research R43 §2 | covered by the refusal variant + the NHL 15 / 19 desktop card |
| `DW8` treated as an unexplained slot reference | research R42 §1 | the census now asserts the join on every descriptor of three trees |
| NHL 07 / 08 descriptors refused as an unmeasured generation | research R42 §2 | the new five-tree census case |
| Pool-head floor fixed at 60 000 — wrong for NHL 09–13 (pool population read as **zero**) | research R53 §2 | `the_pool_head_floor_follows_the_title_era`, five anchors |
| `le32`/`le64` in the **new** PS4 context reader added before bounds-checking | my own new test | fixed with `checked_add`; the parser refuses instead of panicking |

Three of my own mistakes were caught by my own tests and are recorded rather than smoothed over: the
PS4 condition list is a pointer array (a flat stride resolved 67–88 of 440); the NHL 11 sock template
carries a sub-index (0 of 627 rows resolved until that was measured); and the first pool-head
assertion was too strong (a generic head can be worn by exactly one player — which is precisely what
the era floor adds over the uniqueness rule).

## 4. Tests actually run

| Suite | Result |
|---|---|
| `tdb-rx2` lib, with the restored fixtures | **144 passed, 0 failed, 0 ignored** (was 118 passed + 26 failing for missing inputs) |
| `tdb-rx2 --test rw4xb2_descriptor_census --ignored` (5 real trees) | **5 passed** |
| `tdb-speech --test ps4_contexts --include-ignored` (7 PS4 titles) | **2 passed** |
| `studio-api --test gnf_real_resize --ignored` | **3 passed** |
| `studio-api --test arena_venue --include-ignored` | **3 passed** |
| `studio-domain --test kit_selector_join --ignored` (3 real trees) | **1 passed** |
| Integration part 1 (`tdb-rx2` + `tdb-speech` + `studio-domain`) | **403 passed, 0 failed, 22 ignored** |
| Integration part 2 (`studio-domain` selected targets + `studio-api` lib and targets) | **59 passed, 0 failed, 15 ignored**, exit 0 |
| Frontend `npx vitest run` | **27 files, 186 passed**; `npx tsc` clean |
| Bindings | `export-bindings --check` **in sync**; the deliberate export-root guard updated 126 → **128** (the two arena-venue DTOs) |

**Integration total: 462 passed, 0 failed, 37 ignored** across the two parts.

**Ignored / private cases.** The `--ignored` suites above need either the private fixture root
(`E:\nhl-studio-fixtures`) or the real trees on `X:` / `H:`; each fails loudly when selected without
them. 22 cases remain ignored in the integration run (other crates' fixture-gated cases, untouched
by this campaign).

**`zz_diag` was invoked once, in error**, as part of `cargo test -p studio-domain` — it is a
`studio-domain` target, and I had excluded it only from the `studio-api` half. It was stopped as soon
as it appeared, the completed results were preserved verbatim as `r1-integration-part1.log`, and the
remaining work was reissued against explicit test targets as `r1-integration-part2.log`. Part 1 has
no `exit=` line for that reason — the process was stopped, nothing failed. The evidence directory's
[README](../../validation/evidence/overnight-development-2026-09-22/logs/r1-integration-README.md)
states this so the two-part record cannot be misread. Recorded rather than quietly re-run.

## 5. Deferred cards, grouped by title

All in [MORNING-REVIEW.md](MORNING-REVIEW.md). Runtime (needs a console): **R31-GNF-1 rev. 2** and
**R31-GNF-2** (NHL 21 / NHL 16 PS4 — does a resized GNF draw?), **R32-XB2-1 rev. 2** (Legacy X360,
unchanged), **R07-TEX-1** (NHL 07 — nothing to observe until the container reflow exists). Desktop
(no game): **V-23 … V-29** — the changed-size replace and its refusals, the PS4 Contexts card, the
venue card, and the goal-horn name.

Every package delivered after receipt 03 is **read-only inspection** and opens no runtime card.

## 6. Cleanup and retained artifacts

| Artifact | Disposition |
|---|---|
| Scratchpad scripts (≈ 15 across the campaign) | deleted at each checkpoint; the scratchpad is empty |
| Temp roots | the tests remove their own (`nhlms-gnf-real-*`, `nhlms-kit-*`); none left behind |
| Background processes | none running at closeout (verified) |
| **Private fixture root** `E:\nhl-studio-fixtures` (+ ~6 MB: six `tdb-rx2` fixtures and six PS4 GNF sources) | **retained** — a named dependency of 26 `tdb-rx2` cases and `gnf_real_resize`; outside the repository, never committed, and reproducible from the table in [C0-FX1 §9](../../validation/c0-fx1-explicit-fixture-tests.md) |
| Evidence logs (10 + a README) under `docs/validation/evidence/overnight-development-2026-09-22/logs/` | retained; the two census logs are the expensive ones (~12 and ~15 min to reproduce). One duplicate — the stopped run's output, byte-identical to `r1-integration-part1.log` — was removed, and the README explains the two-part split |
| Shared build cache, other lanes' files, game trees, anchors | untouched; nothing deleted by age |

## 7. Where Codex should look first

1. **The R2 correction** (receipt 01) — it is the package the coordinator asked for, and the only one
   with a prior review finding attached.
2. **Contracts consumed from research, and the three places I deviated from a handoff's text after
   measuring** — the PS4 condition-list layout (04 §3c), the NHL 11 sock template (08 §3) and the
   `seat*zone*` class kept as recorded while grouping by the measured colour flag (05 §3). Each is
   recorded in its receipt and in the claim's `observations_for_research`.
3. **New write-path *refusals*** — NHL 07's container layout (03), `SlotJoinMismatch` and
   `NullFormatDescriptor` (02). These change what Studio declines to do; a wrong refusal is as bad as
   a wrong write.
4. **The fixture restoration** (02 §3c, C0-FX1 §9) — it changes what 26 previously-failing cases now
   assert, so it is worth checking that the fixtures are the files the code means.
5. **The export-root guard** (126 → 128) and the two new DTOs (`ArenaVenueDto`, `VenueFieldDto`).

## 8. One exact continuation task

**Surface the kit join on the Uniforms tab.** `studio_domain::kit_files::team_kits` and the
`team_kits` view are written and tested, but **no API seam exists yet** — the DTO / registry /
command edits were drafted and deliberately **not** applied, so nothing half-wired is left behind:
write
`crates/studio-api/src/{dto,convert,registry,commands,lib,bindings}.rs`, add the
`Session::team_kit_rows` accessor the registry method expects, regenerate bindings, and render each
kit's jersey / pant / sock plus the spare pool in `app/src/components/UniformsTab.tsx`. Then extend
`crates/studio-domain/tests/kit_selector_join.rs` to NHL 12 / 13 / 14 and one PS3 tree.
