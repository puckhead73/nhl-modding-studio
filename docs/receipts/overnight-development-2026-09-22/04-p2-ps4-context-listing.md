# Receipt 04 — Priority 2.4: the Contexts listing reaches PS4 (NHL 15–21)

**Developer-verified; not Codex-accepted; nothing game-verified.** Branch `feat/phase5-asset-tooling`,
HEAD `8d60a6d` (unchanged). Real extracted PS4 trees read **only**; nothing written outside the
repository. Nothing staged, committed, pushed, launched or written live.
Claim: `dev-p2-ps4-context-listing`. Research consumed (unedited): 2026-09-21 pm handoff
[15](../../research/overnight-2026-09-21-pm/15-ps4-ctx0-context-records-decoded.md) (`research-r58-ps4-ctx0`).

## 1. Plain-language summary

The Leagues → Commentary page has a read-only "Contexts" card: for each situation the game can
comment on, it shows what starts it, the named conditions it checks, and the lines or follow-up
situations it fires. Until now that card worked on Xbox 360 and PS3 (NHL 11 → Legacy) and simply
reported "not covered" on PS4.

It now covers **PS4 NHL 15 through 21** as well. The PS4 games store the same information in the
same shape, written the other way round for a 64-bit machine, so the card and everything behind it
are shared rather than duplicated — one new reader, no new screen. Checked against the research
lane's independent measurements on all seven PS4 games: every count matches exactly, and every line
the record names is confirmed against the game's own name table.

