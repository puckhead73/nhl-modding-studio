# C0-SLICE2 — isolated canonical-slice verification

**Package:** C0-SLICE2 · **Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling` ·
**HEAD at start and at end:** `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` (unchanged) ·
**Index at start and at end:** empty

> **Scope of writes.** The only file this package writes inside the repository is this receipt.
> Nothing was staged, committed, pushed, stashed, reset, restored or checked out. No live source,
> configuration, `defs/`, `anchors/`, `app/` or `crates/` file was created, edited or deleted. No
> generator was run against the live tree and no `docs/audits/data/**` file was touched. No game
> data was searched or read, no private fixture root was set, and `Nhl19BigStudio-Tester/` was not
> opened. Every build and test ran inside an isolated scratch tree on another volume.
>
> **This record promotes nothing and decides nothing.** It reports what an isolated candidate did
> when executed. Whether that candidate becomes a commit is the architect's call.

**Dependency acceptance, recorded as instructed:** C0-SLICE1 is treated here as an **accepted
dependency inventory only**. Its proposed commit sequence and its blanket inseparability
conclusions are **not** treated as accepted, and §6 below records four corrections to its boundary
interpretation.

---

## 1. Result in one paragraph

**The canonical slice is a reproducible, self-contained candidate on top of HEAD, and that is now
executed proof rather than a static argument.** A tree built from `git archive` of `a2bbdc4` plus
**25 enumerated overlay files** and **two additive `Cargo.lock` lines** passes
`cargo check -p tdb-canonical --all-targets --locked --offline` (exit 0, no warnings),
`cargo test -p tdb-canonical --locked --offline` (**191 passed, 0 failed, 0 ignored**) and
`cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` (exit 0). It also
passes `cargo fmt -p tdb-canonical -- --check` and, unrequested but run, a full
`cargo check --workspace --all-targets --locked --offline` (exit 0). **No network was used and no
dependency version changed.** The one non-obvious finding is a **pre-existing defect at HEAD, not
caused by this candidate**: HEAD's `Cargo.lock` omits the `toml 0.8.2` dependency that HEAD's own
`crates/studio-domain/Cargo.toml` declares, so `--locked` fails on a **pure HEAD checkout with no
overlay at all** (§4.2, proved with a control tree). Repairing that is one added line and is
required by any `--locked` commit, canonical or otherwise.

---

## 2. Scratch construction

### 2.1 Location, validated before creation

| | |
|---|---|
| Scratch root | `H:\c0-slice2\` |
| Pre-existing? | **No** — confirmed absent before creation. Created fresh by this package |
| Sibling evidence preserved | `H:\c0-baseline`, `H:\c0-bulk1`, `H:\c0-helper-tests`, `H:\c0-integration`, `H:\c0-repair`, `H:\c0-s2-int1` — **none read, written, moved or deleted** |
| Inside the live source tree? | **No.** Different volume from `E:\Repositories\nhl-modding-studio` |
| Free space at start | 59 GB on `H:` |

Layout: `candidate\` (the probe tree), `control\` (pure HEAD, §4.2), `logs\` (all evidence),
`target\` + `target-control\` + `target-clippy\` (build outputs). **`CARGO_TARGET_DIR` was set
explicitly on every invocation**, so no build artefact was written into the candidate's source
directories or anywhere under `E:`. **Logs live outside the candidate tree** and are therefore not
candidate inputs.

### 2.2 Base identity

```
git archive --format=tar HEAD  ->  H:\c0-slice2\base-head.tar
```

| | |
|---|---|
| Commit | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` |
| Tree | `e614f768f5a657dd8dc817044630c81e6b4f6804` |
| Archive SHA-256 | `065c72f132367526c1a4627aca53fd6826dd0050d297fbf3d3581fc8375b006b` |
| Files extracted | **1135** |

The base is the committed tree and nothing else. **The dirty working tree was never copied**; the
25 overlay files were copied individually, by name, from an explicit manifest.

### 2.3 Overlay — exact enumerated files (25)

**No directory was copied wholesale.** Hashes are of the live working-tree bytes, verified
byte-identical to the copies placed in the candidate.

**`crates/tdb-canonical/` — 11 sources**

| File | SHA-256 |
|---|---|
| `Cargo.toml` | `38d7f6a348e24e1844f9f2c36c9a2365568181d444d77cca0c47295bf4303328` |
| `src/asset.rs` | `9ce2be7218577f5491319179b181d7b418f65469970e78916c5bdf2483b04170` |
| `src/asset_tests.rs` | `1d926f92ef75e6a69156aac3b942bd4d861d9cc8fe3a71ecff00cdd8309d5bbd` |
| `src/asset_v1.rs` | `f4219f376013348b68abd9b17a0ff29032010e4bcfeae807e12da74138b92aa2` |
| `src/asset_v2.rs` | `c1593a3434fb66fe7f175d23a6933b6c3b9574978cc38b155ae24a0d5bbd207f` |
| `src/build_override.rs` | `96833f3c9e656035fc64f571778b562e11cb038b62d6a7a90302666f9dccbb5e` |
| `src/build_override_tests.rs` | `3cb34528559a15dc3278f738d03041b00b73f9689e988c24c0b85cabf349cdff` |
| `src/lib.rs` | `22d5dc37e59f0a010651ceabc55f185d338cb89123bf795e7a78cc94ca43493e` |
| `src/model.rs` | `5fed315d7a14ea08fab15630e2ea757be8295fb8ad7c82ea594c8d9a96946a26` |
| `src/tests.rs` | `0662abf72b4dfd7fc8f4459e97d78bf8bc430e85f4ec5634de654ba903460a26` |
| `src/v1.rs` | `53c29f33807d59da64de957f7a555bea3b28f1cd2a1157931c9ce577cb0cbaa0` |

**`crates/tdb-canonical/tests/fixtures/` — 10 bundled fixtures**

| File | SHA-256 |
|---|---|
| `asset-profiles/v1/assets-portraits.toml` | `a436865c011cac57b94d44e0beafd1bbdeb818f24d757ae99db313f06d263a8e` |
| `asset-profiles/v2/assets-portraits.toml` | `13e8d9f61cf93b4ba719eacb1b106f3581729d576193317e7c9187e230234e0a` |
| `build-overrides/synthetic-approved-slot.toml` | `9f89ad98126dc81cda5554b6ff6ef3d76e2ec636d3222a5773f2a0372fb84733` |
| `build-overrides/synthetic-blocked-unknown.toml` | `a4dab400c446f14507342d8bd689fd4cb895d5db9957c0042094fe3d4a4ea567` |
| `build-overrides/synthetic-proposal-only.toml` | `093c642244f50ceca98664b972a1648d2b67e68deeae6a711c9f13163f6ddf2a` |
| `build-overrides/synthetic-refused-codec.toml` | `fecd1dfecfdf2abe434002a694043d636efeb1f0803820e0cd4bc642dc5761ab` |
| `synthetic-adapter.toml` | `fced8e983df7879901ea61a25c22abd600fa8f088c88f8a3a9b6c6131a0a6d4c` |
| `synthetic-asset-profile.toml` | `3d7303a5460de4bd60db136b4a733cfb0af989a98b37370a22ba749ff95c2bc5` |
| `v1/nhl10-xbox360.toml` | `b5395fe50441bdfb7e10370e1b17de217804a7db733030fe4545dea9a8ee05e7` |
| `v1/nhl15-xbox360.toml` | `c7d51275e4de71bc5d95a492c30b8b52f891a35632f7d9fe640b630fcf47c780` |

**`defs/` — 4 required canonical/adapter definitions**

| File | SHA-256 |
|---|---|
| `defs/canonical/project-schema.toml` | `702b8e4fc4c7dc9130dfde137266b6b808fe6018b9eeb4f296ec14f0cacd12ac` |
| `defs/adapters/nhl10-xbox360.toml` | `d8073312d444fdbee1f9bfa9315284209dc3360fd7e47e5a9b2aa1a1ca44607a` |
| `defs/adapters/nhl15-xbox360.toml` | `6bfabff9a0b63d1ae24c75cccf17dc03396501152fd28d4cd5a9532ab3337528` |
| `defs/adapters/asset-profiles/assets-portraits.toml` | `425927254a41093758a3ca23575584691c78fd809c98d9e2a6f0323a59323ee1` |

These four are **required, not optional**: `src/tests.rs` and `src/asset_tests.rs` read them from
the repository root via `env!("CARGO_MANIFEST_DIR")/../..` and assert the committed documents
validate clean. Without them the suite fails at load, not silently.

### 2.4 Mixed-file hunks applied surgically (2)

Neither file was copied from the working tree; both were patched in place on the archived HEAD copy.

**`Cargo.toml`** — one line, exactly the `@@ -6,6 +6,7 @@` hunk C0-SLICE1 §S2 identified:

```diff
     "crates/tdb-defs",
+    "crates/tdb-canonical",
```

The working tree's *second* `Cargo.toml` hunk — `"crates/tdb-headmesh"` and `"crates/blender-job"`
— was **excluded**. The split applies cleanly; C0-SLICE1's static claim that these hunks are
separable is confirmed by execution.

**`Cargo.lock`** — see §4.3. Two added lines, no removals, no version changes.

### 2.5 Exclusions — everything in the dirty tree that did **not** enter the candidate

| Excluded | Why |
|---|---|
| `crates/tdb-headmesh/**`, `crates/blender-job/**` and their `Cargo.toml` member lines | Phase-5 feature work; not a canonical input |
| Their `Cargo.lock` entries, plus `zune-core` / `zune-jpeg` (new `image` deps) | Feature-side; **the candidate needs no new registry package at all** |
| `crates/tdb-qa/src/evidence/**`, `build.rs`, `bin/qa_evidence.rs`, `tests/evidence_*.rs`, `defs/qa/scopes.toml` | C0 evidence system (C0-SLICE1 S3). `tdb-canonical` references **no** `evidence::` item — proved by the candidate compiling without them |
| The `pub mod evidence;` line in `crates/tdb-qa/src/lib.rs` | Same; `tdb-qa` was used **at HEAD, unmodified** |
| `crates/studio-api/src/{bindings.rs,bin/export_bindings.rs}` and the `studio-api` DTO changes (C0-DT1) | Not reachable from `tdb-canonical` |
| `crates/tdb-rx2/src/fixtures.rs` and the C0-FX1 changes | Not reachable from `tdb-canonical` |
| All 147 C0-INT1 formatting/lint repairs | Not required; the candidate is `fmt`- and `clippy`-clean at its own boundary **without** them |
| `crates/studio-domain/**`, `app/**`, `src-tauri/**`, all documentation, `docs/audits/data/**` | Out of slice |
| `anchors/**`, `corpus/**`, any mounted game tree, `Nhl19BigStudio-Tester/` | Not read. The candidate's `anchors/` is HEAD's committed copy, untouched and unopened by any check run |

### 2.6 Build/test hooks inspected before execution

| Checked | Finding |
|---|---|
| `.cargo/config.toml` anywhere in candidate | **None** — no registry override, no runner, no rustflags |
| `build.rs` anywhere in candidate `crates/` | **None at HEAD.** (`tdb-qa/build.rs` is untracked C0 evidence-system work and was excluded) |
| `rust-toolchain` / `rust-toolchain.toml` | **None** — no toolchain pin, no implicit download |
| Every file access in `tdb-canonical` sources | 6 sites, all `env!("CARGO_MANIFEST_DIR")`-rooted: `../../defs/canonical/`, `../../defs/adapters/`, and its own `tests/fixtures/`. **No environment-variable fixture root, no absolute path, no game-tree scan** |
| Cross-crate coupling | Exactly one: `src/asset_tests.rs:576` reads `tdb_qa::catalog::FAMILIES` (dev-dependency only) |
| Network | `CARGO_NET_OFFLINE=true` plus `--offline` on every invocation |

---

## 3. Toolchain

`cargo 1.96.0 (30a34c682 2026-05-25)` · `rustc 1.96.0 (ac68faa20 2026-05-25)` ·
`clippy 0.1.96 (ac68faa20c 2026-05-25)`. Host `x86_64-pc-windows-msvc`.

---

## 4. Executed verification

### 4.1 First locked run — failed, evidence retained

```
cargo check -p tdb-canonical --all-targets --locked --offline
error: cannot update the lock file H:\c0-slice2\candidate\Cargo.lock
       because --locked was passed to prevent this
EXIT=101
```

(`logs\01-check-locked-unmodified.log`, taken before any lockfile edit.)

An attempt to let cargo re-resolve the lock offline (`cargo metadata --offline`) failed on
`android_system_properties v0.1.5` not being in the local registry cache, and rewrote the whole
lockfile's line endings as a side effect. **That branch was abandoned and the lockfile restored
from the archived HEAD copy** — restoration verified by SHA-256 equality with `logs\Cargo.lock.base`.
No broad re-resolution was accepted and no dependency upgrade was taken.

### 4.2 The failure is **pre-existing at HEAD**, proved by a control tree

A second tree, `H:\c0-slice2\control\`, was extracted from the **same archive with no overlay of any
kind** — pure `a2bbdc4`:

```
cargo check -p tdb-qa --all-targets --locked --offline     (control, pure HEAD)
error: cannot update the lock file ... because --locked was passed
EXIT=101
```

So `--locked` fails on committed HEAD before this candidate exists. Re-running **without**
`--locked` on the control tree succeeded (exit 0) and revealed the entire delta cargo wanted:

```diff
 [[package]]
 name = "studio-domain"
 ...
  "tdb-savedata",
+ "toml 0.8.2",
 ]
```

**Root cause, newly identified here:** HEAD's `crates/studio-domain/Cargo.toml:25` declares
`toml = "0.8"`, and HEAD's `Cargo.lock` does not record it in `studio-domain`'s dependency list.
The committed lockfile is stale with respect to the committed manifest. `toml 0.8.2` is **already
present in the lockfile as a package**, so the repair adds one line and pulls in nothing new.

This sharpens C0-INT1 §7.1's bare note that "`--locked` fails at HEAD" into a named, one-line cause.
It is a **C0.2-class committed baseline defect**, it is **not** caused by, specific to, or avoidable
by the canonical slice, and **any** `--locked` commit on this branch inherits it.

### 4.3 Lockfile delta — the complete, final change (2 added lines)

```diff
@@ -3373,6 +3373,7 @@                        <- pre-existing HEAD repair (§4.2)
   "tdb-rx2",
   "tdb-savedata",
+  "toml 0.8.2",
  ]

@@ -3808,6 +3809,16 @@                       <- the candidate's own entry
 [[package]]
+name = "tdb-canonical"
+version = "0.0.0"
+dependencies = [
+ "serde",
+ "sha2",
+ "tdb-qa",
+ "toml 0.8.2",
+]
+
+[[package]]
 name = "tdb-core"
```

Why each change is needed:

| Change | Justification |
|---|---|
| `studio-domain` gains `"toml 0.8.2"` | Records a dependency HEAD's own manifest already declares. **Required by HEAD, not by this slice.** Without it, no `--locked` command runs at all |
| new `[[package]] tdb-canonical` | The new workspace member. Its four dependencies — `serde`, `sha2`, `tdb-qa`, `toml 0.8.2` — are **all already pinned in HEAD's lockfile**; each was verified present before editing |

**No package version was added, removed, upgraded or downgraded.** No new registry entry, no new
checksum, no network access. Both edits preserve the file's CRLF line endings, so the diff is two
inserted lines and nothing else.

### 4.4 Locked verification after the adjustment

All run in `H:\c0-slice2\candidate\`, with `CARGO_NET_OFFLINE=true` and an explicit external
`CARGO_TARGET_DIR`:

| # | Command | Exit | Result |
|---|---|---|---|
| 1 | `cargo check -p tdb-canonical --all-targets --locked --offline` | **0** | Clean. **Zero warnings**, zero diagnostics from `tdb-canonical` |
| 2 | `cargo test -p tdb-canonical --locked --offline` | **0** | **191 passed · 0 failed · 0 ignored · 0 filtered.** Unit 191, integration 0, doc-tests 0 |
| 3 | `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | **0** | Clean under `-D warnings` |
| 4 | `cargo fmt -p tdb-canonical -- --check` *(supplementary)* | **0** | Formatted |
| 5 | `cargo check --workspace --all-targets --locked --offline` *(supplementary)* | **0** | Whole workspace, including `src-tauri` and `studio-api`, checks with the member added |

Logs: `logs\10-check.log`, `11-test.log`, `12-clippy.log`, `13-fmt.log`, `14-workspace.log`.

The 191 figure is **exactly** the sum of the crate's `#[test]` attributes — `src/tests.rs` 83,
`src/asset_tests.rs` 64, `src/build_override_tests.rs` 44 — so every declared case ran; nothing was
filtered and nothing returned early on a missing fixture. C0-SLICE1 correctly flagged the
previously-recorded 191 as a full-dirty-tree figure and therefore not evidence at this boundary;
**it is now reproduced at this boundary.**

### 4.5 Dependency diagnostics vs canonical diagnostics, kept separate

| Source | Diagnostics |
|---|---|
| **Dependency / pre-existing HEAD** | One: the stale `Cargo.lock` of §4.2, blocking every `--locked` command on unmodified HEAD. Attributable to HEAD; resolved in the scratch candidate only |
| **`tdb-canonical` itself** | **None.** Zero compiler warnings, zero clippy findings under `-D warnings`, zero rustfmt differences, zero test failures |
| Registry / network | None. No download was attempted or required after §4.1's abandoned branch |

### 4.6 What was **not** verified

- **No full-workspace `test` run.** Run 5 is `check` only. Nothing here says the workspace's tests pass.
- **No workspace-wide `clippy` or `fmt`.** Both are known to fail repo-wide at HEAD (C0.2, open) and
  were deliberately not re-measured. Runs 3 and 4 are **scoped to `tdb-canonical`** and are not a
  statement about any other crate.
- **Nothing is game-verified.** Every fixture is synthetic or a committed `defs/` document. No anchor
  was parsed and no game data was read.
- **Linux/CI behaviour is unverified.** This ran on Windows only; CI is `ubuntu-latest` (Windows CI
  is C0.2 R6, open).
- **No claim about any other candidate slice.** S1, S3, C0-FX1, C0-DT1 and C0-INT1 remain exactly as
  unverified as C0-SLICE1 left them.

---

## 5. Preservation evidence

### 5.1 Candidate inputs, before and after execution

`sha256sum -c` of all **25** overlay files against the pre-run manifest, re-run after all five
check/test/clippy/fmt invocations: **25 OK, 0 failed.** Membership re-counted:
`crates/tdb-canonical/` holds **21** files, unchanged. No build wrote into candidate sources —
`CARGO_TARGET_DIR` was external on every run.

`Cargo.lock` SHA-256 after the final run equals its value after §4.3
(`bdf69054d375414a7fad385bdf75558a5898226b0ce86a1f742aac05b7088b79`), confirming `--locked` never
mutated it.

**Base integrity, whole tree.** `tar -df base-head.tar` run inside the candidate after every check
compares all **1135** archived members against the tree on disk:

| Difference kind | Count | Meaning |
|---|---|---|
| `Contents differ` | **0** | **No archived file's bytes changed** |
| `Size differs` | **2** | `Cargo.toml`, `Cargo.lock` — the two intentional patches of §2.4 / §4.3 |
| `Mod time differs` | 2 | The same two files |
| `Mode differs` / `Uid differs` / `Gid differs` | 1474 / 1135 / 1135 | Extraction metadata on Windows; not content |

So the candidate is provably HEAD plus exactly the 25 additions and the 2 patched files, and nothing
else in it moved during execution (`logs\24-tar-compare.txt`).

### 5.2 Live repository — untouched

| Check | Before | After |
|---|---|---|
| `git rev-parse HEAD` | `a2bbdc4f…` | `a2bbdc4f…` — **unchanged** |
| Index (`git diff --cached`) | empty | **empty** |
| `git status --porcelain` | recorded | **byte-identical** to the recorded copy |
| The 25 live overlay sources | hashed pre-copy | `sha256sum -c`: **all 25 unchanged** |

The only in-repository change made by this package is the creation of this file.

### 5.3 Prior evidence preserved

Nothing under `H:\c0-baseline`, `H:\c0-bulk1`, `H:\c0-helper-tests`, `H:\c0-integration`,
`H:\c0-repair` or `H:\c0-s2-int1` was read, modified or removed. `H:\c0-slice2\` and all its logs
are retained for review; **no scratch material was deleted.**

---

## 6. Corrections to C0-SLICE1's boundary interpretation

C0-SLICE1 is not rewritten. These four corrections supersede the corresponding readings of it.

**6.1 Interleaved hunks mean reconstruction is needed, not proven impossibility.**
C0-SLICE1 §3.5 says of `crates/studio-domain/src/lib.rs` that the region "cannot be hunk-separated",
and the receipt generalises from that toward blanket inseparability. The supported statement is
narrower: *a mechanical patch-hunk split does not exist there.* A separated version can be
**reconstructed** — by writing the module list as it should read with only the C0-INT1 re-sort
applied, and adding the feature declarations in a later commit. That is authoring work and a change
of substance, correctly outside C0-SLICE1's permissions; but "requires reconstruction" and
"impossible" are different findings and only the first is supported. This package demonstrates the
same distinction constructively at a smaller scale: C0-SLICE1 called the `Cargo.toml` members hunk
separable as a static claim, and §2.4 here **executed** that split successfully.

**6.2 Current DTO roots require feature DTOs; broader coupling remains to be demonstrated.**
C0-SLICE1 §3.4 records that C0-DT1's `bindings.rs` names 8 export roots absent at HEAD, and calls
C0-DT1 "hard-blocked by feature work". What is established is that **the drift gate as currently
written** enumerates feature DTOs. That is a property of the present export-root list, not a proof
that the gate mechanism is inseparable from Phase-5. Whether a C0-only gate exists — over HEAD's
export roots, with the feature roots added alongside their DTOs — is **undemonstrated in either
direction** and should be stated as open, not as a blocker. Nothing in this package tests it.

**6.3 C0-DT1 cannot belong to two commits.**
C0-SLICE1 assigns `crates/studio-api/src/{bindings.rs,bin/export_bindings.rs}` to the C0-only file
class in §2 (the 98-file table) while §4/§7 place the same C0-DT1 work behind feature commits in the
proposed sequence. A file belongs to exactly one commit. The proposed sequence is therefore not
internally consistent as written and must be resolved before any ordering is adopted. This package
takes no position on which placement is correct.

**6.4 A dependency relationship is not acceptance of the feature.**
That C0-DT1's current form references Phase-5 DTOs, and that the working tree's `Cargo.toml` and
`Cargo.lock` carry `tdb-headmesh` and `blender-job` beside `tdb-canonical`, says **nothing** about
whether those features are accepted, in scope, or ready. C0-SLICE1's dependency inventory must not
be read as promoting any Phase-5 feature toward acceptance. This candidate deliberately excludes
every one of them (§2.5) and is unaffected by their status.

---

## 7. Static conclusions vs executed proof

**Executed proof — measured at this exact boundary, logs retained**

1. HEAD + the 25 enumerated files + the 2 lock lines **compiles, tests and lints clean** for
   `tdb-canonical` (§4.4 runs 1–4).
2. The workspace-member `Cargo.toml` hunk **splits cleanly** and applies to archived HEAD (§2.4).
3. **191 of 191** canonical cases pass in isolation — no fixture missing, no case skipped (§4.4).
4. `tdb-canonical` needs **no** C0 evidence system, **no** `studio-domain`, **no** `studio-api`,
   **no** `tdb-rx2` change and **no** C0-INT1 repair — proved by their absence from a passing tree.
5. The candidate introduces **no new third-party dependency** (§4.3).
6. `--locked` fails on **pure, unmodified HEAD**; cause identified as the missing `toml 0.8.2` entry
   for `studio-domain` (§4.2, control tree).
7. The whole workspace still **checks** with the member added (§4.4 run 5).
8. The candidate is **exactly** HEAD plus 25 additions and 2 patched files — zero content drift
   across the other 1133 archived members, before and after execution (§5.1).

**Static conclusions — reasoned, not executed**

1. Because the crate's only file reads are manifest-relative to `defs/` and its own fixtures, the
   slice should be location-independent. *Inferred from source inspection; only this one layout ran.*
2. The `Cargo.lock` hunk split C0-SLICE1 proposed is *implied* correct by §4.3 producing a sufficient
   lock, but the working tree's exact lock bytes were never applied — the candidate's lock was
   derived independently. A commit taking the working tree's lock verbatim would also carry
   `blender-job`, `tdb-headmesh`, `zune-*` and `studio-domain`'s `tdb-qa` entry, and is **not** what
   was verified.
3. The `studio-domain` lock repair is *expected* to be harmless to other slices, since it adds an
   already-pinned package to an existing dependency list. *Not separately proved.*
4. Nothing here bears on Linux or CI (§4.6).

---

## 8. Readiness for commit review

**Ready for the architect to review as a commit candidate**, with one dependency stated plainly.

| Question | Answer |
|---|---|
| Reproducible from HEAD without the other uncommitted features? | **Yes — executed.** 25 files + 2 lock lines |
| Self-contained inputs? | **Yes.** All test inputs are in-slice; the only external symbol, `tdb_qa::catalog::FAMILIES`, exists at HEAD |
| Leaves a misleading artefact behind? | **No.** Touches neither `app/src/bindings/` nor `docs/audits/data/**`; runs no generator; adds no documentation claim about unshipped machinery |
| Adds third-party surface? | **No** |
| Clean under the repo's own gates at its boundary? | **Yes** — `check`, `test`, `clippy -D warnings`, `fmt` all exit 0 |
| Blocked on anything? | **One thing:** the `Cargo.lock` repair of §4.2 |

**The one decision for the architect.** The `studio-domain` `"toml 0.8.2"` line is a **pre-existing
HEAD defect**, not canonical work, and this package holds no authority to place it. Either it lands
as a separate one-line lockfile-repair commit *before* the canonical commit — the honest attribution,
and the option this receipt would recommend if asked — or it rides along inside the canonical commit,
which mixes an unrelated baseline repair into a C0 slice. **Without one of the two, no `--locked`
command on this branch runs**, canonical commit or not.

Beyond that, this candidate is verified at its own boundary. It is **not** a claim that the
workspace's tests pass, that any other slice is separable, or that any commit order is correct.

---

## 9. Unresolved questions for the architect

1. Where does the `studio-domain` lockfile repair land (§8)? Should it also be recorded as a new
   C0.2 baseline-defect entry — it is not in `docs/validation/c0-02-baseline-execution.md` today?
2. §6.3: which commit owns `crates/studio-api/src/{bindings.rs,bin/export_bindings.rs}`?
3. Should the C0-DT1 export-root question of §6.2 be probed the way this package probed canonical,
   before any sequence treats it as blocked?
4. Is `H:\c0-slice2\` retained as durable evidence, and should this receipt be mirrored into
   `docs/validation/` once a boundary is accepted?

---

## 10. Package boundaries observed

Scratch construction and verification only. No live source or configuration change, no staging, no
commit, no push, no generator against the live tree, no game-data search, no tester access, no
feature implementation, no fixture-root environment variable, and no deletion of prior material.
**Stopping here for architect review.**

---

## 11. Corrections — appended by C0-LOCK1 (2026-09-09)

Two corrections raised by the architect on acceptance of this receipt. **Every other finding above
stands unchanged**; in particular §7's executed proof, the 191/191 result, the exclusion list and
the four C0-SLICE1 corrections in §6 are unaffected.

**11.1 The lockfile delta is two change *regions*, totalling 11 added lines — not "two added lines".**

§1, §2.4, §4.3, §7 and §8 above describe the candidate's lockfile delta as "two additive lines" or
"2 added lines". That conflates change **regions** with **lines** and understates the size. The
accurate statement:

| Region | Added lines | Attribution |
|---|---|---|
| `studio-domain` gains `"toml 0.8.2"` | **1** | Pre-existing HEAD baseline defect (§4.2) |
| new `[[package]] tdb-canonical` block | **10** | The canonical slice itself |
| **Total** | **11** | |

The ten lines are the block's `name`, `version`, `dependencies = [`, its four dependency entries,
the closing `]`, the separating blank line and the following `[[package]]` marker — exactly as the
diff in §4.3 renders them. Read that diff, not the prose, as authoritative. Nothing else changes:
still two regions, still zero deletions, still no version added, removed or altered, still no new
registry package and no network.

The baseline line of region 1 has since been committed on its own as
`0a791892dcfef11d079e873c2444d5e70bf29262` — see
[c0-lock1-baseline-lockfile-repair.md](c0-lock1-baseline-lockfile-repair.md). The canonical slice's
remaining lockfile delta is therefore the **ten-line** package entry alone, and §8's "one thing"
blocker is resolved. **This does not authorize the canonical commit**, which remains unmade.

**11.2 Distinguish the scoped warning-free checks from the workspace check, which did warn.**

§4.4's table reports run 5 — the supplementary
`cargo check --workspace --all-targets --locked --offline` — as "exit 0" without saying whether it
was clean. It was not silent. It exited 0 **with warnings**:

| Source | Warnings | Origin |
|---|---|---|
| `tdb-rx2` (lib) | 1 | `TAG_FX_RENDERABLE` never used — HEAD code |
| `tdb-rx2` (lib test) | 2 (1 duplicate) | incl. non-camel-case `tdb_rx2_vertex` — HEAD code |
| `studio-domain` (test `nhl07_encodings`) | 1 | unused import `PlayerKind` — HEAD code |
| `studio-domain` (test `read_matrix`) | 1 | unneeded `mut` — HEAD code |
| `studio-domain` (lib test) | 1 | unused variable `year` — HEAD code |
| **`tdb-canonical`** | **0** | — |

All of them are pre-existing HEAD diagnostics in crates outside this slice; **`tdb-canonical`
contributes none**. So §4.4 runs 1–4 and §4.5's "zero warnings" claim remain correct exactly as
scoped — they are `-p tdb-canonical` results — and §4.5's separation of dependency from canonical
diagnostics holds. What must not be inferred from run 5 is that the workspace is warning-clean; it
is not, and §4.6 already declined to make workspace-wide lint claims.
