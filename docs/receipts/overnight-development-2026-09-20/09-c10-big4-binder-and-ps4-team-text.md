# Receipt 09 — C10: BIG4 binder defect (PS3 NHL 10) fixed; Phase 2.3 PS4 team-text parity; 2.2 parity measured

Developer lane, overnight 2026-09-20. **Developer-verified; not Codex-accepted.** Baseline `8d60a6d`.

## Plain-language summary

Two small closures. First, the real-archive sweep of the old-console (`BIG4`) archives found a
genuine defect: PS3 NHL 10's `fe.big` declares an index size whose trailer runs ten bytes past
the first file inside, so Studio refused to open it. The header word was measured on all seven
stock archives; the reader now takes the start of the file data from the directory it actually
parsed (which equals the first member's position on every archive) instead of trusting the
declared word, and never rewrites bytes past that point. A unit test with that exact shape and a
rerun of the PS3 NHL 10 sweep back it. Second, the team-text (localization) card was proven on the
PS4 titles: every PS4 English dictionary carries the same per-team key families keyed by the
team's art abbreviation as the old consoles, and the free-text edit route holds on them — measured
on 14 extracted dictionaries (base and patch archives) plus the committed NHL 15 anchor. Nothing
was written to a game.

## 1. Package, disposition, scope

C10 — delivered, developer-verified. (a) `tdb-archive-big::big4` binder correction; (b) Phase 2.3
PS4 parity evidence for the existing `team_text` surface; (c) Phase 2.2 parity **measured** (no
code): the surface capability probe over the 27 anchors reports lines, strategies, captaincy and
contracts **Editable** on every title; draft picks and team jerseys **NotApplicable** on NHL 07–10
(no `qHav` / `xjgw` table — whether those eras keep the data under other ids is a research
question, not a developer guess); team equipment NotApplicable on NHL 07 only. The 27-anchor read
matrix (`read_matrix.rs`) already asserts those surfaces build where available. No 2.2 package was
opened.

## 2. Changed paths (SHA-256 prefixes)

| Path | SHA-256 | Change |
|---|---|---|
| `crates/tdb-archive-big/src/big4.rs` | `fa47f923d12b63b0` | `BoundBig4.data_start` = `align64(end of the parsed directory)`; `declared_index_end` recorded; `index_bytes` (the rewritable prefix) capped at `data_start`; +1 unit test (`declared_index_size_overrunning_the_first_payload_line_binds_and_stages`) |
| `crates/studio-api/tests/membership.rs` | `49a3688ea9884676` | +1 default case (PS4 NHL 15 anchor + its committed `eng_us.db`) and +1 `--ignored` (every extracted PS4 English dictionary, base + patch archives) for `team_text` and the retext preview |

## 3. Measured facts (developer evidence)

- **BIG4 header word vs directory** (read-only on the seven stock archives): `16 + index_size`
  includes a 24-byte `L2xx` trailer (`L277\x15\x05\0\0Buy ERTS…` on 07/08/09 both consoles and
  Xbox 10 `L286…`); the first payload sits at `align64(16 + directory bytes)` on **all seven**
  (Xbox 07 `174 848`, Xbox 08 `702 848`, PS3 08 `701 568`, Xbox 09 `509 056`, PS3 09 `513 728`,
  Xbox 10 `624 960`, PS3 10 `624 832`). On **PS3 NHL 10** the declared end `16 + index_size =
  624 842` exceeds the first payload `624 832` by 10 bytes (its trailer is `L286\x15\x05` + six
  zeros, 14 bytes, but the word still counts 24) — the old rule `align64(16 + index_size)` gave
  `624 896` and refused every entry as out of range.
- **PS4 team text keys**: `NHLTeamName_{artabbr}`, `NHLCityName_`, `NHLTeamName_Abbr3_`,
  `TEAMLINE1_`/`TEAMLINE2_`, `TXT_NICKNAME_`/`TXT_NICKNAME_ALT_`, `X_XLA_TEAM_`, `NICKLINE2_` exist on
  every PS4 English dictionary (NHL 21: 1 116 / 551 / 405+404 / 917 / 295 / 133 rows per family;
  NHL 15: 636 / 322 / 265+265 / 452 / 240 / 114); Toronto resolves on all of them (`artabbr` `TOR`).

## 4. Checks executed (developer)

| Command | Result |
|---|---|
| `cargo test -p tdb-archive-big --lib big4` | 6 passed (+1) |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-api --test big4_asset_member_route -- --ignored --skip every_real` | 5 passed (synthetic pairs + NHL 07 refusals, rebound binder) |
| `cargo test -p studio-api --test membership` | 6 passed (+1), 1 ignored |
| `NHL_STUDIO_FIXTURES=… cargo test -p studio-api --test membership ps4_team_text_resolves_on_every -- --ignored` | **1 passed** (14 dictionaries) |
| PS3 NHL 10 real `fe.big` + `.bh` rerun (rebuilt exe) | **1 passed** (§5) |

## 5. Background sweep results recorded here (BIG4, C4)

Real archives copied to `%TEMP%` with their `.bh`; both files patched as one change set, restore
byte-exact:

| Target | Archive | `leaguelogos/l0` | `leaguelogossmall/l0` | Result |
|---|---|---|---|---|
| Xbox NHL 10 | `fe.big` 247 494 615 B, 11 004 entries | fit (7 853 + 19 zero slack) | appended + re-pointed (6 025 > 4 250 + 38) | passed (210 s) |
| PS3 NHL 10 | `fe.big` 249 900 951 B, 11 004 | fit (8 048 + 16) | appended (6 576 > 4 656 + 16) | **failed on the old binder** (`OutOfRange`, §3); **passed on the rerun with the fix** (218 s; `logs/c4b_real_big4.log`) |
| Xbox NHL 09 | `fe.big` 207 665 640 B, 9 053 | fit (8 216 + 40) | appended (5 999 > 4 313 + 39) | passed (158 s) |
| PS3 NHL 09 | `fe.big` 210 504 924 B, 9 128 | fit (8 352 + 32) | appended (6 432 > 4 576 + 32) | passed (161 s) |
| Xbox NHL 07 | `data0.big` 284 526 259 B, 3 490 | refused as planned (sidecar cascade) | refused as planned | passed (28 s; listing + refusals only) |
| Xbox NHL 08 | `data0.big` 2 000 405 081 B, 14 162 | fit (8 228 + 28) | appended (6 008 > 4 322 + 30) | passed (1 539 s: 2 GB copy, commit 1 113 s) |
| PS3 NHL 08 | `data0.big` 2 000 883 189 B, 13 921 | fit (raw member, 17 696 = slot + 32) | fit (9 440 = slot + 32) | passed (1 631 s) |

**7 / 7 targets pass** with the corrected binder (NHL 07 = the planned refusals only).

Every "appended" line is the R2-BIG4-1 shape (offset order broken by one entry) the morning card
asks the user to observe.

## 6. Temporary artifacts / processes

- Scratchpad: `c10_*.py`, the copied test exe `c4b_big4_route.exe` + `c4b_run.sh` (removed after
  the rerun is recorded), `closeout_tests.sh` / `.log`.

## 7. Runnable commands

```sh
cargo test -p tdb-archive-big --lib big4
cargo test -p studio-api --test membership ps4_team_text
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p studio-api --test membership ps4_team_text_resolves_on_every -- --ignored
NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures NHL_STUDIO_BIG4_TARGETS=ps3-nhl10 cargo test -p studio-api --test big4_asset_member_route every_real -- --ignored --nocapture
```
