# S2-B — shared copied-target transaction: journal, apply, recovery, restore

> **Package:** S2-B of [s2a-r1-review-and-s2b-dispatch](../validation/s2a-r1-review-and-s2b-dispatch.md).
> **Date:** 2026-09-16. **Author role:** developer. Reviewer: Codex (architect).
> **Branch:** `feat/phase5-asset-tooling` at `8d60a6d` with the dirty working tree preserved.
> **Nothing staged, committed or pushed.** No UI, generated DTO, registry schema, LZX, head
> conversion, live install, or migration of a live write command. `EditSession::save_to`,
> `Workspace::save_in_place` and `hockey_shop::atomic_write_with_backup` are unchanged.

---

## 1. Result in one paragraph

`crates/tdb-txn` (new, dependency-neutral: std + `sha2` + `serde`/`serde_json` + `thiserror`) is
the shared file-transaction engine: a versioned **change set** binds every destination to an
expected prior identity (length + SHA-256, or *absent*), a replacement file and its identity, the
producer's validation and its (unresolved) qualification; **prepare** validates the set and
rechecks every identity on disk; **apply** stages full intact backups and verified replacement
copies inside `<root>/.tdb-txn/<txn>/`, writes the durable *Applying* record, renames each
replacement over its destination with re-verification, and writes the durable *Committed* record;
**recover** resolves any interrupted transaction from the journal and on-disk identities alone;
**restore** puts the exact originals back and removes originally absent files. `studio-domain::txn`
turns an S2-A staged archive (re-hashed against its report), an `EditSession` save (re-parsed
through the loader) and a companion file into artifacts of one change set. Demonstrated on a
real copied target (`patch.big` with fit + append, the Legacy `nhlng.db` anchor with a real
edit, an absent sidecar): apply → reopen through the existing readers → startup recovery no-op →
restore to the exact original hashes and EOFs. 19 fresh-process kill points (13 apply, 6 restore)
recover deterministically; 12 in-process cases cover refusals, injected failures at every point,
cancellation, locked files, tampering, corrupt/escaping journals and missing backups. Every artifact
stays **unqualified**; copied-target success is recorded as such and qualifies nothing live.

## 2. Interface

```text
tdb_txn::ChangeSet { schema "tdb-txn/changeset/1", id = sha256(canonical json), title, mode: CopiedTarget,
                     artifacts: [Artifact] }
Artifact { key, destination (root-relative, forward slashes, no `..`), expected_prior: Absent | Present{len,sha256},
           replacement (path), replacement_identity, validation { producer, structurally_verified,
           deploy_blockers, qualification { state: qualified|unqualified, items[{subject,status,note}] }, detail },
           dependencies [keys] }
engine::create_copied_target_root(path)         // new dir + COPIED-TARGET.tdb-txn marker; only such roots are accepted
engine::prepare(root, &cs, hooks) -> Transaction // validate + recheck disk; <root>/.tdb-txn/<txn>/{changeset.json,journal.json}; ACTIVE marker
engine::apply(&mut txn, &cs, hooks) -> Outcome   // stage → Applying → replace each → Committed; failure ⇒ Aborted | RolledBack | Blocked
engine::recover(root, hooks) -> [Summary]        // deterministic, idempotent; from journal + files only
engine::restore(root, txn_id, hooks) -> Outcome  // Committed → Restoring → Restored; refuses external edits
engine::status(root) -> [Summary]
Hooks { event(&Event) -> io::Result<()>, cancelled() -> bool }   // progress, fault injection, cancellation
studio_domain::txn::{archive_artifact, session_artifact, companion_artifact, qualification_of}
```

Change-set refusals (before any disk state): empty set; duplicate key; duplicate or **aliased**
destination (case-folded, `./`, `//`, trailing dot/space); `..`, absolute, prefixed or
backslash destinations; unknown dependency; dependency cycle; `structurally_verified = false`; any
`deploy_blockers`; unknown qualification state; tampered id. Prepare refusals: root without the
marker; root busy (an `ACTIVE` transaction); destination not the expected prior / not absent;
replacement not its recorded identity; missing parent directory; a recovery directory on another
volume (`CrossVolume`).

