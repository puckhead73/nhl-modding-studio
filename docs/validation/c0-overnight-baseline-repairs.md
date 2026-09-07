# C0 overnight — Windows formatting, lint and vacuous-assertion repairs

> **Type:** repair work performed in an **isolated snapshot**, not in the live tree. No live source
> file was modified; **this report is the only file this run adds to the repository.**
> **Inputs:** the accepted [C0.2 baseline execution report](c0-02-baseline-execution.md) (defects
> B1, B2, B3) and its reviewed snapshot helper `c0-02-snapshot.ps1`.
> **Execution date:** 2026-09-07 (UTC). **Author role:** developer. Reviewer: Codex (architect).
> **Git:** nothing staged, committed, amended or pushed. No cleanup, reset, stash or restore.

Conventions follow C0.2: **Passed / Failed / Blocked / Not run** describe a check that actually
executed. A case reporting `ok` after returning early on an absent fixture is **skipped coverage**,
never a pass.

---

## 1. What this run did

| C0.2 defect | Status after this run |
|---|---|
| **B3** — `read_matrix.rs:305` `assert!(… \|\| true)` is vacuously true | **Repaired.** Replaced with a checked invariant, backed by a negative test *and* a controlled mutation of the production resolver. |
| **B1** — `cargo fmt --all -- --check` fails (1,231 hunks / 122 files / 19 crates) | **Repaired.** Stock rustfmt applied; `--check` now exits 0. |
| **B2** — `cargo clippy … -D warnings` fails (172 unique diagnostics) | **Repaired.** 0 diagnostics; the strict gate exits 0. |
| B4, B5, B6, B7 | **Untouched** — out of scope for this queue. |

Nothing else was changed: no feature work, no QA evidence migration, no CI change, no generator run,
no game-directory write, no dependency or toolchain change, no test removed or `#[ignore]` enabled.

---

## 2. The isolated repair workspace

A **new** snapshot was taken with the reviewed helper. The original C0.2 snapshot at
`H:\c0-baseline\c0-02-20260907-085022` was **not** read from, written to or deleted.

