# C0.2 — Isolated Windows baseline execution

> **Package:** C0.2 (roadmap [C0](../plans/project-completion-roadmap.md) item 2).
> **Revision:** C0.2-R1, 2026-09-07 — the reproduction helper was rewritten to enforce the
> guarantees C0.2 claimed for it, and overstated claims in this report were corrected. The
> historical execution evidence (logs, manifests, hashes, results) is unchanged; see section 2.4
> for what R1 did and did not alter.
> **Input:** the accepted [C0.1 baseline inventory](c0-01-baseline-inventory.md). Its section 5
> snapshot plan and section 6 architect decisions were followed; the deviations and limitations
> that arose are listed in section 2.4 rather than being claimed away.
> **Type:** snapshot construction and check execution. No production source was repaired, no
> generated evidence was refreshed, no game file was written, no QA status was promoted.
> **Execution date:** 2026-09-07 (UTC). **Author role:** developer. Reviewer: Codex (architect).

Conventions: **Passed / Failed / Blocked / Not run** describe a check that actually executed (or
did not) inside the snapshot. A test binary reporting `ok` for a case that returned early before
reaching any assertion is recorded as **skipped coverage**, never as a pass. Where only *part* of a
case was disabled, that is said explicitly rather than written off as a whole-test skip.

---

## 1. Authorized commit

The index was verified empty before staging. Only the C0.1 report was staged; the staged diff was
inspected (`A docs/validation/c0-01-baseline-inventory.md`, 493 insertions) and
`git diff --cached --check` returned clean (exit 0).

| Item | Value |
|---|---|
| Commit | `16825ca64cc70d510855eecfacc52a2b6b17c23f` |
| Subject | `docs(validation): record reviewed C0 baseline inventory` |
| Files | `docs/validation/c0-01-baseline-inventory.md` (new, 493 lines) — nothing else |
| Prior HEAD | `01c8444ed464c27a850a9198aab94826869d5717` |
| Push | **Not performed.** Not authorized. |

The commit was created, its message was found to have been mangled by a shell-quoting error, and it
was amended in place before anything else happened; `16825ca` is the only commit this package
produced and the amended message is the intended one. No other commit, and no staging of any other
path at any point.

**This HEAD was recorded before snapshotting** and is part of the snapshot identity below.

---

## 2. Snapshot identity

| Item | Value |
|---|---|
| Snapshot source | `H:\c0-baseline\c0-02-20260907-085022\source` |
| Evidence directory | `H:\c0-baseline\c0-02-20260907-085022\evidence` |
| Manifest | `H:\c0-baseline\c0-02-20260907-085022\evidence\manifests\snapshot-manifest.txt` |
| **Manifest SHA-256** | **`8bf776f6da8d1a2a523c67a9174502143a2718d9ed3177f9b4b94f08229f9acc`** |
| Git HEAD | `16825ca64cc70d510855eecfacc52a2b6b17c23f` (branch `feat/phase5-asset-tooling`) |
| Members | **1,222 files, 722 MB** — 1,129 tracked + 93 untracked-allowlist |
| Excluded untracked paths recorded | 391 |
| Link/reparse exceptions | **0** |

**Snapshot identity is the manifest hash together with the HEAD reference.** HEAD alone does not
identify it: the snapshot deliberately contains uncommitted bytes HEAD does not describe.

Volume note: the snapshot lives on `H:` (120 GB free), not `E:`. `E:` had **11.3 GB free**, which is
not enough for 722 MB of source plus the ~18k-file Rust `target/` the checks generate. That is a
machine constraint, not a repository property.

### 2.1 Membership and exclusions

The C0.1 section 5.1 allowlist was applied byte-for-byte and produced exactly the set C0.1
enumerated — including `crates/studio-domain/tests/zz_diag.rs` and
`app/src/assets/ui/tabs/builds.svg`, both included per the architect's decision. **Their inclusion
here authorizes nothing about committing them.**

The **accepted C0.1 report is included explicitly**, as a *tracked* member
(`docs/validation/c0-01-baseline-inventory.md`), because its commit succeeded before the snapshot
was taken.

Excluded and **left untouched in the live tree**: `.git/`, `target/`, `**/target/`,
`src-tauri/gen/`, `node_modules/`, `app/node_modules/`, `app/dist/`, `Staging/`, all of `corpus/`,
`Nhl19BigStudio-Tester/`, `Nhl19BigStudio-Tester.zip`, and every untracked path off the allowlist.
All 391 matched-but-excluded paths are listed in the manifest, so a reviewer sees what was left out
rather than inferring it.

### 2.2 Integrity procedure and results

| Step | Result |
|---|---|
| In-progress-git guard (`index.lock`, rebase, merge, cherry-pick) — before and after | clean |
| Link/reparse detection over every selected path, plus the repo's whole tree and its parent chain (`E:\`, `E:\Repositories`, repo root) via PowerShell `-Attributes ReparsePoint` | **none found** |
| Missing members | 0 |
| Pre-hash live (SHA-256 + size + mtime) | 1,222 |
| Copy — raw bytes via a `tar --format=posix` stream, **no newline conversion**, no links/hardlinks/junctions to live inputs | exit 0 |
| Post-hash the copies and compare to source | **1,222 / 1,222 identical, 0 mismatches** |
| Re-enumerate live membership after copying (tracked + untracked) | stable |
| Re-hash live sources after copying | **identical to pre-copy — no torn copy** |

