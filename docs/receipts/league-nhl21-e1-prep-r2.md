# NHL21-E1-PREP-R2 — qualification-path correction receipt (2026-09-18)

**Plain-language summary.** Two remaining holes in the archive-swap script are closed. Finishing an
install that was interrupted between its two renames now has to pass the same environment checks
as a fresh install (emulator stopped, no stray loose database, empty mod/patch/cheat folders,
unchanged config, expected emulator build); if any fails, nothing is touched and the script names
"rollback" as the way out — and that rollback still works with none of those checks. The `status`
mode now prints exactly the plan the install/rollback modes will execute, because both come from
one shared decision function, so it can no longer say "ready" for a state the real action would
refuse. The unreadable-folder test now proves the denial actually took effect before it counts, and
a separately labelled injected-failure case covers the same path when a real denial cannot be
produced. 33 fixture scenarios pass; the live installation is unchanged and nothing was launched.

---

## 1. Package, disposition, scope

* **Package:** NHL21-E1-PREP-R2 per [league-nhl21-e1-prep-r1-review-and-r2-dispatch](../validation/league-nhl21-e1-prep-r1-review-and-r2-dispatch.md).
  Only the two reproduced findings and the negative-test setup; candidate bytes and the accepted
  interruption recovery retained.
* **Disposition:** F1 and F2 fixed with regressions; test 12 repaired; suite 33/33; read-only live
  `status` executed with the exact expected emulator hash. Live `inxp_00.big` re-hashed `1c41f578…`
  at closeout; no emulator/game launch; no application change; nothing staged/committed/pushed; no
  anchor added.

## 2. Findings

### F1 — resumed install bypassed qualification (fixed, tested)

`deploy.ps1` (`63a8a4d4…`) now routes every install completion — fresh (S0) or resumed (S2) —
through `EnvChecks`: emulator stopped, no loose game-tree DB dir, the four user-data folders exist
and enumerate empty (enumeration failure fails closed), config equals the E0 pre-run config except
`windowPosX/Y`, and emulator exe identity when `-ExpectedExeSha` is given. For S2 the gate is
`env` only: **no** candidate-artifact check and **no** free-space requirement, because the verified
`.e1-incoming` candidate already suffices. On a failed gate the plan becomes `REFUSE`, every file is
preserved, and the reason ends with "recovery action: -Mode rollback (restores the verified original
without these gates)". Rollback from S2 remains gate-free.
Regressions: **16a** (S2 + changed config → install REFUSE with `config unchanged vs E0 FAIL`,
files preserved, then rollback MUTATE restores), **16b** (S2 + non-empty `mods_folder`, candidate
artifact deleted, zero free space → install refuses on the overlay alone with no `candidate verified`
line; after clearing the overlay the resume completes with zero space and no artifact).

### F2 — status did not predict the action (fixed, tested)

One `Plan($which)` function derives `{Action: MUTATE|NOOP|REFUSE, Reason, Steps, Gate}` from the
classified disk state (including sidecar conditions such as an unknown incoming, a duplicate
original, a missing secondary, S2 and both fallback states) plus, for install mutations only, the
environment gates. `status` prints the install and rollback plans; `install`/`rollback` print the
same plan and then execute it. Environment qualification is labelled "applies to install completion
only"; a `NOOP` is reported as an already-complete state, never as permission to mutate.
Regressions **17a–17f** use an `Agree` helper: `status` must predict `MUTATE` (exit 0 with a state
change), `NOOP` (exit 0, no change) or `REFUSE` (exit 2, no change) and the mutation mode must then
do exactly that — stock + unknown incoming (install REFUSE / rollback NOOP "unknown sidecars stay
preserved"), duplicate original (REFUSE / NOOP "preserved"), candidate live with no sources (install
NOOP / rollback REFUSE "secondary source is absent"), interrupted install (both MUTATE with the
resume/undo reasons), fallback needed and fallback-mid (rollback MUTATE), already-installed and stock
no-ops.

### Test 12 setup (repaired)

