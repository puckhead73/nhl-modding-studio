# Receipt 63 — PS4 players can be given another shipped head

**Developer-verified; not Codex-accepted; nothing launched; only disposable copies were written; the installs were only read.**
HEAD `8d60a6d` (unchanged). Claim `dev-ps4-head-reassign`. Research consumed: R15 (2026-09-20 handoff 08), R124 row 78 (2026-09-26
handoff 17). Dispatch after receipt 60 §3.

## 1. Plain-language summary

- **On PS4 NHL 15–21 a player's head can now be pointed at another head the game ships.** Use Players → a player → assets →
  Reassign head → Check → Apply. Before, PS4 heads were refused.
- **Studio only offers heads the open game actually has.** It checks the target in that game's own head catalogue (its vault).
  - Another game's catalogue is never used: an NHL 19 project with the NHL 20 install attached is refused.
  - A number the game has no head for is refused, e.g. NHL 15's known missing 10391.
- **On NHL 17–21 the game keeps a second copy of the head number** (`headtexture`). Studio changes both together as one edit, with
  one undo. If that second field is missing, nothing is written.
- **The check tells you what kind of head you are choosing:**
  - a shared generic head;
  - another player's own face (so two players would share it);
  - or a head nobody owns.

  It also warns when the player's row was already unusual.
- **Apply now does exactly what you checked.** If the row changed after you checked, the apply is refused and nothing is written.
  This applies to portraits too.
- What the game draws with the new head is not observed (card P34-1).

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/ps4_heads.rs` (**new**) | `HeadCatalogue`: `ge_nhl_skaterhead` records by their single `SkaterNameAttribute` value (found by type; exactly one such attribute required), identity-checked against the open title (`read_install_vault` + `vault-identities.json`); per-value record counts (≠ 1 = absent / ambiguous); records without a value counted |
| `crates/studio-domain/src/player_reassign.rs` | `Ps4Heads` source parameter. The PS4 head branch: NHL 17–21 require `headtexture` (range-checked) and add it to `writes`; exceptional rows warned; other titles `headid` only (a `headtexture` field there is left alone). Vault check: 0 / 1 / many records; current head without a record noted. Head class and duplicate faces now shared by gen7 and PS4. Plans carry `writes` and a `fingerprint` |
| `crates/studio-api/src/{registry,commands,dto}.rs` | cached head catalogue (read outside the workspace lock); `player_reassign(…, reviewed, defs_root)`; an apply needs the reviewed fingerprint and is refused when it differs; paired writes are one `set_cells_int` operation; DTO `written_fields`, `fingerprint`; the command is async (vault read off the main thread) |
| `app/src/components/PlayerContainmentCard.tsx` (+ test), `app/src/lib/client.ts` | Apply sends the checked plan's fingerprint |
| `app/src/bindings/PlayerReassignPlanDto.ts` | regenerated |
| `crates/studio-domain/tests/player_reassign.rs` | new parameter; the PS4 head refusal without an install now asks for the install |
| `crates/studio-api/tests/ps4_head_reassign.rs` (**new**) | below |

## 3. Evidence

**`ps4_head_reassign` 1 / 1** (`--include-ignored`, 223 s). Genuine NHL 20 (CUSA15439 fixture DB and install) and NHL 21 (anchor
DB, CUSA18182 install), each on a disposable copy:
- another player's bespoke head (Paul Stastny's 1297) is allowed. Both fields are planned, the evidence names the title's vault,
  and the notes disclose the duplicate face and its owner;
- the pool head 60000 is allowed as "a shared pool head", "becomes not bespoke";
- the absent 60211 is refused ("has no head record"), and an apply of it is refused;
- review: an apply without a fingerprint is refused. After `headtexture` was changed, the stale fingerprint is refused with "changed
  since the plan was reviewed … Nothing was written", and the cells are unchanged. An exceptional row is disclosed as "an exception
  to the stock pairing";
- the reviewed apply writes `headid` = `headtexture` = 60000. One undo restores both, redo re-applies, and after save-to-copy and
  reopen both are 60000;
- among the first 150 skaters of each title, 0 rows have `headtexture` ≠ `headid`.

Also on disposable copies:
- **Missing paired field** (a genuine NHL 20 copy whose metadata copy does not name `headtexture`): refused ("no `headtexture`
  field"); the apply is refused, `headid` is unchanged and nothing is pending.
- **Foreign vault:** an NHL 19 project with the NHL 20 install is refused as "source mismatch".
- **NHL 15** (anchor and CUSA00561): no `headtexture`, one written field; 10391 is refused (the catalogue has 0 records for it,
  R15); the rows that carry 10391 get the "no record in this title's vault" note.

**Other suites:**
- domain `player_reassign` 4 / 4 (`--include-ignored`, including receipt 55's archive cases);
- frontend vitest 276 / 276 (PlayerContainmentCard asserts the fingerprint is sent); `tsc` clean;
- bindings `--write` changed one file.

## 4. Limits

- Hair (`hairid`, `GenericHairStyle`) is not reassigned. R15 names it, but this package is heads only.
- New heads, mesh and texture replacement, and head conversion are out of scope.
- Speed: an earlier version of the test that searched targets with repeated plan calls took 633 s, so each Check re-parses the
  whole PS4 database. A single Check was not timed individually, and this was not tuned.
- Game rendering is unobserved: card P34-1 / R15-HEAD-1.
