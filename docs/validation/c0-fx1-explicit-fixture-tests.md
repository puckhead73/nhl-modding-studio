# C0-FX1 — Explicit private-fixture tests

> **Package:** C0-FX1. Closes the silent missing-fixture skips recorded in
> [C0.2 §4.4](c0-02-baseline-execution.md) (defect **B4**) for the `tdb-rx2` crate, and removes the
> `nhl-database-studio` path dependency recorded in [C0.2 §3.3](c0-02-baseline-execution.md)
> (defect **B5**) — for those cases only.
> **Execution date:** 2026-09-07 (UTC). **Author role:** developer. Reviewer: Codex (architect).
> **Git:** one authorized commit — the accepted C0-DT1 record, `a2bbdc4` (§7). **No source file was
> staged or committed. Nothing was pushed.** No dependency, manifest or lockfile change.

Conventions follow C0.2. **Passed / Failed / Blocked / Not run** describe a check that actually
executed. A case that returns before its assertions is **skipped coverage**, never a pass. After
this package no `tdb-rx2` case in the addressed set can produce skipped coverage: it either runs its
assertions or fails.

---

## 1. Result

| | Before | After |
|---|---|---|
| `cargo test -p tdb-rx2` | **93 reported `ok`**, 26 of them without reaching an assertion | **78 executed, 26 ignored, 0 skipped coverage** |
| Fixture absent, case selected | silent early return, reported `ok` | **fails** with a diagnostic naming the variable, key and resolved path |
| Fixture configuration | six different hardcoded lookups, four of them absolute operator paths | one documented root, `NHL_STUDIO_FIXTURES`, six relative keys |
| `E:\Repositories\nhl-database-studio` in Rust source | 2 constants, 5 cases | **none** |
| `H:\Emulators\…\Decompiled\Original\…` in Rust source | 2 cases | **none** |
| Path-resolution coverage | none | **11 self-contained cases** over synthetic temp directories |

No assertion was weakened, removed or relaxed. Every ignored case retains its original body verbatim
apart from the fixture acquisition line.

---

## 2. Inventory — the affected cases

### 2.1 Count discrepancy against C0.2 §4.4 — 26, not 15

C0.2 §4.4 recorded **15** cases. That figure was derived from the `skipping:` lines visible in
`logs/10-mountgated-serial.log`, so it counted only the gates that printed something. Eleven further
cases in the *same* fixture families used a bare `else { return }` (or a helper returning `None`
with no message) and were therefore invisible even under `--nocapture`. Verified against source:

| Source of the count | Cases |
|---|---|
| C0.2 §4.4 list | 15 |
| Additional silent gates on the **same six fixtures**, found by reading every `return` inside a `#[cfg(test)]` module of `tdb-rx2` | **+11** |
| **Total addressed by this package** | **26** |

The eleven C0.2 did not list: `container::replace_payload_grows_data_size_and_reflows`;
`pack::substitute_with_different_counts_grows_and_reparses`;
`playermesh::{head_decodes_as_halffloat_position_and_ushort_uv, skin_palette_is_122_bones,
glb_output_round_trips_through_serde_chunks}`; `xpr::{decode_then_replace_is_identity,
replace_changes_only_pixels, resize_repatches_geometry_and_reparses,
resize_to_same_dims_preserves_header}`; `uv2_observation_on_bundled_helmet`;
`bounding_box_and_normals_make_sense`.

This is a correction to C0.2's figure, not a scope increase: the eleven depend on the same six
fixtures and the same six lookup helpers, and leaving them behind would have left the identical
defect in the identical files.

### 2.2 Test → fixture → previous lookup path

All paths are as they were before this package. None of them is in the repository, and none of the
six fixtures is present on this machine (§5.3).

