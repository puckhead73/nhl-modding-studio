# C0-BULK1 · Checkpoint 4 — Migration (S3)

> **Package:** C0-BULK1, checkpoint 4 of 7. **Date:** 2026-09-07. **Author role:** developer.
> **Status: Passed.** Nothing staged, committed or pushed.
> **The migration candidate was generated but NOT installed, and NOT approved.** Approval and
> installation are the architect's, not this run's.

## Status

**Passed.** Contract stage **S3** is implemented and a real migration candidate was produced from
the three committed legacy sources, in an isolated location. No manual record was written, no legacy
source was modified, and no consumer was retired.

## Files changed

| Path | Change |
|---|---|
| `crates/tdb-qa/src/evidence/migrate.rs` | **New.** Freeze, verify, migrate, cell accounting, divergence, headmesh pair keys, approval-gated install. |
| `crates/tdb-qa/tests/evidence_migration.rs` | **New.** 12 cases (A11, A12, A25, A27, A28, A31, A32 + 5). |
| `crates/tdb-qa/src/bin/qa_evidence.rs` | Added `migrate` and `migrate-install` subcommands. |
| `crates/tdb-qa/src/evidence/mod.rs` | Wired `migrate`. |

## The candidate — for architect review

Generated into an isolated directory outside the repository. **Nothing was installed.**

| Item | Value |
|---|---|
| Migration id | `mig-c0-bulk1` |
| Frozen sources | `H:\c0-bulk1\migration\frozen\` (3 files + `freeze.json`) |
| Candidate | `H:\c0-bulk1\migration\candidate-mig-c0-bulk1\` |
| **Candidate-set digest** | `sha256:39e56bb1ff0c6c82d340bf4d0af6ff1000a867d16760b7f9139dd047b9d046d0` |
| Source cells | **4037** |
| Accounting rows | **4037** — equal, as A12 requires |
| Claims | **399** |
| Unmapped | **2** |
| Manual records | **0** — migration never writes one (R-CLAIM) |
| Divergence rows | **33** |

Frozen source digests:

| Frozen name | SHA-256 |
|---|---|
| `game-qa-matrix.frozen.csv` | `5d2dbd3c5a79257ca55c7c08532e8f3623b555d0a2cf747b7097fd9a101de211` |
| `capability.frozen.csv` | `bffb4ac6c0b74f0c2650fd7c8fda94df942e27ea6b3a8df42cd27c41ee452a2c` |
| `asset-capability-matrix.frozen.csv` | `3767a2c0143293ce12cf852327d2a64fb8c8760cbf350df6920650e7a561e79d` |

Cell arithmetic, verified independently by the test rather than asserted:
392 × 10 + 5 × 13 + 4 × 13 = 3920 + 65 + 52 = **4037**.

### The install command — not run

To install after review, an approval file must name the migration id **and** carry the digest above:

```toml
# approval.toml
schema = "qa-migration-approval/1"
migration_id = "mig-c0-bulk1"
candidate_set_sha256 = "sha256:39e56bb1ff0c6c82d340bf4d0af6ff1000a867d16760b7f9139dd047b9d046d0"
approved_by = "<architect>"
approved_utc = "<utc instant>"
accepted_records = 0
accepted_claims = 399
```

```sh
cargo run -p tdb-qa --bin qa-evidence -- migrate-install \
  --from  H:\c0-bulk1\migration\candidate-mig-c0-bulk1 \
  --approve <approval.toml> \
  --id mig-c0-bulk1 \
  --root <evidence root>
