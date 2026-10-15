# Receipt 59 — Legacy Recompiled: player edits unblocked (defect) and sound banks from its own archives

**Developer-verified; not Codex-accepted; nothing launched, listened to or written.** HEAD `8d60a6d` (unchanged; nothing staged).
Claim `dev-recompiled-id-and-audio`. Research consumed: 2026-09-28 handoff 04 §2 / §6 (R138); R128.

## 1. Plain-language summary

- **Defect fixed: on a Legacy Recompiled project, weight, ratings, birth dates and contract dollars were read-only.**
  - The app names this title `legacyrecomp`, but six places in Studio's core expected a different spelling, `nhllegacy-recompiled`.
    No project ever carries that spelling.
  - The player-encoding check therefore called the project "not a title with a known encoding" and locked those cells.
  - The same mismatch left Recompiled's speech coverage, team-logo families and logo archive route pointing at nothing.
  - Found while reading research handoff 04; reproduced red, then fixed. Tests had used the unused spelling, which is why it
    passed before.
- **Recompiled sound banks now list and preview.** The Speech catalogue's "Gen7 sound banks" reads them straight from the install's
  own archives (`game\nocache.big`, `audioboot*.big`, `cacheaudio.big`); the install has no loose `audio` folder.
  - Every bank, record count and stream-id check equals the X360 Legacy tree.
  - Five previews decode to exactly the X360 Legacy PCM: a ram record, a surround pair, a stream, a looping stream and a six-channel
    pair.
  - Each Recompiled preview says its bytes are the X360 Legacy files and that what the recompiled game plays is not observed.
- Either the install folder or its `game` folder can be attached.

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/encoding.rs` | `legacyrecomp` joins the NHL 11 – Legacy family (the unused spelling stays accepted) |
| `crates/studio-domain/src/{speech_catalogue,team_logos,gen7_speech_preview}.rs` | the same id added where the unused spelling was matched |
| `crates/studio-domain/src/league_logos.rs` | `gen7_logo_archive`: Recompiled's id → `cache.big` |
| `crates/studio-domain/src/gen7_sfx_banks.rs` | a Recompiled layout of its own archive members (`game/…` or root); `xma_title` applies the Legacy measurements to Recompiled's ids; `RECOMP_PREVIEW_NOTE`; coverage table rewritten (the "by lineage" row named a route this install cannot serve, handoff 04 §3) |
| `crates/studio-domain/tests/encoding_contract.rs` | red → green assertion with `legacyrecomp` |
| `crates/studio-domain/tests/gen7_xma_ram_residuals.rs` | new `recompiled_banks_are_the_legacy_banks_from_its_own_archives` |
| `crates/studio-domain/tests/gen7_sfx_banks.rs` | the receipt-30 "Recompiled refused" assertion is replaced by "another title on Recompiled is not a population" |
| `crates/studio-api/tests/gen7_asset_member_route.rs` | the logo route for `legacyrecomp` |

## 3. Evidence

- **Red, first:** `encoding_contract` failed with "this project was opened as legacyrecomp on Legacy Recompiled, which is not a title /
  platform combination with a known player-row encoding. Weight, ratings, birth dates and contract dollars are not editable…". After
  the fix, 10 / 10 pass (`NHL_STUDIO_FIXTURES` set).
- **Recompiled audio (new case, read only):**
  - Registered install `H:\Emulators\games\XBOX\NHL Legacy Decomp Test`; X360 Legacy loose tree; pinned helper `dc3533ed…`.
  - Bank shape lists are equal, and every bank is an archive member.
  - `playercalls` #0 (stream via `cacheaudio.big`), `002_chant` #0 pair 2 (ram), `ambientskatesandpunches` #132 (looping stream),
    `crowds` #0 pair 2 (4-ch loop) and `sfx` #441 pair 3 (6-ch) give byte-identical WAVs and identical bank SHA-256s.
- **Suites:**
  - `gen7_sfx_banks` 7 / 7 · `gen7_xma_ram_residuals` 5 / 5 · `gen7_speech_preview` 1 / 1 · `eaxma_speech_preview` 2 / 2 ·
    `easpeex_preview` 3 / 3 · `team_logos` 4 / 4 (all `--include-ignored`)
  - API `gen7_asset_member_route` 5 / 5 · `studio-domain --lib` 226 passed, 2 ignored
- **First failures, kept:**
  - In one run `encoding_contract`, `gen7_asset_member_route` and `team_logos` failed only because `NHL_STUDIO_FIXTURES` was unset
    ("NHL_STUDIO_FIXTURES is not set"). All passed with it set.
  - `gen7_sfx_banks` failed once on the receipt-30 assertion that this receipt supersedes; it was updated as above.

## 4. Limits and what was not changed

- **Roster-save ceilings stay unset for Recompiled** (`capacity::gen7_ceilings`). Its save compatibility is not established (handoff
  04 §7), so growth there is refused as before. This is recorded, not changed.
- Other Legacy-only match arms (kits, venue art, containment, equipment components, …) deliberately leave Recompiled unmeasured, or
  already use its own route. They were not widened.
- Which archive copy the recompiled executable loads is not observed. Nothing is written to Recompiled audio.
- Included in the r59 bundle (after r56).