| Item | Value |
|---|---|
| Repair workspace | `H:\c0-repair\c0-ovn-20260907-105522` |
| `source\` | the working copy the repairs were made in |
| `base\` | a byte-identical second copy of the snapshot, never edited — the patch base |
| `replay\` | a third copy used only to verify the patches replay |
| `evidence\` | manifests, logs and patches — **outside** the source input set |
| **Snapshot manifest SHA-256** | **`4b376f8dafa6c9fc180ba3c250ed0f60d85265a00317d1eae60174490a46debf`** |
| Git HEAD at snapshot | `2a70b4d0ea8c3a213678f5f76f685da940821434` (branch `feat/phase5-asset-tooling`) |
| Members | **1,225 files** — 1,132 tracked + 93 untracked-allowlist; 394 untracked paths excluded |
| Helper result | `SNAPSHOT-OK` — reparse scan clean, 1,225/1,225 copy-verified, live membership and bytes stable, HEAD unchanged |
| Volume | `H:` (109.7 GB free). `E:` has 11.3 GB, still not enough for the source plus a Rust `target/`. |

**Snapshot identity is the manifest hash together with HEAD**, because the snapshot deliberately
contains uncommitted bytes that HEAD does not describe. This is a *different* snapshot from C0.2's:
HEAD has moved (`16825ca` → `2a70b4d`) and the working tree has changed, so the member count differs
(1,222 → 1,225).

| Toolchain | Version |
|---|---|
| cargo / rustc | 1.96.0 (`30a34c682` / `ac68faa20`, 2026-05-25) |
| rustfmt / clippy | 1.9.0-stable / 0.1.96 |
| node / npm | v24.16.0 / 11.13.0 |
| git | 2.54.0.windows.1 |
| OS | Windows 11 Pro 10.0.26200, 16 logical cores |

Bounded concurrency, as C0.2: `CARGO_BUILD_JOBS=8`, `RUST_TEST_THREADS=8`.

---

## 3. B3 — the vacuous arena assertion

### 3.1 What the old line did and did not prove

```rust
assert!(!arena.name.is_empty() || arena.city.is_empty() || true, "arena ref built");
let _ = arena; // resolution success is implied by Some(ArenaRef)
```

The trailing `|| true` makes the expression unconditionally true, so the arena-reference claim was
unproven. The comment's reasoning was also circular: `Some(ArenaRef)` proves only that *something*
was constructed, not that it was constructed from the right record.

### 3.2 What the code actually guarantees

`team_view` resolves the home rink as `kOtt.buDa → StEO.vfEq`
([view.rs](../../crates/studio-domain/src/view.rs)): `arena_record` finds the first `StEO` record
whose `vfEq` equals the id stored in the team's `buDa` cell, and `arena_ref` reads `arenaname` and
`cityname` **off that record**. `arena_summaries` reaches the same table by an *independent* path —
it iterates `StEO` directly rather than resolving a key.

That yields three checkable consequences, all expressible through the public surface the test
already uses:

1. a `Some` reference carries the arena id actually stored in the team's own `arenaid` cell — read
   from `view.all_fields` as `FieldRaw::Int`, which is the same raw value `arena_ref` consumed;
2. that id appears in the arena master list, and **both paths read the same strings off it** — this
   is what catches a resolution landing on a neighbouring record;
3. a `None` is a documented null: either this schema has no team→arena link at all (the NHL 10–12
   era gap), or the stored id matches no arena row.

### 3.3 What it deliberately does not assert

**Not** that `name` or `city` is populated. NHL07's arena table has no name column, so an empty
string there is data, not a defect. The check therefore holds on all 27 anchors without assuming
anything about arena content — which is precisely the guessed requirement the brief warned against.

The invariant is factored into `check_arena_ref(arena, fk_raw, arenas) -> Result<(), String>` so it
can be exercised directly, and the loop panics with the returned message.

### 3.4 Evidence that it now bites

**(a) Negative test** — `arena_ref_check_rejects_wrong_references`, a new self-contained case with
synthetic rows and no fixture. Four inputs must be accepted (a healthy reference; a name-less
NHL07-style row; a schema with no `arenaid`; a stored id matching no arena row) and six must be
rejected:

| # | Wrong reference | Rejected |
|---|---|---|
| 1 | resolved to a different arena than the FK cell names | yes |
| 2 | **off-by-one** — right id, strings from the neighbouring row | yes |
| 3 | city from the wrong row while the name matches | yes |
| 4 | an id present in no arena row | yes |
| 5 | a reference built although the team row has no readable `arenaid` | yes |
| 6 | resolution gave up on an id that is plainly present | yes |

**(b) Controlled mutation of the production resolver.** `arena_ref` was changed to
`arena_record(db, arena_id)? + 1` — case 2 above, injected into the real code path — and the matrix
test was re-run:

```
test arena_ref_check_rejects_wrong_references ... ok
thread 'read_matrix_over_all_anchors' panicked at crates\studio-domain\tests\read_matrix.rs:320:17:
ps3/nhl13: team 0: arena 0 resolved to ("MTS Centre", "Winnipeg, MB")
           but arena record 0 holds ("Honda Center", "Anaheim, CA")
