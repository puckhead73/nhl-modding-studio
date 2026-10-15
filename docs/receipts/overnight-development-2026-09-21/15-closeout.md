# Receipt 15 — Closeout of the 21 September 2026 overnight developer campaign

**Everything in this campaign is developer-verified only. Nothing is Codex-accepted, nothing was
desktop-exercised, nothing is game-verified; no commit, staging, push, reset, branch change or
clean was made; no game or emulator was launched; no live install, image, archive, save or
profile was written.** Branch `feat/phase5-asset-tooling`, baseline `8d60a6d`; the dirty working
tree the campaign started with (the 18–20 September work) is preserved — 187 modified and 531
untracked paths at closeout, of which this campaign's are the ones named in receipts 01–14 and
listed in the `dev-*` claims. NHL21-E1 stays accepted and closed. Projects / Builds untouched.

## 1. Plain-language summary of the night

Fourteen packages, in the order the instruction set: the colour race was closed (P0); seven
foundations were audited and seven defects fixed (P1); the League archive work landed — ordered
BIG4 reflow proven against the research oracle, the NHL 07 `boot.big` cascade, the gen7 LZX
decoder promoted (50 / 50 byte-exact) and, later, a codec-preserving write route for those
members (loader acceptance still open); the commentary catalogue and the context listing gave
the Leagues page its read-only audio surfaces; texture resizing was lifted on all three loose
container families by the research's measured descriptor rules and proven by reproducing every
shipped descriptor on the real trees (18 984 / 18 984 Xbox 360 Legacy; 26 054 + 20 245 PS3 with
exactly the research's cube maps; 145 127 PS4 NHL 21 accepted with exactly the 89 cubes refused);
Teams gained the re-identification contract (index refused, art-id swap as one undoable step)
and the name-call listing; Players the recorded-name listing and the equipment file check;
Leagues the NHL 07 membership and the PS4 pseudo-league labels; Localization the static glyph
gate. Two read-side defects surfaced by the censuses were fixed (Xbox 360 texlib heights;
PS3 DXT1 / DXT3 rasters), one build-side incident (a full disk from my own 132 GB of stale test
binaries) was diagnosed and cleaned, and every failed test run of the night is described with
its diagnosis in the receipt it belongs to — none was lost to a rerun.

## 2. What users can now do (by title group)

| Titles | New tonight |
|---|---|
| All (any open project) | Colours: no double write after Undo-then-retype (01). Dictionary text edits: per-character "in / not in the shipped fonts" verdict, C1 controls refused (09). |
| Xbox 360 / PS3 NHL 07–10 | BIG4 members grow by ordered reflow instead of append / refuse; NHL 07 `data0.big` growth cascades into `boot.big` (03). NHL 07: league membership editable on the stock row with the 2006-07 labels; All-Star rows guarded (13). |
| Xbox 360 NHL 11–Legacy | `chunklzx` members (rink / ice / logo libraries, attribute DBs) readable everywhere (04) and writable through the store route — flagged, runtime-unqualified (08). Loose texlib rasters ≥ 128 texels resize (11). NHL 11 equipment grammar (12). |
| PS3 NHL 08–Legacy | Loose DXT rasters resize; DXT1 / DXT3 rasters open (11). |
| PS3 / Xbox 360 NHL 11 / 14 / 15 / Legacy (+ X360 12 / 13) | Leagues → Commentary: the catalogue (05) and the Contexts listing on NHL 11–Legacy (14). Team page: Identity (index / art id, swap) (06), Name calls (07). Player page: Recorded name (10), Equipment → art files on this install (12). |
| PS4 NHL 15–21 | Team Identity card + art-id swap (06). GNF 2D block textures resize under the reproducibility gate (11). Pseudo-league groups 77 / 78 named (09). |

## 3. Receipts, claims, evidence

Receipts 01–14 under this directory; `dev-*` claims (14) under
`local/coordination/overnight-2026-09-21/claims/`, all `complete`; logs (40) under
`docs/validation/evidence/overnight-development-2026-09-21/logs/`; coverage addenda appended to
the [Phase 1](../../validation/phase-1-coverage-matrix-2026-09-18.md) and
[Phase 2](../../validation/phase-2-coverage-matrix-2026-09-20.md) matrices; cards in
[MORNING-REVIEW.md](MORNING-REVIEW.md) (desktop V-12 … V-22; runtime R2-BIG4-1 rev. 3, R3-BOOT-1,
R30-LZX-1, T-4, C-LZX-2, LOC-G1, R35-AUD-1, R32-XB2-1, R33-PS3-1, R31-GNF-1, R39-07-1).

Research consumed (all `ready` claims of the 2026-09-21 research lane, none edited): handoffs
01, 06, 07, 08, 09, 10, 11, 12, 13, 15, 16 and the 2026-09-18 handoff 40 vectors; **not**
consumed: handoff 02 (source identity — the archive route for PS3 12 / 13 speech), 04 beyond the
parsers of receipt 05, 05 (bank codecs), 14 (a negative result, honoured by the PS4 refusal), the
2026-09-20 carry-overs named in the research queue.

## 4. Test totals (developer runs; ignored = fixture / mount-gated, never counted as passing)

| Run | Result | Log |
|---|---|---|
| Closeout aggregate `cargo test --no-fail-fast -p tdb-archive-codec -p tdb-archive-big -p tdb-rx2 -p tdb-speech -p tdb-loc -p studio-domain -p studio-api` | **788 passed, 0 failed, 106 ignored, exit 0** (one run, after every receipt incl. the v3.1 reader's crate code; the last edits after its start were receipts 14's domain routing and test-only files, each covered by their own runs) | `closeout-aggregate-default-run1.log` |
| Earlier aggregate (after receipts 07 + 08, four crates) | 632 passed, 0 failed, 60 ignored | `p3-aggregate-default-run3.log` (runs 1 / 2: bindings guard, full disk — receipt 10 §4) |
| Frontend `npx vitest run` | **26 files, 175 passed**; `npx tsc` clean; bindings "in sync" | `closeout-frontend-full.log` |
| `--ignored` real-data runs | every one described in its receipt with its log: BIG4 oracle 16 / 16; chunklzx vectors 50 / 50 (+ store 50 / 50); speech model 10 trees; team name calls 4 anchors; player name calls NHL 15; RW4xb2 census Legacy + NHL 11 (+ NHL 09 in the final run), RW4ps3 census Legacy + NHL 11, GNF census NHL 21 (+ NHL 16 in the final run); equipment files Legacy + NHL 11; CTX0 7 sets; the fixture-backed logo package sweeps | receipts 03–14 |

### 4a. Closing runs (landed before this receipt was finished)

- GNF header census (`p2b-gnf-header-census.log`, 66 min): **NHL 21 145 127 accepted, 89 cube
  maps refused (type field), 2 850 unmodelled formats (2 841 BC6H + 9 uncompressed), 0 rule
  failures, 1 547 non-GNF; NHL 16 68 060 accepted, 23 cubes, 0 failures, 766 non-GNF** — the
  research's 89 / 23 cube counts and 68 060 exactly.
- RW4xb2 census, three trees (`p2b-rw4xb2-descriptor-census.log`, 632 s): **Legacy 18 984 /
  18 984, NHL 11 14 659 / 14 659, NHL 09 12 816 / 12 816 reproduced, 0 mismatches**; 70 Legacy
  cube maps and 243 NHL 09 older-layout (`DW0 = 3`) rasters refused by the gates as designed.
- Every background process ended on its own; none running at the end (checked).

## 5. Where Codex should look first (review areas, most consequential first)

1. **Contracts consumed from research proposals:** the team re-identification contract (06) and
   the store-route qualification (08) shape user-facing behaviour; both are research proposals.
2. **Read-side corrections with broad reach:** the 13-bit texlib height (11 — changes decoded
   heights of rasters not a multiple of 8 on every Xbox 360 title), PS3 DXT1 / DXT3 (11), the
   goalie-stick companion table (12), the `align_to_word` mspack rule in the LZX decoder (08).
3. **Write paths opened:** resizes on three families (11), the store route (08), BIG4 reflow +
   the NHL 07 cascade (03), NHL 07 membership on the stock row (13) — each with its runtime card.
4. **Data-over-prose decisions** recorded in tests: gen7 Polish lowercase glyphs (09), the CTX0
   "id == index" prose vs census (14), the RW4xb2 pitch of 160-wide rasters (11), the audio-id-0
   sentinel (07 / 10), Studio's equipment family definitions vs the research's counts (12).
5. **The bindings root-count guard** (123 → 126 across the night) and the `defs/` additions
   (`fonts/glyph-coverage.toml`, `gen6-nhl07.toml` groups).

## 6. Not done / left out (and why)

- P2-C localization beyond the glyph gate (no new research input consumed); P2-D league identity
  / schedules beyond NHL 07 membership (the NHL 07 era quota row is not added); P6 miscellaneous
  presentation (not reached).
- Crowd contexts in the UI, PS3 12 / 13 archive-route speech, the NHL
  09 / 10 XML speech — coverage refusals say so.
- Reassignment validation for audio ids (handoff 07 §4) and equipment ids (12 §3) — shown, not
  enforced.
- Xbox 360 8_8_8_8 texlib resize; packages inside BIGF stay same-size; PS3 mixed mesh + texture
  containers unreadable (counted).
- Real-archive apply of a store-encoded member (the ~450 MB copies) — a morning run.

## 7. Cleanup

- Scratchpad: every package's scratch deleted at its checkpoint; `xpr2_dw_census.py` (receipt 02's
  census source) deleted at closeout — its tables live in receipt 02. Scratchpad empty.
- Temp roots: the tests remove theirs; two `nhlms-logos-dds-mips-*` roots left by receipt 02's
  first (failing) DDS run were found at closeout and deleted. The six `nhlms-*` roots dated before
  the campaign and the `H:\TEMP\nhl-*` directories of earlier sessions were not touched.
- Build output: 65 GB of stale test binaries older than three hours were deleted from
  `target/debug/deps` when the disk filled (receipt 10 §4); nothing outside `target/`.
- Processes: the census / aggregate runs end on their own; verified at the very end (below).
- The stray root file `--print` (18 Sep, a CSV) predates the campaign and was left alone.

## 8. Exact continuation task

Next package, in the instruction's order: **P2-D — a divisional-quota schedule generator (the
NHL 08 era 8 / 4 / 1 row and NHL 07's 82 / 58 / 29 formats, research handoff 13 §2–3; Studio's
generator is round-robin only)**, then the crowd contexts and the PS3 12 / 13 archive-route
speech sources (handoff 02), then P6 (UI / boards / scorebug / menu music) once research inputs
exist. Before any of it: Codex's review of receipts 01–14, and the morning cards.
