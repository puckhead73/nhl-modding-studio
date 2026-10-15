# Receipt 27 (consolidated) — packages 23–26 review continuation: goalie consistency, ledger authority, decoder recommendation, tournament browse labels, gloves and helmets

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged). Claims: `dev-c15-goalie-consistency`, `dev-ledger-refresh-26sep`,
`dev-decoder-recommendation`, `dev-tournament-browse-labels`, `dev-p42-43-gloves-helmets`. Clock: 03:40 → 04:55 (real clock)
(MDT, 26 Sep 2026). Direction: [packages 23–26 review](../../validation/development-20260926-packages23-26-review.md),
tasks 1–4.

## 1. Plain-language summary

- **Goalie consistency was shown as a rating and it is not one (task 1).** The shipped rosters store only 0, 1 or 2
  in that field on every title, but Studio added the rating offset. So a goalie showed 50–52 on NHL 08 and 36–38 on
  NHL 11 → Legacy, and imported historical values were refused as out of range. It now shows and edits the stored
  value on the seven representative titles tested (Xbox 360 NHL 08, 09, 11, 14, Legacy; PS4 NHL 15, 21 — not an
  all-target sweep; the classification rule itself is title-free), with a note that the meaning of 0 / 1 / 2 is not established. It is not tinted as a rating,
  and its declared range stays the write range (no clamp to 0..2). The fitted goalie Overall depends on this field;
  it keeps the exact input it was calibrated with, and the per-title sums and counts of the derived goalie
  Overall on NHL 14, Legacy, PS4 15 and PS4 21 are unchanged (sums and counts, not a per-player byte comparison). The floppiness field is untouched.
- **The ledger has one current authority (task 2).** A new "Refresh 26 Sep — current authority" section supersedes
  the older tables where they differ. The two withdrawn research citations are struck in place. Schedule generation
  and tournaments are classified (proposal / candidate / implementation-ready / runtime-blocked) with the exact
  dependency. The ledger now says plainly that Phases 1 and 2 are not complete, and lists why.
- **Decoder recommendation published (task 3).** For EA-Speex: libspeex, the reference C library, tested on the
  research vectors: four Xbox NHL 15 announcer picks decoded, and **one** of them (pick 16) was compared sample by
  sample with the reference output (within one unit in the last bit). For XMA2: no suitable standalone alternative to
  FFmpeg's LGPL decoder was found in the recorded search. (correction 26 Sep, receipt 28: narrowed per the receipt-27 review) See
  [`development-20260926-decoder-recommendation.md`](../../validation/development-20260926-decoder-recommendation.md).
  Nothing was added to Studio.
- **Tournament tables are now readable (task 4, League / Team).** The Champions Hockey League, Spengler Cup and
  NHL 17 World Cup schedules were unreachable in the app: they have no bound league, and the schedule card only
  showed bound tables. They now appear on their tournament's league page, read-only, with round labels, seed labels
  and "to be decided" placeholders. The World Cup group stage shows the eight teams through the research's one
  consistent join. Spengler placeholders are marked as an interpretation, and CHL / Spengler slots stay unnamed.