## 3. Journal and durable ordering

`journal.json` (`tdb-txn/journal/1`): `txn_id`, `changeset_id`, `target_root` (informational),
`state`, `applied_unqualified`, per-artifact `{ key, destination, prior, backup?, staged,
replacement, phase }`, `log`. Every write is `journal.json.tmp` → `sync_all` → rename over
`journal.json` → fsync. Paths are relative and validated on read (`<dir>/<file>` only; a destination
that escapes the root is a read error). The recovery directory name is unique
(`<millis>-<changeset prefix>-<pid>[-n]`, `create_dir` refuses reuse); a prior transaction's
backups are never overwritten.

```text
Prepared ──stage: replacement copies + full backups (preallocated, fsynced, identity-verified)──▶ Staged
Staged   ──journal Applying (durable "mutation begins")────────────────────────────────────────▶ Applying
Applying ──per artifact: recheck prior → phase Replacing → rename staged over dest → fsync dest
           → verify identity → phase Replaced ──────────────── all done → journal Committed ──▶ Committed
Committed ──restore: every dest must hold the replacement → journal Restoring → per artifact (reverse):
           phase Reverting → backup copied to restore/<i>.tmp (verified) → rename over dest → fsync →
           verify prior → phase Reverted ───────────────────────────── journal Restored ─────▶ Restored
failure/cancel before Applying ▶ Aborted (nothing outside the recovery dir changed)
failure/cancel after Applying, no commit record ▶ RolledBack (every dest verified back to its prior)
foreign bytes at a dest, or a backup that is not what the journal recorded ▶ Blocked (nothing overwritten;
           the ACTIVE marker stays; the root refuses new transactions until resolved by hand)
```

**Recovery policy (one, deterministic):** `Prepared`/`Staged` → `Aborted`. `Applying` (no commit
record) → every destination inspected: prior → leave; replacement → backup put back; anything else →
`Blocked`; all prior → `RolledBack`. `Committed` → every destination must hold the replacement (or
the prior when the two are identical), else `Blocked`; backups retained. `Restoring` → the restore
is continued. Terminal states are unchanged. A journal that cannot be read blocks the root and is
reported, never deleted. Repeated recovery is a no-op (asserted after every kill point).

**Concurrency:** an exclusively opened `.tdb-txn/lock` (Windows `share_mode(0)`) for the duration
of prepare/apply/recover/restore; a concurrent operation gets `Locked`. The `ACTIVE` marker names
the transaction whose set is not in a terminal state (or is `Blocked`); `prepare` refuses the root
while it exists. Multi-file replacement is not made atomic and is not claimed to be: the journal
plus identities make an interrupted set recoverable, and the marker keeps other Studio operations
off the targets until it is resolved.

## 4. Windows behaviour — tested and assumed

Tested on this host (Windows 11, NTFS, `H:`):

* `std::fs::rename` over an existing destination (`MoveFileExW(MOVEFILE_REPLACE_EXISTING)`)
  replaces it; a destination held open by another handle with `FILE_SHARE_READ` only makes the
  rename fail with a sharing violation — reported as the failing target and rolled back
  (`a_destination_held_open_without_sharing_fails_the_rename_and_rolls_back`).
* `FlushFileBuffers` needs a handle opened for writing: a read-only `File::open` + `sync_all` fails
  with *access denied*. Every fsync in the engine opens read+write.
* Space: every backup and staged copy is preallocated (`set_len`) to its final length before its
  bytes are written; a failing preallocation is reported as the failing target before any
  destination is touched. Injected `StorageFull` failures at every stage/apply point are covered;
  a real disk-full volume was **not** exercised.
* Cross-volume roots are refused by prefix comparison of the canonical root and recovery paths;
  junctions crossing volumes are not specially detected beyond `canonicalize`.