**12a** asserts `icacls` exit 0, then independently confirms enumeration of the fixture folder
throws **before** the helper runs; if the denial does not take effect the case throws
`COVERAGE UNAVAILABLE` instead of passing on a readable folder. After the case the ACE is removed and
readability re-verified. On this host the denial was effective: rollback succeeded and install failed
closed (`mods_folder readable FAIL`). **12b** uses a fixture-only seam (`E1_TEST_ENUM_FAIL=<folder>`,
refused in the live profile) that raises an enumeration error labelled `INJECTED`; install fails
closed and rollback ignores the seam — recorded as injected evidence only. Codex's own run reported
the ACL denial ineffective in its session; 12a now reports that condition explicitly rather than
counting it as a pass.

## 3. Tests and checks executed (developer, this host)

* `tests\run-tests.ps1` (`343a61a4…`) → `tests\results.txt` (`ea482b71…`): **33 passed, 0 failed**
  (24 prior cases + 12a/12b + 16a/16b + 17a–17f). Fixture roots under `tests\fx` (4 KB payloads),
  helper invoked as a child process; fixture profile refuses `H:\`/`ShadPS4`/`anchors\` roots; live
  profile refuses every override and seam (test 15, extended to `E1_TEST_ENUM_FAIL`).
* Read-only live `deploy.ps1 -Mode status -ExpectedExeSha aeb04892…`: state `S0-stock`; Live =
  original `1c41f578…`, Original/Incoming/Used absent, Candidate `4876cd2b…`, Secondary original; all
  automated checks PASS incl. `emulator exe identity` and `config unchanged vs E0`; "environment
  qualification: PASS (applies to install completion only)"; **install plan: MUTATE — install from
  stock** (5 steps listed); rollback prerequisites met, primary absent (expected on stock), secondary
  present; **rollback plan: NOOP — live is already the original**. Exit 0; nothing mutated
  (`deploy.log`, `496f3065…`).
* Not executed: live `install`/`rollback`; any emulator or game launch; candidate regeneration.

## 4. Changed / retained paths (SHA-256 prefixes; full list `local/runs/league-nhl21-e1/SHA256SUMS.txt`)

```text
63a8a4d488c328a9  local/runs/league-nhl21-e1/deploy.ps1                    (R2: shared Plan, gated S2 resume, enum seam)
7054494b5b11c6a5  local/runs/league-nhl21-e1/deploy.superseded-by-r2.ps1   (R1 helper, retained)
3630a8d1f1dfcbec  local/runs/league-nhl21-e1/deploy.superseded-by-r1.ps1   (original helper, retained)
27e6b06e1a860332  local/runs/league-nhl21-e1/MANIFEST.md                   (S2/S7 rows, plan/status wording, 33 cases)
343a61a49e94e977  local/runs/league-nhl21-e1/tests/run-tests.ps1           (12a/12b, 16a/16b, 17a-17f, Agree helper)
ea482b7119b8153c  local/runs/league-nhl21-e1/tests/results.txt
496f3065ed347ac4  local/runs/league-nhl21-e1/deploy.log
4876cd2b3f4b5187  artifacts/inxp_00.big.X-candidate                        (unchanged)
343404153ae51177  artifacts/nhl_game.db.X-loose-evidence                   (unchanged)
1c41f578a595dbd5  copied/CUSA18182/databigs/inxp_00.big                    (rollback source 2, unchanged)
                  docs/receipts/league-nhl21-e1-prep-r2.md                 (this file)
```

Unchanged: `verify_db_diff.py` / `db-diff-audit-r1.txt` (PASS, independently rerun by Codex),
superseded prep report, harness source, engine journal and backups (not touched by hand).

## 5. Cleanup

* `tests\fx\` — deny-ACE residue checked (0 remaining, test folder readable), then removed with
  containment checks; absent afterwards. One-shot `patch_r2.py` and temporary head/tail assembly
  files removed.
* Retained under `local/runs/league-nhl21-e1/` for the named E1/E5 dependency: candidate, loose
  evidence, restored copied root + engine journal (≈2.35 GB), manifest, helper (+ two superseded
  helpers for review), verifier + audit, superseded report, test runner + results, harness source.
* Live install read-only (`databigs\` holds only `inxp_00.big`); no emulator process; index empty;
  anchors untouched.

## 6. Next action

Codex: review the shared `Plan` logic against the 17x agreement cases and the S2 gate. If accepted,
live placement readiness can be cleared for the E1 dispatch: `status -ExpectedExeSha …`, the stock
restart/re-entry check still owed from E0, then `install` only on explicit approval of candidate
`4876cd2b…`, the modified boots, and `rollback` + stock boot (E5). Stopping here; no live
replacement, no launch.
