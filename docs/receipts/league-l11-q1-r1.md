# L1.1-Q1-R1 — runtime-card correction receipt (2026-09-17)

**Plain-language summary.** The in-game test plan for the NHL 21 league rename was corrected so
that it can only claim what an experiment can actually show. The "stock game boots" gate now
passes on its own (one reachable screen with the original label), and the "put everything back"
check is the final gate that closes a run instead of being tangled into the first one. The
experiment that was supposed to tell which of two file locations the game prefers was redesigned:
instead of putting the same edited file in both places (which proves nothing), it places two
different, individually verified renames in the two locations and swaps them, so only a
consistent result counts. Every outcome row now records what was seen, what is still open and
the next small check, rather than blaming the archive, the loose file, the emulator or a "different
consumer" without proof. The parity matrix now leads with the current state, labels the earlier
pre-implementation text as historical, gives every PS3/Xbox 360 game its own row, and describes
packaging as "Studio can produce the artifact", not "the game reads it". Nothing was run in a game
and nothing was installed; the next step remains the stock-boot check only.

---

## 1. Package, disposition, scope

* **Package:** L1.1-Q1-R1 per [league-l11-q1-review-and-r1-dispatch](../validation/league-l11-q1-review-and-r1-dispatch.md).
  Documentation-only correction of the runtime card and the parity matrix for findings 1–4,
  applying the coordinator decisions. No emulator/game launch, live install, application change,
  anchor addition, build, staging, commit or push. Dirty tree preserved.
* **Disposition:** all four findings corrected; the accepted NHL 21 provenance chain is preserved
  verbatim (now marked "payload identities independently re-hashed by Codex"). Phase 1.1 remains
  open; L1.1-Q1 is presented again for acceptance as an executable handoff.

## 2. Changed paths (SHA-256 after this correction)

```text
f5298e703b0faa9ae20210a62e908b5902bbbae00562f4396fdd2529b05cf3ae  docs/plans/league-identity-parity-matrix.md         (head + §0 replaced; §1–§6 untouched)
a16f9a71073d5fa7e62a3a608e7f154df151b35d8a87b22d3b2fc9fd1bed860b  docs/plans/league-identity-runtime-qualification.md  (rewritten)
                                                                  docs/receipts/league-l11-q1-r1.md                    (this file)
```

Previous values (L1.1-Q1 receipt): matrix `45a8941f…`, card `669613cf…`. No other file changed;
`git status anchors` empty; index empty.

## 3. Disposition of each finding

### F1 — E3 could not establish precedence (fixed)

The card's E3 no longer places identical bytes in both locations and no longer claims a fallback
test. It is now a **swapped-control** experiment run only if **both** E1 (archive only) and E2
(loose only) established a label change: two distinguishable existing-key re-points on the same
row — **X** `Allsvenskan → TXT_ROW_LEAGUE` ("Rest Of World") and **Y** `Allsvenskan →
TXT_LEAGUE_NORWAY` ("Norway"; verified present on NHL 21's own dictionary, hash `838edd73`, from
the retained `loc_keys.out`) — each validated offline as its own Studio transaction with the
allowed-difference audit and its own expected hashes. 3a places X in the archive and Y loose; 3b
swaps them. Precedence is established only if the label follows the *location* in both halves;
a label that follows the *edit*, or halves that disagree, is inconclusive. "Identical edited bytes
in both locations establish nothing" is written into the experiment table and the decision table.
No archive deletion or repacking is introduced (§1.6 "never").

### F2 — verdicts over-attributed (fixed)

§1.3 introduces the observation-first order (observed → established → unresolved → next
diagnostic) and the verdict words *established / inconclusive / blocked*, with "fail" reserved for
the restoration gate. The experiment table gained a "what it cannot establish" column; the verdict
table replaces "Fail — packaging/loader", "Fail — precedence", "Fail — lookup" and "different
consumer" with **inconclusive** plus conditional diagnostics: a crash → reboot once (nondeterminism),
then E2, then a *no-change stager control* (the original archive re-written through the stager with
the member bytes unchanged) before the archive artifact can be named; an inert loose file → the
alternatives are listed (path, other source, cached/mode-local data, unobserved consumer) and the
next step depends on E1; a plain boot is recorded as tolerance, not as "member loaded"; two
surfaces disagreeing → positive for the changed surface only, reload check, then a 1.2 trace —
"do not write different consumer". The service-schema lead in the matrix is restated as "declares a
localized string; does not say where or how localization happens". Positive qualification is
preserved: a controlled, repeatable label change on a named surface with unrelated checks
unchanged, consistent reload/second boot and an exact restore.

### F3 — circular G0/E5 gate (fixed)

