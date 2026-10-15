# S2-A — archive replacement planning and streamed, validated staged output

> **Package:** S2-A of [s1-review-and-s2a-dispatch](../validation/s1-review-and-s2a-dispatch.md).
> **Date:** 2026-09-16. **Author role:** developer. Reviewer: Codex (architect).
> **Branch:** `feat/phase5-asset-tooling` at `8d60a6d` with the dirty working tree preserved.
> **Nothing staged, committed or pushed.** No UI, no generated DTO, no LZX, no registry-schema
> change, no live install, no swap into a game tree, no QA generator run, no `capability.csv` edit.
> S1 closure is recorded in [s1-chunkzip-multichunk-codec §10](s1-chunkzip-multichunk-codec.md#10-closure-after-codex-review-2026-09-16-s1-review-and-s2a-dispatch).

---

## 1. Result in one paragraph

`crates/tdb-archive-big/src/stage.rs` is a planner and streamed writer for `EB\0\3` archives with
16-byte rows — the PS3 NHL Legacy `patch.big` layout. It binds a source (length, SHA-256, tables,
every entry's span and stored digest, the layout facts), resolves explicit replacements into
**no-op / fit-in-slot / append-and-retarget**, streams a new file from the source with checked
64-bit arithmetic and representability checks done before any output exists, and verifies the
result by re-parsing it, decoding every changed entry against the expected decoded content and
comparing every untouched byte with the source. `EbFile`'s full reflow is not used. Bounded
measurement of the real layout resolved the row `+12` word: it is the **djb2 hash of the entry's
full path** on all 16 541 rows of the three 16-byte-row archives on record, so a replacement that
keeps the path leaves it correct. Demonstrated on the real `patch.big` (60 596 976 bytes): a no-op
reproduces the source byte-for-byte; two recompressed members fit in place; a four-edit plan with
three appends verifies with every untouched byte equal and every old payload still in place. 17
synthetic cases cover the required failure paths. The output is staged only; apply/restore is S2-B.

## 2. Inputs and layout findings (bounded)

| Input | Use |
|---|---|
| `crates/tdb-archive-big/src/eb.rs` (`EbHeader`, `EbArchive`, `decode_payload`) | reused for parsing; `FILE_TABLE_OFFSET` made `pub` |
| C0-AS7 member records (`docs/receipts/data/c0-as7-*-members.csv`) + `patch.big` | the layout measurements below |
| `H:\Emulators\RPCS3\dev_hdd0\game\BLUS31540\USRDIR\patch.big` (SHA-256 `d2d26273…9bbe0`) | the demonstration specimen; opened read-only |
| `sha2 0.10` (already in the lock) | `tdb-archive-big` dependency; lock gains the edge only |

Measured on `patch.big` (459 rows) and cross-checked on the two Xbox 360 Legacy records (3 237 +
12 845 rows) and, for the PS4 shape, `boot.big` / `overlays_21.big`:

| Fact | PS3 `patch.big` | X360 `cache`/`nocacherender` | PS4 20-byte rows |
|---|---|---|---|
| row `+12` (16-byte) / `+0x10` (20-byte) | `djb2(path)` 459/459 | 16 082/16 082 | `+0x10` = `djb2(path)` 7 365/7 365 (`boot.big`) |
| row `+4` | 0 on all | 0 on all | 0 on all |
| PS4 `+0x0C` | — | — | **not** a content or path hash (30 distinct values over 257 `overlays_21` rows; 11 candidates tried); **unknown** |
| gaps between payloads | 0–15 zero bytes, file-table order = offset order, no gap non-zero | (from AS7: last end = archive size) | — |
| `archive_size` | = file length = end of last payload, 16-aligned | = file length | every observed size is 16-aligned |
| header `0x10` (17 376 here) | points inside the name table; **meaning unknown**, copied verbatim | | |
| `0x20..0x30`, file-table→name-table gap (464 zero bytes) | opaque, copied verbatim | | |

`djb2`: `h = 5381; h = h*33 + byte` over the forward-slash path. The identification is a fact about
these rows, not a statement that the game validates the word.

## 3. What the module does

### 3.1 Bind (`BoundSource::bind`)

Reads the source only. Refuses: a version other than `0x0000_0400` / 16-byte rows
(`LayoutUnqualified` — the PS4 variant is refused, see §2), `archive_size ≠ file length`, any span
outside `[data_start, EOF)`, parse failures. Records per entry: path, offset, size, both row words,
stored SHA-256, stored form (raw / `chunkzip` / `chunklzx`), **slot end** (the next 16-byte line
after the span, or the next entry's offset if earlier), and **shared span** (intersects another
entry). Records layout facts: path hash verified, `+4` all zero, padding all zero, nonzero gap
regions (exact runs), table order = offset order, opaque regions. Deploy blockers: any row whose
`+12` ≠ path hash or whose `+4` ≠ 0 (the meaning of such a row is unknown). Three passes: tables,
sequential hash + gap check, per-entry digests.

### 3.2 Plan (`plan`)

Each `Replacement` binds `entry_index`, `expected_path`, `expected_stored_sha256`, the new stored
bytes and the **expected decoded content** (length + SHA-256). Refused: unknown index, path or
stored-hash disagreement, duplicate index, shared span, `chunklzx` new bytes, size > `u32`, new
bytes that do not decode to the expectation (decoded at plan time, so a plan that could only fail
at verification never exists), append offset > `(2^32−1)·16` (`OffsetNotRepresentable`), any
overflow (`Overflow`, never a wrapped offset). Placement, in ascending entry order against the
staged state:

| Placement | Rule | Row edit | Bytes written |
|---|---|---|---|
| **no-op** | new digest = stored digest | none | none |
| **fit-in-slot** | `offset + new_len ≤ slot_end` (own span + own zero pad up to the 16-byte line; never a neighbour, never a nonzero pad) | `size` only | `[offset, max(old_end, new_end))`; slack to the old end zero-filled |
| **append-and-retarget** | otherwise; at `align16(staged_eof)` | `block` + `size` | `[staged_eof, align16(end))`; old bytes stay |

`new_eof = align16(last append end)` (or the original EOF when nothing was appended); a header
`archive_size` edit follows. `changed_ranges` is the sorted, merged union of row edits, the header
edit, fit spans and appended spans. Two plans over the same inputs are identical (asserted).

### 3.3 Stage (`stage`, `stage_file`)

Streams in 1 MiB chunks: patched prefix (row/header edits applied to the source's bytes; opaque
regions verbatim), the data region with fit spans substituted, then the appends and final pad.
Hashes the source while copying and returns `SourceChanged` if it differs from the bound identity
(length checked before any read; content after the copy). `stage_file` opens the source read-only,
creates the output with `create_new` (an existing file is refused and left alone), syncs, verifies,
and **removes the output on any failure**. The source is never opened for writing.

### 3.4 Verify (`verify`)

Output length = planned; output SHA-256 = the stage report's; header re-parsed and `archive_size` =
planned; entry count unchanged; **every** row equals the planned prefix (edited rows planned,
unedited rows identical); every changed entry's stored bytes hash to the plan, decode, and match the
expected decoded content; every byte of `[0, original_eof)` outside `changed_ranges` equals the
source. Counts are returned, not booleans.

## 4. Demonstrations on the real `patch.big`

`cargo run -p tdb-archive-big --example s2a_stage_demo -- --source <patch.big> --out <new> --edit …`
(outputs in the session scratchpad; source untouched, SHA-256 re-checked `d2d26273…` afterwards).
Bind: 459 entries, path hash verified, `+4` zero, padding zero, table order = offset order, opaque
regions `[16..20, 32..48, 7392..7856, 23921..23936, 25224..25232]`. **No structural
contradictions** (`deploy_blockers` empty) — which is not qualification: the qualification state of
every run below is **`Unqualified`** (§10, R2), with header word `0x10 = 17376` unresolved, each
replacement unresolved, and native loader acceptance unresolved.

| Run | Edits | Placement | Output | Verified |
|---|---|---|---|---|
| no-op | 131 `blocker_0_82_0_dm.rpsgl` with its own bytes | no-op; `changed_ranges = []` | 60 596 976 bytes, SHA-256 **`d2d26273…9bbe0` = source** | 459 rows, 1 decoded, 458 rows compared, 60 596 976 untouched bytes equal |
| fit | 139 `blocker_0_86_1_tm.rpsgl` recompressed (2 041 → 2 022) and 306 `name_1_365_7_cm.rpsgl` recompressed (1 397 → 1 399) | both **fit-in-slot** (139 shrinks, slack zeroed; 306 grows 2 bytes into its own pad, slot end 27 190 448) | same length, SHA-256 `ca837b17…`; rows `size` `07f9→07e6`, `0575→0577` | 2 decoded, 457 rows, 60 593 504 untouched bytes equal |
| growth | 131 recompressed (43 859 → 44 586 > slot 43 872) | **append-and-retarget** @ 60 596 976; row `000fee05…ab53` → `0039ca2f…ae2a`; header `archive_size` → 60 641 568 | 60 641 568 bytes | 1 decoded, 458 rows, 60 596 952 untouched bytes equal |
| multi | 0 `na.ini` no-op; 2 `nhl_cze_cz-meta.xml` +100 000 bytes; 4 `nhl_eng_us-meta.xml` +4 096; 131 recompressed | no-op; three appends at 60 596 976 / 60 697 952 / 60 703 024, each on a 16-byte line; `new_eof` 60 747 616 | 60 747 616 bytes, SHA-256 `52cb257e…` | 4 decoded, 455 rows, 60 596 920 untouched bytes equal |

Independent Python check on all four outputs: **no byte inside the original span differs outside
the planned ranges**; in the multi output the old payloads at 36 256, 1 990 960 and 16 703 568 are
byte-identical to the source; the header `archive_size` equals the file length.

After R2 (§10) the same tool on `--edit 139:recompress --edit 4:grow` prints, with
`deploy blockers carried: []`, `qualification: Unqualified (deployable: false)` and these items:
`+12` path hash **Resolved**; `+4` **Resolved**; **header word 0x10 Unresolved** (nonzero 17 376,
meaning unknown, preserved verbatim, not established to stay correct after payload/table/length
changes); `0x20..0x30`, table gaps, payload gaps, `archive_size` **Resolved**; **entry 4
replacement Unresolved** (append-and-retarget; acceptance not established); **entry 139 replacement
Unresolved** (fit-in-slot; `chunkzip` framing differs from native — `chunk_size` 131 712, one chunk,
where every observed member uses 131 072; the single-chunk encoder is not qualified for native
loading); **native loader acceptance Unresolved** (no game-verified load exists).

## 5. Synthetic cases (`crates/tdb-archive-big/src/stage/tests.rs`, 17)

bind: identity/entries/layout facts and opaque regions · 20-byte variant refused · `archive_size`
mismatch, span past EOF, truncated header · unknown `+12`/`+4` words → deploy blockers, shared spans
flagged, nonzero pad → that entry loses slack only, region reported · plan: identical bytes are a
no-op and the staged output is byte-identical (padding and EOF included) · smaller payload fits,
slack zero-filled, nothing moves, row `block`/hash words unchanged · growth into own zero pad up to
the line, one byte past it appends · append at the next line, row retargeted, header edited, old
bytes left, every other byte identical · three edits coalesce deterministically (fit + two appends,
predictable offsets, identical plans on repeat, re-bound output plannable with old bytes reported
as nonzero gaps) · refusals: duplicate, wrong path, wrong hash, no such entry, `chunklzx`, wrong
expected decode, undecodable new bytes, shared span · **offsets beyond 4 GB** on a hand-built bound
source (5 GiB unaligned EOF → append at exactly 5 GiB, block word `= offset >> 4`; at the block-word
limit accepted, one past refused; `u64::MAX−3` → `Overflow`) · stage refuses a source changed by
one byte, a source of another length (before reading), a plan for another source · truncated
writes at 0 / 10 / half / EOF+5 bytes → `Io`, source bytes unchanged · verify catches: whole-file
hash, an untouched payload byte, an opaque header byte, an untouched row, the appended payload, the
retargeted row, `archive_size`, length · `stage_file` creates new, refuses existing (left intact),
removes the output when the source changed · range helpers · `path_hash` against two recorded PS3
row words.

Not exercised: streaming a real multi-gigabyte sparse file (planning beyond 4 GB is covered; the
copy loop is offset-agnostic).

## 6. Checks run

| Check | Result |
|---|---|
| `cargo test -p tdb-archive-big` | 45 passed, 1 ignored (`chunklzx` oracle) — 17 new |
| `cargo test -p tdb-archive-codec` | 45 passed, 6 ignored; `NHL_STUDIO_FIXTURES=E:/tmp/nhl-studio-fixtures … --ignored` → 6 passed |
| `cargo clippy -p tdb-archive-big -p tdb-archive-codec --all-targets -- -D warnings` | clean |
| `rustfmt --check` on every file touched by S1/S2-A | clean |
| `cargo test -p studio-domain --lib`, `-p tdb-canonical` | 148 / 1 ignored; 552 — unchanged |
| demonstrations §4 | four staged files verified; source SHA-256 unchanged |

## 7. Blockers and unknowns carried forward

1. **Deployability of `patch.big` output is not claimed, and the handoff now says so
   machine-readably** (§10, R2): `qualification.state = Unqualified` with the open items named —
   header `0x10` (nonzero, unknown), every replacement (game acceptance unmeasured; the
   single-chunk encoder's framing is not native), and native loader acceptance (no game-verified
   load). `deploy_blockers` (structural contradictions) is a separate, narrower list.
2. **PS4 20-byte rows are refused**: `+0x0C` is unknown (§2). No PS4 plan can be made until it is.
3. **Patch-versus-loose precedence** stays unresolved (C0-AS7 §F.4); staging an identified entry
   does not decide it.
4. Old payload bytes are left in place after an append (by requirement). A re-bound staged output
   reports them as `nonzero_gap_regions`; they are preserved on further staging and could be
   reclaimed only by a compaction that S2-A deliberately does not perform.
5. Header `0x10` meaning; `0x20..0x30`; the 464-byte zero gap — opaque, preserved, unexplained.

## 8. Files changed (S1 closure + S2-A)

| Path | Change |
|---|---|
| `crates/tdb-archive-big/src/stage.rs` | **new** — bind / plan / stage / verify / `stage_file` |
| `crates/tdb-archive-big/src/stage/tests.rs` | **new** — 17 cases |
| `crates/tdb-archive-big/examples/s2a_stage_demo.rs` | **new** — the §4 tool |
| `crates/tdb-archive-big/src/lib.rs` | `pub mod stage;` |
| `crates/tdb-archive-big/src/eb.rs` | `FILE_TABLE_OFFSET` made `pub` (one line) |
| `crates/tdb-archive-big/Cargo.toml` | `sha2 = "0.10"` |
| `crates/tdb-archive-codec/src/lib.rs` | S1 closure: attribution, stale comments, mojibake, encoder flag `1`, 2 encoder tests, `fixtures` module registration |
| `crates/tdb-archive-codec/src/fixtures.rs` | **new** — private-fixture contract, 6 fixture cases, 5 self-contained cases |
| `crates/tdb-archive-codec/Cargo.toml` | `[dev-dependencies] sha2 = "0.10"` |
| `Cargo.lock` | the two dependency edges only |
| `docs/receipts/s1-chunkzip-multichunk-codec.md` | §10 closure |
| this receipt | |

Outside the repository: `E:\tmp\nhl-studio-fixtures\chunkzip\…` (six fixture files, S1 §10).

## 9. Proposed S2-B transaction interface (revised by R2)

S2-A hands the apply step exactly this (all present in `Plan` + `StageReport` +
`VerificationReport` today):

```text
StagedArtifact {
  source:   { path, len, sha256 }              // bound identity; apply re-checks it
  output:   { path, len, sha256 }              // the staged file, verified
  layout:   LayoutReport                       // version, row_len, opaque regions, facts,
                                               //   qualification_items (structural)
  original_eof: u64                            // = source.len
  new_eof:      u64                            // = output.len; header archive_size
  changed_ranges: [Range<u64>]                 // sorted, merged; everything else in
                                               //   [0, original_eof) is byte-equal to source
  edits: [{ entry_index, path, placement, old_offset, old_size, old_stored_sha256,
            new_size, new_stored_sha256, expected_decoded{len,sha256},
            row_edit{row_offset, old_row[16], new_row[16]} }]
  header_edit: { old_archive_size, new_archive_size }?
  verification: { entries_reparsed, changed_entries_decoded, unchanged_rows_compared,
                  untouched_bytes_compared }   // STRUCTURAL: the output matches the plan
  deploy_blockers: [String]                    // structural contradictions of the layout
  qualification: {                             // DEPLOYMENT: affirmative, never implied
    state: Unqualified | Qualified,            //   Qualified only when every item is Resolved
    items: [{ subject, status: Resolved | Unresolved | Blocked, note }]
  }
  deployable(): deploy_blockers.is_empty() && qualification.state == Qualified
}
```

**Consumer contract.** Applying to a **live target** requires all three: `verification` present
and complete, `deploy_blockers` empty, and `qualification.state == Qualified`. `Unqualified` is the
default and the only state any S2-A output can reach today (the native-loader item is unresolved
for every title/platform, and every replacement is unresolved until a game-verified load of that
kind of change exists); a consumer may never treat an absent or unresolved item as accepted.
Applying to a **copied tree or synthetic target** for transaction testing requires only the
structural facts (`verification`, `deploy_blockers` empty) and must record that it did so with an
`Unqualified` artifact; it grants no live-game eligibility. Resolving an item is a separate,
evidence-carrying act (a recorded game-verified run or a format finding), never a side effect of
staging succeeding.

Recovery inputs S2-B needs, and has: the source identity (to refuse applying onto a changed
file), `changed_ranges` and `original_eof`, and the output identity (to verify a copy-then-rename
apply). Old **fit-in-slot** bytes are overwritten in the staged file and are **not** recoverable
from the staged output — S2-B must persist a pre-image of every changed range (or keep the intact
original) before any in-place overwrite; the old offsets and `old_row` values alone do not restore
overwritten payload data. Old **append** payloads remain at `old_offset` in both files. S2-A does
**not** claim that a range log plus the intact original is a crash-recovery protocol; S2-B owns the
pre-image/backup, the journal, the commit boundary (copy-then-rename vs in-place), fsync ordering,
restart recovery, conflict handling, restore verification, and integration with the unified
DB/save/asset change set. The earlier S1 "next package" sketch is superseded by the S2-B dispatch;
neither `EbFile` reflow nor an install swap is authorized by anything here.

## 10. S2-A-R1 corrections (2026-09-16, [s2a-review-and-correction-dispatch](../validation/s2a-review-and-correction-dispatch.md))

### R1 — malformed table layouts are refused before any slice or allocation

`EbHeader::table_layout(input_len)` (new, `eb.rs`) computes the fixed file-table end, name table,
directory table and `data_start` with **checked `u64` arithmetic**, and refuses: a name table that
starts before the fixed file table ends (`EbError::TablesOverlap`), arithmetic overflow
(`EbError::TableArithmeticOverflow`), and tables extending past the input
(`EbError::TruncatedTables`). `EbArchive::parse` now validates against `prefix.len()` before any
indexing, so the shared parser cannot index outside its buffer; `table_end()` is kept for existing
callers, now saturating and including the file table. `BoundSource::bind` and the `verify` reparse
call `table_layout(len)` **before** allocating a prefix or slicing, and additionally require
`data_start ≤ len`. Errors occur before any output is created (`stage_file` creates the output only
after a successful bind + plan).

The review reproducer (64 zero bytes; magic; file count 1; version `0x400`; name table at 32; name
slot 3; dir slot 1; dir count 1; archive size 64) now returns `TablesOverlap { file_table_end: 64,
name_table_offset: 32 }` from bind, from `EbArchive::parse` at prefix lengths 32/49/64, and from
`table_layout`. Regressions (`stage/tests.rs`): the exact reproducer; a 1 000-row file table
claimed past the input; a `0x7FFF_FFFF` file count (refused by header arithmetic alone — no
prefix sized); a directory table claimed past the input; tables that fit but a missing first
payload line; a name table one byte inside the file table and at offset 0 on an otherwise valid
archive; the verification reparse refusing a staged output whose header was rewritten to the
reproducer's shape or to claims beyond the file. Valid fixtures unchanged: every prior case passes.

### R2 — unresolved qualification is explicit in the handoff

`QualificationItem { subject, status: Resolved | Unresolved | Blocked, note }` and
`DeploymentQualification { state, items }` (`stage.rs`). `LayoutReport.qualification_items` lists
every header word/region preserved without being understood (`0x10`, `0x20..0x30`, table gaps,
payload-region gaps — **Unresolved** when nonzero, Resolved when zero) and every row word
understood (`+12` path hash, `+4` — Resolved when consistent, **Blocked** when contradicted, which is
also a `deploy_blockers` entry) plus `archive_size`. `plan` adds one **Unresolved** item per
replacement that changes bytes (placement named; for `chunkzip` payloads the framing compared with
the native 128 KiB chunking — the single-chunk encoder is called out as not qualified; native-framed
payloads still carry an unresolved provenance note) and the always-**Unresolved** "native loader
acceptance" item. `state` is `Qualified` **only** when every item is Resolved; `StageReport`
carries the plan's qualification unchanged and `deployable()` requires empty blockers **and**
`Qualified`. Structural staging stays usable while unqualified. Nothing here invents header
semantics or assumes the encoder is accepted; no runtime experiment was run.

Regressions: bind reports each preserved unknown as an item (nonzero `0x10`/`0x20` Unresolved, all
zero → Resolved; a contradicted row → Blocked + blocker); a fully structurally-resolved layout is
still `Unqualified` through plan → stage → report (no-op plan: no per-entry item, loader item
Unresolved, `deployable() == false` after a passing verification); a single-chunk recompression
carries the framing note and a raw growth does not; a native-framed payload stays Unresolved; a
contradiction is both a blocker and a Blocked item after staging; `from_items` is Qualified only
with every item Resolved; a blocker alone makes `deployable()` false.

### Receipt repair

S1 §10's fixture-root path had a literal TAB and LF where `\t`/`\n` were intended, and line 40's
`EB\0\3` had become NUL/ETX bytes; both restored, the file is UTF-8 text again.

### Checks (R1)

| Check | Result |
|---|---|
| `cargo test -p tdb-archive-big` | 52 passed, 1 ignored (7 new) |
| `cargo test -p tdb-archive-codec` | 45 passed, 6 ignored — unchanged |
| `cargo clippy -p tdb-archive-big -p tdb-archive-codec --all-targets -- -D warnings` | clean |
| `rustfmt --check` on `stage.rs`, `stage/tests.rs`, `eb.rs`, `s2a_stage_demo.rs` | clean |
| demo on `patch.big` (`139:recompress`, `4:grow`) | staged + verified; `Unqualified`, items as in §4; source SHA-256 unchanged |
| `cargo test -p studio-domain --lib`, `-p tdb-canonical` (consumers of `eb.rs`) | 148 passed / 1 ignored; 552 passed — unchanged |
