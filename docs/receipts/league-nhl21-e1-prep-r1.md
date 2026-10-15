# NHL21-E1-PREP-R1 — deployment helper correction receipt (2026-09-18)

**Plain-language summary.** The script that will swap the NHL 21 game archive in and out was
rewritten so it can always be run again safely: every time it starts it looks at what is actually on
disk — the live file, the saved original, a half-copied incoming file, a used candidate — identifies
each by exact size and hash, and then either finishes the interrupted step, undoes it, or refuses
with a plain explanation. Restoring the original no longer depends on the extra checks that only an
install needs (free space, mod folders, config), unknown files are never deleted, and every listed
precondition is now either genuinely checked by the script or clearly marked as a manual gate. All
of this was exercised on throwaway 4 KB fixtures — 24 scenarios including a kill between the two
renames and a restart — and the checksum audit of the edited database now passes with every changed
byte explained. The live installation is unchanged and nothing was launched.

---

## 1. Package, disposition, scope

* **Package:** NHL21-E1-PREP-R1 per [league-nhl21-e1-prep-review-and-r1-dispatch](../validation/league-nhl21-e1-prep-review-and-r1-dispatch.md).
  Bounded to the deployment/recovery helper, its verification/reporting, the manifest and receipt.
* **Disposition:** F1–F4 corrected and tested; the DB audit re-run to a successful exit; candidate X
  and all rollback sources preserved; live `inxp_00.big` re-hashed `1c41f578…` at closeout; no
  emulator/game launch, no application change, nothing staged/committed/pushed, no anchor added.

## 2. Disposition of each finding

### F1 — interrupted-rename recovery (fixed, tested)

`deploy.ps1` (`7054494b…`) is a state machine: each invocation classifies Live, `.e1-original`,
`.e1-incoming`, every `.e1-candidate-used-*`, the candidate and the secondary source by exact length
and SHA-256 (`original` / `candidate` / `unknown` / `absent`; reparse points and non-files are refused),
derives one of the documented states S0–S7 (or "live unknown"), and acts from that state. There is
no in-process try/catch recovery; a process killed after either rename is recovered by running the
wanted mode again. The two renames are explicitly documented as **not** one atomic operation; the
guarantee is that no state loses the original bytes while a verified source exists. Same-directory
staging (`.e1-incoming`) is verified by hash before any rename; the emulator is re-checked
immediately before every copy and every rename; absolute paths are normalised and the live directory
and every sidecar are checked for reparse points before any mutation. The secondary-source fallback
(copy → verify → rename) is implemented and tested, including its own interruption points.

### F2 — install-only preconditions blocked restoration (fixed, tested)

Install qualification (space, loose game-tree dir, overlay/patch/cheat/config folders, config drift,
optional exe identity) runs only for `install` (and is reported by `status`). `rollback` needs
only a stopped emulator, resolvable plain files and a verified original source; it restores with
zero free space (test 09), with a non-empty `mods_folder` and a loose game-tree directory present
(test 10), and with an unreadable overlay directory (test 12). The secondary-copy branch is the only
rollback path that needs space, and it checks space only there (test 08b refuses without mutating).

### F3 — unknown sidecars were deleted (fixed, tested)

Nothing is deleted on suffix alone. `.e1-incoming` with unknown bytes makes `install` refuse and
name the file (test 11b); a verified-candidate `.e1-incoming` is **reused** without recopying
(test 03) or removed on rollback (test 11e); unknown `.e1-original` is preserved and rollback goes
through the secondary source (11c); unknown `.e1-candidate-used-*` files are preserved and reported
(11b); a duplicate verified original makes install refuse and rollback a no-op with the sidecar kept
(11d); unknown live bytes make both modes refuse without touching anything (11a). `status`
classifies every sidecar and states exactly what each mode would do.

### F4 — manifest promised unperformed checks (fixed)

`MANIFEST.md` (`d74c83b0…`) now separates **automated install checks** (each printed PASS/FAIL by
`status`/`install`: emulator stopped, live/candidate identity, loose dir absent, four user-data
folders exist and enumerate empty with enumeration failure **failing closed**, config equal to the
E0 pre-run config except `windowPosX/Y`, emulator exe SHA-256 when `-ExpectedExeSha` is passed) from
**manual gates** (run-log version line; profile/cache identified) and from **rollback
prerequisites** (no environment gates). The read-only live `status` run below shows the real
result of every automated check.

### Also corrected: the CRC classifier (dispatch item 4)

The harness report that printed `UNEXPLAINED` is retained unchanged as
`prep-report.superseded-by-r1.txt` (`e5db9708…`). A focused verifier `verify_db_diff.py`
(`2c446bdb…`) implements `tdb-core`'s rule exactly (file-header hash, per-table `prior_crc` over the
previous table's body `[block+0x28, next_block)`, per-table `header_crc`, EOF CRC) and re-audited the
two retained DBs (`db-diff-audit-r1.txt`, `da827326…`): all **232 CRC words verify in both files**;
differing ranges = 2: `[0x63ef82,0x63ef90)` → `intended record fsLK#8`; `[0x63f2b8,0x63f2bc)` →
`CRC word mkrU.prior_crc @0x63f2b8 scope [0x63edec,0x63f2b8) recomputed-match before/after: True`;
**RESULT PASS, exit 0**. Candidate X was not regenerated.

## 3. Tests executed (developer, this host; disposable fixtures under `tests\fx`, removed after)

