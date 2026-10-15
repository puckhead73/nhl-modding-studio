# C0-CAN-COMMIT1 — commit the verified canonical slice

**Package:** C0-CAN-COMMIT1 · **Date:** 2026-09-09 · **Author role:** developer · Reviewer: Codex (architect)
**Branch:** `feat/phase5-asset-tooling`

| | |
|---|---|
| **Result** | **BLOCKED — no commit was made.** Verification of the exact staged tree failed |
| **Commit created** | **none** |
| HEAD at start and at end | `0a791892dcfef11d079e873c2444d5e70bf29262` (unchanged) |
| Index at start and at end | empty |
| Blocker | `cargo test -p tdb-canonical --locked --offline` → **exit 101, 184 passed / 7 failed** on the staged tree |
| Cause | The crate's tests assert on `\n`-joined fixture substrings. Committed fixtures are LF blobs that check out as **CRLF** under this repository's `core.autocrlf=true`, so the assertions cannot match |
| Pushed? | No. Nothing was committed, so nothing could be pushed |

> The slice was **not** committed, **not** repaired and **not** broadened. Evidence is retained at
> `H:\c0-can-commit1\`. The index was returned to empty with `git reset` (mixed), which does not
> touch working-tree files; all 27 live paths are byte-identical to their pre-package hashes.

---

## 1. Result in one paragraph

Staging was constructed exactly as authorized — 25 files added by explicit path, plus two
index-only mixed-file changes built from HEAD's own blobs — and the staged diff is precisely the
**27 authorized paths, 0 deletions**, with `Cargo.toml +1/-0` and `Cargo.lock +10/-0`. But the
exported staged tree **fails its own test suite**: 7 of 191 cases in `build_override_tests.rs`
panic. The cause is the newline accounting this package was told to perform. C0-SLICE2 tested
**working-tree bytes**, and 23 of the 25 files are **LF on disk**; git normalizes them to LF blobs
and this Windows checkout expands them back to **CRLF**, so the staged tree's fixtures differ from
the tested ones by line endings. The crate's test helpers assert `text.contains("…\n…")` against
fixture text, and those literals cannot match CRLF content. **`cargo test -p tdb-canonical` therefore
fails on any fresh Windows clone of the repository as it would have been committed** — while
`clippy`, `fmt` and the full workspace `check` all pass. Committing would have landed a crate whose
test suite is broken on the project's required initial desktop platform, so per the package rules
the commit was withheld.

---

## 2. Preflight

| Check | Required | Observed | Verdict |
|---|---|---|---|
| HEAD | `0a791892dcfef11d079e873c2444d5e70bf29262` | `0a791892dcfef11d079e873c2444d5e70bf29262` | **match** |
| Index | empty | 0 staged paths | **empty** |
| 25 live files vs C0-SLICE2 §2.3 hashes | all match | `sha256sum -c` → **25 OK, 0 failed** | **match** |
| `Cargo.toml` SHA-256 | recorded | `5272dcbddaaeb834e1defbb659f090becc6eca5167f63c512c9481b849459e82` | recorded |
| `Cargo.lock` SHA-256 | recorded | `7d0c4876be54ba5bb486a76d49417322a4c8eb11422b72928841d7bed8e3ba6d` | recorded |
| Worktree | recorded | **242** `git status --porcelain` entries | recorded |
| Newline configuration | — | `core.autocrlf=true`; `.gitattributes` covers only `anchors/**` binaries, **none of the 27 paths** | recorded |

HEAD blobs before staging: `Cargo.toml` `9d52f2c5…`, `Cargo.lock` `953c3370…`.

---

## 3. Staging — exactly the authorized 27

**The 25 files** were staged one at a time by explicit path from the C0-SLICE2 manifest
(`git add -- "$f"` in a loop). **No directory argument, no `git add -A`, no pathspec glob.**

**The two mixed files** were never staged from their dirty working-tree copies. Each was rebuilt
from HEAD's blob (`git show HEAD:<path>`), patched with a single guarded insertion, written with
`git hash-object -w`, and installed with `git update-index --cacheinfo` — index-only, so the
working-tree bytes were neither modified nor normalized.

| Path | HEAD blob | Staged blob | Change |
|---|---|---|---|
| `Cargo.toml` | `9d52f2c5…` | `61355c8848bb0abe340f40e560936925cd22b9f1` | **+1 line**: `"crates/tdb-canonical",` after `"crates/tdb-defs",` |
| `Cargo.lock` | `953c3370…` | `f68b91406671619982d7b2095412a03c1c795599` | **+10 lines**: the `[[package]] tdb-canonical` entry before `tdb-core` |

### 3.1 Staged diff assertions

| Requirement | Observed |
|---|---|
| Exactly the 27 authorized paths | `diff` of the sorted staged list against the authorized list → **identical** |
| Composition | 2 `M` (`Cargo.toml`, `Cargo.lock`) + 25 `A` |
| Deletions | **0** |
| `Cargo.toml` numstat | `1	0` |
| `Cargo.lock` numstat | `10	0` |
| `git diff --cached --check` | exit **0**, clean |
| No evidence-system paths | `evidence`, `qa_evidence` → 0 matches |
| No Phase-5 features | `tdb-headmesh`, `blender-job`, `zune` → 0 matches in paths and 0 in the manifest/lock diff |
| No other workspace members | `Cargo.toml` diff adds one line only |
| No unrelated lockfile changes | `Cargo.lock` diff is the ten-line entry only |
| No docs/receipts | 0 paths under `docs/` |
| `studio-domain`'s `tdb-qa` edge still uncommitted | Confirmed absent from the staged `studio-domain` block. The one `+ "tdb-qa",` line lies **inside** the `tdb-canonical` entry — its own dev-dependency, part of the authorized ten lines |

---

## 4. Verification of the staged tree

### 4.1 Scratch location

| | |
|---|---|
| Root | `H:\c0-can-commit1\` — **new**, confirmed absent before creation |
| Prior evidence | `H:\c0-slice2\`, `H:\c0-lock1\` and the older `H:\c0-*` directories were **read only** (for the C0-SLICE2 manifest, its hashes and its tested candidate); none was written, moved or deleted |
| Build output | `H:\c0-can-commit1\target\`, external to the candidate |
| Logs | `H:\c0-can-commit1\logs\`, outside candidate inputs |
| Live worktree | never switched, reset, stashed or checked out during verification |

### 4.2 Export

`git write-tree` → **`b2f1ee3deca0e63aa0513a434947d91cec12e1d2`**, archived
(SHA-256 `7a2d26c8c908f0e75dca709edbaf49109d327abf87d638ee30d485fedec64ddd`) and extracted to
`H:\c0-can-commit1\candidate\`: **1160 files** = HEAD's 1135 + the 25 additions. This is the staged
tree itself, so what was checked is exactly what would have been committed.

### 4.3 Hooks inspected before execution

No `.cargo/config.toml`, no `build.rs` under `crates/`, no `rust-toolchain` pin. `CARGO_NET_OFFLINE=true`
plus `--offline` on every invocation.

### 4.4 Newline normalization — reported explicitly

Raw working-tree hashes do **not** prove staged-byte identity here, and this is where the package
turned. Comparing each of the 25 files in the **exported staged tree** against the **C0-SLICE2
tested candidate**:

| Classification | Count | Files |
|---|---|---|
| **Identical bytes** | **2** | `src/model.rs`, `src/tests.rs` — CRLF on disk, so unchanged by the round-trip |
| **Newline-only difference** | **23** | everything else — **LF on disk**, LF in the blob, **CRLF after checkout** |
| **Content differs** | **0** | — |

Corroborated independently by `git add` itself, which emitted
`LF will be replaced by CRLF the next time Git touches it` for exactly those 23 paths.

The two mixed files were also checked: the staged `Cargo.toml` and `Cargo.lock` are content-identical
to the blobs built in §3, differing only by CRLF.

So no content changed anywhere — **but 23 of 25 files reach a fresh checkout with different bytes
than C0-SLICE2 ever tested.** That is not a cosmetic difference for this crate (§4.6).

### 4.5 Check results on the staged tree

| # | Command | Exit | Result |
|---|---|---|---|
| 1 | `cargo test -p tdb-canonical --locked --offline` | **101** | **FAILED — 184 passed, 7 failed, 0 ignored** |
| 2 | `cargo clippy -p tdb-canonical --all-targets --locked --offline -- -D warnings` | 0 | clean |
| 3 | `cargo fmt -p tdb-canonical -- --check` | 0 | clean |
| 4 | `cargo check --workspace --all-targets --locked --offline` | 0 | see diagnostics below |

Logs: `logs\20-test.log`, `21-clippy.log`, `22-fmt.log`, `23-workspace.log`.

**Diagnostics, kept separate:**

| Source | Diagnostics |
|---|---|
| **`tdb-canonical` — canonical** | **7 test failures** (§4.6). Zero compiler warnings, zero clippy findings under `-D warnings`, zero rustfmt differences |
| **Dependency / pre-existing HEAD** | Run 4 exits 0 **with warnings**, all from crates outside the slice: `tdb-rx2` lib 1, `tdb-rx2` lib-test 2 (1 duplicate), `studio-domain` tests `nhl07_encodings` 1, `read_matrix` 1, lib-test 1. **None from `tdb-canonical`** |
| Registry / network | none — no download attempted or required |

### 4.6 The blocker in detail

All 7 failures are in `crates/tdb-canonical/src/build_override_tests.rs`:

```
build_override_tests::a_pinned_constraint_permits_a_set_of_one_and_refuses_everything_else
build_override_tests::a_prior_value_is_either_recorded_or_explicitly_absent_and_never_both
build_override_tests::a_prior_value_must_describe_the_setting_the_proposal_changes
build_override_tests::a_record_bound_to_nothing_is_incomplete
build_override_tests::an_unknown_field_anywhere_is_a_parse_error_not_a_silent_drop
build_override_tests::an_unknown_setting_or_a_mistyped_operand_is_a_parse_error
build_override_tests::no_override_record_can_name_a_canonical_source_field_to_change
```

Representative panic:

```
build_override_tests.rs:35:
synthetic-approved-slot does not contain "setting = \"atlas-slot\"\n[proposal.proposed.operand]\nslot = 7"
```

**Mechanism.** The helper `record_from` (line 30) and three sibling loops (lines ~707, ~764, ~805)
edit fixture text by literal substring:

```rust
assert!(text.contains(from), "{name} does not contain {from:?}");
text = text.replace(from, to);
```

Every `from` that spans a line break is written with `\n`. The fixture is LF in the working tree and
LF as a blob, but **CRLF once checked out** — verified directly: the same file is
`UTF-8 text` in the live tree and as a blob, and `UTF-8 text, with CRLF line terminators` in the
exported staged tree. The multi-line literals therefore match nothing, `contains` fails, and the
negative cases silently stop testing what they claim to.

**Why C0-SLICE2 did not see it.** That package copied working-tree bytes, which were LF, so all 191
passed. The failure appears only after a git round-trip on a CRLF platform — exactly what a commit
creates. C0-SLICE2's result was not wrong; it measured a tree that a fresh clone will not reproduce.

**Reach.** The pattern is not confined to the failing file: `\n` appears in 845 literals in
`build_override_tests.rs`, **1098** in `tests.rs` and **1459** in `asset_tests.rs`. Only 7 assertions
fail today, but any of the others that spans a line break is at risk. The 184 that pass on CRLF have
not been shown to be line-ending-independent — merely not to have tripped.

**Platform shape.** This is Windows-specific. On `ubuntu-latest` CI, where no CRLF expansion occurs,
the suite would very likely pass — so the defect would land green in CI and fail for every developer
on **the project's required initial desktop platform**. Not verified on Linux; no Linux run was made.

Per the package's instruction, verification failing means **do not commit, do not repair, do not
broaden**. No fix was attempted: making the tests newline-independent, or pinning the fixtures with a
`.gitattributes` `-text`/`eol=lf` rule, are both changes of substance outside this package's
authority, and each has trade-offs the architect should choose between.

### 4.7 Candidate integrity, before and after execution

| Check | Result |
|---|---|
| The 27 authorized inputs | `sha256sum -c` after all four runs → **all 27 unchanged** |
| Whole-tree content drift | `tar -df staged.tar` → **zero** `Contents differ` / `Size differs` / `Mod time differs` across all 1160 members |
| Membership | **1160 → 1164.** Four files appeared |

**The four new files, reported rather than dismissed:** `src-tauri/gen/schemas/{acl-manifests,
capabilities,desktop-schema,windows-schema}.json`, written by the Tauri codegen during run 4's
workspace check. They confirm the package's warning that an external `CARGO_TARGET_DIR` alone does
not bound writes — this build wrote into the **source tree**. They are covered by `.gitignore:17`
(`src-tauri/gen/`), were correctly absent from the staged tree, and are unrelated to the canonical
slice; nothing was staged from them. Nothing was removed, and no authorized path was touched. The
live repository already carries its own copies from earlier work; **no build or test was run against
the live tree**, so those were not regenerated by this package.

---

## 5. No commit, and preservation

Because run 1 failed, §4 of the package ("after verification succeeds") was not entered. The index
was returned to empty with `git reset` — mixed, which rewrites the index only and leaves every
working-tree file untouched.

| Check after unstaging | Observed |
|---|---|
| HEAD | `0a791892dcfef11d079e873c2444d5e70bf29262` — **unchanged, no commit created** |
| `git log --oneline -2` | `0a79189` · `a2bbdc4` — unchanged |
| Index | **empty** |
| All 27 live source/config bytes | `sha256sum -c` → **all 27 byte-identical** to preflight, `Cargo.toml` and `Cargo.lock` included |
| Working-tree status | **identical** to the preflight capture, all **242** entries — the 25 files are untracked again and the two mixed files are modified-unstaged, exactly as before |

The only file this package writes in the repository is this receipt, left **uncommitted**. Earlier
receipts were not included in anything, and no historical record was edited.

---

## 6. What the architect now has to decide

The slice's *boundary* is proved sound: 27 paths exactly, cleanly separable, `clippy`/`fmt`/workspace
`check` all clean, no feature or evidence-system coupling. What is not sound is the crate's own test
suite under committed line endings. Two directions, neither taken here:

1. **Make the tests newline-independent** — normalize fixture text on read, or build the literals
   without embedded `\n`. Touches test code only, leaves fixtures as ordinary text files, and fixes
   the latent risk across all three test files (§4.6). Larger edit.
2. **Pin the fixtures' line endings** — a `.gitattributes` rule (`eol=lf`, or `-text`) over
   `crates/tdb-canonical/tests/fixtures/**`, and arguably over `defs/**`, so checkouts keep LF.
   One-line-ish, but it makes the tests pass by constraining the checkout rather than by removing the
   fragility, and it changes a repository-wide convention.

A third question sits underneath both: the same `\n`-literal pattern appears throughout `tests.rs`
and `asset_tests.rs`, so whichever fix is chosen, the architect may want the whole crate audited for
line-ending assumptions rather than just the 7 failing cases.

---

## 7. Unresolved questions

1. Which remedy above, and does it come as a new package? No fix is authorized today.
2. Should the other two test files be audited for the same pattern before the slice is re-verified?
3. Does the Windows/Linux split matter to acceptance — this would pass CI and fail locally, which
   inverts the usual failure mode and is worth a policy note.
4. Should the workspace check's source-tree write (§4.7) be recorded as a C0.2-class observation? It
   is benign and gitignored, but it means "external target dir" is not a sufficient containment
   claim in future probes.
5. `H:\c0-can-commit1\` is retained — keep or discard? Prior scratch evidence is untouched.

---

## 8. Package boundaries observed

Preflight, explicit-path staging, index-only mixed-file construction, isolated verification,
unstaging, and this receipt. **No commit**, no push, no repair of the blocker, no broadening of the
slice, no feature work, no generator, no live-tree build or test, no game-data access, no
`.gitattributes` or configuration edit, no historical C0.2 edit, and no deletion of prior scratch
evidence. **Stopping here for architect review.**

---

## 9. Corrections — appended by C0-CAN-EOL1 (2026-09-09)

Three corrections from the byte-level conversion analysis in
[c0-can-eol1-test-portability.md](c0-can-eol1-test-portability.md). **The staging boundary, the
27-path scope, the withheld commit and all preservation evidence above are unaffected**, as is the
existence and character of the blocker itself.

**9.1 "Fails on any fresh Windows clone" was overstated and was not demonstrated.**
§1 and §4.6 say the suite fails on *any* fresh Windows clone. What this package actually exercised
was `git archive`, not a clone. A throwaway repository has since observed the checkout path directly:
an LF blob becomes CRLF **iff** the consuming clone sets `core.autocrlf=true` (3 CR), and stays LF
under `false` or `input` (0 CR); `git archive` under `autocrlf=true` matches the `true` clone exactly.
So the demonstrated claim is **"fails on any checkout that expands LF to CRLF — i.e. `core.autocrlf=true`,
the Git for Windows default"**. A Windows clone configured `false` or `input` receives LF and passes.
The blocker is real and reachable by the default Windows setup; it is not unconditional.

**9.2 Conversion happens at checkout/archive, not at `git add`.**
§1 and §4.4 describe git as normalizing the files "to LF blobs" on staging, implying staging changed
them. For the fixture examined, the staged blob is **byte-identical to the live file** (1962 bytes,
0 CR, `9f89ad98…`) — it was already LF, so staging normalized nothing. The 57 CR bytes appear only in
the archive member and the extracted candidate (2019 bytes, `af7ace4e…`). The `git add` warning
quoted in §4.4 is a statement about the *next checkout*, not about the staging that emitted it. The
§4.4 classification table (2 identical, 23 newline-only, 0 content differences) remains correct.

**9.3 The escape-sequence counts did not measure exposure.**
§4.6's "845 in `build_override_tests.rs`, **1098** in `tests.rs`, **1459** in `asset_tests.rs`"
counted *lines containing* `\n`, and the surrounding sentence invited reading them as cases at risk.
A call-site audit gives the real figure: `tests.rs` has **zero** mutation sites, `asset_tests.rs` has
**zero** file-derived multi-line anchors (all ten of its `replace`/`replacen` calls mutate inline Rust
literals, or use a single-line anchor), and the entire exposure is **six sites in
`build_override_tests.rs`**. The claim that "the 184 that pass have not been shown to be
line-ending-independent" was fair as caution but is now settled: they are, and all 200 cases pass in
both representations after the repair.
