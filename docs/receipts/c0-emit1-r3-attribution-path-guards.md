# C0-EMIT1-R3 — close attribution and path-validation loopholes

**Package:** C0-EMIT1-R3, correcting the two findings Codex raised on [C0-EMIT1-R2](c0-emit1-r2-fixture-freshness.md).
**Date:** 2026-09-15 (UTC) · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` · **Baseline and HEAD throughout:** `58d1bc073bbf9e80d5afb4782bc6c132725e5fed` ·
**Index before and after:** empty · **Nothing staged, committed or pushed.**

> **Result.** Both loopholes are closed and both were reproduced against the R2 code before the
> fix. (1) `run_fixture_verdict` now requires **every executed observation** (pass, fail,
> refused-clean) to carry a fixture identity that is among the run's attributed inputs; an empty
> hash on an executed observation is `fixture-identity-missing` → `unknown`, while
> `skipped-missing-fixture` and the deliberate `not-attempted` parent cells need none. (2)
> `verify_fixture_inputs` now fails closed on canonicalization and classifies **every** component
> below the repository root without following links: a junction/symlink/reparse point at any
> ancestor or at the fixture itself, an ancestor that is not a directory, or an unresolvable root
> is `Unreadable` and never fresh — including a junction that redirects to byte-identical content
> elsewhere inside the repository. Seven regressions were added (17 in `evidence_fixtures`);
> the one platform case this account cannot create (a *file* symlink) is reported as an explicit
> gap, and the same final-entry refusal is exercised with a junction instead.
>
> No emitter, scope or capture code changed, so **no recapture**: the retained R2 runs rebuild
> under the corrected verifier **byte-identical** to R2's `out1` and `check` against R2's
> disposable published set is still `ok: 2175 row(s)`; all four verdicts `verified`. The R1
> legacy runs produce the same verdicts as under R2. `tdb-qa` **254 passed · 0 failed · 1
> ignored**; clippy `tdb-qa` and fmt clean; the overlay contributes zero clippy diagnostics; the
> `studio-domain` lib gate remains **not clean** (8 HEAD diagnostics, C0.2 baseline).
>
> **Final boundary: 20 paths in total — 18 M · 2 A — of which one is `Cargo.lock`** (19
> source/definition/test paths + the lock), and the lock delta is one line.

---

## 1. Finding 1 — missing fixture identity on executed observations

### 1.1 Reproduction (R2 code)

A copy of the R2 candidate with only the new test file added, its own target directory:

| Case (new) | R2 result | What it shows |
|---|---|---|
| `a_completed_pass_with_an_empty_hash_is_withheld_even_though_the_run_attributes_a_valid_input` | **FAILED** — `left: Fresh, right: Unknown` | a completed pass with an empty `fixture_sha256` rode on the run's one valid attributed input and qualified |
| `mixed_hashed_and_unhashed_executed_observations_make_the_whole_run_unknown` | **FAILED** — `Fresh` | one hashed pass made the whole run verified; the unhashed refused-clean beside it was invisible |
| `an_unhashed_failure_still_blocks_qualification` | **FAILED** — `fresh` was `Fresh` (the reason was already `unreviewed-failure`; the freshness axis was wrong) | |

### 1.2 Corrected rule (`merge::run_fixture_verdict`)

For a run with non-empty `input_digests`, every observation with `outcome.is_executed()` — pass,
fail, refused-clean — is checked:

| Observation | Verdict | Finding |
|---|---|---|
| executed, `fixture_sha256` empty | `Unknown` | `fixture-identity-missing:<run_id>:<count>` |
| executed, hash not among the run's attributed digests | `Unknown` | `fixture-unattributed-observations:<run_id>:<count>` (unchanged from R2) |
| `skipped-missing-fixture`, `not-attempted` | not judged | — |

Run-level, as accepted: one unhashed executed observation makes the whole run `unknown`. The
verdict still enters qualification only at §4.3 row 6, so a recorded failure (row 4a) keeps its
`unreviewed-failure` reason with the identity finding beside it. **No default hash is supplied
anywhere in production**: the emitters already hash every executed observation (R2 §4.2 audit —
0 unhashed executed observations on the real runs), and the rule simply refuses what they never
produce.

### 1.3 Synthetic fixtures updated deliberately

* `testkit::observation` now carries `FIXTURE_DIGEST` on **executed** outcomes only; skips and
  non-attempts stay empty, exactly as the emitters record them.
* Every synthetic emitter in the integration suites (`evidence_discovery`, `evidence_emit`,
  `evidence_file_reads`, `evidence_journal`) attributes `defs/qa/scopes.toml` through one helper
  (`attribute_scopes_file`) and records its observations with `observe_hashed` / `observe_faceted`
  carrying that identity (25 call sites converted mechanically, each from `ev.observe(k, c, o, d)`
  to `ev.observe_hashed(k, c, o, &fixture, "", d)`); `evidence_freshness::capture_all` hashes with
  the synthetic DB; `evidence_journal::declared_pass` carries the synthetic fixture on its
  in-memory observation.

## 2. Finding 2 — path handling vs. its stated guarantee

### 2.1 Reproduction (R2 code)

| Case (new) | R2 result | What it shows |
|---|---|---|
| `a_junction_ancestor_redirecting_inside_the_repository_is_refused` | **FAILED** — `got Some(Present("5b639ae0…"))` | `anchors/ps3/nhl15/databases` replaced by a junction to a sibling directory *inside* the repository with byte-identical content: R2 checked only the final entry, canonicalized through the junction, found the target under the root, and **verified** it |
| `an_ancestor_that_is_a_file_and_an_unresolvable_repository_root_are_refused` | **FAILED** — `got Some(Missing)` | a regular file where the `databases` directory belongs: the stat below it returned `NotFound`, which R2 read as the fixture being absent |

### 2.2 Corrected behaviour (`store::verify_fixture_inputs`)

1. **Root, fail closed.** `repo_root.canonicalize()` failing marks every attributed path
   `Unreadable("repository root cannot be resolved: …")` and reads nothing.
2. **Path shape.** Every component must be `Component::Normal`: an absolute path, a prefix, a
   root or `..` is `Unreadable("not a repository-relative path")`, unread.
3. **Every component below the root**, in order: `symlink_metadata` (never following) →
   `paths::entry_kind_of`, which refuses any reparse point (symlink, junction, mount point —
   `FileType::is_symlink` plus the reparse attribute on Windows) with `discovery-reparse-point`
   before classifying. An intermediate component must be a directory
   (`Unreadable("ancestor <path> is not a directory")` otherwise); the final one must be a regular
   file (`"a directory, not a file"` / `"not a regular file"`). `NotFound` at any component is
   `Missing`.
4. **Resolution, fail closed.** The final path's `canonicalize()` failing is
   `Unreadable("cannot be resolved: …")`; a canonical path outside the root is
   `Unreadable("resolves outside the repository")`. Only then is the file read, once per resolved
   path per invocation.

What this does **not** claim: classification and reading are two operations, so a writer that
swaps the entry between them is not defended against. The doc comment says so. This is
discovery-time verification, not race-proof filesystem access.

### 2.3 Rejection coverage and platform gaps

| Case | Mechanism | On this machine |
|---|---|---|
| junction at an **ancestor** redirecting inside the repository | `mklink /J` (no elevation) | executed; `Unreadable(…reparse…)`, `files_hashed == 1` (only the save fixture), read+edit rows `unknown`, save row still qualified |
| junction at the **fixture's own path** | `mklink /J` | executed; refused as a reparse point before the wrong-type check |
| regular **file** where an ancestor directory belongs | plain `fs::write` | executed; `Unreadable("ancestor … is not a directory")` |
| **unresolvable repository root** | a non-existent directory | executed; every state `Unreadable("repository root cannot be resolved…")`, 0 files hashed |
| `..` / absolute path to a readable file outside the repository | — | executed (R2 case); 0 files hashed |
| **file symlink** at the fixture's path | `symlink_file` | **GAP on this account**: Windows file symlinks need Developer Mode or elevation; the case reports `GAP: file symlinks cannot be created by this account…` and returns without asserting. It executes where the platform allows; the junction-at-final-entry case covers the same refusal path here |

## 3. Checks executed

| Check (R3 candidate, external target, `QA_*` unset) | Result |
|---|---|
| `cargo test -p tdb-qa --test evidence_fixtures` | **17 passed** (10 from R2 + 7 new) |
| `--test evidence_discovery` / `evidence_file_reads` / `evidence_emit` / `evidence_journal` / `evidence_publication` | 13 / 9 / 9 / 31 / 36 passed |
| `cargo test -p tdb-qa --locked --offline --no-fail-fast` (whole crate) | **254 passed · 0 failed · 1 ignored** (72 unit · 32 acceptance · 13 discovery · 9 emit · 9 file-reads · 17 fixtures · 7 freshness · 31 journal · 19 + 1 ignored migration · 36 publication · 9 inventory); ~3 m including compilation |
| `cargo clippy -p tdb-qa --all-targets --locked --offline -- -D warnings` | exit 0 (after removing two unused bindings the conversion had left in `evidence_file_reads` / `evidence_journal`, by giving those faceted observations their identity) |
| `cargo fmt -p tdb-qa -- --check` | exit 0 |
| `rustfmt --check` on all 17 overlay `.rs` files | clean |
| `cargo clippy -p studio-domain --no-deps --test {4 emitters} -- -D warnings` | exit 101: **8** diagnostics in `studio-domain/src/{assets ×3, attributes, formula, head_catalog ×2, view}.rs` — **baseline** (HEAD lib, C0.2); the four emitter targets contribute **0** |

## 4. Retained runs under the corrected verifier — no recapture

Nothing in the capture path changed (emitters, `emit.rs`, `defs/qa/scopes.toml` are byte-identical
to R2), so the R2 runs keep valid identities and completeness.

| Step | Result |
|---|---|
| `build --root <R2 evroot>` ×2 | `fixtures: 148 attributed path(s), 148 file(s) hashed, 466 003 284 byte(s)`; byte-identical to each other **and to R2's `out1`** (10 files + `build-inputs.json`) |
| `check --root <R2 evroot> --data <R2 data-scratch>` | **`ok: 2175 row(s), no validation failure, no drift`** — the corrected verifier changes nothing for valid real runs |
| verdicts | `surface_matrix` / `mutation_matrix` / `operation_matrix` / `save_matrix` = `verified` |
| R1 legacy runs (`…/emit1r1/evroot`) | identical verdicts to R2's run: `mutation` verified; `operation` `unknown:fixture-unreadable:anchors/ps3/nhl07/databases:a directory, not a file`; `surface`/`save` `unknown:fixture-attribution-missing`; 0 qualified, 411 cited `unknown` |
| R2 evidence root | file manifest identical before and after; all three retained roots still hold 4 runs each |

Outputs therefore differ from R2 **nowhere** on retained real evidence; the correction changes
results only for the malformed inputs the new cases construct (unhashed executed observations,
linked or non-directory ancestors, an unresolvable root), which no real run carries.

## 5. Final integration boundary

Baseline HEAD + this overlay = the accumulated EMIT1 / R1 / R2 / R3 slice.
**20 paths in total: 18 M · 2 A, one of them `Cargo.lock`.** SHA-256 as placed in the candidate
(CRLF); `git hash-object` equals the live blob for every path except the lock, whose live copy
carries unrelated drift. Full hashes: `…/scratchpad/emit1r3/logs/overlay-manifest.txt`.

| # | Path | Op | SHA-256 (candidate) | Blob | Changed by |
|---|---|---|---|---|---|
| 1 | `crates/studio-domain/Cargo.toml` | M | `e87e8f12a7ceac18…` | `af8e2b5d` | EMIT1 |
| 2 | `crates/studio-domain/tests/surface_matrix.rs` | M | `ed48dc8f1afc3bc3…` | `e1586e25` | EMIT1, R2 |
| 3 | `crates/studio-domain/tests/mutation_matrix.rs` | M | `b90c921e0a740027…` | `6e0117af` | EMIT1 |
| 4 | `crates/studio-domain/tests/operation_matrix.rs` | M | `3ef283efd526a4fe…` | `26e14def` | EMIT1, R2 |
| 5 | `crates/studio-domain/tests/save_matrix.rs` | M | `d9fb9d2f9cb50423…` | `51e693cf` | EMIT1, R2 |
| 6 | `crates/tdb-qa/src/bin/qa_evidence.rs` | M | `ef916551b3830048…` | `a145e738` | R2 |
| 7 | `crates/tdb-qa/src/evidence/merge.rs` | M | `ab41f1d23cd58edc…` | `110882d7` | R2, **R3** |
| 8 | `crates/tdb-qa/src/evidence/model.rs` | M | `d9be5e07d03020f6…` | `1b5ecf96` | R2 |
| 9 | `crates/tdb-qa/src/evidence/publish.rs` | M | `f890a2c07cadf11c…` | `88603971` | R2 |
| 10 | `crates/tdb-qa/src/evidence/store.rs` | M | `78d912bf81c5c643…` | `0a2dd32f` | R2, **R3** |
| 11 | `crates/tdb-qa/src/evidence/testkit.rs` | M | `7881bf6211d42b20…` | `b7271328` | R2, **R3** |
| 12 | `crates/tdb-qa/tests/evidence_discovery.rs` | M | `82a8fe8dc229ee8f…` | `73e8e0d9` | R2, **R3** |
| 13 | `crates/tdb-qa/tests/evidence_emit.rs` | M | `259c771f9d3da7ca…` | `241cf72a` | EMIT1, R2, **R3** |
| 14 | `crates/tdb-qa/tests/evidence_file_reads.rs` | M | `92d938ae2aa3ecf8…` | `57c661c6` | R2, **R3** |
| 15 | `crates/tdb-qa/tests/evidence_journal.rs` | M | `184f065a68afffda…` | `8520616e` | R2, **R3** |
| 16 | `crates/tdb-qa/tests/evidence_publication.rs` | M | `79270d56facc881f…` | `c1b83602` | R2 |
| 17 | `crates/tdb-qa/tests/evidence_fixtures.rs` | **A** | `9cb6f55a983cbd13…` | `97cd09ee` | R2, **R3** |
| 18 | `crates/tdb-qa/tests/evidence_freshness.rs` | **A** | `f8dba5bc568f1099…` | `c6d1d077` | R1, R2, **R3** |
| 19 | `defs/qa/scopes.toml` | M | `71c944736355f61b…` | `1a2bf8a1` | EMIT1, R1 |
| 20 | `Cargo.lock` | M | `4a4cc094aa6f2776…` | `8decfca0` | EMIT1 |

**Lock delta — the single line**, built from `HEAD:Cargo.lock`:

```
@@ -3372 +3373 @@ name = "studio-domain"
+ "tdb-qa",
```

Root `Cargo.toml` is HEAD's. The candidate (1 194 files) was unchanged by execution; live
`anchors/**` + `docs/audits/**` 621 files, 0 mismatches; `docs/audits/evidence` absent; `git status`
269 → 269; HEAD `58d1bc07…`, index empty. Documentation (the EMIT1 … R3 receipts) stays outside
the commit. Subject proposal unchanged:
`feat(qa): wire matrix emitters to the evidence observation lifecycle`.

## 6. Remaining decisions (unchanged) and non-defects

Activation decisions carried from R2 §7: U3 storage / D2 retention, R-TRANSITION, R-NO-CI,
per-observation precision, hashing-cost policy, Q1–Q4 / R3. Known cosmetics carried: the
`save_matrix` parent-cell detail spacing and the `paths.rs` rule-message spacing. Platform gap: the
file-symlink final-entry case does not execute on an account without symlink privilege (§2.3).
No implementation defect is open.

## 7. Limits honoured

No staging, commit or push; no live publication or migration; no CI change; no game installation;
no registry expansion; no headmesh work; no external repository; no baseline repair. Mutations
happened only in disposable copies; retained runs and live anchors are byte-unchanged. Stopping for
Codex review.