| # | Test (`tdb-rx2`) | Fixture key | Previous lookup |
|---|---|---|---|
| 1 | `container::tests::round_trip_template_byte_identical` † | `mesh/player_20500.rx2` | `<repo>/Working Assets/Template Mesh/player_20500.rx2` |
| 2 | `container::tests::replace_payload_grows_data_size_and_reflows` | `mesh/player_20500.rx2` | same |
| 3 | `pack::tests::template_lists_all_five_mesh_handles` † | `mesh/player_20500.rx2` | same |
| 4 | `pack::tests::substitute_head_round_trips_geometry_weights_and_uvs` † | `mesh/player_20500.rx2` | same |
| 5 | `pack::tests::substitute_with_different_counts_grows_and_reparses` | `mesh/player_20500.rx2` | same |
| 6 | `playermesh::tests::parses_template_into_five_submeshes` † | `mesh/player_20500.rx2` | same |
| 7 | `playermesh::tests::patch_round_trips_positions_and_palette_on_template` † | `mesh/player_20500.rx2` | same |
| 8 | `playermesh::tests::head_decodes_as_halffloat_position_and_ushort_uv` | `mesh/player_20500.rx2` | same |
| 9 | `playermesh::tests::skin_palette_is_122_bones` | `mesh/player_20500.rx2` | same |
| 10 | `playermesh::tests::glb_output_round_trips_through_serde_chunks` | `mesh/player_20500.rx2` | same |
| 11 | `texlib::tests::make_descriptor_matches_hair_256_dxt3` † | `legacy/texlib_67.rx2` | `E:\Repositories\nhl-database-studio\Working Assets\Template Texture\NHL Legacy\texlib_67.rx2` |
| 12 | `texlib::tests::make_descriptor_matches_head_512_dxt1` † | `legacy/texlib_67.rx2` | same |
| 13 | `texlib::tests::replace_with_resize_same_dims_round_trip` † | `legacy/texlib_67.rx2` | same |
| 14 | `texlib::tests::replace_with_resize_grows_earlier_slot` † | `legacy/texlib_67.rx2` | same |
| 15 | `texlib::tests::parses_real_texlib_67` † | `legacy/texlib_67.rx2` | `H:\Emulators\games\Decompiled\Original\rendering\player\texlib_67.rx2` |
| 16 | `texlib::tests::replace_same_size_round_trip` † | `legacy/texlib_67.rx2` | same |
| 17 | `texlib::tests::make_descriptor_matches_real_centerlogo_1024` † | `legacy/centerlogo_0_cm.rx2` | `E:\Repositories\nhl-database-studio\Working Assets\Template Texture\NHL Legacy\centerlogo_0_cm.rx2` |
| 18 | `xpr::tests::parses_real_portrait_geometry` † | `legacy/playerheadssmall_p1512.xpr2` | `<repo>/Working Assets/Template Texture/NHL Legacy/playerheadssmall_p1512.xpr2` |
| 19 | `xpr::tests::decode_then_replace_is_identity` | `legacy/playerheadssmall_p1512.xpr2` | same |
| 20 | `xpr::tests::replace_changes_only_pixels` | `legacy/playerheadssmall_p1512.xpr2` | same |
| 21 | `xpr::tests::resize_repatches_geometry_and_reparses` | `legacy/playerheadssmall_p1512.xpr2` | same |
| 22 | `xpr::tests::resize_to_same_dims_preserves_header` | `legacy/playerheadssmall_p1512.xpr2` | same |
| 23 | `tests::parses_glove_mesh_smoke` † | `equipment-preview/feprop_glove_0.rx2` | `crates/tdb-gui-api/assets/equipment-preview/feprop_glove_0.rx2` |
| 24 | `tests::bounding_box_and_normals_make_sense` | `equipment-preview/feprop_glove_0.rx2` | same |
| 25 | `tests::parses_helmet_mesh_smoke` † | `equipment-preview/helmet_14_2.rx2` | `crates/tdb-gui-api/assets/equipment-preview/helmet_14_2.rx2` |
| 26 | `tests::uv2_observation_on_bundled_helmet` | `equipment-preview/helmet_14_2.rx2` | same |

† = one of the 15 C0.2 §4.4 listed.

Two facts worth recording about the previous lookups:

- **`crates/tdb-gui-api` does not exist in this repository.** It is a crate name from the prior
  application, so cases 23–26 could never have found their fixture here regardless of what is
  mounted. That is a second, previously unrecorded instance of the B5 clean-room dependency.
- **Cases 11–16 name the same asset twice.** The `texlib.rs` source comment stated it outright —
  *"the working-assets copy of `texlib_67.rx2` … the legacy operator cache at `H:\Emulators\…` is no
  longer mounted"* — so both lookups are one NHL Legacy `rendering/player/texlib_67.rx2`. They now
  share the single key `legacy/texlib_67.rx2`. The dimension/format assertions on either side agree
  (hair 256×256 DXT3, head 512×512 DXT1), which is consistent with that, but the two are **not**
  independently proven identical here because neither file is available (§5.3).

