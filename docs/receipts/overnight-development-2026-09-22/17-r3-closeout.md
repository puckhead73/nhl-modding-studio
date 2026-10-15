# Receipt 17 — Closeout of the R3 fresh-session extended dispatch (22 September 2026)

**Everything in this continuation is developer-verified only. Nothing is Codex-accepted, nothing
was desktop-exercised, nothing is game-verified; no commit, staging, push, reset, branch change or
clean was made; no game or emulator was launched; no live install, image, archive, save, anchor or
profile was written.** Real trees on `X:` / `H:` and the private fixture root were read only; every
write went to a disposable temp copy or to memory.

Branch `feat/phase5-asset-tooling`, HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` — **unchanged
from start to finish**; the index is empty. The working tree the dispatch inherited (733 entries)
now holds **775 entries (186 modified, 586 untracked)**, the difference being this continuation's own new files. NHL 21
E1 stays accepted and closed. Projects / Builds untouched. Dispatch:
[overnight-development-20260922-review.md](../../validation/overnight-development-20260922-review.md).

## 1. What users can now do (plain language)

- **Trust the kit card.** A kit is called complete only when every file the title's measured
  contract requires is there; an absent sock folder now reads as *incomplete*, never as a complete
  spare. The Uniforms tab shows each kit's jersey, pants and every sock file with present / missing /
  unknown evidence, the spare candidates with their own verdicts, and why re-pointing is not offered.
- **Know what an Xbox 360 texture replacement really wrote.** The same-size route rewrites only the
  levels whose placement was verified by content (the base and everything above 64 texels) and keeps
  the shipped small-level tail, saying so in the result; a changed-size result says its tail layout is
  unestablished.
- **See an arena's shipped assets.** Venue model and jumbotron by art id, centre-ice logo and
  banners through the home team, the bounce map as a candidate, the other ice layers as shipped
  files with no established binding, and on PS4 the venue folders under a vault reference Studio
  does not read.
- **See what the install ships for the roster.** Own heads and pool heads with their files, the
  `hasportrait` flag as stored beside the file evidence (never derived, never written), the
  shipped-but-unreferenced portraits and heads, and the players shipped without their own asset.
- **See whether an equipment id carries its family's full file set**, and what it lacks.
- **Browse the PS4 song catalogue** — what can be chosen, labelled as such, with the measured
  negatives stated.
- **Resize team logos with a swept route**: 168 packages classified — resizable families resize with
  every invariant, the 07–11 package forms refuse by name, crop-quad families refuse as such.

## 2. Packages, with per-title / per-platform coverage reached

| # | Package | Coverage reached (developer-verified) |
|---|---|---|
| [10](10-r3-kit-completeness-correction-and-uniforms.md) | F1 kit completeness corrected; read-only Uniforms card; coverage | **X360 NHL 11 / 12 / 13 / 14 / 15 / Legacy and PS3 NHL 11 / 14 / 15 / Legacy**: 100 % of rows under the contract rule; PS3 12 / 13 not covered (in-archive); PS4 / 07 / 08 unmeasured; NHL 09 / 10 no selector table |
| [11](11-r3-xbox360-mip-layout-audit.md) | Xbox 360 mip-layout audit; same-size route keeps the packed tail | **X360 Legacy (jersey, player) + NHL 11 (jersey)**: 1 100 rasters, 2 427 large levels placed (0 contradicted), 892 / 3 300 tail levels contradicted; codec-wide route change; changed-size route disclosed, unchanged |
| [12](12-r3-arena-assets-card.md) | Arena assets card | **X360 NHL 09 / 11 / 13 / 15 / Legacy, PS3 Legacy**: venue 100 %, jumbotron 100 % (12+), spare ids equal to R62; PS4 (seam on NHL 21): venue directories listed, no binding claimed; 07 / 08 unmeasured |
| [13](13-r3-team-logo-dimension-sweep.md) | Team-logo dimension sweep | **168 gen7 packages** (X360 07–Legacy, PS3 08–Legacy, Legacy cache): 07–11 same-size-only by name; 12–Legacy resizable families resize with invariants; crop families refuse |
| [14](14-r3-player-containment-card.md) | Player portrait / head containment | **X360 NHL 09 / 13 / Legacy, PS3 Legacy**: R53 reproduced to the unit; PS4 (seam on NHL 21 / NHL 15 rule): portraits by art id, heads unmeasured; 07 / 08 unmeasured |
| [15](15-r3-equipment-component-sets.md) | Equipment component sets | **X360 Legacy / 13 / 15, PS3 Legacy**: R52 reproduced (two recorded reading differences); NHL 11 / 12 single container (seam on NHL 11); 07–10 / PS4 not measured |
| [16](16-r3-song-catalogue.md) | PS4 song catalogue listing | **PS4 NHL 21 / 19 / 18 / 17 real installs**: R54 rows exact; NHL 15 refused by contract; gen7 refused with the reason |

Dispatch items 1–4 are closed in receipts 10 (F1, reporting corrections, coverage), 11 (the
late-addendum audit before any Xbox mip expansion) and the
[observations file](OBSERVATIONS-FOR-RESEARCH.md) (item 3, rows O-1 … O-10). Item 5 ran through
Teams 2.4 / 2.5 / 2.6 / 2.7, Players 3.4, Equipment 4.x and Misc 5.4; the remaining Leagues rows
are runtime- or research-blocked and were not reopened.

## 3. Defects found and fixed in this continuation

| Defect | Found by | Regression |
|---|---|---|
| Kit completeness relaxed by an absent sock directory (F1) | coordinator review | 8 unit cases, the registry-seam reproduction, the card test; ten real trees |
| Xbox 360 packed-tail levels written under an unverified layout (the PR11 in-game finding, now quantified) | this campaign's content-based audit | `keep_tail_writer_rewrites_only_the_content_verified_levels`; the audit test on three real families |
| Reporting: receipts 02 / 05 / 08 / 09 overstated (dialog vs route, pixels vs header, "meaningless" constants, "users can see the kits") | coordinator review | dated correction notes in place |

Six of my own expectation mistakes were caught by the real-tree tests and corrected against the
research data rather than the assertions being loosened blindly: the NHL 15 spare-jumbotron count
(12 → 14, from the research JSON), the NHL 11 two-file sock requirement, the silhouettes directory
in the portrait count, R53's "other" class, R52's blocker / trapper id 0, the NHL 13 mask paint 260.
Each is recorded in its receipt and, where the research lane should know, in the observations file.

## 4. Tests actually run (closeout regression — [r3-closeout-regression.log](../../validation/evidence/overnight-development-2026-09-22/logs/r3-closeout-regression.log))

| Suite | Result |
|---|---|
| `cargo test -p studio-api` (lib + every test target) | **33 targets: 121 passed, 0 failed, 33 ignored** (private-fixture / real-tree cases), exit 0 |
| `cargo test -p studio-domain --lib` | **216 passed, 0 failed, 2 ignored** (was 198 at the start of R3) |
| `cargo test -p tdb-rx2 --lib` | **119 passed, 0 failed, 26 ignored** (the private-fixture cases; 144 / 0 with `--include-ignored` in receipt 02) |
| `export-bindings --check` | **in sync — 142 roots** (128 at the start of R3; the drift test is among the studio-api passes) |
| `npx vitest run` (full) | **31 files, 202 passed** (was 27 / 186) |
| `npx tsc --noEmit` | clean |

`--ignored` real-tree / private-fixture runs of this continuation (each fails loudly without its
inputs): `kit_selector_join` (10 trees), `rw4xb2_mip_placement` (3 families), `arena_asset_join`
(6 trees), `team_logo_dimension_sweep` (168 packages), `player_containment_join` (4 trees),
`equipment_component_census` (4 trees), `equipment_files` (2 trees), `song_catalogue` (5
installs) — all passed, logs under `docs/validation/evidence/overnight-development-2026-09-22/logs/r3-*`.
`zz_diag` was not invoked.

## 5. Deferred cards

Desktop (no game): **V-30, V-30b, V-31, V-32, V-33, V-34, V-35** in
[MORNING-REVIEW.md](MORNING-REVIEW.md). Runtime: **R32-XB2-1 rev. 2** (changed-size Xbox 360
texture) is unchanged in scope but now carries the audit's statement that its tail levels are placed
by an unestablished layout; no new runtime card was opened — every package after 11 is read-only
inspection.

Blockers recorded, not campaign stoppages: the Xbox 360 packed-tail contract (research Q-R65-1);
the PS4 Attrib / VLT vault reader (arena venues, PS4 heads, PS4 equipment); PS3 NHL 12 / 13
in-archive kits; the catalogue → bank binding for songs; the kit sock sub-index meaning on
NHL 11 / 12.

## 6. Limitations (exact)

- Every coverage claim above is on the trees named; titles sharing a contract but not run are
  *unclaimed*, listed per receipt (kits: PS3 12 / 13; arena: X360 10 / 12 / 14, PS3 11–15;
  containment: X360 10 / 11 / 12 / 14 / 15, PS4 real trees; equipment: NHL 14, PS3 13 / 15; songs:
  NHL 20).
- Nothing here writes a kit, an arena asset, a portrait / head id, an equipment id or a song; every
  reassignment surface says so and names its missing contract.
- The Xbox 360 audit says where the tail is *not*; the correct packed layout is not derived.
- Title keys (`$TXT_SONG_*`, `$NHLTeamName_*`) are shown as keys where no dictionary route exists yet.

## 7. Cleanup

| Artifact | Disposition |
|---|---|
| Scratchpad scripts (7 across the continuation) | deleted at each checkpoint; the scratchpad is empty (verified) |
| Temp roots created by this continuation's tests (`nhlms-kit-*`, `nhlms-kitapi-*`, `nhlms-arena*`, `nhlms-contain*`, `nhlms-equipset-*`, `nhlms-songs*`, `nhlms-teamlogo-dims-*`) | removed on drop; verified absent. Six `nhlms-*` roots from **earlier campaigns** remain in `%TEMP%` and were left in place (not owned by this session's manifests; listed in receipt 12 §7) |
| Background processes | none running at closeout |
| Evidence logs (`r3-*`, 15 files) | retained |
| Shared build cache, other lanes' files, game trees, anchors, private fixtures | untouched; nothing deleted by age |

## 8. Where Codex should look first

1. **Receipt 10 §3a** — the F1 red / green and the single completeness rule; and the corrected
   receipts 02 / 05 / 08 / 09.
2. **Receipt 11 and the audit record** — the content-based method, the supported-versus-unqualified
   table, and the same-size route change (`replace_large_levels_keep_tail`).
3. **The three contract tables** (`kit_contract`, `arena_asset_contract`, `containment_contract`,
   `set_contract`) — each decides from title / platform alone; a wrong entry there is a wrong claim
   on a whole title.
4. **The observations file** — ten measured departures the research lane should reconcile.

## 9. One exact continuation task

**Consume the research lane's packed-tail contract (Q-R65-1) when it lands:** replace the sequential
tail model in `Texlib::replace_with_resize` / `replace_large_levels_keep_tail` with the established
per-level addresses, extend `tests/rw4xb2_mip_placement.rs` to assert every tail level places, and
only then lift receipt 11's tail disclosures. Until it lands, the next independent package is the
PS4 dictionary resolution of the `$TXT_SONG_*` / `$TXT_*` keys on the song and arena cards through
the existing team-text route.
