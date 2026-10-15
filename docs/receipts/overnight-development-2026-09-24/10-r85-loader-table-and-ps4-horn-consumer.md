# Receipt 10 — research R85 consumed: the gen7 loader asset table and the PS4 horn consumer (queue rows 5 / 7)

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing listened to; nothing
game-verified.** HEAD `8d60a6d` (unchanged). Claim: `dev-r85-loader-table`. Clock: 05:05 → 05:50
(MDT, 25 Sep 2026, the system clock as the logs record it).

## 1. Plain-language summary

The arena page's horn and chant columns on Xbox 360 / PS3 titles now also read the game's own
**loader asset table** (`audio/na_en_loadfile.xml`, research R85): the list the engine uses to
bind a horn or chant id to a bank file. A club id now says "listed in the loader asset table"; a
residual id (the DEL / SHL / Liiga arenas) says it has no loader asset at all, not merely no file
in the folder; a chant id says whether it is a team chant or a **goalie** chant bank (R85 found the
four-digit chant banks are keyed by the goalie's announcer audio id, so they are not consumers of
the arena's `chantaudioid`). The table is read loose when shipped loose and otherwise from the
`audioboot.big` member (RefPack on NHL 09, chunkzip on PS3 13, chunklzx on X360 15 — all three
unwrapped and measured on real trees). On PS4 the `goalhorn` note names the located consumer
(`goalhornpatch.ptc`, an undecoded Harmony patch) and `periodhorn` values 0–2 are presented as the
`PeriodHornID` enum reading R85 made on NHL 21 (`Generic` / `Siren` / `Buzzer`), hedged to that
title and never as a bank id. Nothing is written; no new operation is claimed.

One measurement departs from R85 §1's "100 %" chant statement and is pinned in the test: on the
Xbox 360 NHL 15 tree the loader lists goalie chant `0587_chant.abk`, which is **not shipped**, while
the shipped `0584_chant.abk` is **not listed** (the r85 data compared only the goal family against
the members). Both cases carry explicit "report it" wording in the resolved note.

## 2. Files changed

| Path | Change |
|---|---|
| `crates/studio-domain/src/gen7_horn_sources.rs` | `LoaderTable` (`parse` of the flat `<Asset assettype= path= descr=/>` grammar; `read`: loose first, else `audioboot.big` / `data0.big` / `audio.big` / `nocache.big` member unwrapped from chunkzip / chunklzx / RefPack); `Gen7HornSources.loader`; `inspected()` names the table or its absence; `resolve()` appends the listed / unlisted / goalie-chant / listed-but-unshipped / shipped-but-unlisted wording; module doc |
| `crates/studio-domain/src/gen7_speech_preview.rs` | `locate_member` is `pub(crate)` (reused; no behaviour change) |
| `crates/studio-domain/src/arena_venue.rs` | PS4 `goalhorn` note names the `sfx_ptc.big` → `goalhornpatch.ptc` consumer (located, undecoded); PS4 `periodhorn` 0–2 as the NHL 21 `PeriodHornID` enum candidate, other values "outside the enum"; two stray `;;` from a P4.4 patch removed in the test; test asserts both |
| `crates/studio-domain/tests/gen7_horn_sources.rs` | real X360 NHL 15: loader goal / period sets equal the loose sets, 73 team + 45 goalie chants, the 584 / 587 discrepancy pinned, club ids "listed", residual ids "no loader asset"; NHL 09 X360 loose table (0–29, 100); PS3 NHL 13 (chunkzip) and PS3 NHL 09 (RefPack) archived-only copies read via `LoaderTable::read` |

No DTO, binding or frontend change: the text flows through the existing `resolved_note`.

## 3. Evidence ([`p47-r85-loader-table-real.log`](../../validation/evidence/overnight-development-2026-09-24/logs/p47-r85-loader-table-real.log))

| Tree | Loader copy read | goal / period | team chant / goalie chant |
|---|---|---|---|
| X360 NHL 15 (loose tree) | loose `audio/na_en_loadfile.xml` | 32 / 32 (0–29, 100, 194 = the loose sets) | 73 / 45; listed − shipped = {587}, shipped − listed = {584} |
| X360 NHL 09 | loose | 31 / 31 (0–29, 100 = the `data0.big` member set) | — |
| PS3 NHL 13 (no loose copy) | `audioboot.big` member, chunkzip | 31 / 31 | 73 / 45 |
| PS3 NHL 09 (no loose copy) | `audioboot.big` member, RefPack | 31 / 31 | 73 / 32 |

The PS3 NHL 09 goalie-chant count (32) is a new measurement; R85 §1 reports 45 as the NHL 10–Legacy
figure and 44 / 45 matches on NHL 10, and did not tabulate NHL 09's four-digit set — recorded for
research, not asserted in the test.

## 4. Checks

| Run | Result |
|---|---|
| `cargo test -p studio-domain --lib -- gen7_horn_sources gen7_speech_preview arena_venue` | 5 passed |
| `cargo test -p studio-domain --test gen7_horn_sources -- --ignored` (real trees) | **1 passed** (four trees, 9.7 s) |
| `cargo test -p studio-domain --lib -- --skip zz_diag` | **222 passed, 0 failed, 2 ignored** (unchanged count) |
| `export-bindings --check` | in sync (no DTO change) |
| `cargo clippy -p studio-domain --tests` (scan of the touched files) | one pre-existing `too_many_arguments` on `arena_venue::build` (P4.4); nothing new |

Repo-wide fmt / clippy remain the recorded C0.2 baseline defects; not re-measured.

## 5. Observations for research (recorded in the claim)

- X360 NHL 15: loader lists goalie chant 0587, folder ships 0584 (§1 above). R85 §1's chant "100 %"
  was not measured against the members in the r85 data.
- PS3 NHL 09 loader table lists 32 goalie chant banks (R85 tabulates 45 from NHL 10).
- X360 NHL 15's archived copy of the table is `chunklzx`-wrapped (R85 copies); the developer's
  reader unwraps it, so trees without a loose copy on X360 14–Legacy would read too — not measured
  (all of those ship the loose copy).

## 6. Limitations

- The loader table is read for what it lists; what the engine does for an unlisted id remains the
  runtime question (card H-HORN-1, unchanged).
- The PS4 `periodhorn` enum is an NHL 21 reading applied only as a hedged candidate to every PS4
  title's note; the goal-horn patch is located, not decoded — no element / flair semantics.
- PS3 12 / 13 horn probing stays refused (archive-only audio not inspected by R78 / R81); only the
  table read was exercised there.

## 7. Cleanup

Scratchpad patch scripts (`patch_r85.py`, `patch_r85b.py`) deleted by literal path after this receipt;
the unit test's `nhlms-horns-<pid>` temp root is removed by the test; no processes left.

## 8. Next action

Per the closing receipt §6: **Phase 3 players, step 3.1** in the parity roadmap's order — unless the
morning review returns a League / Team defect.