### 2.3 Mixed tests

None. In all 26 the fixture gate was the first statement of the body, so there is no self-contained
portion to keep in the default suite. Nothing was converted to ignored coverage that could have
stayed independently tested.

---

## 3. Implementation

### 3.1 Files changed

| Path | Change |
|---|---|
| `crates/tdb-rx2/src/fixtures.rs` | **New.** Fixture keys, resolution, loading, diagnostics, and 11 self-contained tests. `#[cfg(test)]` only. |
| `crates/tdb-rx2/src/lib.rs` | Declared `#[cfg(test)] pub(crate) mod fixtures;`; `load_asset` now requires; 4 cases ignored. |
| `crates/tdb-rx2/src/container.rs` | Removed the `Working Assets` lookup; 2 cases ignored. |
| `crates/tdb-rx2/src/pack.rs` | Removed the `Working Assets` lookup; 3 cases ignored. |
| `crates/tdb-rx2/src/playermesh.rs` | Removed the `Working Assets` lookup; 5 cases ignored. |
| `crates/tdb-rx2/src/texlib.rs` | Removed both `nhl-database-studio` constants and both `H:\Emulators\…\Decompiled` literals; 7 cases ignored. |
| `crates/tdb-rx2/src/xpr.rs` | Removed the `Working Assets` lookup; 5 cases ignored. |
| `docs/validation/c0-fx1-explicit-fixture-tests.md` | **New.** This record. |
| `CLAUDE.md` | Added the fixture-selection commands to the conventions section. |

`crates/tdb-rx2/Cargo.toml` is **unchanged** — no dev-dependency was added. The synthetic temp
directories in §3.4 are built from `std` alone, so `Cargo.lock` is untouched (§6).

### 3.2 The contract

`crates/tdb-rx2/src/fixtures.rs` exposes three things to the crate's tests:

| Item | Property |
|---|---|
| `resolve(root: Option<&OsStr>, key) -> Result<PathBuf, FixtureError>` | Pure path resolution against an **explicitly supplied** root. Reads no environment and touches no file content. |
| `load_from(root: Option<&OsStr>, key) -> Result<Vec<u8>, FixtureError>` | `resolve` + existence, file-kind, readability and non-emptiness checks. |
| `require(key) -> Vec<u8>` | The only function that reads `NHL_STUDIO_FIXTURES`. Returns bytes or **panics** with the `FixtureError` message. It has no `Option` in its signature, so an early return cannot be written at a call site. |

`std::env::set_var` is never called, so there is no process-global race: the testable surface takes
its root as an argument and only `require` consults the environment (read-only).

`FixtureError` distinguishes nine conditions, each with its own message: root unset, root empty,
root missing, root not a directory, malformed key, fixture missing, fixture not a regular file,
fixture unreadable, fixture zero bytes. The first four and the sixth append the configuration
paragraph — the variable, the exact command, the expected `<root>/<key>` path, a pointer to this
document, and an explicit statement that fixtures are private data that nothing searches for.

**No fallback and no scanning.** There is no default root, no search of a mounted game tree, no
reference to any other application's repository, and no walking of parent directories. An unset
variable is an error, never a cue to look somewhere.

### 3.3 Configuration and commands

Point `NHL_STUDIO_FIXTURES` at a directory laid out exactly like this. It may live anywhere outside
the repository; the files are private game data and are never committed.

```text
<root>/
  equipment-preview/
    feprop_glove_0.rx2
    helmet_14_2.rx2
  legacy/
    centerlogo_0_cm.rx2
    playerheadssmall_p1512.xpr2
    texlib_67.rx2
  mesh/
    player_20500.rx2
```

```sh
# Default suite — synthetic and self-contained cases only. Never touches a fixture.
cargo test -p tdb-rx2

# Every fixture-dependent case. Fails loudly if the root is unset or incomplete.
NHL_STUDIO_FIXTURES=<root> cargo test -p tdb-rx2 -- --ignored

# One case.
NHL_STUDIO_FIXTURES=<root> cargo test -p tdb-rx2 -- --ignored texlib::tests::parses_real_texlib_67

# Everything at once.
NHL_STUDIO_FIXTURES=<root> cargo test -p tdb-rx2 -- --include-ignored
```