test read_matrix_over_all_anchors ... FAILED
```

The mutation was then **reverted**; it is not part of any patch. Log:
`evidence/logs/12-read_matrix-mutation-offbyone.log`.

**(c) Clean run.** With the resolver unmutated, `cargo test -p studio-domain --test read_matrix`:
**2 passed, 0 failed** over all 27 anchors (`evidence/logs/11-read_matrix-after-repair.log`).

This repair is patch `01` and is kept **separate from formatting**.

---

## 4. B1 — the formatting baseline

Per the architect's decision, **stock rustfmt defaults** were used; no `rustfmt.toml` or
`.rustfmt.toml` was introduced, and nothing was hand-formatted.

| | |
|---|---|
| `cargo fmt --all -- --check` before | **exit 1**, 1,235 diff hunks |
| `cargo fmt --all` | exit 0 |
| `cargo fmt --all -- --check` after | **exit 0**, empty output |
| Files changed | **122**, all `.rs`, across **19 crates** |

The 1,235 hunks are C0.2's 1,231 plus four introduced by the patch-01 test code, which this pass
also formats. Per-crate: `studio-domain` 63 files, `studio-api` 9, `tdb-qa` 7, `tdb-edit` 6,
`tdb-headmesh` 5, `tdb-savedata`/`tdb-relations`/`tdb-formula`/`tdb-core`/`tdb-archive-big` 4 each,
then single digits. **No non-Rust file and no generated DTO binding was touched** — verified by
filtering the patch's file list for anything not ending `.rs` (empty).

This is patch `02` and contains **no semantic change**.

---

## 5. B2 — the strict clippy baseline

### 5.1 Starting inventory

Taken on the formatted tree, from `cargo clippy --workspace --all-targets --locked --keep-going`
with `--message-format=json`, de-duplicated by `(lint, file, line, column)`:

**172 unique diagnostics** — `tdb-rx2` 57, `tdb-headmesh` 50, `studio-domain` 40,
`tdb-archive-codec` 13, `tdb-fhm-build` 6, `studio-api` 2, `tdb-fhm-store` 2, `tdb-edit` 1,
`tdb-qa` 1. Largest lints: `needless_range_loop` 36, `unnecessary_cast` 24, `manual_div_ceil` 17,
`manual_is_multiple_of` 15, `nonminimal_bool` 14, `type_complexity` 12.

C0.2's deny-by-default `overly_complex_bool_expr` error is absent: patch 01 removed it.

### 5.2 How the work was bounded

`clippy --fix` was **not** run blindly across the workspace. Where clippy's own suggestion is marked
`MachineApplicable`, it was applied **one narrow lint set at a time**, using
`-A clippy::all -A warnings --force-warn <lint>` so that only the named lints could produce an edit,
and the resulting diff was then read line by line. Everything else was hand-written.

The lint groups, in the order the brief specified, each captured as its own patch:

| Patch | Scope | Files | Diagnostics cleared |
|---|---|---|---|
| `03` | `tdb-archive-codec`, then `tdb-edit`, `tdb-qa`, `tdb-fhm-build`, `tdb-fhm-store`, `studio-api` | 11 | 25 |
| `04` | machine-applicable **numeric/binary** lints, workspace-wide | 18 | 53 |
| `05` | machine-applicable non-numeric lints, workspace-wide | 21 | 18 |
| `06` | everything remaining, by hand: `tdb-rx2`, `tdb-headmesh`, `studio-domain` | 25 | 76 |

Patch `05` also carries the reformat of the lines its own edits touched; running `cargo fmt` after a
lint edit is a consequence of that edit, not an independent formatting pass. Patch `02` remains the
only whole-tree formatting change.

### 5.3 Numeric and binary code — what was checked, and what was refused

Every suggestion touching integer width, rounding, signedness, overflow, endianness, bounds or
serialized output was read against the code it changes.

| Change | Why it preserves behaviour |
|---|---|
| `(w + 3) / 4` → `w.div_ceil(4)` (17 sites: DDS/DXT block counts, chunkzip chunk counts, UV bitmask sizing) | identical for every value the manual form does not overflow, and `div_ceil` cannot overflow at all. All unsigned; no cast added or removed. |
| `x % n == 0` → `x.is_multiple_of(n)` (15 sites) | all on unsigned types; `is_multiple_of` exists only for unsigned. Identical. |
| `unnecessary_cast` (24 sites, e.g. `t[0] as u32` on a `u32`, `(v as f32).to_le_bytes()` on an `f32`) | same-type casts. **The GLB/RX2 byte output is unchanged** — the cast never converted anything. |
| `checked_mul(k).unwrap_or(usize::MAX)` → `saturating_mul(k)` (RX2 vertex/index buffer bounds guards) | `checked_mul` yields `None` on overflow → `usize::MAX`; `saturating_mul` yields `usize::MAX`. Exactly equal, including the guard's rejection behaviour. |
| `(x).max(1).min(0xFF)` → `x.clamp(1, 0xFF)` (texlib DDS header byte `d[54]`) | identical; the bounds are constants with `min <= max`, so `clamp` cannot panic. Same emitted byte. |
| LZX repeated-offset rotation → `std::mem::swap` (2 sites) | `let o = r1; r1 = r0; r0 = o; o` leaves `r0 == old r1 == o` and `r1 == old r0`, which is `swap(&mut r0, &mut r1)` yielding `r0`. **See the flag in §8.** |
| 36 `needless_range_loop` rewrites in mesh/texture codecs | every byte-offset expression is carried over verbatim (`o + i * stride`, `m_off + (row * 4 + col) * 4`, `off + i * comp.size()`); only the way the element is reached changed. |
| `&mut Vec<u8>` → `&mut [u8]` (2 functions) | neither resizes its buffer; both only overwrite in place. Callers are unchanged (deref coercion). |

**One suggestion was refused.** At `crates/tdb-archive-codec/src/lib.rs:285` clippy's
`manual_checked_ops` fix proposes `checked_div`, but the guarded branch is a **ceiling** division
(`(fullsize + chunk_size - 1) / chunk_size`). `checked_div` floors, which would under-count chunks
and truncate a decompressed payload. The zero guard was kept and the division expressed as
`div_ceil` instead — which clears the lint without changing the result.

### 5.4 Lint allowances — five, each at one item

No blanket allowance, no `warn`-level downgrade, no crate-level `allow`, no lint disabled in
`Cargo.toml`.

| # | Item | Lint | Reason |
|---|---|---|---|
| 1 | `studio_api::commands::team_kit_preview` | `too_many_arguments` | It is a `#[tauri::command]`: the parameter list **is** the IPC contract the frontend calls with, and it feeds the generated bindings. Collapsing it into a struct is an API redesign this queue is not authorized to make. |
| 2 | `studio_api::registry::Registry::team_kit_preview` | `too_many_arguments` | Mirrors (1) one-for-one. |
| 3 | `studio_domain::head_catalog::write_head_group` | `too_many_arguments` | `pub`, and the shared entry point for both the player export and the Phase-4 staff/generic sweep. |
| 4 | `studio_domain::head_catalog::write_hair_only` | `too_many_arguments` | Mirrors (3). |
| 5 | `tdb_rx2::playermesh::TAG_FX_RENDERABLE` | `dead_code` | A verified RW4/RX2 container tag recorded alongside the tags the parser does read. Under this repository's evidence rules that is format evidence, not dead logic, so it is kept and documented rather than deleted. |

