# Receipt 16 (P3.2-E) — inline strings carry their byte width; the editor refuses a too-long name before the write

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD
`8d60a6d` (unchanged). Claim: `dev-p32e-string-width`. Clock: 07:26 → 07:40 (MDT, 25 Sep 2026; overlapping package 15's regression run).

## 1. Plain-language summary

Player names and birth cities are fixed-width inline strings in the database. Until now the editor
learned that a name was too long only from the writer's refusal after the write attempt. Every
string field view now carries its byte width, and the inline editor checks the typed text against
it (in UTF-8 bytes, so "Litoměřice" counts 12, not 10) before anything is sent: a value that would
not fit shows "13 bytes — this field holds at most 12; shorten the text (accented letters take two
bytes). Nothing was written" next to the unchanged value, and a value that fits commits as before.
The writer's own refusal stays as the second line of defence and is pinned to the same width by the
identity sweep on all 27 anchors.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/view.rs` | `FieldView.text_max_bytes: Option<u32>` — `Some(bit_width / 8)` for inline strings, `None` otherwise |
| `crates/studio-api/src/dto.rs`, `convert.rs` | `FieldViewDto.text_max_bytes` |
| `app/src/bindings/FieldViewDto.ts` (generated) | the new optional field |
| `app/src/components/EditableField.tsx` | the string control measures UTF-8 bytes on commit; a refusal is reported to the owning field and shown outside the click-to-edit shell (which closes on Enter / blur), cleared on the next write |
| `app/src/components/EditableField.test.tsx` (**new**) | over-width refused with nothing sent; exact-fit multi-byte value commits; bytes counted, not characters |
| `crates/studio-api/tests/player_identity.rs` | the last-name view states a width in 8..=64 and the writer's refusal quotes the same width |

## 3. Checks

| Run | Result |
|---|---|
| `export-bindings --write` then `--check` | `FieldViewDto.ts` rewritten; in sync (277 files, roots unchanged) |
| `npx vitest run` (full) | **38 files, 236 passed** (EditableField 2) |
| `npx tsc --noEmit` | clean |
| `cargo test -p studio-api --lib` | 31 passed |
| `cargo test -p studio-api --test player_identity` (27 anchors) | **1 passed — 27 anchors (169 s); last-name widths 160 B on NHL 07, 25 B on 15 anchors, 40 B on 10** |

## 4. Limitations

- The pre-check counts bytes as the writer does; the game's own rendering of long names (truncation
  in overlays) is a runtime question outside this package.
- Other string editors (team names on the team page, the nameplate editor) read the same
  `FieldViewDto` and go through the same `EditableField`, so they gain the check; surfaces with
  their own inputs (dictionary re-text) were not touched.

## 5. Cleanup

Scratchpad `patch_text_width.py`, `patch_text_width_b.py` deleted by literal path; no temp roots.

## 6. Next action

Per the ledger: the **contract-dollar granularity** for NHL 08–10 (10-bit) and PS4 18–21 (14-bit)
— a research question unless a known contract anchors it; then the ratings-scale confirmation for
NHL 07–10 (research card) — unless the morning review returns a defect.
