# C0-MIG-REVIEW1 — the two recorded migration questions

**Package:** C0-MIG-REVIEW1 (read-only diagnosis) ·
**Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` ·
**HEAD at start and at end:** `ef0fdd74b37a15666d4eba25f66107bdc8948bd8` (unchanged) ·
**Index at start and at end:** empty

> **Verdict.**
>
> 1. **`capability.frozen.csv` → `nhllegacy-xbox360` is compliant.** The constant is a restatement
>    of the only fact that identifies that file's target — its repository path — and the contract's
>    own §6.3 accounting example spells the same attribution. **No incorrect outcome is
>    demonstrable**, and even a wrong target could not qualify anything: a layer-L claim's
>    `qualifies()` is structurally `false`. One **latent maintenance hazard** is recorded: the
>    literal lives in a second table that nothing binds to `SOURCES`, so re-pointing the source
>    path would silently keep the old attribution and no check could catch it.
> 2. **The `"unknown"` substitution in `headmesh_pair_keys` is compliant** — contract §2.2 R-CONV
>    sanctions it verbatim, an absent `target` is **fail-closed** (the key does not parse and the
>    whole call errors), and an `unknown` in `from` / `path` / `recipe` **withholds** qualification
>    at merge rows 7 and 8. **But the question points at the wrong token.** The same function fills
>    two conversion fields with *known-looking* tokens where nothing is recorded — `class` is the
>    hardcoded literal `head-authored` for every pair, and `recipe` is filled from `id_scheme`,
>    which is not a recipe id. Those **do not** block qualification, where `"unknown"` would. That
>    is the fabricated-identity path, and I record it as a **defect, currently unreachable**.
>
> Read-only. Nothing built, nothing executed, no production code altered, no candidate rendered, no
> registry or capability data acquired, no staging, commit or push. The only live-repository write
> is this file.

---

## 0. What was read

`crates/tdb-qa/src/evidence/migrate.rs`, `key.rs`, `merge.rs` (qualification derivation),
`validate.rs`, `vocab.rs` (applicability), `model.rs` (`LegacyClaim`, `EvidenceSet`),
`crates/tdb-qa/src/bin/qa_evidence.rs` (`migrate_cmd`, `migrate_install_cmd`),
`crates/tdb-qa/tests/evidence_migration.rs`, `docs/plans/qa-evidence-contract.md` §2.2, §4.3, §6,
and the three frozen sources' headers plus `defs/appearance/headmesh-matrix.toml`.

**Method: static only.** No execution was needed, and none was performed: every claim below is a
consequence of code that is quoted or of file content that is quoted. Where a conclusion is a
consequence rather than an observation, it is labelled *(static)*.

---

## Checkpoint 1 — the `nhllegacy-xbox360` attribution

### 1.1 Contract requirement

| Where | Requirement |
|---|---|
| §2.1 (row 48) | `docs/audits/data/assets/nhllegacy/xbox360/capability.csv` — **none — hand-maintained** — is listed as one specific file with one specific path |
| §6.1 **R-FREEZE** | sources are copied byte-for-byte and hashed together with the ordering used to interpret them. The contract's own `freeze.json` example records `name → sha256` and **no source path** |
| §6.1 **R-POSITIONAL** | where a row's *own content* contradicts its positional mapping, the row is emitted without a family and preserved in `_unmapped/`. "No family is guessed" |
| §6.3 | the worked accounting rows are `…,capability.frozen.csv,2,install,external,preserved-raw,cap-nhllegacy-xbox360-portraits-install,…` — the contract itself attributes this file's rows to `nhllegacy-xbox360` |

### 1.2 Actual reachable behavior

The frozen file's header is (verbatim, first line of the committed source):

```
asset_family,format,discover,identify,preview,export,reimport_unchanged,replace_modified,reassign_existing,rebuild,install,game_verified,evidence
```

**There is no `game_id` and no `platform` column.** The target is recoverable from exactly one
thing: the file's directory, `docs/audits/data/assets/nhllegacy/xbox360/`. `find docs/audits/data
-name capability.csv` returns that path and nothing else — it is the only such file in the tree.
`SOURCES` ([migrate.rs:50-63](../../crates/tdb-qa/src/evidence/migrate.rs#L50-L63)) pins the frozen
name to that path. So the literal is not an invention: it is the declared source's path, restated.

This is also why R-POSITIONAL's content cross-check — which `migrate` *does* apply to
`game-qa-matrix.frozen.csv` at [migrate.rs:328-333](../../crates/tdb-qa/src/evidence/migrate.rs#L328-L333)
— is **absent here rather than omitted**: there is no content to check the target against. The
second matrix, which *does* carry `game_id,platform`, correctly derives its target per row
([migrate.rs:438-452](../../crates/tdb-qa/src/evidence/migrate.rs#L438-L452)) and passes `""` as its
constant. The two-branch design is coherent.

**Freeze → migrate, checked end to end:**

1. `freeze()` reads `repo_root.join(rel)` for each `SOURCES` entry and records `name → sha256`. It
   **does not record the repo-relative path** — matching the contract's example, so this is not a
   deviation, but it means the name→path binding survives only as a code constant.
2. `verify_freeze()` re-checks the frozen bytes against `freeze.json` and the live catalog ordering
   against the frozen ordering. It is **self-consistency only**: it never re-reads the repository
   source, so a `freeze.json`/CSV pair produced against a different `--repo` root, or hand-authored,
   is accepted verbatim.
3. `migrate()` then attributes rows using a **second, independent table**
   ([migrate.rs:400-413](../../crates/tdb-qa/src/evidence/migrate.rs#L400-L413)) keyed on the frozen
   *name*, in which `("capability.frozen.csv", "cap", "asset_family", "nhllegacy-xbox360")` is a
   literal. Nothing ties this tuple to the `SOURCES` entry it is about.

**Can other input be accepted and wrongly attributed?** Yes, in one way that matters *(static)*:
because the target literal and the source path are two unlinked constants, editing `SOURCES` to
point at another target's capability CSV — precisely the generic shape the contract writes as
`assets/<title>/<platform>/capability.csv` in its §2.1 `matrix_guard` row — keeps the
`nhllegacy-xbox360` attribution, and **no downstream check can detect it**: the CSV carries no
target, `freeze.json` carries no path, and `validate.rs` never checks a key target against
`catalog::CATALOG`. The operator-supplied-`--frozen`-directory route reaches the same place, but it
is a general property of trusting a freeze, applies equally to all three sources, and is bounded by
the approval digest at `install`.

**Blast radius if it did happen.** Bounded, and *not* qualification. `LegacyClaim::qualifies()`
returns `false` unconditionally ([model.rs:428-431](../../crates/tdb-qa/src/evidence/model.rs#L428-L431)),
and `merge` cites claims by `target/family` prefix with `selected: false`
([merge.rs:573-588](../../crates/tdb-qa/src/evidence/merge.rs#L573-L588)). A wrong target would
therefore surface a non-qualifying citation and its `unresolved` tokens on the wrong target's rows —
misleading, never qualifying. If the wrong target were also off-catalog, `vocab::applicability`
returns `Unknown` ([vocab.rs:182-188](../../crates/tdb-qa/src/evidence/vocab.rs#L182-L188)) and
merge row 2 withholds the row outright.

### 1.3 Existing regression coverage

[`evidence_migration.rs:403`](../../crates/tdb-qa/tests/evidence_migration.rs#L403) asserts
`heads_claim.target == Some("nhllegacy-xbox360")` over a synthetic source root. It pins the current
behavior, but it **cannot detect the hazard above**: the synthetic root writes its capability CSV at
the same `assets/nhllegacy/xbox360/` path, so `SOURCES` and the literal always agree inside the
test. There is no case in which they disagree.

### 1.4 Assessment

**Compliant.** The constant is a justified restatement of the declared source's path, it matches the
contract's own worked example, no reachable input produces a demonstrably wrong attribution today,
and the failure mode it could produce is citation-level, never qualification.

**Smallest justified correction — optional, hazard-closing, not required for compliance:** move the
target into the `SOURCES` tuple (`(frozen name, repo-relative path, target)`) and have `migrate`
read it from there, so the path and the target cannot be edited apart. Code-only; no schema, no
file-format and no evidence change. Recording the repo-relative path in `freeze.json` would give
real provenance as well, but that changes `qa-legacy-freeze/1` and is outside this package's limits;
it is offered as a separate question, not a recommendation.

---

## Checkpoint 2 — the `"unknown"` substitutions

### 2.1 Contract requirement

- **§2.2 R-CONV:** "All four fields are present when the component is present, in the fixed order
  above; **unknown fields take the literal `unknown`, which makes the key valid but blocks
  qualification** (§4.3). Two headmesh pairs with different sources, paths or recipes therefore have
  **different keys** and can never collapse."
- **§3 R-UNKNOWN:** "Any field whose value is not recorded is the literal `"unknown"`. It is never
  omitted, never defaulted and never inferred."
- **§6.5 R-NO-BACKFILL:** pair-specific keys start `unqualified`; the destination-level claim is
  never copied onto them.

So an `"unknown"` in a conversion field is not merely tolerated — it is the **prescribed** encoding,
and its whole purpose is to *block* qualification.

### 2.2 Exactly which absent fields become `"unknown"`

[migrate.rs:668-704](../../crates/tdb-qa/src/evidence/migrate.rs#L668-L704):

| Key component | Source | On absence |
|---|---|---|
| `target` (key target) | `pair.target` via `field()` | `"unknown"` |
| `from` | `pair.source` via `field()` | `"unknown"` |
| `path` | `pair.path` via `field()` | `"unknown"` |
| `recipe` | `pair.id_scheme` | `"unknown"` |
| `class` | **hardcoded literal `head-authored`** | never unknown |
| `operation` | caller argument | n/a |

### 2.3 Actual reachable behavior, traced

**a) Absent `target` is fail-closed, not fabricated.** `field("target")` yields `"unknown"`, the raw
key becomes `unknown/assets-heads/…`, and `CoverageKey::parse` requires the target to split on `-`
([key.rs:126-133](../../crates/tdb-qa/src/evidence/key.rs#L126-L133)); `"unknown"` has no hyphen, so
parse returns `KeyError::Target` and `headmesh_pair_keys` returns `Err` for the whole registry. No
fabricated target reaches validation, persistence or merge *(static)*.

**b) Absent `from` / `path` / `recipe` are valid keys that cannot qualify.** `Conversion::is_fully_known`
([key.rs:76-82](../../crates/tdb-qa/src/evidence/key.rs#L76-L82)) is consulted in merge at both
qualification-granting rows — row 7 game-acceptance
([merge.rs:703-712](../../crates/tdb-qa/src/evidence/merge.rs#L703-L712)) and row 8 automated-pass
([merge.rs:714-723](../../crates/tdb-qa/src/evidence/merge.rs#L714-L723)) — each returning
`Withheld` with `reason = unknown-conversion-identity`. **No missing value can produce accepted
qualification.**

**c) Collisions are possible in principle and absent in fact.** Two pairs differing only in an
absent field render byte-identical keys. `headmesh_pair_keys` returns a `Vec` with no duplicate
check, and every consumer collects into `BTreeSet<CoverageKey>`
([model.rs:462](../../crates/tdb-qa/src/evidence/model.rs#L462)), so duplicates would **merge
silently, with no diagnostic** *(static)*. Against the working-tree registry this is unreachable:
all 15 `[[pair]]` entries declare `source`, `target`, `path` and `id_scheme`, and the 15 resulting
`(target, from, class, path, recipe)` tuples are distinct (enumerated from the file; no collisions).

**d) There is no production caller.** `headmesh_pair_keys` is invoked only from
`evidence_migration.rs`; `qa_evidence.rs` never calls it, and `defs/appearance/headmesh-matrix.toml`
is **untracked at HEAD** (`git ls-files --error-unmatch` fails on it). Nothing persists these keys
today, so every finding here is latent by construction.

### 2.4 The actual fabrication, which is not the `"unknown"` token

Two fields are filled with **known-looking tokens for values that are not recorded** — the exact
inverse of R-UNKNOWN, and unlike `"unknown"` they leave qualification **unblocked**:

1. **`class` is always `head-authored`**, hardcoded in the format string
   ([migrate.rs:687-689](../../crates/tdb-qa/src/evidence/migrate.rs#L687-L689)), including for the
   registry's `donor-weights` and `donor-morph` pairs. The `conversion.class` vocabulary is open
   question **U2** in the contract (§14) — i.e. there is no vocabulary yet from which this value
   could have been drawn.
2. **`recipe` is filled from `id_scheme`.** An id scheme is an id-numbering convention, not a
   recipe. The code's own comment says "*No recipe id exists in the registry yet, so the status
   token stands in and the `unknown` case is explicit rather than fabricated (contract U2)*" — which
   is wrong twice: the code reads `id_scheme`, not `status`, and the substitution is precisely what
   makes the `unknown` case **inexplicit**. Concretely, 8 of the 15 working-tree pairs share
   `id_scheme = "rw-ppad"`, so `recipe` distinguishes nothing; today's key distinctness comes from
   `(target, from, path)` alone. Should two recipes ever share a scheme on one `(target, source,
   path)`, R-CONV's "can never collapse" fails silently per (c).

Had both been emitted as `"unknown"`, `is_fully_known()` would be false and every pair key would be
withheld from qualification — which is what §6.5 step 3 says these keys are supposed to be.

### 2.5 Existing regression coverage

| Behavior | Coverage |
|---|---|
| absent `target` fails closed | **none** |
| absent `from`/`path`/`recipe` withholds qualification | **none** for the migration path (`key.rs:306` asserts `is_fully_known()` on a hand-built key only) |
| pair keys distinct / no collapse | synthetic A11 ([evidence_migration.rs:353-392](../../crates/tdb-qa/tests/evidence_migration.rs#L353-L392)) — all five synthetic pairs are **fully populated**, so the substitution path is never entered; the real-registry distinctness assertion is in the `#[ignore]`d case and remains unexecuted |
| `class` / `recipe` placeholders | **none** |