`core.autocrlf=true` and there is no `* text=auto`, so git-derived content is not a substitute for
the working-tree bytes; the copy is a byte stream and never touched line endings in either
direction.

### 2.3 Reproduction

> **The historical snapshot above was NOT produced by the helper described here.** It was built on
> 2026-09-07 by an ad-hoc Git Bash sequence, and the manifest hash
> `8bf776f6…` identifies *that* artefact. The helper below is the C0.2-R1 rewrite, written so a
> future run enforces the same guarantees automatically instead of relying on the operator. Its
> manifest layout differs from the historical one, so re-running it will not reproduce
> `8bf776f6…` byte-for-byte; it reproduces the same **selection and verification**, not the same
> manifest file.

`docs/validation/c0-02-snapshot.ps1` rebuilds and re-verifies a snapshot from a repository root and
a new destination. It is read-only against the repository and runs no build or test check.
Manifests live in a sibling `evidence/` directory, **never inside the snapshot source**, so they
cannot alter what a later run hashes.

```
powershell -NoProfile -ExecutionPolicy Bypass -File docs\validation\c0-02-snapshot.ps1 `
    -RepoRoot E:\Repositories\nhl-modding-studio -Destination H:\c0-baseline\<new-dir>
```

It is self-contained PowerShell — no Git Bash — because the Windows-specific guarantees (reparse
resolution, canonical path comparison, long paths) cannot be enforced from Git Bash, whose `-L`
test does not see junctions. Each of the following is **fatal, before or during the run, and never
deferred to a manual step**:

| Guarantee | How it is enforced |
|---|---|
| Destination is new, outside the repository, and not an alias into it | Both paths canonicalised via `GetFinalPathNameByHandle` through the nearest existing ancestor, then segment-aware containment checks in both directions — all before any directory is created |
| Source is a real git working-tree root | `git rev-parse --show-toplevel`, canonicalised and compared |
| No reparse point in the selection | Every member, every ancestor directory inside the repo, and the repo's own parent chain are attribute-scanned |
| Every member exists and is readable | Each is opened for read before hashing |
| Filenames survive the copy | NUL-delimited enumeration; paths with control/reserved characters, relative segments, trailing space-or-dot or device names are rejected **before** any copy rather than mangled |
| Raw bytes | Per-file `[IO.File]::Copy`; no archive, no newline conversion in either direction |
| Exclusions apply at any depth | Build/dependency rules match nested `node_modules/`, `target/`, `dist/` as well as top-level ones; an exclusion that would drop a **tracked** path is a fatal rule conflict, never applied silently |
| Nothing moved under the copy | After copying: git guards repeated, HEAD re-checked, **both** tracked and allowlisted-untracked membership re-enumerated, and every member's live bytes re-hashed |
| Failure is visible | Any failure exits non-zero, leaves partial output in place marked `SNAPSHOT-INCOMPLETE.txt`, and never prints `SNAPSHOT-OK` |
| Provenance is in the artefact | The manifest records the rule sets, exclusions, per-file size and both hashes, HEAD, branch, helper path and its own SHA-256, invocation, host, tool versions — so the snapshot is explicable without shell history |

The helper's selection rules are transcribed from C0.1 section 5.1 unchanged and were **not**
broadened. Re-evaluating them against the live tree on 2026-09-07 yields the same **1,222 members
(1,129 tracked + 93 untracked-allowlist)** as the historical run, with 0 tracked paths dropped by
exclusions.

`docs/validation/c0-02-snapshot.tests.ps1` verifies the above on small synthetic repositories in a
temporary directory. It never touches this repository and never builds an application snapshot.
Results in section 2.5.

**Snapshot construction proves isolation and provenance only.** It proves nothing about
buildability; every claim in section 4 rests on a command that actually ran inside the snapshot.

### 2.4 Deviations and limitations of the historical run

Recorded rather than claimed away.

| # | Deviation / limitation |
|---|---|
| D1 | **The snapshot was built by an ad-hoc script, not by a reviewed helper.** The steps were the C0.1 plan's, but the guarantees rested on the operator performing them in order. The C0.2-R1 helper exists because that is not a reproducible control. |
| D2 | **The reparse scan was a separate manual PowerShell command**, not part of the copy path, so a link appearing between the scan and the copy would not have been caught. The scan did return clean for the repository, its whole tree and its parent chain. |
| D3 | **The Git Bash copy used `tar`.** No member path required special handling (0 paths with control or reserved characters; 309 with spaces, all copied and re-hashed identically), but nothing rejected an unsupported filename in advance — the run would have discovered it as a failure, or not at all. |
| D4 | **Exclusions were expressed as an untracked-allowlist complement**, not as depth-independent rules. It produced the right result here (no tracked path lies under a build or dependency directory) but did not enforce it. |
| D5 | **Only tracked membership was compared after copying** in the original sequence; allowlisted untracked membership was re-listed but the comparison was reported as advisory. R1 makes both comparisons fatal. |
| D6 | **The snapshot was placed on `H:` rather than `E:`** because `E:` had 11.3 GB free — not enough for 722 MB of source plus the Rust `target/`. This is a machine constraint. |
| D7 | **The authorized commit needed an in-place amend** after a shell-quoting error mangled its message. Only the message was affected; the tree and the single staged path were correct throughout. |
| D8 | **The manifest's per-file table was assembled by a helper script whose intermediate files live in the evidence directory.** The manifest is self-describing for review, but the historical run's provenance is not recorded *inside* the manifest the way R1 records it. |

### 2.5 C0.2-R1 helper verification

`powershell -NoProfile -ExecutionPolicy Bypass -File docs\validation\c0-02-snapshot.tests.ps1`
— **10 passed, 0 failed, 0 skipped** (work root `H:\c0-helper-tests\run4`).

| Case | Asserted |
|---|---|
| T1 happy-path | Paths with spaces, non-ASCII (`src/pägès/Ünï cöde.tsx`), a 256-byte binary and **both** CRLF and LF files copy byte-exact with no newline conversion; the untracked allowlist is honoured; `corpus/`, top-level `node_modules/`, **nested** `app/src/nested/node_modules/` and **nested** `crates/tdb-headmesh/target/` are all excluded; manifest and hash sidecar are written outside the snapshot source |
| T2 existing-destination | Rejected, exit 1, no `SNAPSHOT-OK`; the destination's sentinel file is byte-identical afterwards and the entry count is unchanged |
| T3 destination-inside-source | Rejected **before any output directory is created**; the repository's entry count is unchanged |
| T3b destination-alias-via-junction | A destination reached through a junction that resolves into the repository is rejected before output — path-string checks alone would have passed it |
| T4 reparse-in-selection | A junction planted inside an allowlisted directory is detected and rejected **before copying** |
| T5 missing-input | A tracked file deleted from the working tree fails the run before output, with no success marker |
| T5b locked-input | A member held under an exclusive lock fails the run before output |
| T6 membership-drift | An allowlisted untracked file created *during* the snapshot (via the documented `-TestHookAfterCopy` verification hook) is caught by the post-copy re-enumeration; partial output is marked `SNAPSHOT-INCOMPLETE.txt` |
| T7 content-drift | A member's bytes changed *during* the snapshot are caught by the post-copy re-hash and reported as a torn snapshot; partial output is marked incomplete |
| T8 not-a-repository | A plain directory is rejected before output |

T6 and T7 use the helper's `-TestHookAfterCopy` parameter, which runs a caller-supplied command
between the copy and the re-verification. That makes the two drift cases deterministic instead of
racing the copier. The hook is recorded in the manifest whenever it is used, so a snapshot produced
with it can never be mistaken for a baseline.

**Prerequisites:** junction creation (`mklink /J`) succeeded here, so T3b and T4 ran rather than
skipping. On a host where it is unavailable both would report SKIP, and the reparse guarantee would
be unverified for that run.

**Not covered by these tests:** the helper has never been run end-to-end against the full
application repository — deliberately, since C0.2-R1 must not build another application snapshot.
Its rule evaluation against the live tree was checked (same 1,222 members) but its copy, hashing and
re-verification path at 722 MB scale is unexercised.

---

## 3. Pre-execution inspection of hooks and external writes

### 3.1 First-party hooks

- The only workspace build script is `src-tauri/build.rs` calling `tauri_build::build()`. It writes
  into **`src-tauri/gen/schemas/`, inside the source tree**, confirming C0.1 section 4.1. Four files
  appeared there during the run (`acl-manifests.json`, `capabilities.json`, `desktop-schema.json`,
  `windows-schema.json`).
- Neither `package.json` declares any `pre*`/`post*`/`prepare` script. `.git/hooks` holds only
  samples; there is no `.husky/`.
- DTO generation is the `#[cfg(test)] mod bindings` / `fn export_typescript` test at
  [crates/studio-api/src/lib.rs:152-161](../../crates/studio-api/src/lib.rs#L152-L161). Its target is
  `concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src/bindings")`, so inside the snapshot it writes
  to the snapshot's bindings, never to the live tree.
- The QA matrix tests resolve their CSVs from `env!("CARGO_MANIFEST_DIR")`, likewise snapshot-local.

### 3.2 Environment cleared in every child process

`QA_WRITE`, `QA_ASSET_OUT`, `QA_CENSUS_OUT`, `QA_CENSUS_MANIFEST`, `QA_CENSUS_SAMPLE`, `FHM_CORPUS`,
`NHLMS_ONLY`, `BOOT_TARGET`, `LZX_DEBUG`, `TS_RS_EXPORT_DIR`. None was set in the parent shell to
begin with; all were explicitly unset for every command anyway.

### 3.3 Hardcoded external paths — a filesystem copy is not a sandbox

33 distinct hardcoded absolute paths exist across 11 source files, rooted at:

| Root | Files | Present on this machine? |
|---|---|---|
| `X:\Games\Emulation\Roms\XBOX360\...` (NHL 07–15, Legacy) | `studio-domain/src/{assets,equipment_texture,head_catalog,head_preview,kit_preview}.rs`, `tdb-archive-big/src/eb.rs`, `tdb-rx2/src/{gnf,texlib}.rs` | **yes — mounted** |
| `H:\Emulators\ShadPS4\Games\CUSA*` (PS4) | `studio-domain/src/{assets,discovery,extract,head_preview}.rs`, `tdb-rx2/src/{gnf,mcd}.rs` | **yes — mounted** |
| `H:\Emulators\games\Decompiled\Original\...` | `tdb-rx2/src/texlib.rs` | mounted, but the named file is absent |
| `E:\Repositories\nhl-database-studio\Working Assets\...` | `tdb-rx2/src/texlib.rs` | repo present, named files absent |

**A copied tree is not a sandbox.** It relocates the *sources* the tests compile from; it does not
confine what those tests then open, read or write, because the paths above are absolute and are
resolved at runtime against the real filesystem. Nothing in this package's setup would have stopped
a write to `X:` or `H:\Emulators`.

What was actually done instead, and what it does and does not establish:

- **Inspected write behaviour (before execution).** Every write site reachable from a test that
  names one of those roots was read. Each resolves its destination to either `QA_ASSET_OUT` — left
  unset, so those cases return before writing — or `std::env::temp_dir()`. No inspected write site
  derives its destination from a game-tree root. This is a source reading, not a runtime
  guarantee: it does not cover writes reached through indirection this inspection missed.
- **Endpoint verification (after execution).** All 552 included anchor files and all 1,222 snapshot
  members re-hashed unchanged (section 5). That is strong evidence for those endpoints, and no
  evidence at all about the live game trees on `X:` and `H:\Emulators`, whose contents were **not**
  hashed before the run and therefore cannot be shown to be unmodified.

On that basis the tests were allowed to run. The exclusions in section 4.4 are therefore *fixture
unavailability* and *deliberately disabled probes*, not a safety exclusion imposed by this package.

The isolation limitation is that **27 test cases read live, private game data outside the snapshot
during this baseline** (section 4.5), and that a hypothetical unnoticed write to those trees would
not have been detected by any check performed here.

Two hygiene observations for the architect, not repaired here:

- `crates/tdb-rx2/src/texlib.rs` hardcodes fixture paths inside
  `E:\Repositories\nhl-database-studio` — the prior application's repository, which `CLAUDE.md`
  hard rule 1 places off limits. Those cases skipped (the files are absent), but the dependency is
  written into the source.
- Two temp artefacts survived the run: `H:\TEMP\bj_hang_9844` and `H:\TEMP\p1_rebuilt.big`.

---

## 4. Check results

Bounded concurrency, recorded: `CARGO_BUILD_JOBS=8` and `RUST_TEST_THREADS=8` on a 16-logical-core
machine. Working directory is the snapshot root unless stated. Log paths below are relative to
`H:\c0-baseline\c0-02-20260907-085022\evidence\`; their SHA-256 values are in
`EVIDENCE-INDEX.sha256` in that directory.

| # | Check | Dir | Exit | Duration | Result | Log |
|---|---|---|---|---|---|---|
| 1 | `npm.cmd ci` | `source/app` | 0 | 22.8 s | **Passed** — 212 packages from the tracked lockfile | `logs/02-npm-ci.log` |
| 2 | `npm.cmd run build` (`tsc && vite build`) | `source/app` | 0 | 14 s | **Passed** — 794 modules, built in 6.56 s | `logs/03-npm-build.log` |
| 3 | `npm.cmd test` (`vitest run`) | `source/app` | 0 | 2 s | **Passed** — 4 files, **33/33** | `logs/04-npm-test.log` |
| 4 | `cargo fmt --all -- --check` | root | **1** | 2 s | **Failed** — 1,231 hunks / 122 files / 19 crates | `logs/05-cargo-fmt.log` |
| 5 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | root | **101** | 22 s | **Failed** — 12 lint errors in `tdb-archive-codec`; the build stops there | `logs/06-cargo-clippy.log` |
| 5b | `cargo clippy --workspace --all-targets --locked --keep-going` (enumeration; warnings **not** denied) | root | **101** | 70 s | **Failed** — 174 warning diagnostics at 158 unique spans **plus one deny-by-default correctness error** (accounting in 4.2) | `logs/07-cargo-clippy-enumerate.log` |
| 6 | `cargo test --workspace --locked` | root | 0 | **1,859 s** | **Passed** — **667 passed, 0 failed, 7 ignored** | `logs/08-cargo-test.log` |
| 7 | Mount-gated coverage probe, `--nocapture` (parallel, then `--test-threads=1`) | root | 0 | 50 s + 45 s | Diagnostic — see 4.4 | `logs/09-…`, `logs/10-…` |

Checks 1–3 ran **before** any Rust test, against the bindings exactly as they exist in the working
tree, per architect decision 3.

### 4.1 Defect B1 — `cargo fmt --all -- --check` fails repo-wide (blocks the CI fmt step)

Evidence: `logs/05-cargo-fmt.log`; per-file counts in `manifests/fmt-affected-files.txt`.

**1,231 diff hunks across 122 files in 19 of the 20 crates** (only `tdb-defs` is clean). There is
**no `rustfmt.toml` or `.rustfmt.toml`** anywhere in the repository, so this is stock rustfmt 1.9.0
against code written to a wider line budget.

Largest contributors: `studio-domain` 725 hunks, `studio-api` 169, `tdb-edit` 101,
`tdb-fhm-build` 55, `tdb-qa` 38. Worst single files: `crates/studio-domain/src/assets.rs` (115),
`crates/studio-api/src/registry.rs` (70), `crates/tdb-fhm-build/src/bin/convert.rs` (48).

**Not confined to the uncommitted work.** Files the uncommitted work never touched fail too — e.g.
`crates/tdb-core/tests/alloc.rs` (19 hunks), `crates/tdb-edit/tests/field_ops.rs` (43). That
establishes the failure is **not solely** a product of the uncommitted changes. It does **not**
establish the origin of every hunk: hunks inside the 60 modified files could come from either the
new lines or the pre-existing ones, and this package did not attribute them.

`.github/workflows/ci.yml` runs exactly this command as its first Rust step, so a hosted run of that
workflow against this working tree would fail at that step. **No hosted CI run was observed**, and
this report makes no claim about whether the workflow has ever run on this branch or what its
historical status was.

Representative excerpt:

```
Diff in ...\crates\blender-job\src\bundle.rs:50:
-    let exe = if cfg!(windows) { "blender.exe" } else { "blender" };
+    let exe = if cfg!(windows) {
+        "blender.exe"
+    } else {
+        "blender"
+    };
```

### 4.2 Defect B2 — clippy fails with `-D warnings` (blocks the CI clippy step)

The CI-equivalent command stops at the first crate to exceed the deny gate:

```
error: could not compile `tdb-archive-codec` (lib) due to 12 previous errors
error: manual checked division             crates\tdb-archive-codec\src\lib.rs:278:23
error: manually reimplementing `div_ceil`  crates\tdb-archive-codec\src\lib.rs:281:9
error: manually reimplementing `div_ceil`  crates\tdb-archive-codec\src\dds.rs:53,54,58,59
error: this looks like you are swapping `r1`/`r2` and `r0` manually
error: manual `RangeInclusive::contains` implementation   (x4)
```

Because that halts the run, check 5 alone does **not** measure the baseline. The enumeration pass
(5b) does. Its accounting, stated precisely rather than as a single headline number:

| Quantity | Value |
|---|---|
| Lines beginning `warning: ` in the log | 208 |
| — of which per-target summary lines (`` `crate` (lib test) generated N warnings ``) | 33 |
| — of which `build failed, waiting for other jobs` | 1 |
| **Individual warning diagnostics** | **174** |
| Duplicate re-emissions clippy suppressed across lib / lib-test targets | 104 |
| Unique `(file, line)` primary spans behind those diagnostics | 158 |

**The earlier "208 warnings" figure counted the summary lines as warnings; it is corrected here.**
174 is the number of printed diagnostics, and even that is not 174 independent defects: clippy emits
a crate's lib warnings again for its lib-test target and reports 104 such re-emissions as
duplicates, and several diagnostics share a source line, leaving **158 unique source locations**.

By unique span, the concentration is `tdb-rx2` (57), `studio-domain` (39), `tdb-headmesh` (36),
`tdb-archive-codec` (13), `tdb-fhm-build` (6), then single digits elsewhere. Most frequent lints:
`unnecessary_cast` (8), `type_complexity` (5), `needless_range_loop` (5), `unnecessary_sort_by` (4),
`manual_is_multiple_of` (4), `manual_div_ceil` (4), `doc_lazy_continuation` (4).

Several of the lints involved (`manual_checked_ops`, `manual_div_ceil`, `manual_is_multiple_of`)
are recent additions or recent broadenings in clippy. That makes **toolchain drift a plausible
hypothesis** for much of this surface — but it is only a hypothesis. Demonstrating it would require
running an older clippy against the same tree, which this package did not do, and would not
authorize a different repair either way.

### 4.3 Defect B3 — a vacuously-true assertion (correctness of evidence, medium)

Surfaced by the enumeration pass as a **deny-by-default** clippy error, i.e. it fails clippy even
with no `-D warnings`:

```
error: this boolean expression contains a logic bug
   --> crates\studio-domain\tests\read_matrix.rs:305:25
305 | assert!(!arena.name.is_empty() || arena.city.is_empty() || true, "arena ref built");
    |         help: it would look like the following: `true`
    = note: `#[deny(clippy::overly_complex_bool_expr)]` on by default
```

The trailing `|| true` makes that expression unconditionally true. **This one assertion checks
nothing**, and has presumably checked nothing for as long as it has existed.

Scope, stated precisely: the enclosing test is **not** vacuous. The same loop body runs
`assert_profile_ok`, `assert_all_fields_editable_honest`, and real assertions on absent strategies
and draft picks, all of which did execute and pass. What is unproven is specifically the
**arena-reference** claim that line was written to make. `cargo test` uses rustc rather than clippy,
so the case still reports `ok` in check 6; the green result at
[crates/studio-domain/tests/read_matrix.rs:305](../../crates/studio-domain/tests/read_matrix.rs#L305)
is not evidence about arena references, while the rest of that test's coverage stands. Severity:
medium (misleading evidence, not a wrong edit). It also shows clippy's failure surface here is not
limited to style.

### 4.4 Cases that reported `ok` without exercising their subject — 15 skipped + 2 disabled

`cargo test` captures test output, so check 6's log contains **no** `skipping:` lines. Re-running
the affected crates with `--nocapture --test-threads=1` (`logs/10-mountgated-serial.log`) shows the
real picture. **None of these is a fixture pass.** Two distinct situations are involved and are kept
apart:

**(a) Missing fixtures — 15 cases.** Each gate is an early return placed before the first
assertion, so the case exercised nothing at all.

| Missing fixture | Cases reported `ok` without exercising it |
|---|---|
| `template player_20500.rx2` (path not in-repo) | `tdb-rx2` `container::round_trip_template_byte_identical`, `pack::substitute_head_round_trips_geometry_weights_and_uvs`, `pack::template_lists_all_five_mesh_handles`, `playermesh::parses_template_into_five_submeshes`, `playermesh::patch_round_trips_positions_and_palette_on_template` |
| `E:\Repositories\nhl-database-studio\…\texlib_67.rx2` | `texlib::make_descriptor_matches_hair_256_dxt3`, `texlib::make_descriptor_matches_head_512_dxt1`, `texlib::replace_with_resize_grows_earlier_slot`, `texlib::replace_with_resize_same_dims_round_trip` |
| `E:\Repositories\nhl-database-studio\…\centerlogo_0_cm.rx2` | `texlib::make_descriptor_matches_real_centerlogo_1024` |
| `H:\Emulators\games\Decompiled\Original\…\texlib_67.rx2` | `texlib::parses_real_texlib_67`, `texlib::replace_same_size_round_trip` |
| `playerheadssmall_p1512.xpr2` | `xpr::parses_real_portrait_geometry` |
| bundled helmet / glove mesh | `parses_helmet_mesh_smoke`, `parses_glove_mesh_smoke` |

So the whole of `tdb-rx2`'s **texlib descriptor construction and resize/replace round-trip** surface
and its **XPR2 portrait geometry** surface are unproven by this baseline, as is `playermesh`/`pack`
**head-substitution round-tripping against a real template**. That is a substantial hole hidden
behind a green suite.

**(b) Deliberately disabled write probes — 2 cases**, and these are a different thing entirely.
`assets::legacy_x360_boot_write_probe` and `assets::legacy_x360_boot_checksum_probe` are gated
*twice*: first on the real `boot.big`, which **is** present and was read successfully, then on
`QA_ASSET_OUT`, which this package deliberately left unset. They are not missing-fixture skips —
their fixture was available and they were switched off on purpose, because their whole function is
to emit an artefact for manual loading in an emulator. Enabling them is a C9 manual-QA decision, not
a baseline gap.

Separately **`#[ignore]`d and therefore not run** (7): `head_census::run_census`,
`ea_match_key_coverage`, `formula_real::computes_ratings_for_a_real_player`,
`import_corpus_e2e::corpus_person_imports_into_anchor`, `import_history_cost`,
`eb::chunklzx_decodes_against_extracted_oracle`, and one `studio-api` doc-test. Per the brief none
was enabled. The two corpus-dependent ones could not have run regardless: `corpus/` is excluded from
the snapshot by architect decision 2.

### 4.5 Live private game data was read during the baseline

27 cases executed against the mounted trees on `X:` and `H:\Emulators\ShadPS4`. They were allowed to
run because section 3.3 established that none writes there, and all 552 anchors plus all 1,222
snapshot members were verified unchanged afterwards. Examples, with the evidence they printed:

```
nhl09_full_catalog_sweeps_generic_and_staff_heads ... NHL09 full catalog: 949 entries
    (player=253, generic=652, coach=44); heads=506, hair=443, failures=0
legacy_full_catalog_names_retired_orphan_heads    ... Legacy retired orphans named: 66
phase1_file_truth_recovers_divergent_id_heads     ... nhl08: 2962 players - 165 own-head ...
ps4_gnf_head_write_roundtrip_real                 ... 1024x1024 BC1, red patch round-tripped
ps4_real_shadps4_detects_all_installed_titles     ... nhl15: nhlng=true extract_needed=false
```

**This is the principal isolation limitation of C0.2**, and it is a reproducibility problem, not
merely a hygiene one: on a machine without those mounts the same suite still returns exit 0 with 27
fewer cases actually testing anything, and says so only under `--nocapture`.

### 4.6 The five named matrices — actual execution

All five ran inside the full suite (check 6); results are reused, not re-derived.

| Matrix | Test | Result | Time |
|---|---|---|---|
| `mutation_matrix` | `mutation_matrix_over_all_anchors` | **ok** | 28.6 s |
| `operation_matrix` | `operation_matrix_over_all_anchors` | **ok** | ~80 s |
| `save_matrix` | `save_matrix_over_all_fixtures` | **ok** | 66.4 s |
| `surface_matrix` | `surface_matrix_matches_committed` | **ok** | 2.6 s |
| headmesh `matrix_guard` | 6 tests incl. `promoted_pairs_match_the_capability_csvs`, `appearance_defs_load_and_self_validate` | **6/6 ok** | 0.03 s |

`QA_WRITE` was unset throughout, so each asserted its committed CSV rather than rewriting it. The
sibling headmesh guards also passed: `cc5_defs_guard` 9/9, `cc5_authored_pipeline` 6/6 — confirming
the 11 untracked `defs/appearance/*.toml` (C0.1 gap G6) are sufficient once included in the
snapshot.

### 4.7 DTO bindings — no drift

| Point | SHA-256 of the 124-line binding hash list |
|---|---|
| Original working-tree bindings (before any Rust test) | `c73811c3de9856e905994664f1e5d714051ef2e679d353f8a96c58ce32de3c2e` |
| After `cargo test --workspace` ran `export_typescript` | `c73811c3de9856e905994664f1e5d714051ef2e679d353f8a96c58ce32de3c2e` |

**All 124 binding files are byte-identical before and after generation.** The uncommitted
`PlayerAttributesDto.ts` modification and the nine untracked DTO files are exactly what the current
Rust DTOs emit — which supports, without proving, C0.1's assumption in section 4.4 about their
origin.

Consequently there is **no regenerated-bindings variant to report separately**: the regenerated
state *is* the original state, so the check 1–3 results stand for both, and the single frontend
re-run the brief allows was unnecessary and was not performed. `studio-api` was never omitted from
coverage — it compiled and tested inside check 6, which is what produced the bindings.

This does **not** retire C0.1 defect 4.4: `cargo test` remains simultaneously the generator and the
gate, with no `--check` mode and no drift assertion. Today's zero delta is a property of a
synchronized tree, not a guarantee.

---

## 5. Before/after manifests, anchors, and live-tree verification

Three full snapshot states were captured — **A** before the frontend checks, **B** after them,
**C** after all Rust execution.

**What these three states do and do not prove.** They bracket *groups* of commands, not every
individual command: A→B spans `npm ci`, `npm run build` and `npm test` together, and B→C spans
`cargo fmt`, both clippy passes, the full Rust suite and the two `--nocapture` probes together. They
therefore establish **endpoint preservation** — that every member file held identical bytes at each
boundary — and not the absence of transient changes in between. A file written and restored, or
written and later overwritten with its original content, within a group would not appear here. The
per-command before/after manifest C0.1 section 4.1 asked for was **not** implemented; this is a
recorded limitation, not a substitute for it.

| Comparison | Member content (1,222 files) | New paths |
|---|---|---|
| A → B | **no change** | 10,871 — `app/node_modules/**` (10,816), `app/dist/**` (55) |
| A → C | **no change** | 29,085 — `target/**` (18,210), `app/node_modules/**` (10,816), `app/dist/**` (55), **`src-tauri/gen/**` (4)** |

The `state-A`, `state-B` and `state-C` member-hash files are **identical** (all
`d78851f27709a7c2b2b2667b53d13afd74efa5d4fca68e500eaa92464a16dc67`). Nothing was deleted. Every
generated path is an expected build output; the only one written **inside the source tree** is
`src-tauri/gen/schemas/` (4 files), exactly as predicted in section 3.1.

**Anchor re-verification (complete, not a spot check):** all **552** anchor files were re-hashed
after execution and compared against the manifest — **all unchanged**. No anchor mutation.

**Live-tree verification, at the end of the package:**

| Item | Result |
|---|---|
| HEAD | `16825ca…` — unchanged since the commit |
| Staged paths | 0 |
| Live bytes of all 1,222 member files | **identical to the pre-snapshot hashes** |
| Live untracked set | unchanged except this package's own outputs under `docs/validation/` (the report, and — added by C0.2-R1 — `c0-02-snapshot.ps1` and `c0-02-snapshot.tests.ps1`; the original `c0-02-snapshot.sh` was authored and then retired by C0.2-R1) |
| `target/`, `node_modules/`, `app/node_modules/`, `app/dist/`, `src-tauri/gen/`, `Staging/`, `corpus/`, `Nhl19BigStudio-Tester/`, `Nhl19BigStudio-Tester.zip` | all still present, untouched |

No external concurrent change to any live input was observed at any checkpoint (pre-copy, post-copy,
mid-run, end).

---

## 6. Tool versions, prerequisites and dependency sources

| Item | Value |
|---|---|
| cargo / rustc | 1.96.0 (`30a34c682` / `ac68faa20`, 2026-05-25), `stable-x86_64-pc-windows-msvc` |
| rustfmt / clippy | 1.9.0-stable / 0.1.96 |
| node / npm | v24.16.0 / 11.13.0 |
| git | 2.54.0.windows.1 |
| OS | Windows 11 Pro 10.0.26200, 16 logical cores |
| WebView2 runtime | present |
| MSVC linker | `cl.exe` not on `PATH`; the toolchain nonetheless linked every crate — **not a blocker in practice** |
| Blender | **not on `PATH`** — see the qualification below (C0.1 gap G8) |
| npm registry | reachable (`npm ping` PONG 128 ms) |

Dependencies: `npm ci` installed **only** what the tracked `app/package-lock.json` defines, inside
the snapshot. No upgrade, no lockfile change, no global install; live `node_modules` and build
outputs were never copied in. Rust dependencies resolved under `--locked` from the local cargo
registry cache with no lockfile change. This also **closes C0.1 section 3.1's open question**: the
workspace does resolve its full dependency graph and does compile.

**Blender, qualified.** `where blender` found nothing, so Blender is not on `PATH`. That is **not**
proof it is absent from the machine: `crates/blender-job/src/bundle.rs` resolves an executable from
(1) an explicit Settings-configured path or (2) a managed install at
`<managed_root>/blender/4.5/blender.exe` — **neither of which consults `PATH`**. Whether either
location holds a Blender 4.5 was not checked, and checking it is outside this package's scope.

What the run does establish is narrower and still useful: **no default test failed for want of
Blender**, and no default test invoked it (nothing in the logs reports a Blender resolution or
launch). C0.1's assumption under gap G8 — that no default test requires Blender — is therefore
supported by this run, while the question of whether Blender 4.5 is installed anywhere on this
machine remains open.

---

## 7. Baseline defect summary

| ID | Severity | Location | Statement |
|---|---|---|---|
| B1 | High (blocks the CI fmt step) | 122 files, 19 crates; worst `crates/studio-domain/src/assets.rs`, `crates/studio-api/src/registry.rs` | `cargo fmt --all -- --check` fails with 1,231 hunks. No `rustfmt.toml` exists. Files the uncommitted work never touched are affected, so it is not solely a product of that work; per-hunk origin was not attributed. |
| B2 | High (blocks the CI clippy step) | `crates/tdb-archive-codec/src/{lib,dds}.rs` first; 174 diagnostics at 158 unique spans overall | `cargo clippy … -D warnings` fails. Recent-clippy toolchain drift is a plausible but undemonstrated explanation for much of it. |
| B3 | Medium (correctness of evidence) | [crates/studio-domain/tests/read_matrix.rs:305](../../crates/studio-domain/tests/read_matrix.rs#L305) | One `assert!(… or true)` is vacuously true, so the arena-reference claim is unproven while the enclosing test's other assertions did run and pass. |
| B4 | Medium (evidence integrity) | `tdb-rx2` (13 cases), `studio-domain` (2) | 15 cases report `ok` while returning early on an absent fixture; texlib descriptor/resize, XPR2 geometry and template head-substitution are unproven. (The 2 boot probes are excluded from this count — they are deliberately disabled, not missing a fixture; see 4.4(b).) |
| B5 | Medium (reproducibility + clean-room) | 11 files, 33 hardcoded paths; `tdb-rx2/src/texlib.rs` points into `E:\Repositories\nhl-database-studio` | Test fixtures are addressed by absolute machine paths, including the prior app's repo (`CLAUDE.md` hard rule 1). Results are machine-dependent, and silently so. |
| B6 | Low | `docs/audits/qa-ledger.md` domain (carried from C0.1 D9/D10) | Unchanged by this package; owned by C0.4. |
| B7 | Low | `H:\TEMP\bj_hang_9844`, `H:\TEMP\p1_rebuilt.big` | Tests leave temp artefacts behind. |

**No defect above was fixed, and no input was regenerated or repaired to make a check pass.**

---

## 8. Recommended bounded repair groups (not implemented)

Proposed only; sequencing and acceptance are the architect's.

1. **R1 — formatting baseline (B1).** Decide whether to adopt a `rustfmt.toml` matching the existing
   width or to accept stock rustfmt, then apply one mechanical formatting commit. One crate at a
   time keeps the diff reviewable. Must land before any CI gate can be meaningful.
2. **R2 — clippy baseline (B2).** Mechanical lint fixes crate by crate, smallest first
   (`tdb-archive-codec` → `tdb-fhm-build`/`tdb-fhm-store`/`studio-api` → `studio-domain` →
   `tdb-rx2` → `tdb-headmesh`). Keep separate from R1 so a formatting diff never hides a semantic
   one. `manual_swap` and `manual_checked_ops` in codec paths deserve reading, not a blind `--fix`.
3. **R3 — the vacuous assertion (B3).** Decide what `read_matrix.rs:305` was meant to assert and
   either assert it or delete the case. Do not simply drop the `|| true` — that changes what passes.
4. **R4 — fixture addressing (B4, B5).** Replace hardcoded absolute paths with the explicit
   fixture register C0 item 5 already calls for, and make an unavailable fixture **fail or report
   loudly** rather than `eprintln!` under captured output. Removing the `nhl-database-studio`
   dependency is a clean-room requirement, not a preference.
5. **R5 — DTO drift gate.** Add a `--check` mode, or a separate assertion that emitted bindings
   match committed ones, so C0.1 4.4 stops depending on the tree happening to be synchronized.
6. **R6 — Windows CI (Q05, C0.1 decision 9).** A later C0 package. This report supplies the command
   set, tool versions, the ~31-minute suite duration and the bounded concurrency it ran under.

---

## 9. What was not done

No push. No commit other than `16825ca`. No source repair, no formatting or lint fix, no dependency
upgrade or lockfile change, no global install. No `qa-inventory`, no `QA_WRITE` refresh, no
`edit-qa`, no corpus generator — their determinism and merge work stays with C0.4. **No `#[ignore]`d
test was enabled, and no test that was gated off was switched on** — in particular `QA_ASSET_OUT`
was left unset, so the two boot write probes stayed disabled. This is *not* a claim that no test
touched private data: 27 default-enabled cases did read live game data on `X:` and
`H:\Emulators\ShadPS4` (sections 3.3 and 4.5), which is recorded as this baseline's principal
isolation limitation. No game file was written and no QA status was promoted. No
scope or feature decision was taken; the questions raised in sections 3.3, 4.3 and 8 are referred to
Codex and the user. No canonical document was rewritten. `zz_diag.rs` and `builds.svg` were included
in the snapshot and nowhere else; their commit disposition remains open. The snapshot and its
evidence directory are preserved for review and were not deleted.