`tests\run-tests.ps1` (`d99f8f4a…`) → `tests\results.txt` (`749620c3…`): **24 passed, 0 failed**.
The helper is invoked as a child process per step with `-Profile fixture -Root <case>` (the helper
refuses fixture roots on `H:\`, containing `ShadPS4`, or under `anchors\`; the live profile refuses
every fixture override and test hook — test 15). Payloads: two 4 KB files differing in 18 bytes,
plus a junk file; interruptions injected with `-FailAfter copy|rename1|rename2` (exit 3); emulator /
free-space hooks via fixture-only environment variables.

| # | Scenario | Result |
|---|---|---|
| 01 | clean install → rollback | S0→S1→S0 |
| 02 | idempotence: rollback on stock, install ×2, rollback ×2 | no-ops, states unchanged |
| 03 | install killed after copy; re-entry | incoming reused, install completes |
| 04a/04b | install killed after rename 1 (live absent); re-entry `install` / `rollback` | completes / restores |
| 05 | live absent + incoming lost | install refuses ("run rollback first"), rollback restores |
| 06 | rollback killed after rename 1 (live absent, used present); re-entry | install refuses; rollback resumes |
| 07 | live missing, verified original only | rollback restores |
| 08a–08e | primary source missing: secondary fallback; insufficient space (refused, untouched); secondary missing (blocked, exact text); fallback killed after copy / after rename 1 and resumed | all as specified |
| 09 | zero free space | install refused; rename rollback succeeds |
| 10 | non-empty `mods_folder` + loose dir | rollback succeeds; install refused with both checks FAIL |
| 11a–11e | unknown live / incoming / original / used; duplicate original; stale known incoming | preserved/refused/reused exactly as §2 |
| 12 | unreadable `mods_folder` (deny ACE) | install fails closed (`mods_folder readable FAIL`); rollback unaffected |
| 13 | emulator running | both modes refuse before any mutation |
| 14 | candidate artifact missing | install refused |
| 15 | guards | fixture root on H:/ShadPS4 refused; live profile refuses hooks |

**Read-only live check (executed, no mutation):** `deploy.ps1 -Mode status -ExpectedExeSha aeb04892…`
→ state `S0-stock`; Live = original `1c41f578…`; Original/Incoming/Used absent; Candidate =
`4876cd2b…`; Secondary = original; every automated install check **PASS** (incl. exe identity and
config unchanged vs E0); rollback prerequisites: emulator stopped, primary source absent (expected
on stock), secondary present. Exit 0. (`deploy.log`, `c53ef38d…`.)

**Not executed:** `install` and `rollback` against the live install. No emulator or game launch.

## 4. Changed / retained paths (SHA-256 prefixes; full list `local/runs/league-nhl21-e1/SHA256SUMS.txt`)

```text
7054494b5b11c6a5  local/runs/league-nhl21-e1/deploy.ps1                    (rewritten: state machine)
3630a8d1f1dfcbec  local/runs/league-nhl21-e1/deploy.superseded-by-r1.ps1   (previous helper, retained for review)
d74c83b05e488df6  local/runs/league-nhl21-e1/MANIFEST.md                   (preconditions/state table/sequences rewritten)
2c446bdb60f7c23b  local/runs/league-nhl21-e1/verify_db_diff.py             (new authoritative DB diff audit)
da827326c75d03a4  local/runs/league-nhl21-e1/db-diff-audit-r1.txt          (PASS)
e5db970896732dbf  local/runs/league-nhl21-e1/prep-report.superseded-by-r1.txt (original failed report, unchanged)
d99f8f4a8d7d2b8e  local/runs/league-nhl21-e1/tests/run-tests.ps1           (regression source)
749620c3fa3525b7  local/runs/league-nhl21-e1/tests/results.txt
c53ef38d2ff1069a  local/runs/league-nhl21-e1/deploy.log                    (preflight 09-17 + status 09-18)
4876cd2b3f4b5187  artifacts/inxp_00.big.X-candidate                        (unchanged)
343404153ae51177  artifacts/nhl_game.db.X-loose-evidence                   (unchanged)
1c41f578a595dbd5  copied/CUSA18182/databigs/inxp_00.big                    (rollback source 2, unchanged)
                  docs/receipts/league-nhl21-e1-prep-r1.md                 (this file)
```

Unchanged: `prep/` harness source (its classifier gap is documented, not edited — the audit is the
separate verifier), `copied/CUSA18182/.tdb-txn/…` engine journal and backups (not touched by hand).

## 5. Cleanup

* `tests\fx\` (24 fixture roots, 4 KB payloads; the test-12 deny ACE cleared first) — **removed**,
  containment-checked, absent afterwards. One-shot `fix_manifest.py` — removed.
* Retained under `local/runs/league-nhl21-e1/` (successor dependency: E1 execution, then E5
  closeout): candidate, loose evidence, restored copied root + engine journal (≈2.35 GB), manifest,
  helper (+ superseded helper), verifier + audit, superseded report, test runner + results, harness
  source. No build cache exists.
* Live install read-only (`databigs\` holds only `inxp_00.big`, `1c41f578…`); no emulator process;
  index empty; anchors untouched.

## 6. Next action

Codex: review `deploy.ps1`'s state table against the test matrix and the manifest's split of
automated checks vs manual gates. If accepted, live placement readiness can be re-evaluated: the
E1 dispatch would run `status` (with `-ExpectedExeSha`), the stock restart/re-entry check owed
from E0, then `install` only on explicit approval of candidate `4876cd2b…`, the modified boots, and
`rollback` + stock boot for E5. Stopping here; no live replacement, no launch.