```

Any mismatch in either field refuses and writes nothing — proven by A28.

## Implemented behavior

**R-FREEZE.** Sources are copied byte-for-byte and hashed *together with* the catalog family and
unit ordering used to read them. `verify_freeze` refuses when either the live ordering or a frozen
source's bytes have changed.

**R-POSITIONAL.** Every game-QA claim carries `family_source = "positional-assumption"`,
`confidence = "positional"` and the `positional-family-assumption` token. A row whose own
`game`/`platform` columns contradict its positional unit is written **without a family** into
`_unmapped/`. **Zero rows contradicted** in this run — the committed matrix's row order still
matches the live catalog.

**R-CLAIM.** No run manifest, receipt, `run_id`, `observations.csv`, `performed_utc` or `entered_utc`
appears anywhere in the candidate (asserted per file by A27). Every claim carries a structural
`qualifies = false`.

**R-ACCOUNT.** One row per source cell, including every blank and every `not_run`. `external` keeps
`external-meaning-undefined` (Q-A), `not_run` is `preserved-no-evidence`, `partial` is
`partial-not-rounded`, and `format = "BIGF->refpack->XPR2/RX2"` is preserved verbatim with
`variant-not-mapped` — no variant is invented (A31).

**R-DUP.** Both matrices are frozen and retained; their 33 differences are published, including the
`heads` row that exists in one and not the other. Neither wins.

**R-NO-BACKFILL.** Pair keys are derived from `defs/appearance/headmesh-matrix.toml` only. With the
migrated destination-level `heads` claim installed, every pair key is `not-attempted` /
`unqualified` — asserted by A11.

**R-MIG-SEP.** `install` is the only writer into layer B in the whole crate. It refuses on a digest
or id mismatch, refuses to overwrite an existing claim or record, and never touches the frozen
sources.

## Corrections to earlier records

| Earlier statement | Measured now |
|---|---|
| C0.4a §8 (E11): *"15 pairs across 12 distinct destinations"* | **14 pairs across 8 distinct destinations**, measured from the committed `headmesh-matrix.toml`. Six pairs share `nhllegacy-xbox360` — the one destination the capability CSV has a single `heads` row for. The rule E11 supports is unaffected and in fact **stronger** than recorded. |
| Contract §4.4/§6.5 references to "15 headmesh pairs" | Same correction; the acceptance case asserts the measured numbers, not the quoted ones. |

## Exact checks and results

| Check | Command | Result |
|---|---|---|
| S3 acceptance cases | `cargo test -p tdb-qa --test evidence_migration` | **Passed** — 12/12 |
| Real migration run | `qa-evidence migrate --frozen … --out … --id mig-c0-bulk1` | **Passed**, exit 0; accounting reconciles exactly |
| Whole crate | `cargo test -p tdb-qa` | **Passed** — 47 + 32 + 12 + 9 + 8 = **108**, 0 failed, 0 ignored |
| Strict clippy, scoped | `cargo clippy -p tdb-qa --all-targets -- -D warnings` | **Passed**, exit 0 — the checkpoint-3 `dead_code` is cleared: `write_manual_record` now has its single approved caller |
| Formatting, scoped | `cargo fmt -p tdb-qa -- --check` | **Passed**, exit 0 |

## Evidence / log locations

- `H:\c0-bulk1\logs\cp4-migration.log` — the migration transcript with the candidate digest.
- `H:\c0-bulk1\logs\cp4-tdb-qa-test-summary.txt` — full-crate test summary.
- `H:\c0-bulk1\migration\` — frozen sources and the candidate, preserved for review.

## Preservation checks

| Check | Result |
|---|---|
| HEAD / index | `a2bbdc4…` / **empty** |
| Legacy sources (`game-qa-matrix.csv`, both capability CSVs) | **read only**; `git status` shows no new modification. The pre-existing `capability.csv` modification recorded in C0.4a §7.1 is untouched. |
| `docs/audits/data/**` | not written |
| `docs/audits/evidence/` | **still does not exist** in the repository — the candidate and every install test lived outside it |
| Manual records | **none exist and none were created.** Every install in the tests targeted a temp sandbox |
| Existing consumers (`tests/inventory.rs`, `matrix_guard.rs`, the four `*_matrix.rs` emitters) | **unmodified and still passing** |
| `Cargo.lock` | unchanged since checkpoint 2 |

## Unresolved decisions / blockers

| # | Question | Why it is not decided here |
|---|---|---|
| **M1** | The legacy asset-family token **`uniforms`** has no exact coverage family. `assets-jerseys` is the obvious candidate, but the mapping decides which cell that evidence lands on, so it is left `unmapped` with `family-not-mapped:uniforms`. Both `uniforms` rows (one per duplicate matrix) are the **only** 2 unmapped claims. | A mapping is a claim about what the evidence covers. Guessing it is exactly the invention the contract forbids. **Architect decision.** |
| **M2** | `conversion.recipe` currently uses the registry's `id_scheme` token because no recipe id exists (contract U2). Pair keys are therefore stable but not yet semantically final. | U2, open |
| **M3** | Whether to install this candidate at all, and where the evidence root lives (U3). | Architect / user |

## Prerequisites for the next checkpoint

None. Checkpoint 5 (fixture/runtime register) is documentation over already-recorded repository
evidence and depends on nothing here.
