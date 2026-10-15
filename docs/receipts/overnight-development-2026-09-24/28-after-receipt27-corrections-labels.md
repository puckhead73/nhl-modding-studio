# Receipt 28 — after receipt 27: corrections, title-scoped nationality labels, range audit, goalie-Overall comparison (first checkpoint)

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged; nothing staged or committed). Direction:
[after-receipt-27 prompt](../../plans/development-20260926-after-receipt27.md), first checkpoint (tasks 1–4).
Claims: `dev-r27-corrections`, `dev-c16-nationality-labels`, `dev-identity-range-audit`,
`dev-goalie-overall-input-comparison`. The stable review build is receipt 28 §6 / the plan's §2
record; Speex and XMA2 follow in later receipts.

## 1. Plain-language summary

- **Receipt 27 corrected (task 1).** The ledger now shows tournament browsing as delivered, not "not
  built". The Speex evidence is narrowed to what was measured: four announcer lines decoded and
  **one** compared sample by sample. The goalie-consistency wording names the seven titles tested
  and says the Overall check compared totals, not each player. The consistency import test now
  checks the actual write and carries it through save and reopen; its compiler warning is gone.
- **Nationality labels are now per title (task 2).** Before this change Studio showed an NHL 10
  Czech player as "Croatia" (and ten other European countries wrong on NHL 10), left Japan
  unlabelled on NHL 11 / 12. Each title now uses the
  codes its own national-team rosters carry. NHL 08 / 09 were already right on the player page
  (Studio's NHL 07 country table applies there too) and are unchanged. Codes no national team
  carries on NHL 08–12 are shown as numbers rather than borrowed from NHL Legacy. The import and
  the birth-state country list follow the same numbering, and the save-time check stopped
  comparing a title's codes against other titles' lists.
- **Range handling on the newest titles (task 3).** On the genuine NHL 20 and on NHL 21, the
  nationality and birth-state fields are declared 0..255 and Studio edits, reviews, imports, saves
  and reopens a value of 200 without cutting it down; 256 is refused. One real gap was in the
  editor: a labelled dropdown offered only labelled codes, so an unlabelled in-range code could
  not be entered. A small "stored code" box beside the dropdown now accepts any in-range code.
  Zero-height placeholder rows are left as they are (shown as the minimum height) — unresolved.
- **Goalie Overall comparison (task 4).** Read-only, per goalie, on nine anchors and the genuine
  NHL 20. Feeding consistency at +36 on PS4 makes PS4 NHL 15 and 16 match their PS3 twins exactly
  (552 of 555 and 447 of 533 goalies closer), is mixed on NHL 17–19 and worse on NHL 20 / 21. The
  formula is **unchanged**; a scoped proposal is recorded for review.

## 2. Files changed

| Path | Change |
|---|---|
| `docs/receipts/overnight-development-2026-09-24/27-review23-26-continuation.md` | Speex claim narrowed to pick 16; seven-title consistency wording; fingerprints described as sums / counts; "only decoder" wording replaced |
| `docs/validation/development-20260926-decoder-recommendation.md` | the same two narrowings |
| `docs/receipts/overnight-development-2026-09-24/LEDGER.md` | tournament browse row → done (dev), gap list and order updated; decoder row → decided 26 Sep |
| `crates/studio-domain/tests/goalie_consistency.rs` | import case asserts the `ImportOp` (update of the matched goalie, one companion write, `Int(value)`), applies it on a disposable copy and checks save → reopen; redundant `.clone()` removed |
| `defs/enums/nationality.toml` | `[scope]` NHL 13 → 21 + Legacy (Recompiled by identity); provenance text |
| `defs/enums/nationality-nhl07.toml` (**new**) | NHL 07 table = `attributes::GEN6_COUNTRIES` minus the inferred 78 (for the registry / import; display unchanged) |
| `defs/enums/nationality-nhl08-09.toml`, `-nhl10.toml`, `-nhl11-12.toml` (**new**) | the 21 national-team codes per title family from R120b |
| `crates/studio-domain/src/attributes.rs` | `apply_title_birth_state_countries` + `BIRTH_STATE_COUNTRY_NOTE`: before NHL 13 the birth-state country region (≥ 66) follows the title's nationality labels, provinces / states (≤ 65) preserved, with a continuity provenance note |
| `crates/studio-domain/src/view.rs` | calls it; the note becomes the field's `vocabulary_note` |
| `crates/tdb-domain-validation/src/lib.rs` | `enum_vocabulary` checks only the def that applies to the registry's title |
| `app/src/components/EditableField.tsx` + test | `CodeInput`: the stored code beside a labelled dropdown; any declared-range value, out-of-range typed values neither written nor clamped |
| `crates/studio-domain/tests/nationality_labels.rs` (**new**) | 4 cases (below) |
| `crates/studio-domain/tests/identity_ranges_nhl20_21.rs` (**new**) | NHL 21 anchor + genuine NHL 20 (private fixture) |
| `crates/studio-domain/tests/goalie_overall_consistency_input.rs` (**new**) | read-only comparison, 9 anchors + genuine NHL 20 |
| `crates/studio-domain/tests/import_change_set.rs`, `crates/studio-api/tests/player_identity.rs`, `crates/tdb-enums/tests/enums.rs` | test premises updated for title scope (see §4) |

## 3. Evidence (logs under `docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- **Task 1** — `r28-consistency-import.log`: `goalie_consistency` 4 / 4 (twice: before and after the
  clone fix; the second compile shows no `goalie_consistency.rs` warning).