The nine `type_complexity` diagnostics were cleared by **naming** the tuple shapes
(`CsvTable`, `AwardRow`, `JointWeightArrays`, `Surface`, `EyeClusters`, `Influences`, `TextureJob`,
`UvSubmesh`) — no signature's meaning changed, and no public parameter or return type was
restructured.

### 5.5 One incidental documentation repair

`clippy::empty_line_after_doc_comments` at `crates/tdb-rx2/src/playermesh.rs` exposed a doc comment
orphaned from its function by an earlier code move: `btm_col_major` at the bottom of the file
carried **no** documentation, while its doc block sat 400 lines above under an empty section header.
The block was moved back onto the function rather than deleted or demoted to a plain comment.

### 5.6 Result

```
cargo fmt --all -- --check                                    exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings  exit 0
```

Unique diagnostics: **172 → 0**. The four groups account for all of them exactly:
25 + 53 + 18 + 76 = 172.

---

## 6. Verification

### 6.1 Before running anything

External-path write behaviour was re-inspected as C0.2 requires. The 33 hardcoded absolute paths
rooted at `X:\Games\Emulation\…` and `H:\Emulators\…` are unchanged, and **no patch alters a write
destination or a write gate**: filtering all six patches for `QA_ASSET_OUT`, `temp_dir` and those
roots returns only rustfmt context lines whose text is byte-identical on both sides of the hunk.
`QA_WRITE`, `QA_ASSET_OUT`, `QA_CENSUS_*`, `FHM_CORPUS`, `NHLMS_ONLY`, `BOOT_TARGET`, `LZX_DEBUG`
and `TS_RS_EXPORT_DIR` were unset for every command.

**The isolation limitation C0.2 recorded still applies**: a copied tree is not a sandbox, and the
27 default-enabled cases that read live private game data on `X:` and `H:\Emulators\ShadPS4` did so
again here. Those trees were not hashed before the run and cannot be shown to be unmodified.

### 6.2 Checks

| # | Check | Exit | Result | Log |
|---|---|---|---|---|
| 1 | `cargo test -p studio-domain --test read_matrix` (focused, B3) | 0 | **Passed** — 2/2, all 27 anchors | `logs/11-…` |
| 2 | Controlled off-by-one mutation of `arena_ref`, same test | **101** | **Failed as required** — the new assertion caught it | `logs/12-…` |
| 3 | `cargo fmt --all -- --check` | **0** | **Passed** — empty output | `logs/50-fmt-check.log` |
| 4 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | **0** | **Passed** — 0 diagnostics | `logs/51-clippy-strict.log` |
| 5 | Focused codec/mesh tests (`tdb-archive-codec`, `tdb-archive-big`, `tdb-rx2`, `tdb-headmesh`, `tdb-edit`, `tdb-core`) | 0 | **Passed** — 308 passed, 0 failed | `logs/60-focused-codec-tests.log` |
| 6 | `cargo test --workspace --locked` | 0 | **Passed** — **668 passed, 0 failed, 7 ignored** | `logs/61-full-workspace-tests.log` |

