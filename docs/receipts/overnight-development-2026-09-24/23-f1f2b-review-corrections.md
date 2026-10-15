# Receipt 23 (review 2026-09-26 F1 / F2) — a sound bank's streams are read only from its own stream file; an unrecognised database layout gets no encoding

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged). Claim: `dev-f1f2b-review-corrections`. Direction:
[packages 20–22 review](../../validation/development-20260926-packages20-22-review.md), F1 and F2.

## 1. Plain-language summary

Two gaps the coordinator found are closed. First, the sound-bank preview could decode a stream from
a stream file whose identity did not match the bank. The listing warned, but the preview went
ahead. Now every stream entry is framed and decoded only after the stream file's header is read on
the same file handle and its id equals the bank's. A missing, malformed, mismatched or
replaced-after-listing stream file refuses the stream entries in words, in the tree and in a direct
request, while the bank's inline (ram) samples keep working.

Second, any little-endian database was treated as a PS4 title, and any big-endian one with a
`draftyear` field as NHL 11 – Legacy. The classifier now checks a bounded signature of exactly the
fields the encoding covers: weight, birth month / day / year, contract dollars, the draft field,
the helmet width and the rating class of both companion tables. A layout that matches no measured
family is unresolved: those cells show their stored number, are read-only, and refuse writes.
A project opened under a title / platform pair with no known encoding is now a conflict, not
silent agreement, and the import seams refuse under a conflict too.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/gen7_sfx_banks.rs` | `frame_stream` checks the stream header magic and id on the reading handle before any framing (F1); later reworked into the archive-aware resolver of receipt 24, keeping the same checks |
| `crates/studio-domain/src/encoding.rs` | `schema_family` = bounded descriptor signature (types, widths, declared ranges, rating classes, discriminators), unresolved otherwise; `encoding_conflict` treats an unsupported declared title / platform as a conflict; `nhllegacy-recompiled` recognised |
| `crates/studio-domain/src/session.rs` | `require_encoding_agreement` guards `lower_change_set` and `apply_import`; the raw `set_cells_int*` routes stay ungated by design (documented) |
| `crates/studio-domain/tests/gen7_sfx_banks.rs` | F1 regressions on owned disposable copies: unchanged pair, flipped id, flipped magic, truncated, missing, replaced after listing, ram-only bank, ram entries of a mixed bank beside a broken stream file |
| `crates/studio-domain/tests/encoding_contract.rs` | F2 regressions: 5 altered / contradictory descriptor negatives + six-bit-declared-0..99; unresolved layout → raw read-only weight, refused write, other cells write; unsupported declared title / platform → conflict, `set_field` and `apply_import` refuse; Recompiled agrees |

No DTO / binding change for these two corrections.

## 3. Evidence

- [`f1b-sfx-stream-identity.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f1b-sfx-stream-identity.log) —
  4 passed incl. the identity regression; "ram record 1 of crowds.abk previews with a mismatched
  crowds.ast (105 ram entries previewable)".
- [`f2b-layout-descriptor-measurement.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f2b-layout-descriptor-measurement.log) —
  the per-anchor descriptors the signature is built from (27 directories; temporary probe deleted).
- [`f2b-encoding-signature.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f2b-encoding-signature.log) —
  encoding suite 9 passed, 1 ignored (private fixture).
- [`f1f2b-aggregate.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f1f2b-aggregate.log) —
  studio-domain lib **222 passed, 0 failed, 2 ignored**; every studio-domain test target except `zz_diag`
  **242 passed, 0 failed, 31 ignored**; studio-api **140 passed, 0 failed, 35 ignored**; private NHL 20
  witness 1 / 1. 0 `FAILED` / `panicked` lines. Built from the F1 / F2 code (started before receipt 24's
  edits; the domain and API test binaries were compiled at the start, the NHL 20 step at the end).
- [`f1f2b-nhl20-witness.log`](../../validation/evidence/overnight-development-2026-09-24/logs/f1f2b-nhl20-witness.log) —
  `encoding_contract` with `--include-ignored` on the code after receipt 24: 10 / 10, including the real NHL 20 private
  fixture (`the_real_nhl20_database_carries_the_ps4_contract`).

## 4. Red evidence

F1's red is the coordinator's reproduction (flipped id byte 7 → preview returned 33 207 samples);
not re-run by the developer on the old code. F2 was a code-inspection finding; no red run exists —
under the old classifier the new negatives would classify as PS4 / NHL 11–Legacy by construction.

## 5. Limits

- The signature allows exactly the measured variations (month / day ranges on PS4 16–21; 13 / 14-bit
  contract and 8–10-bit helmet on PS4). A future title with other widths is unresolved until
  measured, by design.
- Negatives alter metadata descriptors, not binary field widths; binary-width alterations are not
  synthesised.
- Unsupported-title conflict applies when a title is declared; the legacy file-pick path (no title)
  remains "unknown title", as before.