- **Task 2, red first** — `c16-nationality-red.log` (before any `defs/` change): NHL 10 PS3 / X360
  `CZE: raw 81 shown as "Croatia"`, GBR 82 "Czech Republic", FRA 83 "England", GER 84 "France", …;
  NHL 11 / 12 `JPN: raw 108 shown as "108"`, GBR 83 "England"; NHL 08 / 09 and NHL 13 → PS4 21
  already correct on all 21 teams. **Green** — `c16-nationality-green.log`: 4 / 4 —
  `every_national_team_shows_its_own_country_on_every_title` (25 anchor directories — every one except the two NHL 07 copies — × 21 national
  teams: roster's dominant code = R120b, the player page label and the dropdown entry),
  `nhl10_czech_player_edit_review_and_unmeasured_codes` (PS3 and X360: raw 81 → "Czech Republic"
  with the NHL 10 national-team provenance note; edit to 84 → pending "Czech Republic → Germany";
  71 shown as "71"), `the_import_label_lookup_resolves_each_titles_own_code` (NHL 07 / 08 / 10 /
  12 / 13 / PS4 21 codes; Belgium, Lithuania, Estonia, Fiji unresolved where not title-owned),
  `birth_state_country_codes_follow_the_title_numbering_before_nhl13` (NHL 10: 81 Czech Republic,
  71 raw, Ontario and Minnesota preserved; NHL 11: 108 Japan, 109 raw; Legacy unchanged: 94
  Serbia, 109 Japan).
- **Task 3** — `r28-affected-suites-2.log`: `identity_ranges_nhl20_21` 2 / 2 with `--include-ignored`
  (NHL 21 anchor and the genuine NHL 20, `NHL_STUDIO_FIXTURES=E:\nhl-studio-fixtures`): declared
  `intlcountry` / `state` 0..255, height 0..31, weight 0..255; 200 written, re-read, pending "200",
  256 refused "out of range", undo / redo, import of 109 on another player (shown "Japan"), save →
  reopen on a disposable copy. `r28-frontend.log`: `tsc` exit 0; vitest 40 files / 249 tests
  (includes the new `CodeInput` case).
- **Task 4** — same log, `goalie_overall_consistency_input` 2 / 2 (table in §5).
- **Affected suites** — `r28-affected-suites.log` then `-2.log` (`--no-fail-fast`, explicit
  targets; `zz_diag` not selected): studio-domain lib 223 passed / 2 ignored; `capabilities` 5,
  `import_change_set` 3 (after the premise fix), `import_corpus_e2e` 0 + 1 ignored (private corpus),
  `nhl07_encodings` 8, `read_matrix` 2, `roster_builder_e2e` 1, `surface_matrix` 1, `surfaces` 4;
  `tdb-domain-validation` lib 2 + `audit` 14; `tdb-enums` lib 5 + `enums` 2 (after the count
  update); studio-api lib 31,
  `player_identity` 2 (after the premise fix), `roster_builder` 1. First run failures, all from the
  intended scope change: the enum count pin (30 → 34 files), a test session opened NHL 14 without
  naming its title (scoped labels never apply to an unknown title), and the identity sweep assumed
  every NHL 10 player's code has a label.

## 4. Decisions and limits

- **NHL 08 / 09 display unchanged.** Studio's NHL 07 country table (`GEN6_COUNTRIES`, user-confirmed)
  applies on NHL 07–09 by schema shape (3-bit style field) and agrees with all 21 national-team
  codes R120b measured there. The new NHL 08 / 09 file is what the registry and the import use and
  holds only the 21 title-owned codes; the dropdown on the player page still shows the NHL 07 table
  (e.g. Lithuania 74), which is NHL 07 evidence carried by the existing override — noted for review.
- **Unlabelled on NHL 08–12:** every code no national team carries, including 127 (used from NHL 10
  onward for players without a set code; its meaning there has no title-owned evidence) and the
  NHL Legacy extras (Estonia 71, Fiji 106, …). Shown as numbers; the banner omits nationality for
  such a player and the field shows the stored code.
- **"Great Britain" vs "England":** before NHL 13 the British code is labelled from the national team
  ("Great Britain"); NHL 13 onward keeps the operator-confirmed Legacy label "England".
  **Corrected 26 Sep (after the coordinator's R119–R121 review, which says national-team membership
  fixes the code, not the exact label):** NHL 10 and NHL 11 / 12 now show the British code as
  "England", the operator-confirmed wording, as before this receipt on NHL 11 / 12. NHL 08 / 09 keep
  NHL 07's user-confirmed "Great Britain" (their displayed table). `nationality_labels` 4 / 4 again
  (`c16-nationality-green-2.log`). The 26 Sep stable review build predates this correction and shows
  "Great Britain" for that code on NHL 10–12 (recorded in the plan).
- **Unknown title:** a session never told its title (the Advanced single-file open path) now shows
  nationality as numbers, as the equipment catalogues already did (25 Sep scope rule). The normal
  install open path always passes the title.
- **Birth state:** continuity evidence (R120 §7), so the note says so; no separate title-owned table.
- **Range handling:** the backend already used the metadata's declared range everywhere measured;
  the only defect was the dropdown's reach. The number box elsewhere clamps a typed out-of-range
  value to the declared range (existing behaviour, unchanged; the pending review shows the result).
- **D3 zero heights (unresolved, published):** raw height 0 is the minimum encodable height (54 in,
  4'6"); the 180–185 placeholder rows R120 found also have weight 0 and are unnamed, but one named,
  rostered NHL 12 player (Colton Heffley) has it too. Missing-data versus legitimate minimum cannot
  be decided from a population census; nothing is reinterpreted or rewritten.
- **D6 / D7 (style, eye colour):** continuity leads only; no dictionary change.

## 5. Goalie Overall — current vs candidate consistency input (read-only)

`computed − stored` per authored goalie (stored Overall > 36). A = consistency + 0, B = + 36.
Current input: +36 on the six-bit PS3 titles, +0 on PS4. B − A is 3 or 4 for every goalie.

| Source | goalies | A median / mean / ±1 | B median / mean / ±1 | B closer / further / equal |
|---|---|---|---|---|
| PS3 NHL 14 (current B) | 517 | 4 / −3.70 / 0.2 % | 0 / −0.10 / 98.3 % | 515 / 1 / 1 |
| PS3 NHL 15 (current B) | 552 | 4 / −3.73 / 0.2 % | 0 / −0.17 / 98.6 % | 550 / 1 / 1 |
| PS3 Legacy (current B) | 533 | 4 / −3.31 / 7.1 % | 1 / +0.28 / 68.3 % | 447 / 67 / 19 |
| PS4 NHL 15 (current A) | 555 | 4 / −3.72 / 0.2 % | 0 / −0.15 / 98.6 % | 552 / 1 / 2 |
| PS4 NHL 16 (current A) | 533 | 4 / −3.31 / 7.1 % | 1 / +0.28 / 68.3 % | 447 / 67 / 19 |
| PS4 NHL 17 (current A) | 446 | 4 / −2.70 / 12.6 % | 2 / +0.89 / 44.6 % | 298 / 131 / 17 |
| PS4 NHL 18 (current A) | 656 | 3 / −2.34 / 15.5 % | 1 / +1.25 / 65.9 % | 463 / 160 / 33 |
| PS4 NHL 19 (current A) | 519 | 4 / −0.90 / 15.8 % | 2 / +2.69 / 49.9 % | 296 / 207 / 16 |
| **genuine NHL 20** (current A) | 448 | 4 / +1.12 / 14.7 % | 3 / +4.71 / 36.8 % | 211 / 224 / 13 |
| PS4 NHL 21 (current A) | 362 | 4 / +1.38 / 14.1 % | 4 / +4.98 / 31.8 % | 161 / 187 / 14 |

Reading: PS4 NHL 15 / 16 reproduce their PS3 twins' error profile exactly under B (same roster
values, 0..99 storage), so the fit expects the six-bit input there. From NHL 17 the goalie Overall
drifts (max errors 39–67 from outliers) and B is not uniformly better; on NHL 20 / 21 it is worse on
the mean. **Proposal for review (not implemented):** consistency input +36 on PS4 NHL 15 and 16
only, keyed by title, with this table as the regression; NHL 17–21 keep +0 until a re-fit or other
evidence. Per-title sums are not proof of every individual result; the per-player counts are shown.

## 6. Stable review build (claim `dev-stable-review-build-0927`)

Built 26 Sep 12:14:44 → 12:17 MDT with `npm run build -- --no-bundle` at the repo root (existing
release cache reused; `tauri-build.log`, exit 0), then copied with a `defs/` snapshot to
`E:\nhlms-review-build-2026-09-27\` (50 MB). Executable SHA-256
`4dc5c540769b86858dd3be28255fdbe5e2282b3a146d20a51f4ca0b48455c502`; `defs` tree `200f04d9…`;
frontend dist tree `ae467dcf…`; dirty-source manifest of 828 compiled-input entries re-hashed after
the build with no drift. Includes receipts through 28; excludes Speex / XMA previews and goalie gear.
Not launched (not authorised). Full record: plan §2 "Stable review build — record" and
`docs/validation/evidence/overnight-development-2026-09-24/review-build/`. Tracker / results headers
and receipt 27 / 28 rows filled; the plan's receipt-27 notes now apply. Removal: delete the folder.

## 7. Cleanup and closing checks

- Owned temporary roots of the new tests (`nhlms-identity-ranges-*`, `nhlms-consistency-*`) are
  removed by their `Drop` guards: none present in `%TEMP%` after the runs (the remaining `nhlms-*`
  roots there belong to older runs and other lanes and were left alone).
- Scratch patch scripts are in the session scratchpad only. The upstream `speex-1.2.1.tar.gz` there is
  kept until the Speex package ends (vendoring provenance; named dependency).
- Bindings: no DTO signature changed in this checkpoint; the exporter was not needed.
- Frontend: `tsc` exit 0; vitest 40 files / 249 tests (`r28-frontend.log`).

## 8. Next

EA-Speex diagnostic preview (claim `dev-speex-preview`, started), then the XMA2 helper — recorded
blocker at the time of this receipt: no GNU make / MinGW / MSYS environment on this machine to run a
pinned FFmpeg `configure` (installing MSYS2 would be a global tool install, not done) — then goalie
gear 4.5.