PowerShell: `$env:NHL_STUDIO_FIXTURES = '<root>'` before the `cargo test` call.

A partial root is supported and behaves correctly: cases whose fixture is present run and assert;
cases whose fixture is absent **fail** naming the missing file. There is no configuration in which a
selected case reports success without reading its fixture.

### 3.4 Path-resolution tests

Eleven cases in `fixtures::tests` run in the **default** suite. Each builds a synthetic directory
under `std::env::temp_dir()` with a name unique per process and per call, and removes it on drop. No
fixture, no game tree and no repository path is involved.

| Case | Asserts |
|---|---|
| `unset_root_is_an_error_that_names_the_variable_and_the_key` | `RootUnset`; message contains the variable, the key and this document's filename |
| `empty_root_is_rejected_rather_than_treated_as_the_cwd` | `RootEmpty` — an empty value never silently resolves relative to the working directory |
| `missing_root_is_reported_with_the_path` | `RootMissing`, path echoed |
| `a_file_used_as_the_root_is_rejected` | `RootNotADirectory` |
| `a_nested_key_resolves_under_the_root` | `<root>/mesh/player_20500.rx2` |
| `malformed_keys_are_rejected_before_the_root_is_examined` | `BadKey` for `""`, `/abs/x.rx2`, `mesh//x.rx2`, `../x.rx2`, `mesh/../x.rx2` |
| `a_missing_fixture_names_the_root_and_the_expected_file` | `Missing`; message contains both |
| `a_directory_at_the_fixture_path_is_not_accepted_as_the_fixture` | `NotAFile` |
| `a_zero_byte_fixture_is_invalid_rather_than_an_empty_success` | `Empty`; message says `0 bytes` |
| `a_present_fixture_loads_its_exact_bytes` | byte-exact read |
| `one_configured_root_serves_every_declared_key` | all six keys are well-formed and resolve under one root |

---

## 4. Verification

Working directory: the repository root, on the live working tree. `cargo` 1.96 toolchain as recorded
in C0.2 §6.

### 4.1 Default suite — executed vs ignored

```
cargo test -p tdb-rx2
running 104 tests
test result: ok. 78 passed; 0 failed; 26 ignored; 0 measured; 0 filtered out
   Doc-tests tdb_rx2: 0 passed; 0 failed; 0 ignored
```

| | Count |
|---|---|
| Total cases in the crate | **104** (93 pre-existing + 11 new resolution tests) |
| **Executed and asserted** | **78** (67 pre-existing + 11 new) |
| **Ignored** (fixture-dependent, §2.2) | **26** |
| **Skipped coverage** (reported `ok` without asserting) | **0** |
| Pre-existing `#[ignore]` in this crate | **0** — all 26 are introduced here, and no previously ignored test was enabled |

Before this package the same crate reported 93 cases `ok` of which 26 asserted nothing. The 15
`cargo test --nocapture --test-threads=1` `skipping:` lines C0.2 recorded for `tdb-rx2` are gone.

### 4.2 Negative tests — selected with no or bad fixture configuration

| # | Condition | Command | Result |
|---|---|---|---|
| N1 | `NHL_STUDIO_FIXTURES` unset, all 26 selected | `cargo test -p tdb-rx2 -- --ignored --test-threads=1` | **FAILED — 0 passed, 26 failed**, each with `fixture '<key>' is required but NHL_STUDIO_FIXTURES is not set.` plus the configuration paragraph |
| N2 | root set to an empty synthetic directory | `NHL_STUDIO_FIXTURES=<tmp> … parses_real_texlib_67` | **FAILED** — `fixture 'legacy/texlib_67.rx2' is missing.` naming both the root and the expected file |
| N3 | root set, fixture present but **0 bytes** | as N2 after `: > <tmp>/legacy/texlib_67.rx2` | **FAILED** — `fixture 'legacy/texlib_67.rx2' is invalid: … is 0 bytes` |
| N4 | root set, fixture present but **not a valid RX2** | as N2 with 17 bytes of text | **FAILED** — the case's own parser assertion: `real file must parse: TooShort { needed: 96, have: 17 }` |
| N5 | root points at a non-existent directory | `NHL_STUDIO_FIXTURES=<tmp>/nope …` | **FAILED** — `the NHL_STUDIO_FIXTURES root does not exist: <path>` |