- **Gloves, helmets and skates swept on every title (task 4, equipment fallback).** The glove, helmet / visor and skate columns are
  edited, shown in the pending review, refused past their range, undone, redone, saved and reopened on all 25
  anchors that have an equipment table, and on the genuine NHL 20 database. NHL 07's equivalents live on a
  custom-player table and are recorded, not edited.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/attributes.rs` | `is_goalie_consistency`, `GOALIE_CONSISTENCY_NOTE`, `formula_input_offset` (keeps the fitted goalie Overall's calibrated input); `rating_offset` excludes goalie consistency; the attribute view's formula inputs use `formula_input_offset` |
| `crates/studio-domain/src/view.rs` | the consistency note on the field view; the roster-OVR reader's formula inputs use `formula_input_offset` |
| `crates/studio-domain/src/assets.rs` | doc comment: birth-date 1-indexing is NHL 07 → 10 by contract (R116 observation) — wording only |
| `crates/studio-domain/src/schedules.rs` | tournament templates: `round_label` / `home_label` / `away_label` on games, `tournament_note` on the view, `shown_with_league` on the table summary (display grouping only; `league_code` and the slot map stay empty); World Cup names by R115 J1, with an abbreviation fallback for national teams without a display name; the "dangling slot" finding suppressed for tournament placeholders |
| `crates/studio-domain/src/schedule_edit.rs` | edit-preview games carry no tournament labels |
| `crates/studio-api/src/{dto,convert}.rs`; `app/src/bindings/{ScheduledGameDto,ScheduleViewDto,ScheduleTableDto}.ts` (generated) | DTO fields for the above |
| `app/src/components/LeagueScheduleCard.tsx` + test | shows tables with `shown_with_league`; slot labels (raw slot in the tooltip) and round labels; the tournament note replaces the "cannot be named" notice; new jsdom case |
| `crates/studio-domain/tests/goalie_consistency.rs` (**new**) | 4 cases × 7 titles (NHL 08, 09, 11, 14, Legacy; PS4 15, 21): display / edit form / note / tier; edit, pending, undo, redo, save → reopen, declared-range refusal; import lowering; goalie-Overall fingerprints (recorded **before** the change) |
| `crates/studio-domain/tests/capabilities.rs` | goalie rating counts 21 → 20 (Legacy) and ≥ 21 → ≥ 20 (PS4 21), with the reason |
| `crates/studio-domain/tests/tournament_labels.rs` (**new**) | NHL 17 World Cup (CAN / CZE / EUR / USA, FIN / NA / RUS / SWE = R115's groups), CHL and Spengler on NHL 18 / 19 / 21, league tables unaffected |
| `crates/studio-domain/tests/equipment_gloves_helmets.rs` (**new**) | the 27-anchor sweep + the real NHL 20 case (private fixture) |
| `docs/receipts/overnight-development-2026-09-24/LEDGER.md` | the current-authority section; two withdrawn citations struck |
| `docs/validation/development-20260926-decoder-recommendation.md` (**new**) | task 3 |

## 3. Evidence

Logs under `docs/validation/evidence/overnight-development-2026-09-24/logs/`:

- **Task 1** — [`c15-goalie-consistency-red.log`](../../validation/evidence/overnight-development-2026-09-24/logs/c15-goalie-consistency-red.log)
  (**red, before any source change**):
  - Display / edit offset: stored 3 shows as 53 (+50) on NHL 08 and as 39 (+36) on NHL 11 / 14 / Legacy.
  - Note: missing on NHL 09 and PS4.
  - Import: "human value 2 out of range" on NHL 08 / 11 / 14 / Legacy.
  - Goalie-Overall fingerprints recorded: NHL 14 Σ 31 313 / 523 goalies, roster Σ 328 266 / 5 374; Legacy 35 427 /
    566, 362 403 / 5 632; PS4 15 33 420 / 559, 360 682 / 5 555; PS4 21 46 381 / 749, 463 869 / 7 247; NHL 08 / 09 /
    11 report none (era gate).

  [`c15-goalie-consistency-green.log`](../../validation/evidence/overnight-development-2026-09-24/logs/c15-goalie-consistency-green.log)
  (**green**): 4 / 4 passed. Stock values {0, 1, 2} on all 7 titles; fingerprints identical.

  [`c15-affected-suites.log`](../../validation/evidence/overnight-development-2026-09-24/logs/c15-affected-suites.log):
  - Domain lib 223 passed, 0 failed.
  - Private NHL 20 encoding case 1 / 1.
  - `capabilities` failed 2 on the expected count change (21 → 20 goalie ratings), then updated. That run stopped
    at its first failing target.

  [`c15-affected-suites-2.log`](../../validation/evidence/overnight-development-2026-09-24/logs/c15-affected-suites-2.log)
  (`--no-fail-fast`, every affected target + studio-api `player_ratings` / `roster_builder` / `roster_full_refusal`):
  every affected studio-domain target (capabilities, capacity_projection, encoding_contract, goalie_consistency, the eight import targets, overall_formula, rating_scale_probe, ratings_scale, ratings_write_codec, read_matrix, roster_builder_e2e, roster_overall_column, surface_matrix, surfaces) **54 passed, 0 failed, 3 ignored**; studio-api `player_ratings` / `roster_builder` / `roster_full_refusal` **3 passed, 0 failed** — `overall_formula` (the fitted-formula guard) green on the changed classification. Closing checks ([`r27-closing-checks.log`](../../validation/evidence/overnight-development-2026-09-24/logs/r27-closing-checks.log)): domain lib 223 passed; bindings in sync; `tsc` clean; vitest 40 files / 248 tests.
- **Task 3** — [`t3-decoder-evaluation.log`](../../validation/evidence/overnight-development-2026-09-24/logs/t3-decoder-evaluation.log):
  - 4 / 4 R83 picks decode to frames × 640 with R83's trims.
  - Against ffmpeg `libspeex`, **pick 16 only** (the one sample-by-sample comparison): max |diff| 1 LSB, corr 1.00000,
    95.5 % identical. The other three picks were checked for length and trims only. (correction 26 Sep, receipt 28: narrowed per the receipt-27 review)
  - This machine's ffmpeg reproduces both R83 hashes.
  - The installed ffmpeg is a GPLv3 build.
  - The harness was deleted after publication.
- **Task 4, tournaments** — [`t4-tournament-labels.log`](../../validation/evidence/overnight-development-2026-09-24/logs/t4-tournament-labels.log):
  - `tournament_labels` 3 / 3; `schedules` 2 / 2; `schedule_edit` 12 passed, 1 ignored (private).
  - `export-bindings` `--write` (3 files) then `--check` in sync.
  - `tsc` exit 0; vitest `LeagueScheduleCard` 10 / 10.
  - studio-api lib 31 / 31; `schedule_edit` 1 / 1.
  - A first test version passed with empty World Cup labels (the teams carry no display name). It was found by
    reading the printed labels; the abbreviation fallback and a stronger assertion fixed it.
- **Task 4, gloves / helmets** — [`p42-43-gloves-helmets.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p42-43-gloves-helmets.log):
  2 / 2 with `--include-ignored`. The per-title table (declared ranges):

  | Titles | gloves | helmet | visor type | visor colour |
  |---|---|---|---|---|
  | NHL 08 | 0..31 | 0..15 | 0..15 | 0..15 |
  | NHL 09 | 0..31 | 0..15 | 0..3 | 0..3 |
  | NHL 10 → PS4 15 | 0..127 | 0..255 | 0..3 (PS4 16: 0..15) | 0..15 |
  | PS4 17 / 18 | 0..255 | 0..255 | 0..31 | 0..31 |
  | PS4 19 / 20 anchors | 0..511 | 0..511 | 0..31 | 0..31 |
  | PS4 21 **and the real NHL 20** | 0..1023 | 0..1023 | 0..15 | 0..31 |

  Other results:
  - Labels only on NHL Legacy; pending text is the stored number elsewhere.
  - `itechvisorcolor` is declared in the metadata, but on no title is it a column of the skater equipment companion.