Assumed, **not** qualified: that NTFS's own metadata journaling makes the rename durable once the
replaced file's data was flushed (directory handles cannot be flushed on Windows); anything about
write caches, power loss or hardware. What was exercised is **process interruption** (`process::exit`
at a point, no unwinding) plus injected I/O failures — not power-loss durability.

## 5. Demonstration (real copied target)

```sh
cargo run -p studio-domain --example s2b_copied_target_demo -- --copy-root <new> \
  --archive H:/…/BLUS31540/USRDIR/patch.big --db anchors/ps3/nhllegacy/databases/nhlng.db \
  --meta anchors/ps3/nhllegacy/databases/nhlng-meta.xml --defs defs
```

| Step | Result |
|---|---|
| copied target | `USRDIR/patch.big` 60 596 976 / `d2d26273…`; `USRDIR/databases/nhlng.db` 2 901 504 / `105d2186…` |
| S2-A | entry 139 recompressed → **fit-in-slot**; entry 4 `nhl_eng_us-meta.xml` +4 096 → **append-and-retarget**; staged 60 596 976 → 60 602 048, `Unqualified`, `deployable false` |
| session | `Project::load` → `set_field(ubPc[0].vjla)` → dirty; `session.save()` re-parsed by `Project::reparse_packaged` (94-table inner DB) |
| change set | 3 artifacts, all `unqualified` (S2-A's 10 items carried; session and companion 1 item each); `applied_unqualified = true` in the journal |
| apply | `Prepared → Staged → Applying → patch Replacing/Replaced → nhlng … → sidecar … → Committed`; outputs `5d60def6…` (archive), `18fbb7c7…` (db), 59-byte companion |
| reopen | `BoundSource::bind` on the new archive: 60 602 048 bytes, entry 139 at 16 880 272 (2 022), entry 4 at 60 596 976 (5 058); `Project::load` returns exactly the session's working bytes; companion present |
| startup recovery | `committed set verified intact`; nothing changed |
| restore | `Restoring → sidecar → nhlng → patch → Restored`; `patch.big` `d2d26273…` **original: true**; `nhlng.db` `105d2186…` **original: true**; companion removed |
| source archive | SHA-256 unchanged afterwards |

## 6. Failure and recovery coverage

**In-process (`tdb-txn` lib, 12):** full round trip incl. dependency order, backups, event
order, idempotent restore, second transaction keeps the first's backups · every change-set refusal
above · non-copied root, changed destination, present-but-expected-absent, changed replacement,
missing parent, busy root freed by recovery · **injected I/O failure at 21 points** (stage copies,
backups, `Staged`/`Applying` journal writes before/after, every `BeforeReplace`/`AfterReplace`/
`AfterReplaced`, `BeforeCommitRecord`, `BeforeJournal(Committed)`) → pre-images, terminal journal,
free root, recovery no-op; failure after the commit record → committed set stands and restores ·
cancellation before mutation → `Aborted`; after the first replacement → `RolledBack` · a
destination edited by a third party mid-apply → `a` reverted, `b` untouched, `Blocked`, root refuses
new transactions, recovery leaves it · corrupt backup during rollback and missing backup at restore
→ `Blocked`, nothing overwritten · external edit after commit → restore refuses, recovery blocks ·
truncated journal, escaping destination, escaping backup path, txn-id/directory mismatch → root
blocked, material retained, good journal frees it · Windows locked destination → `RolledBack` ·
root lock excludes a concurrent operation · marker/identity helpers.

**Fresh process (`tests/crash.rs`, 4 tests / 20 kill points):** the child process `exit(9)`s at
`AfterStageReplacement(b)`, `AfterBackup(b)`, `AfterJournal(Staged)`, `AfterJournal(Applying)`,
`AfterReplace(a)` (renamed, not yet journaled), `AfterReplaced(a)` (**first target changed, second
not**), `BeforeReplace(b)`, `AfterReplace(b)`, `AfterReplace(c)`, `BeforeCommitRecord`,
`BeforeJournal(Committed)`, `AfterJournal(Committed)`, `AfterCommitRecord`; the parent reads the
journal state and the destination bytes as left, runs `recover` from a process that never held
the transaction, checks the policy outcome (`Aborted` / `RolledBack` / `Committed` with **exact
hashes** of every old/new target), checks the root is freed and backups retained, runs `recover`
again (identical), and restores committed sets. Restore killed at `AfterJournal(Restoring)`,
`AfterRevert(c)`, `AfterReverted(c)`, `AfterRevert(b)`, `BeforeRevert(a)`, `AfterRevert(a)` →
recovery completes it to the exact originals, repeated recovery and repeated restore are no-ops. A
partially rolled-back `Applying` set (one destination already back) is finished by recovery.

**Domain (`tests/txn_coordination.rs`, 2):** the coordinated archive + session + companion round
trip on a synthetic EB archive and the anchor DB, reopened through `BoundSource::bind` and
`Project::load`; `archive_artifact` refusing a tampered staged output, a changed source and a
verification report that does not cover the output.

## 7. Checks run

| Check | Result |
|---|---|
| `cargo test -p tdb-txn` | lib 12 passed; `crash` 4 passed (20 child processes) |
| `cargo test -p studio-domain --test txn_coordination` | 2 passed |
| `cargo test -p studio-domain --lib`, `--test session`, `--test capacity` | 148 / 1 ignored; 11; 2 — unchanged |
| `cargo test -p tdb-archive-big -p tdb-archive-codec` | 52 / 1 ignored; 45 / 6 ignored — unchanged |
| `cargo build -p studio-api` | builds |
| `cargo clippy -p tdb-txn -p tdb-archive-big -p studio-domain --all-targets -- -D warnings` | clean |
| `rustfmt --check` on every new/changed file | clean |
| demonstration §5 | as recorded; source archive SHA-256 unchanged |

## 8. Files changed

| Path | Change |
|---|---|
| `crates/tdb-txn/{Cargo.toml, src/lib.rs, src/identity.rs, src/changeset.rs, src/journal.rs, src/engine.rs, src/tests.rs, tests/crash.rs}` | **new crate** |
| `Cargo.toml` | workspace member `crates/tdb-txn` |
| `Cargo.lock` | `tdb-txn` package + `studio-domain` edge (all dependencies already in the lock) |
| `crates/studio-domain/src/txn.rs` | **new** — archive / session / companion artifacts |
| `crates/studio-domain/src/project.rs` | `Project::reparse_packaged` (+ `ReparsedDb`) |
| `crates/studio-domain/src/error.rs` | `DomainError::Validation` |
| `crates/studio-domain/src/lib.rs`, `Cargo.toml` | `pub mod txn`; `tdb-txn` dependency |
| `crates/studio-domain/tests/txn_coordination.rs`, `examples/s2b_copied_target_demo.rs` | **new** |
| `crates/tdb-archive-big/src/stage/synth.rs`, `src/stage.rs` | `#[doc(hidden)] pub mod synth` (synthetic 16-byte-row archive builder for tests/demos) |

## 9. Remaining blockers and limits

1. **Nothing is qualified for a live target.** Every artifact carries `unqualified`; the engine
   accepts that only for a root created by `create_copied_target_root`. No live mode exists yet.
2. **Durability is process-interruption tested, not power-loss tested** (§4). Directory-level
   flush is unavailable on Windows; NTFS metadata journaling is assumed, not measured.
3. **Real disk-full / flush hardware failures** were simulated by injection and by preallocation
   semantics, not on a full volume.
4. **A `Blocked` set is resolved by hand** (inspect the retained journal and backups); no
   automated conflict resolution is offered, by design.
5. Old **fit-in-slot** bytes live only in the full backup; restore depends on the backup's recorded
   identity (a corrupt one blocks, never overwrites).
6. S2-A's unknowns are unchanged: header `0x10`, native encoding acceptance, PS4 20-byte rows,
   patch-vs-loose precedence.

## 10. Smallest next integration

Route one existing write through the engine on a copied target end to end **without** touching
its live path: give `Workspace::save_in_place` a sibling `Workspace::stage_change_set(staging_dir)`
that returns the `nhlng` + roster session artifacts **plus** their `content_header` sidecars as
companion artifacts in one change set (today `ensure_content_header` discovers and writes the
sidecar after the save — the dispatch's "do not write a sidecar after commit"), and a Studio-API
command that runs prepare/apply/recover/restore against a copied-target root only. That is the
first consumer of the journal that a UI (Texture Lab, save) can sit on; live-target eligibility
stays a separate, evidence-carrying decision.

---

## 11. S2-B-R1 corrections (2026-09-17, [s2b-review-and-correction-dispatch](../validation/s2b-review-and-correction-dispatch.md))

The shared engine and full-backup architecture are retained; Workspace/API integration stays
parked. Every finding was reproduced by its regression before the fix and passes after it.

### R1 — filesystem containment, not relative spelling

**Policy.** Every path the engine touches goes through one resolver (`contained` /
`contained_dir` in `engine.rs`): the root is canonicalized and must carry the marker as a regular
file; every component from the root to the file's parent is inspected with `symlink_metadata` and
refused if it is a symlink or junction (any reparse point Rust classifies as a link) or not a
directory; the canonical parent must start with the canonical root; the leaf, if present, must be
a regular file; the parent's canonical prefix must be the root's (volume identity on the
**resolved** path — a junction to another volume resolves to another prefix and is refused as an
escape first). This is applied to destinations, `.tdb-txn`, every recovery directory and every
backup/staged/restore/temporary file, the lock, the marker and the sequence file — at prepare,
at staging, **at each rename and each revert**, and during recovery/restore. The engine never
creates a directory outside `.tdb-txn`. Reserved names: a destination may not start with
`.tdb-txn` or name the marker; `:` (drive/device/alternate data stream), Windows device names
(`CON`, `NUL`, `COM1`…, any extension), `<>"|?*`, control characters and UNC/device spellings are
refused. Transaction ids are validated (`[A-Za-z0-9-]`, no leading `-`/`.`, ≤ 80) before being
joined to a path; adapter staging names come only from validated keys (`valid_key`) and are
created with `create_new`.

**Reproduced and fixed.** The review's junction escape (`copy/escape -> outside`, artifact
`escape/a`): prepare now fails with `Escaping`, `outside/a` unchanged. A junction inserted
*after* prepare is refused at staging (`Aborted`); one inserted after the `Applying` record is
refused at the rename, rollback cannot resolve the path either, the set stays `Applying` with the
marker, recovery reports it as a problem **without following the junction**, and once the
junction is removed recovery rolls it back — outside bytes unchanged throughout. A `staged/`
subdirectory replaced by a junction → `Aborted`, nothing written outside; `.tdb-txn` itself as a
junction → prepare and recover refuse. Regressions:
`a_junction_under_the_destination_parent_is_refused_at_prepare_and_at_apply`,
`a_junctioned_recovery_directory_or_txn_dir_is_refused` (Windows, `mklink /J`),
`reserved_device_stream_and_engine_namespace_destinations_are_refused` (12 spellings),
`escaping_transaction_ids_never_reach_the_filesystem`, and in `studio-domain`
`session_artifact_binds_the_loaded_identity…` (keys `../x`, `a/b`, `a\b`, `.hidden`, empty;
no overwrite of an existing staging file).

### R2 — transaction history and restore ordering

**Policy.** `prepare` allocates a per-root `sequence` from `.tdb-txn/SEQUENCE` (temp → fsync →
rename); ids are `NNNNNN-<millis>-<changeset12>` and sort in sequence order. Each journal artifact
records `lineage_prior`: the transaction whose replacement its expected prior was. A destination
is **owned** by the highest-sequence transaction that took it (`Applying`, `Committed`,
`Restoring`, `Blocked`) and has not given it back (`Restored`, `RolledBack`, `Aborted`). Recovery
processes journals newest first and verifies a committed destination only when that transaction
still owns it; a superseded destination is reported (`Summary.superseded = [(destination,
later_txn)]`), never blamed. The marker is recomputed after the pass as the oldest transaction
that blocks the root, or cleared. **Restore order:** `restore(T)` is refused with
`RestoreOutOfOrder { destination, later }` — no state change — while any later owner of one of
its destinations exists; restoring the later transaction first returns the destination to T's
replacement, after which T restores normally. Disjoint later transactions do not block. Prior
backups are never touched.

**Reproduced and fixed.** `a: old → one`, `a: one → two`, recover → both `Committed`, T1
superseded on `a`, root free; repeated recovery identical; `restore(T1)` refused, `restore(T2)`
then `restore(T1)` → `old`, both backups retained. Overlapping sets (`{a,b}`, `{b,c}`, `{a}`):
restore of the disjoint later one allowed early, the rest ordered by ownership. A later
transaction that fails after replacing `a` rolls back and the earlier one owns `a` again;
a crash-shaped `Applying` journal on the later one is rolled back **before** the earlier one is
judged. A foreign edit after two commits blocks the owner (T2), not the history (T1).
Regressions: `successive_commits_on_one_destination_are_legitimate_history`,
`overlapping_and_disjoint_sets_order_restore_by_ownership`,
`a_crash_or_rollback_of_the_later_transaction_leaves_the_earlier_one_intact`,
`a_foreign_edit_blames_the_owner_not_the_superseded_history`.

### R3 — authoritative state under the lock

`apply` now, under the lock: reloads the persisted journal and refuses a handle that differs from
it (`StaleHandle`), requires persisted state `Prepared`, requires the supplied change set to
recompute to its id, match the journal's `changeset_id` **and** equal the persisted
`changeset.json` (`ChangeSetMismatch`), and requires the `ACTIVE` marker to name this transaction
(`NotActive`). `restore` refuses while another transaction holds the marker (`RootBusy`) and never
replaces it. The marker is only ever claimed when absent or already ours, released only when ours,
and recomputed from all journals by recovery. `Transaction::load` binds the journal to the
persisted change set (id, count, per-artifact key/destination/prior/replacement). `Journal::read`
validates semantics: schema, valid txn id, sequence ≥ 1, SHA-256 change-set id, unique keys and
normalized destinations, plain `backups/…` and `staged/…` paths, backup present iff prior
present, phases legal for the state. The `unreachable!` and `expect` paths are gone (a journal
without a backup blocks; a missing change-set key is `ChangeSetMismatch`).

**Reproduced and fixed.** prepare → recover (`Aborted`) → apply the old handle: `StaleHandle`,
file unchanged; with a second transaction prepared in between, T2 keeps the marker; an in-memory
mutated handle is refused; a different or forged change set is refused and the genuine one still
applies; restore while another set is active is `RootBusy` with the marker untouched; six
semantically invalid journals (present prior without backup, duplicate key, duplicate destination,
sequence 0, illegal phase, backup outside `backups/`) and an edited `changeset.json` each block
the root as retained errors — no panic — and the good record frees it. Regressions:
`a_stale_handle_cannot_resurrect…`, `a_different_change_set_cannot_be_applied…`,
`restore_respects_another_active_transaction_and_its_marker`,
`semantically_invalid_journals_are_retained_errors_never_panics`.

### R4 — artifact validation and source binding

`archive_artifact` no longer takes reports: an archive enters only as a `StagedArchive`, a
private-field value that `stage_archive` builds by running the S2-A stager itself (bound source,
plan, paths, reports). The adapter **re-runs `stage::verify`** on the output and source as they
are on disk now, against the recorded plan, and requires the fresh counts and output identity to
equal the staging-time ones. A caller cannot fabricate or mutate the record. `session_artifact`
binds `expected_prior` to `Project::loaded_identity()` — the identity of the file the session
was **opened** on (recorded by `Project::load`) — and refuses a destination that no longer
matches it ("changed since; reopen before saving") without staging anything. Qualification notes
are scoped to the artifact ("no game-verified load of this file on ps3 is recorded"; "whether the
game reads or tolerates this file"). Regressions:
`archive_artifact_reverifies_the_staged_output_against_its_plan` (output tampered → refused,
restored → accepted, source tampered → refused, existing output path refused, bad key refused),
`session_artifact_binds_the_loaded_identity_not_the_current_disk_bytes`. The coordinated
demonstration (§5) reruns unchanged: `Committed` → intact → `Restored`, both originals
byte-identical, source archive unchanged.

### Checks (R1)

| Check | Result |
|---|---|
| `cargo test -p tdb-txn` | lib **25 passed** (12 prior + 13 new); `crash` 4 passed (20 child kill points) |
| `cargo test -p studio-domain --test txn_coordination` | 3 passed |
| `cargo test -p studio-domain --lib`, `--test session`, `--test capacity` | 148 / 1 ignored; 11; 2 — unchanged |
| `cargo build -p studio-api` | builds |
| `cargo clippy -p tdb-txn -p studio-domain -p tdb-archive-big --all-targets -- -D warnings` | clean |
| `rustfmt --check` on every changed file | clean |
| demonstration §5 | rerun: `000001-…` `Committed` → `Restored`, originals `d2d26273…` / `105d2186…` **true** |

### Durability — supported guarantee, stated

What the engine guarantees, on the host tested (Windows 11, NTFS, same volume): after any
**process** interruption at any protocol point, `recover` restores every destination to a state
the journal can name (its pre-image, or the committed replacement) using only on-disk material,
without overwriting bytes it does not own, and repeatedly. What it does **not** guarantee: that a
rename or a journal write survives power loss or a write-cache flush failure — directory entries
cannot be flushed on Windows, NTFS's own journaling is assumed and unmeasured, and no power-loss
or hardware experiment was run. Real disk-full was not exercised (preallocation + injected
`StorageFull` only). Live-target eligibility remains unauthorized; every artifact stays
`unqualified`.

---

## 12. S2-B-R2 corrections (2026-09-17, [s2b-r1-review-and-r2-dispatch](../validation/s2b-r1-review-and-r2-dispatch.md))

The completed R1 corrections are retained; Workspace/API integration stays parked. One bounded
mutation-boundary repair, not a redesign.

### R2 — the mutation boundary

**Reproduced.** The review's data-loss case: commit `a: old -> new`, then `restore` with a hook
that writes `external edit` to `a` at `Event::BeforeRevert`. Before the fix, `revert_all` resolved
the destination and inspected it at the *top* of the loop, then fired `BeforeRevert`, wrote the
`Reverting` journal record and copied the backup, and only then renamed the backup over the old
resolved path — so the foreign `external edit` (written at `BeforeRevert`) was overwritten with
`old` and the call returned `Restored`. Regression
`restore_leaves_a_foreign_edit_made_at_before_revert_and_blocks` asserts the corrected behavior.

**Fix — exact ordering.** Every destination mutation now re-establishes containment and re-reads
the destination's identity **immediately before** the rename/remove syscall, after the hook, the
journal write and the backup preparation:

* `revert_all` (restore, rollback, recovery — one code path): `inspect` (top, to classify
  prior/replacement/foreign) → `BeforeRevert` → `Reverting` journal write → prepare the pre-image
  into `restore/<i>.tmp` and verify it against the recorded prior → **boundary: re-resolve
  `dest` (containment) + re-`inspect`; if it is no longer this transaction's replacement, `block`
  without touching it** → rename `tmp` over `dest` (present) or `remove_file(dest)` (absent) →
  fsync → post-check `inspect` must be the prior. The absent-target *deletion* goes through the
  same boundary, so a foreign file that appeared where the companion was is left, not deleted
  (`restore_of_an_absent_companion_leaves_a_foreign_file_and_blocks`).
* `replace_all` (apply): `BeforeReplace` → `inspect` must be the prior → `Replacing` journal write
  → new `BeforeRename` hook point → **boundary: re-resolve `dest` + re-`inspect`; must still be
  the prior, else `PriorMismatch` before any rename** → rename staged over `dest` → fsync →
  identity must equal the replacement. A parent swapped for a junction *after* the `Replacing`
  write is refused at the boundary re-resolution — the write never follows it out of the root
  (`a_parent_junction_swapped_between_the_replacing_write_and_the_rename_is_refused`); a parent
  swapped for a real sibling directory is caught by the identity re-check
  (`a_parent_swapped_to_a_sibling_directory_at_the_boundary_does_not_redirect_the_write`).

On a boundary conflict the foreign bytes stay, recovery material stays, the journal goes `Blocked`
(or the escaping-junction case is left non-terminal for recovery), the `ACTIVE` marker stays, and
the call returns `Blocked`/`Failed` — never `Restored` or `RolledBack`. Recovery does not resolve
it on its own; the operator restores the replacement (or removes the intruder) by hand.

**Exclusion primitives (measured).** A standalone probe confirmed the Windows rename semantics the
boundary relies on: `std::fs::rename` over an existing destination held open by another handle
**fails** (`Access denied`) unless that handle allows `FILE_SHARE_DELETE` (share modes 5 and 7);
a destination held for read-only or deny-all cannot be renamed over. So a real external editor
holding a file open (no delete sharing) makes our rename fail rather than clobber — the set then
rolls back. We cannot hold the destination open ourselves across our *own* rename (the same rule
would block us), so the boundary re-check, not a held handle, is the primitive; the probe bounds
what the residual window can and cannot reach.

### Regressions added (`crates/tdb-txn/src/tests_r2.rs`, 7)

`restore_leaves_a_foreign_edit_made_at_before_revert_and_blocks` (the exact reproducer),
`restore_of_an_absent_companion_leaves_a_foreign_file_and_blocks` (absent-target deletion),
`an_edit_after_backup_preparation_is_still_caught_at_the_boundary`,
`rollback_during_apply_leaves_a_foreign_edit_at_the_boundary`,
`recovery_applies_the_boundary_rule_to_a_committed_set_edited_during_its_own_revert`
(fresh interrupted-restore journal + edit during recovery),
`a_parent_junction_swapped_between_the_replacing_write_and_the_rename_is_refused` (Windows),
`a_parent_swapped_to_a_sibling_directory_at_the_boundary_does_not_redirect_the_write`. Each asserts
the foreign bytes / outside file are unchanged and the set stays blocked until resolved by hand.
The passing history, stale-handle, containment and crash tests are preserved (the new
`Event::BeforeRename` variant does not affect them).

### Checks (R2)

| Check | Result |
|---|---|
| `cargo test -p tdb-txn` | lib **32 passed** (25 prior + 7 new); `crash` 4 passed (20 child kill points) |
| `cargo test -p studio-domain --test txn_coordination` | 3 passed |
| `cargo test -p studio-domain --lib` | 148 / 1 ignored — unchanged |
| `cargo build -p studio-api` | builds |
| `cargo clippy -p tdb-txn --all-targets -- -D warnings` | clean |
| `rustfmt --check` on every changed file | clean |
| demonstration §5 | rerun: `Committed` -> recovery intact -> `Restored`, originals `d2d26273...` / `105d2186...` **true**, source archive unchanged |

### Windows guarantees, restated by failure class

* **Process interruption** (crash/kill/error at any point): fully handled; recovery restores a
  journal-named state from disk alone, idempotently, without overwriting unowned bytes.
* **External concurrency**: an edit visible up to the pre-syscall boundary check is preserved and
  the set blocked. A residual window remains between that check and the single rename/remove
  syscall — a genuine narrowed TOCTOU, not race-free — bounded on Windows by the share-mode rule
  above (an external writer holding the file makes our rename fail). Documented in the module note.
* **Power loss / write-cache durability**: unqualified. No directory flush on Windows; NTFS
  metadata journaling assumed and unmeasured; no destructive experiment run. Not requested.

Live-target eligibility remains unauthorized; every artifact stays `unqualified`.