N4 is the important one for the package's objective: an *invalid* fixture is not caught by the
loader (it is not the loader's job to validate an RX2), and it still produces a clear failure rather
than a pass, because the case now runs its assertions unconditionally.

All synthetic directories used for N2–N5 were created under the session scratch directory and
deleted afterwards. No game tree was read or written.

### 4.3 Lints and formatting

| Check | Command | Result |
|---|---|---|
| Strict clippy, scoped | `cargo clippy -p tdb-rx2 --all-targets -- -D warnings` | **Passed**, exit 0, no warnings |
| Formatting, scoped | `cargo fmt -p tdb-rx2 -- --check` | **Passed**, exit 0 |
| Formatting, repo-wide | `cargo fmt --all -- --check` | **Passed**, exit 0, empty output |
| Strict clippy, repo-wide | — | **Not run.** Out of the package's verification scope; only `tdb-rx2` changed. |
| Compilation | `cargo test -p tdb-rx2 --no-run` | **Passed**, no warnings |

The repo-wide `fmt` result reflects the C0-INT1 repairs already present in the working tree; it is
not a claim about HEAD, where C0-INT1 §7.2 records that B1 is still open.

No full workspace suite was run; the package did not require one and only `tdb-rx2` changed.

---

## 5. Coverage — what this does and does not establish

### 5.1 What improved

The default suite no longer reports success for the `tdb-rx2` texlib descriptor/resize surface, the
XPR2 portrait surface, the `playermesh`/`pack`/`container` template surface or the equipment-preview
mesh surface. Those 26 cases are now visibly **ignored**, and 11 new cases genuinely execute.

### 5.2 What did **not** improve

**The underlying coverage hole is unchanged.** C0.2 §4.4 called it "a substantial hole hidden behind
a green suite"; this package removes the hiding, not the hole. Those surfaces remain unproven until
someone runs the ignored cases against real fixtures. The C0-INT1 §6.1 statement that patches 04–06
(`texlib`, `pack`, `playermesh`) carry no fixture-level coverage still stands.

### 5.3 Real-fixture cases — **not run**

None of the six fixtures is present. Verified by searching the whole repository for each filename
(0 hits) and by the working-tree `Working Assets` directory not existing. `crates/tdb-gui-api` does
not exist either. The two `nhl-database-studio` paths were **not** visited: that repository is off
limits under CLAUDE.md hard rule 1 and no authorization was given for this package.

No fixture was acquired, copied, extracted, generated or committed. **All 26 fixture-dependent cases
are recorded as not run.** They stay not run until an authorized fixture set exists.

### 5.4 Still open in `tdb-rx2` — deliberately out of this package's scope

Seven cases keep a hardcoded absolute path into a mounted private game tree and still return early
and report `ok` when it is absent. They are the `tdb-rx2` members of the C0.2 §4.5 set — cases that
*did* execute during the baseline because the mounts were present — not §4.4 missing-fixture skips,
and the package boundary excludes them:

| Case | Root |
|---|---|
| `gnf::tests::nhl16_portrait_bc7_decodes` | `H:\Emulators\ShadPS4\Games\CUSA01909\…` |
| `gnf::tests::parses_real_head_cm_dimensions` | `H:\Emulators\ShadPS4\Games\CUSA00561\…` |
| `mcd::tests::decodes_real_logo_mesh` | `H:\Emulators\ShadPS4\Games\CUSA00561\…` |
| `mcd::tests::decodes_real_player_head` | `H:\Emulators\ShadPS4\Games\CUSA00561\…` |
| `mcd::tests::decodes_real_nhl16_player_head` | `H:\Emulators\ShadPS4\Games\CUSA01909\…` |
| `texlib::tests::nhl07_award_argb8888_is_xbox_tiled` | `X:\Games\Emulation\Roms\XBOX360\NHL 07\…` |
| `texlib::tests::nhl09_nameplate_argb8888_is_linear` | `X:\Games\Emulation\Roms\XBOX360\NHL 09\…` |

All seven executed here (the mounts are present), so the crate currently prints no `skipping:` line
at all. On a machine without those mounts they would silently skip exactly as before. Converting
them is the same mechanical change and the module already supports it; it needs an architect
decision about whether a *mounted game install* belongs behind the same fixture root, which is a C0
fixture-register question, not this package's.

Also unchanged, and explicitly untouched: the two boot write probes
(`assets::legacy_x360_boot_write_probe`, `assets::legacy_x360_boot_checksum_probe`), the seven
pre-existing `#[ignore]`d cases listed in C0.2 §4.4, defect **B3**, the remaining **B5** paths in
`studio-domain` and `tdb-archive-big`, **B6**, **B7**, Windows CI (C0.2 R6), and C0's comprehensive
fixture register. No QA generator was run, no `docs/audits/data/**` file was touched, no capability
was promoted and **no cell anywhere is game-verified**.

---

## 6. Preservation checks

The working tree carried extensive pre-existing mixed and untracked work before this package. It was
preserved: no reset, restore, checkout, stash, clean, branch change or blanket staging at any point.

| Check | Result |
|---|---|
| Index before the authorized commit | **empty** (`git diff --cached --name-only` → no output) |
| Index after the authorized commit | **empty** |
| `Cargo.lock` SHA-256 | `d8c6487f6917e92e0c20ee1c836d8df9a5ff9afc1b4f35bf9e664ad14d0b1622` — byte-identical to the value pinned in [C0-DT1 §2.1](c0-dt1-dto-drift-gate.md) |
| `crates/tdb-rx2/Cargo.toml` | unchanged; no dependency added |
| DTO bindings | unchanged — still exactly the 10 paths modified/untracked relative to HEAD that C0.2 §4.7 and C0-DT1 record (`PlayerAttributesDto.ts` modified, 9 untracked) |
| Files this package modified | the 6 `tdb-rx2` sources in §3.1 plus 1 new file; **no other working-tree path was written** |
| Line endings | preserved per file — the CRLF files (`container.rs`, `pack.rs`, `xpr.rs`) were rewritten as CRLF and the LF files (`lib.rs`, `playermesh.rs`, `texlib.rs`, already LF from the C0-INT1 repairs) as LF |
| Pre-existing hunks in the touched files | intact; the C0-INT1 repair hunks (`div_ceil`, `is_none_or`, `clamp`, `saturating_mul`, rustfmt) are still present and unmodified |
| Game trees | not written. The only filesystem writes outside the repository were synthetic temp directories under the session scratch path and `std::env::temp_dir()`, all removed |

---

## 7. Authorized commit

The index was verified empty before staging. Only the accepted C0-DT1 record was staged; the staged
diff was inspected (`A docs/validation/c0-dt1-dto-drift-gate.md`, 423 insertions) and
`git diff --cached --check` returned clean (exit 0).

| Item | Value |
|---|---|
| Commit | `a2bbdc4f5308a1c6fa124c475fcbf72555df4e25` |
| Subject | `docs(validation): record accepted DTO drift gate` |
| Files | `docs/validation/c0-dt1-dto-drift-gate.md` (new, 423 lines) — nothing else |
| Prior HEAD | `bdcda0c` |
| Push | **Not performed.** Not authorized. |

This package's own implementation, and this document, are **uncommitted**.

---

## 8. What was not done

No source file staged or committed. No push. No dependency, manifest, lockfile or toolchain change.
No `#[ignore]`d test enabled — neither the two boot write probes nor any pre-existing ignored case.
No test removed, weakened or given a looser assertion. No generator run and no `docs/audits/data/**`
file touched. No CI change. No DTO regenerated. No anchor, game directory or private data written,
acquired or copied. No capability promoted and no target qualified. The evidence-contract
implementation (C0.4) was not started, and C0's comprehensive fixture register was not designed.

---

## 9. Addendum — 22 September 2026: the private fixture root is populated

Developer campaign OVERNIGHT-DEV-2026-09-22, Priority 1B (coordinator R1 followup: "resolve the two
tests expecting `legacy/texlib_67.rx2`… find the correct source by provenance and content identity…
do not substitute an arbitrary similarly named file"). Before this, `NHL_STUDIO_FIXTURES` resolved
to `E:\nhl-studio-fixtures`, which held only the archive / database / logo trees; **every**
`tdb-rx2` fixture key above was absent, so the 26 `--ignored` cases failed loudly when selected
(correct behaviour, zero coverage).

Each file below was located by the **provenance recorded in `fixtures.rs`** — not by name search —
and its content verified against the documented description before use. The originals are read-only
inputs and were **copied**, never moved or modified.

| Fixture key | Source (read-only) | SHA-256 | Content check |
|---|---|---|---|
| `legacy/texlib_67.rx2` | NHL Legacy X360 `rendering/player/texlib_67.rx2` | `5e7d56d230df131f83cc01066b7c8acbc828cb3b575d6066cf0e7f986c950c61` | file table = data/desc/data/desc/name; descriptor 1 = `hair_67_cm` 256×256 Dxt3, descriptor 2 = `head_67_cm` 512×512 Dxt1 — exactly the documented "hair 256×256 DXT3 plus head 512×512 DXT1 in one texlib" |
| `legacy/centerlogo_0_cm.rx2` | NHL Legacy X360 `rendering/icesurface/centerlogo_0_cm.rx2` | `0997f5a54467565d97afe7ecbce7e5bfece5480d5e9b14c7a093ae2774254879` | `make_descriptor_matches_real_centerlogo_1024` asserts 1024×1024 Dxt3 |
| `legacy/playerheadssmall_p1512.xpr2` | NHL Legacy X360 `fe/ion/artassets/playerheadssmall/p0_4000/p1512.big`, member `194`, RefPack-decompressed (8 455 → 32 864 B, the declared decompressed length) | `579f20edbceddc2f483262a0d0da930ac30cffda1bac170d424d8dae293ba761` | Studio's own `Xpr2Texture::parse` reads 256×128 DXT5 — the documented shape — which independently validates the extraction |
| `mesh/player_20500.rx2` | NHL Legacy X360 `rendering/player/player_20500.rx2` | `1e4da208975f994628f8483405b5dadb0cee9562ab4762bdc7f7213408fb09ff` | `parses_template_into_five_submeshes`, `skin_palette_is_122_bones` |
| `equipment-preview/feprop_glove_0.rx2` | NHL Legacy X360 `rendering/feprop/feprop_glove_0.rx2` | `353c72f46fafc777e9f4fc79edd9817dc71d0b37b7534705ec1f089800dfa3cf` | `parses_glove_mesh_smoke` |
| `equipment-preview/helmet_14_2.rx2` | NHL Legacy X360 `rendering/helmet/helmet_14_2.rx2` | `4d70480e6f3a1eb28739f44d1e4d423af7a9556ed130e7cc27d0a60884c25c6d` | `parses_helmet_mesh_smoke`, `uv2_observation_on_bundled_helmet` |

**Coverage this supplies** (`NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures cargo test -p tdb-rx2 --lib
-- --include-ignored`: **144 passed, 0 failed, 0 ignored**, was 118 passed / 26 loudly failing):

- `texlib_67` — the two `replace_with_resize` cases the R1 review recorded as missing-fixture
  failures (same-dims round trip; growing the *earlier* slot, which exercises the data-region
  reflow and file-table patching), plus `parses_real_texlib_67`, `replace_same_size_round_trip`
  and the two `make_descriptor` ground truths. This is the **only** real multi-raster container in
  the default-selectable set, so it is also where the new `DW8` slot-join check is exercised on
  real data.
- `centerlogo_0_cm` — the 1024² third-resolution descriptor reference.
- `playerheadssmall_p1512` — the five XPR2 cases: parse geometry, decode→replace identity,
  replace-changes-only-pixels, and both resize cases.
- `player_20500`, the glove and the helmet — the mesh parser, GLB export, skin palette and
  container round-trip cases.

**Still not covered by a real fixture** (recorded, not implied): BC2 / BC4 / BC7 / BC6H PS4 `GNF`
members (the PS4 set added the same day covers BC1 / BC3 / BC5 — see
[the P1 receipt](../receipts/overnight-development-2026-09-22/02-p1-resize-evidence.md)); PS3
`RW4ps3` containers; NHL 07 / 08 `RW4xb2` layouts.

The fixture root is outside the repository and is never committed; this table is the record of what
it must contain, so a fresh machine can rebuild it from the same originals.