C0.2's full-suite baseline was 667 passed / 0 failed / 7 ignored. This run is 668 / 0 / 7; the
difference in the passed total is patch 01's new negative test.

**Anchors and DTO bindings, hashed before and after all Rust execution:**

| Set | Files | Before | After | Verdict |
|---|---|---|---|---|
| `anchors/` | 552 | `bf944f456dd5939f…` | `bf944f456dd5939f…` | **unchanged** |
| `app/src/bindings/*.ts` | 124 | `1cf08fbe361d8726…` | `1cf08fbe361d8726…` | **unchanged** |

(The values above are the SHA-256 of the sorted per-file hash list; the lists themselves are in
`evidence/manifests/`.) The repaired `source\` tree was also re-compared against `replay\` after
the suite finished: **0 differing files**, so nothing the tests ran modified the source under them.
The only path written inside the source tree was `src-tauri/gen/schemas/`, exactly as C0.2 predicted. The bindings result is the evidence behind §6.4: `export_typescript` ran
inside the full suite and emitted byte-identical files, so no DTO changed.


### 6.3 Skipped coverage — unchanged from C0.2, and unchanged *by* this run

None of C0.2's skips was closed, and none was newly introduced. That is **verified, not assumed**:
`cargo test` captures output, so the full-suite log carries no `skipping:` lines, and the affected
crates were therefore re-run with `--nocapture --test-threads=1`
(`logs/62-mountgated-serial.log`, 347 passed / 0 failed / 5 ignored). It reports exactly **15
missing-fixture skips and the 2 deliberately disabled boot probes** — the same set, fixture for
fixture, that C0.2 recorded. They are restated so the pass totals above are not read as coverage:

- **15 cases report `ok` while returning early on an absent fixture** (C0.2 defect B4) — the whole of
  `tdb-rx2`'s texlib descriptor/resize surface, XPR2 portrait geometry, and `playermesh`/`pack`
  head-substitution against a real template. Several of those files were edited by patches 04–06,
  so those edits carry **no fixture-level coverage**.
- **2 boot write probes** remain deliberately disabled (`QA_ASSET_OUT` unset). Not a missing fixture.
- **7 `#[ignore]`d cases** were not enabled, including
  `eb::chunklzx_decodes_against_extracted_oracle` — the only oracle for the LZX decoder.

The `texlib` skips are also the standing clean-room problem (C0.2 defect B5): four of them are gated
on fixtures inside `E:\Repositories\nhl-database-studio`, the prior application's repository, which
`CLAUDE.md` hard rule 1 places off limits. This run did not touch that, and it is not fixed here.

### 6.4 Frontend

The frontend checks were **not re-run**, and the C0.2 evidence is reused, for a stated reason: no
Rust DTO type changed. Patches 03–06 add only `#[allow]` attributes and internal type aliases inside
`studio-api`; no `#[derive(TS)]` struct, field, or `#[ts(...)]` attribute was touched. That claim is
not taken on inspection alone — all 124 binding files were hashed before and after the full Rust
suite ran `export_typescript`, and the result is in §6.2. C0.2 defect B4's `--check` gap (C0.2 R5)
is untouched by this run.

---

## 7. Handoff

