# Receipt 31 (consolidated) — after receipt 27: corrections, nationality labels, review build, EA-Speex, EA-XMA helper route, goalie gear 4.5

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051` (unchanged; nothing staged, committed or
pushed). Direction: [after-receipt-27 prompt](../../plans/development-20260926-after-receipt27.md). Detailed
receipts: [28](28-after-receipt27-corrections-labels.md), [29](29-easpeex-preview.md),
[30](30-xma2-helper-invocation.md); this receipt adds goalie gear 4.5 and one SBKR reader fix, and closes
the prompt.

## 1. Plain-language summary

- **Corrections and labels (receipt 28).** Receipt 27's wording is narrowed as the review asked, and
  its import test now proves the write. Nationality labels are now per title: NHL 10 no longer calls
  Czech players "Croatian" (ten other NHL 10 countries fixed), NHL 11 / 12 label Japan, and codes with
  no title-owned evidence show as numbers. The British code keeps the confirmed wording "England" from
  NHL 10 on (corrected after the coordinator's research review). NHL 08 / 09 were already right.
- **Stable review build (receipt 28 §6).** `E:\nhlms-review-build-2026-09-27\nhl-studio.exe` with its own
  `defs` snapshot, hashed and recorded in the review plan. It contains receipts through 28 as first
  published — i.e. it still shows "Great Britain" for the British code on NHL 10–12 (recorded as
  expected in the plan). Nothing later is in it.
- **Speex previews (receipt 29 + this receipt).** The official Speex library, unchanged, now decodes EA's
  Speex speech for previews on Xbox 360 NHL 11 → Legacy, PS3 NHL 14 and PS4 NHL 15 → 21, within one unit
  of the reference on every research sample. The one family held back (Xbox 360 NHL 14 announcer) is now
  admitted too: Studio's bank reader tripped on eight empty slots at the end of that bank; it now skips
  them.
- **XMA previews (receipt 30).** Studio's side is finished and tested — wrapping, a safe helper runner,
  and the Xbox 360 sound-bank route — but the helper program could not be built on this machine (no
  build toolchain; installing one needs authorisation). Without it, XMA entries stay listed and framed
  and say why.
- **Goalie gear 4.5 (this receipt).** Masks, pads, blocker, trapper, goalie stick and neck protector are
  edited, reviewed, refused past their range, undone, redone, saved and reopened on every anchor that
  has goalie equipment (25) and on the genuine NHL 20. No title has a goalie-gear catalogue, so every
  value shows as its stored number. Which mask or pad the game draws for a number is not claimed.

## 2. This receipt's files

| Path | Change |
|---|---|
| `crates/studio-domain/tests/equipment_goalie_gear.rs` (**new**) | the 4.5 sweep: 27 anchor directories (25 with the goalie companion, NHL 07 recorded) + the genuine NHL 20 (private fixture) |
| `crates/tdb-speech/src/lib.rs` | `parse_bank` skips all-zero id-table slots (`empty_slots`), refuses partly zero ones; unit test |
| `crates/tdb-snd/src/easpeex.rs`, `crates/studio-domain/tests/easpeex_preview.rs` | X360 NHL 14 `ann_eng` admitted with its regression |
| `defs/enums/nationality-nhl10.toml`, `-nhl11-12.toml`, `crates/studio-domain/tests/nationality_labels.rs` | British code "England" (receipt 28 §4 correction) |
| records | receipts 28 / 29 addenda, LEDGER, STATUS, closeout, MORNING-REVIEW (V-74, A-SPX-2), claims, the review plan / tracker |

## 3. Evidence for this receipt (`docs/validation/evidence/overnight-development-2026-09-24/logs/`)

- `p45-goalie-gear.log` (`NHL_STUDIO_FIXTURES=E:\nhl-studio-fixtures`, `--include-ignored`): 2 / 2.
  Declared ranges: NHL 08 `masktype` 0..3, `maskpattern` 0..255, `maskrank` 0..3, `pads` / `blocker` /
  `trapper` 0..31, `stick` 0..190; NHL 09 the same mask columns with `masktype` / gear 0..127 and `stick`
  0..7; NHL 10 → PS4 16 `maskid` 0..511, gear 0..127; PS4 17 / 18 gear 0..255; PS4 19 / 20 anchors
  `maskid` 0..1023, gear 0..511; PS4 21 **and the genuine NHL 20** `maskid` and gear 0..1023;
  `neckprotector` 0..1 everywhere. 7–8 columns edited per title; pending text = stored number; past-range
  refused; undo / redo; save → reopen on a disposable copy.
- `c16-nationality-green-2.log`: `nationality_labels` 4 / 4 after the "England" correction.
- `r31-sbkr-empty-slots.log`: `tdb-speech` 4 / 4 + its real-tree cases 4 / 4 (`--ignored`), `tdb-snd` lib
  20, `easpeex_preview` 3 / 3 with `--include-ignored` (11 gen7 tuples incl. X360 NHL 14 `ann_eng`:
  sample 45, 143 frames → 90 795 samples; 14 PS4 banks), `gen7_speech_preview` 1 / 1.
- Measured: the X360 NHL 14 `ann_eng` bank ends with 8 all-zero slots (id 0, count 0, pointer 0) of
  23 255; X360 NHL 11 / 13 / 15 banks have none.

## 4. Coverage statement (targets, not anchors)

The 27 anchor directories are not the 27 shipping targets: NHL 07 PS3 is research-only; Legacy
Recompiled has no independent anchor and was not swept (the XMA and Speex routes refuse it by name);
the committed `ps4/nhl20` directory is an NHL 19 base copy and the genuine NHL 20 was covered through
the private fixture. Database parity does not establish file mapping or rendered selection.

## 5. Checks across the prompt (receipts 28–31)

Rust targets run with explicit selection (`zz_diag` never selected): studio-domain lib 223 / 2 ignored;
`goalie_consistency` 4; `nationality_labels` 4; `identity_ranges_nhl20_21` 2 (incl. genuine NHL 20);
`goalie_overall_consistency_input` 2 (incl. genuine NHL 20); `equipment_goalie_gear` 2; `easpeex_preview`
3; `gen7_speech_preview` 1; `gen7_sfx_banks` 6 (+1 without a helper); `ps4_speech_bank` 5;
`capabilities` 5, `import_change_set` 3, `nhl07_encodings` 8, `read_matrix` 2, `roster_builder_e2e` 1,
`surface_matrix` 1, `surfaces` 4; `tdb-speex` 3; `tdb-snd` lib 20 + `helper_runner` 3 +
`ea_speex_real_vectors` 1 (96 vectors) + `ea_xma_real_vectors` 1 (83 vectors); `tdb-speech` 4 + 4
ignored-real; `tdb-domain-validation` 16; `tdb-enums` 7; studio-api lib 31, `player_identity` 2,
`roster_builder` 1. Bindings: `export-bindings --write` (1 file, receipt 29) then `--check` exit 0.
Frontend: `tsc` exit 0; vitest 40 files / 250 tests. Not run: full workspace suite, repo-wide fmt /
clippy (baseline defects, C0.2), any desktop / game / listening run.

## 6. Cleanup

Scratch patch scripts deleted from the session scratchpad; the upstream Speex tarball deleted after
vendoring (hashes kept); no `nhlms-*` temp root of these packages remains in `%TEMP%` (older runs' and
other lanes' roots left alone); no extra build cache (release cache reused once for the review build;
`E:\nhlms-review-build-2026-09-27` is kept on purpose — it is the user's stable review build; delete the
folder to remove it). No toolchain, helper binary or source tarball installed.

## 7. Limits and open items

- **Helper build (XMA2)** — blocked: an authorised build toolchain is needed; recipe for review in
  receipt 30 §4.
- **Goalie Overall on PS4** — proposal only (+36 consistency input on PS4 NHL 15 / 16), receipt 28 §5.
- **Zero-height placeholders (C16 D3)**, **code 127 before NHL 13**, **style / eye-colour labels
  (D6 / D7)** — unresolved; nothing reinterpreted.
- **SNS XMA speech / music listing** (X360 NHL 09–12) validated in `tdb-snd` but not wired; `.ast` XMA
  (R92) and NHL 09 archived banks (R100) stay framed / refused.
- Goalie gear: assignment only — no catalogue, geometry, texture or thumbnail chain for 4.5 yet.

## 8. Next task

**Wire the already validated SNS EA-XMA route into the gen7 speech / music listing for Xbox 360 NHL 10–12
(`ann_eng`, `pbp_eng`, `music_samples`) behind the helper gate, with the same per-tuple listing / preview
regression as the Speex tuples** — unless the coordinator returns a defect, a helper-toolchain
authorisation or an implementation-ready League / Team contract, which take precedence. The goalie-gear
id → catalogue / asset question (4.5's model and paint bindings) is published for research in claim
`dev-p45-goalie-gear` rather than started here.