- **Task 4, skates (4.4, same sweep)** — [`p44-skates-sweep.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p44-skates-sweep.log):
  2 / 2 with `--include-ignored` after adding `skatebrand` / `skaterocker` / `skateguard`: `skatebrand` 0..15 (NHL 08 / 09),
  0..127 (NHL 10 → PS4 18), 0..511 (PS4 19 / 20 anchors), 0..1023 (PS4 21, real NHL 20); `skaterocker` 0..15 (NHL 08), 0..3
  elsewhere; `skateguard` 0..7 (PS4 16), 0..15 (PS4 17 → 21, real NHL 20); labels only on Legacy (`skatebrand`); 6–7
  columns edited per title, all through pending / refusal / undo / redo / save → reopen.

## 4. Limits and observations

- **Consistency:**
  - The meanings of 0 / 1 / 2, and whether other declared values act differently in game, are open (research /
    runtime).
  - The goalie Overall formula still consumes consistency at +36 (six-bit) / +0 (PS4), exactly as calibrated. PS4
    NHL 15's −3.7 goalie bias equals the coefficient × 36, which suggests the fit expects +36 there too. Reported,
    **not changed**.
  - Floppiness is unchanged and still unresolved per title.
- **Tournaments:**
  - Browse-only. No slot → club binding, no edit / import / export / generation (the slot map stays empty).
  - CHL / Spengler participant slots are unnamed by design (R115).
  - NHL 11 / 12 `fcc_tournaments` is an Ultimate Team name catalogue, out of scope.
- **Gloves / helmets:**
  - The real NHL 20 declares wider glove / helmet ranges than the committed `ps4/nhl20` anchor (an NHL 19 base copy).
  - No per-title catalogues outside NHL Legacy.
  - Which glove or helmet an id renders is runtime.
  - Goalie gloves (trapper / blocker) are goalie gear, not swept here.
- **Decoders:** no dependency added; two coordinator questions in the recommendation §5.

## 5. Cleanup

Deleted:
- the decoder harness (scratch crate, PCM / Ogg dumps, R83 scratch directory);
- the owned patch scripts in the session scratchpad;
- the alternate build directory `E:/nhlms-target-check`, removed at close: done.

Disposable test roots are removed by their tests' drop guards. Nothing under `anchors/`, the private fixtures,
research artifacts or other lanes' files was written.