§1.1: G0's exit condition is **E0 alone** — one reachable named surface showing the baseline.
Second-surface coverage is requested and, if unreachable, recorded as "coverage incomplete" with
no diagnosis. E5 is the **restoration gate** that closes the run and every G1 verdict ("and E5
passed" on each established row) and is never a precondition of G0. Boot failure and
surface-unreachable are separate G0 outcomes in the decision table. The consistency walkthrough
(§3.1) ends with an explicit gate check.

### F4 — current-state claims tightened (fixed)

* Matrix head: a **Current authority** banner comes first; the L1.1-A and overnight banners are
  retitled **Historical banner — …** with their "nothing implemented" / "ship none" statements
  rewritten as *at that time* facts (the dictionaries were not among the supplied fixtures; NHL 21's
  has since been located).
* Packaging legend renamed **Packaging (offline artifact)**: "whether Studio can produce and
  structurally verify an artifact in the shape the game ships … says nothing about whether the game
  reads that location, accepts the artifact, or which location wins"; the NHL 21 row says so
  explicitly.
* PS3/Xbox 360 rows split: 21 individual target rows (Legacy PS3, Legacy X360, Recompiled, NHL
  15–08 × PS3/X360, NHL 07 X360, NHL 07 PS3 research) each with its own verdict columns, linked to
  shared measurement ids `M-legacy`, `M-12-15`, `M-11`, `M-10`, `M-09`, `M-08`, `M-07`.
* gen7 label source: "the game's own label source is **unresolved** — `nhl_eng_us.db` is the obvious
  candidate … whether some labels come from UI data or the executable has not been traced" (matrix
  §0.2 and card §4.2), replacing "the name lives in `nhl_eng_us.db`".
* NHL 19 card: base-vs-patch archive precedence is "a **hypothesis to verify** (patch-first is the
  usual expectation, not a fact for this title/emulator)"; the dictionary is not to be committed
  until base/patch provenance is established (coordinator decision).
* Absence claims qualified to inspected paths: no update folder *under `Games\`*; no `savedata`
  *under the two roots searched to depth 5*; the only log present records NHL 24; `sys_modules/
  CUSA18182` exists; "whether an NHL 21 profile exists elsewhere is unknown".
* Unexpected output hash: §1.4 **pauses qualification** — no placement, explain, rerun the
  allowed-difference audit, re-record the expected value for this run, same edit; "never substitute
  a different edit under the same card". Edit Y and the E4 edit get their own expected values
  recorded before placement.

### Coordinator decisions applied

The modified shadPS4 fork is written into §1.2 as the **provisional initial diagnostic
environment**, identified at run time by exe hash, log version line, config hash and title version;
results qualify that environment only. Dictionaries are **not committed**: NHL 21 uses its
hash-bound local source (`databigs/Extracted/…/eng_us.db` = member 1156 = `24693e2f…`), the card's
attach step checks that hash; NHL 19 needs base/patch provenance first. The external reader was not
retried.

## 4. Consistency walkthrough (logical check, not execution)

Card §3.1 walks the corrected decision table through the eight required hypotheticals:

| Hypothetical | Established | Unresolved | Smallest next action |
|---|---|---|---|
| E0 reaches one surface | G0 pass (that surface) | second surface | E1 on surface 1; coverage incomplete |
| E1 crashes; E2 changes the label | loose placement positive (pending E5); edited bytes tolerated | crash cause (archive artifact / nondeterminism / environment) | reboot E1 once; no-change stager control; E5 before any verdict |
| E1 boots unchanged | artifact tolerated | member loaded? field consumed? source? | E2 |
| E2 stays unchanged | with E1 positive: archive placement positive, loose path inert for that surface (cause unknown); with E1 unchanged: nothing | loose-path semantics / consumer | continue archive placement + E4, or consumer research (1.2) |
| Identical edited copies in both locations | nothing about precedence | — | discard; E3 with X/Y swap if both singles positive |
| Two surfaces disagree | positive for the changed surface | other surface's source | reload check; 1.2 trace; no attribution |
| Edited output hash differs | nothing; paused | why | explain; re-audit; re-record; same edit |
| Restore hashes pass, restore boot fails | nothing about the edit | environment state | environment investigation; run stays open (E5 not passed) |

Gate check: G0 exits on E0 alone; E1–E4 require G0; no established verdict without E5; E5 never
conditions G0.

## 5. Checks executed

Documentation only. Splice of the matrix head via a one-shot script (`local/runs/league-l11-q1-r1/splice.py`,
guarded against double application; LF preserved); headings verified after the run; document
hashes above. No build, test, emulator, game or installed-file action. The durable inputs cited
(dictionary key lookups, member identities) are the L1.1-Q1 outputs preserved by Codex under
[`docs/validation/evidence/league-l11-q1/`](../validation/evidence/league-l11-q1/); no new
measurement was taken.

## 6. Temporary artifacts and cleanup

| Item | Purpose | Disposition |
|---|---|---|
| `E:\Repositories\nhl-modding-studio\local\runs\league-l11-q1-r1\` (`matrix_head.md` 15 208 B, `splice.py` 665 B) | staged matrix head and its splice script | **removed** after the splice (literal path, containment and reparse-point checks; `Test-Path` false) |
| `local/runs/league-l11-q1/` | previous package scratch | confirmed **absent** (removed by Codex after preservation) |
| `local/runs/league-overnight-2026-09-17/` | an **empty** directory left over from the removed series cache | not package-owned; **not touched**; reported for Codex |
| Processes | none launched beyond `python`/`git` | none running |

Anchors untouched; index empty; no `git clean`/`reset`.

## 7. Exact remaining execution gate

The next execution package is **E0 only** (card §2.5): stock boot of the unmodified CUSA18182
01.00 on the provisional diagnostic environment, recording the exe hash / log version line / config
hash / `param.sfo` fields / save-data state, reaching one (requested: two) named league surface
with "HockeyAllsvenskan", and hashing the install files first (`SHA256SUMS.pre.txt` must match
card §2.1). It requires separate authorization; nothing in this correction launches anything.
Stopping for Codex review.