Two honest limits are carried in the card's own text rather than hidden: the condition **argument
values** are not decoded (the count is shown, the values are not invented), and neither is the
compiled bytecode. A 16-byte-per-record table that the research also left undecoded is reported as a
count.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/tdb-speech/src/ctx0_ps4.rs` | **new** — `is_ctx0_ps4`, `parse_moir_le`, `parse_ctx0_ps4` producing the **existing** `Ctx0` type, plus `Ctx0Ps4Notes` for what that type has nowhere to put (file version, the header's inflated record count, block entries, groups). Every read is offset-checked. |
| `crates/tdb-speech/src/lib.rs` | module + re-exports. |
| `crates/tdb-speech/tests/ps4_contexts.rs` | **new** — the seven-title census against the research figures, plus a refusal case that needs no fixture. |
| `crates/studio-domain/src/speech_catalogue.rs` | `context_coverage` admits PS4 NHL 15–21; `ps4_context_catalogue` + `ps4_speech_dir`; the view builder extracted to `context_view` and shared by both routes (no duplicated presentation). |
| `crates/studio-domain/src/assets.rs` | `ps4_kettle_root` made public — one anchor walk now serves the art roots and the speech tree. |
| `crates/studio-api/tests/speech_catalogue.rs` | the PS4 case through the registry seam on real NHL 21 / NHL 16 trees. |
| `app/src/components/ContextCatalogueCard.tsx`, `app/src/pages/SpeechCatalogue.tsx`, `app/src/pages/Leagues.tsx` | coverage wording only — the card was never platform-gated in the UI, so it started working as soon as the domain answered. No behavioural change, no DTO change. |

No DTO, binding or command signature changed.

## 3. Evidence

### 3a. All seven PS4 titles reproduce the research census exactly

`cargo test -p tdb-speech --test ps4_contexts -- --include-ignored`
([r1-p2-ps4-contexts.log](../../validation/evidence/overnight-development-2026-09-22/logs/r1-p2-ps4-contexts.log)):

| Title | ver | records | triggered | speech items | context items | conditions | block entries | groups |
|---|---|---|---|---|---|---|---|---|
| NHL 15 | 3 | 3 864 | 3 813 | 4 005 | 515 | 15 434 | 0 | 108 |
| NHL 16 | 3 | 4 396 | 4 336 | 4 534 | 650 | 18 565 | 19 | 110 |
| NHL 17 | 3 | 5 178 | 5 119 | 5 316 | 777 | 22 355 | 11 697 | 121 |
| NHL 18 | 3 | 2 676 | 2 640 | 2 692 | 552 | 10 277 | 8 073 | 93 |
| NHL 19 | 3 | 2 998 | 2 962 | 3 019 | 586 | 11 790 | 9 284 | 97 |
| NHL 20 | 4 | 1 418 | 1 397 | 1 414 | 273 | 2 704 | 6 732 | 44 |
| NHL 21 | 4 | 2 153 | 2 119 | 2 147 | 479 | 4 313 | 9 853 | 54 |

Every figure equals handoff 15 §2. Additionally asserted, not merely counted: every speech / context
item's **name spelled in the record equals the name in the title's MOIR table** (a mismatch is a
parse refusal, not a warning); no item lands in an unknown namespace; every trigger name is a
`trigger_*` string from pool 2; a record carries a trigger **iff** its kind word is 1.

### 3b. Through the public route

`cargo test -p studio-api --test speech_catalogue the_ps4_context -- --ignored`: **1 passed** — the
registry seam on the real NHL 21 and NHL 16 extracted trees returns `platform: "ps4"`, the census
counts, a note naming the file version and the undecoded parts, the query narrowing and the cap
truncating, and a hand-checked record (`CUP_ENDINGS_SEQ5_TROPHY_HOIST_EASTERN_CONFERENCE_NHL` →
`trigger_CUP_ENDINGS`, one condition, non-empty items).

### 3c. A layout the research did not specify, measured before use

Handoff 15 §1 gives the 24-byte condition struct but not how a record addresses its condition list.
The obvious reading — a flat array at the pointer — resolved only **67 / 440** sampled NHL 15
conditions (and 88 / 440 one entry later, 44–72 / 440 at other strides). Measuring the alternatives
showed the word addresses an **array of `u64` pointers**, one per condition: **440 / 440**. That is
what the parser implements, with the measurement recorded in a comment beside it, and the
seven-title test asserts the resulting condition counts. Nothing was adopted on the strength of the
first plausible reading.

### 3d. A defect in my own code, caught by my own test

The first run panicked with "attempt to add with overflow" inside the new reader: `le32`/`le64` did
`o + 4` before checking bounds, and a `0xFFFF…FF` sentinel pointer read out of the file reached it —
the same class of defect the R2 correction had just fixed in the texture writers. All reads are now
`checked_add`-guarded and return a refusal naming the offset. The parser refuses rather than panics
on any pointer a malformed file can carry.

## 4. Checks run

| Run | Result |
|---|---|
| `cargo test -p tdb-speech --test ps4_contexts -- --include-ignored` | **2 passed** (7 titles + the refusal case) |
| `cargo test -p studio-api --test speech_catalogue the_ps4_context -- --ignored` | 1 passed |
| `cargo test -p tdb-speech -p studio-domain --lib` | **177 passed, 0 failed, 2 ignored** |
| `npx vitest run src/components/ContextCatalogueCard.test.tsx` | 2 passed |
| `npx tsc --noEmit` | clean |

## 5. Coverage effect (Phase 1 matrix 1.5 / Phase 2 matrix 2.7)

| Row | Before | After |
|---|---|---|
| Context listing (`pbp_*contexts`) | read-only listing, **gen7 only** — X360 / PS3 NHL 11–Legacy | **+ PS4 NHL 15–21, impl+tested (dev)**, all seven titles reproducing the research census through the public route. Parity for this operation is now complete across the measured families |
| Condition argument values | not shown (gen7 decodes them) | **PS4: not decoded** — the count is shown, the values are not invented; a named gap, not a silent one |
| `.gas` bytecode, runtime selection, priority / time / kind semantics | undecoded | unchanged |

Writing / reassigning contexts remains **not implemented** on every platform: this is inspection.

## 6. Open / not done

- PS4 condition **argument values** (layout unspecified by the research; the count is exposed).
- The per-record 16-byte block table (0–12 entries; the PS4 form of Legacy's slot block) — counted.
- The PS4 **speech catalogue** (events / sentences / samples) is still refused on PS4: that is the
  SBle / `pbp_sentencedata` chain, a separate package. Only the *contexts* card crossed over.
- No runtime card is opened: this is a read-only listing, and nothing about which context the engine
  picks is claimed.

## 7. Cleanup manifest

| Artifact | Disposition |
|---|---|
| Scratchpad `p2_ps4ctx_domain.py`, `p2_ps4ctx_domain2.py`, `p2_ps4ctx_api.py` | deleted at checkpoint |
| Evidence `r1-p2-ps4-contexts.log` | retained |
| Temp roots / background processes | none |
| Real PS4 trees on `H:` | read-only |

## 8. Next action

Priority 2 continues. The next actionable research-backed slices are the arena venue parameter
inventory (handoff 09 → Phase 2.5's "venue parameters beyond identity — not inventoried") and the
gen7 player portrait / head containment (handoff 11 → the Player asset card on every gen7 title).