No test in the crate writes a `[[pair]]` with a field missing.

### 2.6 Assessment

- **The behavior as asked — substituting `"unknown"` for absent registry fields: compliant.** It is
  the contract's prescribed encoding, an absent `target` is fail-closed, and no missing value can
  yield fabricated qualification.
- **`class = "head-authored"` and `recipe = id_scheme`: defect, currently unreachable.** They
  fabricate identity in a way that does *not* block qualification, contrary to R-UNKNOWN and to the
  intent of R-CONV. Unreachable today because there is no production caller and the registry is
  uncommitted — which is why this is reported for decision, not repaired.
- **Duplicate-key silence: unresolved.** `headmesh_pair_keys` returning a `Vec` that consumers
  silently set-dedup means a future collapse would be invisible. Not currently reachable.

**Smallest justified correction, when the registry lands and U2 closes** (not now, and not by this
package): emit `recipe = "unknown"` until a real recipe id exists — carrying `id_scheme` as a
recorded attribute rather than as the key field — and either resolve U2 or emit `class = "unknown"`
for pairs whose class is not declared. Both changes only move keys **into** the withheld state, so
neither can promote anything. A duplicate-key refusal in `headmesh_pair_keys` (error, not dedup)
would close 2.6's third item at the same cost.

---

## 3. Observation recorded, deliberately not pursued

The registry uses `path = "both"` on six pairs, which renders one key covering two conversion paths.
Whether that is an admissible key or two keys is a **contract/registry question**, not a migration
defect, and is outside this package. Recorded for Codex; no action taken.

---

## 4. Limits honoured

No schema change, no feature import, no real-registry execution, no live migration, no publication,
no CI change, no staging, no commit, no push, no deletion or modification of prior evidence,
candidates or game data. No build or test run. `docs/receipts/c0-mig-review1.md` is the only live
write. Stopping here for Codex review.
