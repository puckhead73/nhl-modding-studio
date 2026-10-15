# NHL21-E1 — first modified run (archive-only) and E5 restore receipt (2026-09-18)

**Plain-language summary.** The edited NHL 21 archive was placed into the live installation for the
first time, the game was booted twice with it, and the original was then put back and verified.
Before that, the unmodified game was restarted with the existing profile (no first-boot prompts
again; HOCKEYALLSVENSKAN on Rosters / Edit Player, re-entered — two screenshots), which closed the
check owed from E0. With the edited archive in place the user, driving the game, reported **REST OF
WORLD** in the tile where HOCKEYALLSVENSKAN had been, on both cold boots, and after the restore
reported HOCKEYALLSVENSKAN back — but declined to take screenshots of the modified and restore
boots, so those three observations are user reports attested here, not screenshot evidence.
Install and rollback went exactly as planned (hash-verified, no leftovers); the installation is
byte-identical to before. Nothing about the game's loading order or any other title is claimed.

---

## 1. Package, disposition, scope

* **Package:** NHL21-E1 per [league-nhl21-e1-prep-r2-review-and-e1-dispatch](../validation/league-nhl21-e1-prep-r2-review-and-e1-dispatch.md);
  card §2.5 E1 + E5 of [league-identity-runtime-qualification](../plans/league-identity-runtime-qualification.md).
* **Disposition:** run completed through E5; live archive restored and verified.
  **G1 verdict for Rosters / Edit Player: user-observed positive, not screenshot-backed** (see §4 for
  the exact evidence class). Play Now / Select Team under the edit: **not observed** (user did not
  report it). No E2/E3/E4; no other title; no emulator development; automated game input off
  throughout (launches only).

## 2. Preconditions and environment (executed, evidence `pre-run-inventory.txt`, `deploy-transcript-e1.txt`)

| Item | Value |
|---|---|
| Emulator | `H:\Emulators\ShadPS4\shadPS4.exe` SHA-256 `aeb048921e7749ef9dc580a6009bc0cbff08b193ae9f04cce209c9e7cf012790` (unchanged after the run); logs: `shadps4 v0.16.1 DMC`, rev `d96133b0…`, `dev-next`, `…-dirty` |
| Config | `be6decd2…` before and after (byte-identical; `config.before-e1.toml` = `post-config.toml`) |
| Title | `param.sfo` `bb8c97e3…` (CUSA18182 01.00); `eboot.bin` `fc95ca66…`; loose game tree = the 5 shipped config files only |
| Profile / cache before | `Profile0\DATA` `1bc473ff…` (from E0), `cache\CUSA18182.zip` `682f3016…` — identified, preserved, never reset |
| Status gate (00:30:26 and again 00:38:13) | `S0-stock`, no sidecars; all automated checks PASS incl. `emulator exe identity` and `config unchanged vs E0`; install plan MUTATE, rollback plan NOOP |
| Manual gates | run-log version line recorded per boot; profile/cache identified above |

## 3. Boots and file state (exact sequence)

| Boot | Live `inxp_00.big` | Launched | Session (log) | User-driven observation | Evidence |
|---|---|---|---|---|---|
| **A — stock restart (E0 gap)** | `1c41f578…` (original) | 00:30:36 | 7 270 lines, 481 stub errors, `/savedata0/DATA` opened (profile loaded), play time 0:06:19 | no legal / favourite-team prompts; Rosters / Edit Player **HOCKEYALLSVENSKAN**; left and re-entered | **F12 ×2**: `A1-…hockeyallsvenskan.png` (`95ccc1b4…`), `A2-…reentry.png` (`b525c952…`); `shad_log.BOOT-A-stock.txt` |
| install | — | 00:38:17–00:38:23 | `deploy.ps1 -Mode install -ExpectedExeSha …`: copy → verify `4876cd2b…` → rename live→`.e1-original` → rename incoming→live → verify. Independently re-hashed: live `4876cd2b3f4b51878fcbf08f8a56249df2eecbc6c086340b869fc9eada9da582`, `.e1-original` `1c41f578…` | — | `deploy-transcript-e1.txt`, `post-install-hashes.txt` |
| **B — modified, cold boot 1** | `4876cd2b…` (candidate X) | 00:38:34 | 6 501 lines, 500 stub errors (same classes as stock), play time 0:09:28, clean exit | **user report:** "ROW" / "Rest Of World" shown persistently on Edit Player in the edited slot | `shad_log.BOOT-B-modified.txt`; **no screenshot (user decision)** |
| **C — modified, cold boot 2** | `4876cd2b…` | 00:40:47 | 5 064 lines, 479 stub errors, play time 0:11:05, clean exit | **user report:** "done" (Rest Of World confirmed again, as asked) | `shad_log.BOOT-C-modified.txt`; **no screenshot (user decision)** |
| rollback | — | 00:42:22–00:42:24 | `deploy.ps1 -Mode rollback`: `S1-installed` → rename live→used → rename `.e1-original`→live → verify → used candidate removed (hash-verified). Independently re-hashed: live `1c41f578a595dbd5aae7ad921445626636f3a842d8fb869bc054d35e3e849e87`, no sidecars | — | `deploy-transcript-e1.txt`, `post-rollback-hashes.txt` |
| **D — E5 stock restore boot** | `1c41f578…` | 00:42:33 | 6 251 lines, 490 stub errors, play time 0:12:20, clean exit | **user report:** "confirmed back to original name" (HOCKEYALLSVENSKAN) | `shad_log.BOOT-D-e5-stock.txt`; **no screenshot (user decision)** |