Everything below lives in `H:\c0-repair\c0-ovn-20260907-105522\evidence\`.

### 7.1 Patches — apply in this order

Generated against the **working-tree snapshot bytes, not HEAD**. HEAD (`2a70b4d`) omits substantial
uncommitted work, so these patches will not apply to it. Apply with `patch -p1 --binary` at the
repository root.

| # | Patch | Files | Contents |
|---|---|---|---|
| 1 | `01-arena-assertion.patch` | 1 | The B3 repair and its negative test. Semantic; reviewable on its own. |
| 2 | `02-rustfmt.patch` | 122 | Mechanical `cargo fmt --all`. No semantic change. |
| 3 | `03-lint-codec-and-foundational.patch` | 11 | `tdb-archive-codec` first, then the foundational/data crates and `studio-api`. |
| 4 | `04-lint-numeric-machine-applicable.patch` | 18 | Numeric/binary lints, each reviewed against §5.3. |
| 5 | `05-lint-mechanical-machine-applicable.patch` | 21 | Non-numeric mechanical lints + the reformat of the lines they touched. |
| 6 | `06-lint-hand-remaining.patch` | 25 | Hand fixes: bool simplification, type aliases, range loops, the five allowances. |

`patches/APPLY-ORDER.md` carries, for every patch, the **expected base tree digest**, the resulting
digest, and the per-file before/after SHA-256 of every changed file.

| | |
|---|---|
| Starting source manifest digest | `ef63918863c21f47ec8ef15865d65fbd433dd209bad3dc01177c09b705106394` |
| Final repaired source manifest digest | `6c52dfbdd15be2c831df9572e88773f86ecea4ad7d82cfdfbb50f2efdd9569df` |

### 7.2 Replay verification

A third copy of the snapshot (`replay\`) received the six patches in order with
`patch -p1 --binary`. All six applied with **exit 0, no rejected hunk and no fuzz**, and the result
was compared file-by-file against the repaired `source\`: **0 differing files**. The patches
therefore reproduce the final source hashes exactly. No second full build was run for the replay.

### 7.3 Evidence layout

```
H:\c0-repair\c0-ovn-20260907-105522\
  source\                     repaired working copy
  base\                       pristine snapshot (patch base)
  replay\                     patch-replay verification copy
  evidence\
    PROGRESS.md               running progress log
    manifests\
      snapshot-manifest.txt   helper manifest (1,225 members, per-file hashes, rules, provenance)
      snapshot-manifest.sha256
      source-start.sha256     per-file hashes of the unmodified snapshot
      source-final.sha256     per-file hashes of the repaired source
      anchors-before.sha256   552 anchors, before Rust execution
      anchors-after.sha256    552 anchors, after
      bindings-before.sha256  124 DTO bindings, before
      bindings-after.sha256   124 DTO bindings, after
    patches\                  the six patches, their file lists, and APPLY-ORDER.md
    logs\                     every command's output, numbered in execution order
```

---

## 8. Behaviour-sensitive decisions and open items

1. **The LZX repeated-offset rewrite has no automated coverage.** `crates/tdb-archive-codec/src/lzx.rs`
   has no unit tests, and the only oracle (`eb::chunklzx_decodes_against_extracted_oracle`) is
   `#[ignore]`d. The `manual_swap` transformation is identical by inspection (§5.3), but nothing in
   this run executed the LZX decoder. **Flagged for the architect**: either accept it on inspection,
   or hold patch 04's `lzx.rs` hunk until the oracle case can run.
2. **Files edited by patches 04–06 that sit behind missing fixtures** — `tdb-rx2`'s `texlib.rs`,
   `xpr.rs`, `pack.rs` and `playermesh.rs` head-substitution paths — were changed under §5.3's
   equivalence argument, with the automated coverage listed in §6.3 and no more.
3. **The five allowances (§5.4) are architectural**, not stylistic: four say the parameter list of a
   command or shared entry point is its contract, and one says a verified format tag is evidence.
   If the architect prefers a parameter struct for `team_kit_preview`, that is a separate, scoped
   package with a bindings regeneration, not a lint fix.
4. **No `rustfmt.toml` was added**, per the architect's decision. The consequence is that the
   repository is now committed to stock rustfmt defaults; a later width change would produce another
   whole-tree diff.
5. **Not repaired, still open:** C0.2 defects B4 (fixture skips), B5 (hardcoded absolute paths,
   including the clean-room violation pointing into `E:\Repositories\nhl-database-studio`), B6 and
   B7, and the DTO `--check` gate (C0.2 R5) and Windows CI (R6).
6. **These repairs were not copied back into the live tree**, per the brief. Integration — and
   whether it happens before or after the user's in-flight work lands — is the architect's call.

---

## 9. What was not done

No commit, no staging, no amend, no push. No cleanup, reset, stash, restore or checkout anywhere.
The original C0.2 snapshot and its evidence are untouched. No live repository file was modified
except this report. No feature was implemented and no feature scope changed. No QA evidence was
migrated, no generator (`qa-inventory` and friends) was run, no `docs/audits/data/**` file was
touched, no QA status was promoted, and no capability was claimed. No CI workflow was changed. No
`#[ignore]`d test was enabled and no gated test was switched on. No dependency, lockfile or
toolchain change. No test was removed or weakened. No game directory was written. The archive
texture work (C3a) was not started.