Play-time values are the emulator's cumulative counter for the title. Each boot has its own log
copy. No boot exceeded the 10-minute startup bound; no retry was needed; no crash, hang or blank
was reported.

## 4. Verdicts (card §2.7 vocabulary)

| Surface | Verdict | Basis | Limits |
|---|---|---|---|
| Rosters / Edit Player | **Established — identity edit produced the expected label (user-observed)**: REST OF WORLD in the former HOCKEYALLSVENSKAN slot on two cold boots with the edited archive; baseline label back after the verified restore (E5 passed on hashes; restore boot user-confirmed) | user's direct observation while driving; boot logs; hash chain of the placed/restored files | **not screenshot-backed** for boots B/C/D — the user overrode the screenshot requirement. Unrelated-label checks (SHL/DEL/LIIGA/ICE HOCKEY LEAGUE/NHL) and an Allsvenskan team/player under the edit were **not reported**. Re-entry under the edit was not separately reported ("persistently" was the user's word for the label across boots) |
| Play Now / Select Team | **not observed** under the edit | — | user did not reach/report it |
| E5 (restoration gate) | **passed on bytes** (live = original, no sidecars, independently hashed); **restore boot user-confirmed**, no screenshot | `post-rollback-hashes.txt`, BOOT-D log + user report | — |

What this run establishes, at most: on the modified shadPS4 fork `aeb04892…`, CUSA18182 01.00,
placing the same-length raw member replacement in `inxp_00.big` (with the unresolved `+0x0C` and
header `0x10` words preserved verbatim) did not prevent boot, and — per the user — the edited
`stockleagues.name` key was rendered as its dictionary text on Rosters / Edit Player. It does **not**
establish loose-file precedence (E2), the `+0x0C` semantics, literal fallback (E4), behaviour on any
other surface, save persistence, or anything on another title or a console. The boot logs contain
no line naming the archive member that was read; the loaded source is inferred only from the file
swap plus the user's label report.

## 5. Evidence

[`docs/validation/evidence/league-nhl21-runtime/e1/`](../validation/evidence/league-nhl21-runtime/e1/)
(`SHA256SUMS.txt`, 7.1 MB): five emulator logs (pre-run NHL 21 log from E0 + boots A–D),
`deploy-transcript-e1.txt` (helper lines for both status runs, the install and the rollback),
`post-install-hashes.txt`, `post-rollback-hashes.txt`, `boots.log` (launch timestamps/pids),
`pre-run-inventory.txt`, `post-run-inventory.txt`, `config.before-e1.toml` / `post-config.toml`,
the two stock-restart F12 screenshots, and the exact helper used (`deploy.ps1`, `63a8a4d4…`).

## 6. State left on the machine

* Live install: `databigs\inxp_00.big` = `1c41f578…`, no `.e1-*` sidecars, loose tree unchanged (5 files).
* Emulator user data: `Profile0\DATA` now `5b036d30…` and `cache\CUSA18182.zip` now `b2bae24a…`
  (rewritten by the game on each session — genuine save/cache, preserved); two new F12 files in
  `screenshots\` (boot A); `log\shad_log.txt` = boot D. Config byte-identical.

## 7. Cleanup (after E5 closed, original verified, engine transaction `restored`)

* Removed with containment/reparse checks: `local/runs/league-nhl21-e1/artifacts/inxp_00.big.X-candidate`
  (777 MB), `local/runs/league-nhl21-e1/copied/` (restored root + engine journal/backups), and the
  `run/` scratch (all content preserved to evidence first). Free space on E: 38.4 GB.
* Retained under `local/runs/league-nhl21-e1/`: `nhl_game.db.X-loose-evidence` (11 MB, the member
  bytes), `verify_db_diff.py` + `db-diff-audit-r1.txt`, `deploy.ps1` + `deploy.log`, `MANIFEST.md`,
  `tests/`, `prep/` source, superseded prep report — small review inputs; disposable after Codex's
  review of this receipt.
* No emulator process; index empty; anchors untouched; no staging/commit/push.

## 8. Matrix

[`league-identity-parity-matrix.md`](../plans/league-identity-parity-matrix.md) §0.2 NHL 21 in-game
cell updated to exactly this scope (stock restart observed with F12; E1 user-observed without
screenshots; environment named).

## 9. Next action

Codex: decide whether a user-observed, non-screenshot label change on one surface is accepted as
the NHL 21 identity-edit qualification for this environment, or whether a repeat with F12 captures
(and the Select Team surface + unrelated-label checks) is required before Phase 1.1 is marked
qualified for NHL 21. Either way the next runtime experiments in card order are E4 (literal
fallback) and, for the placement question, E2. Stopping here.
