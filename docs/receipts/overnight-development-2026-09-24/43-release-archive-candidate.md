# Receipt 43 — one self-contained release-candidate archive (Studio + defs + XMA helper with source and notices), verified; review bundle r43

**Developer-verified; not Codex-accepted; not launched; not published or uploaded.** HEAD `8d60a6d` (unchanged; nothing
staged or committed). Claim `dev-release-archive`. Direction: [beta decisions](../../plans/beta-20261001-scope-decisions.md)
3 and [dispatch after receipt 40](../../plans/development-20260927-after-receipt40.md) §1 / §3.

## 1. Plain-language summary

- **One zip holds everything Studio needs:**
  - Studio itself and its `defs` folder;
  - the XMA2 helper for Xbox 360 audio previews;
  - the helper's exact FFmpeg source, licences, runtime notices, build scripts and configuration;
  - a third-party notices file, licence texts for the libraries compiled into Studio, a build record and a hash
    manifest.

  Extract it anywhere, including a path with spaces, and run `nhl-studio.exe` in place. Nothing else needs to be
  downloaded or built.
- **Checked without launching Studio.** From a folder path with spaces, run from an unrelated working directory,
  with no developer override:
  - every file matches the manifest;
  - Studio's own rule finds `defs` beside the executable, and a title opened with them names its 252 teams;
  - Studio's own lookup finds the archive's helper;
  - all 83 research XMA samples decode through it bit-identical to research.
- **Broken archives are caught.** The checker refuses an archive missing the helper, the source, the notices, the
  FFmpeg licence, the runtime notices or `defs`. It also refuses one with a changed, added or replaced file, even
  when the broken archive's manifest was rewritten to match. 13 of 13 cases were refused.
- **This build is also the new review bundle.** It contains receipts through 42: the NHL 12 pads file check and the
  NHL 08 / 09 mask paint (receipt 40), the schedule preservation fix (41), and the team mask paints (42).
- **Drafted, not published:** download-page wording and a licence / EULA audit
  ([draft](../../release/draft-download-page-attribution.md)).
  - Studio has no licence file yet: README says "to be determined", while decision Q46e says MIT. This is
    reported, not settled; the archive says so.
  - 15 of the 331 bundled libraries publish no licence file of their own. They are listed with their declared
    licence and flagged for the release.

## 2. Artefacts

| Item | Value |
|---|---|
| Archive | `E:\nhlms-release-candidate-20260927\NHL Modding Studio rc-20260927-r43.zip`: 45 755 047 bytes, SHA-256 `7e81d48b56c300030884064ff6d07d91fdd4b2171a2a72226b154a2b8e618776`, 655 files under one root folder |
| Review bundle (extracted, verified) | `E:\nhlms-review-build-r43-2026-09-27\NHL Modding Studio rc-20260927-r43\` — run `nhl-studio.exe` there |
| `nhl-studio.exe` | 49 758 720 bytes, SHA-256 `0ee1c38c3cb3fdcfbdedd6976ac66b1cca1a27ce26c07645046149b4dd498b80` (= `target\release` at assembly) |
| Built | 27 Sep 2026 12:56:39 → 12:58:48 MDT, `npm run build -- --no-bundle`; HEAD `8d60a6d` + 861 dirty inputs, manifest `f1465c8a…66c6`, re-hashed after the build: identical |
| `defs\` | 121 files, tree `2f763a6e…0cd8` · frontend 55 files, tree `5f2dc1ba…c107` · `helpers\` 21 files, tree `1caa5f4d…f5c4e`, helper `dc3533ed…8c21`, FFmpeg source `7138d28c…78a3` |
| Third party | 236 Rust crates linked (normal-dependency closure of `nhl-studio` on x86_64-pc-windows-msvc, proc-macros excluded), 95 npm packages (non-dev superset), libspeex 1.2.1 |
| Records | `docs/validation/evidence/overnight-development-2026-09-24/review-build-r43/` (build record input, manifests, `tauri-build.log`, `assemble.log`, `verify.log`, `release-root-tests.log`) |
| Included receipts | through 42 |

## 3. Files

| Path | Change |
|---|---|
| `tools/release-archive/assemble.py` (**new**) | deterministic assembly (sorted entries, fixed timestamps): exe, defs, helpers, libspeex notice, third-party notices + licence files (Rust from `cargo metadata --offline`, npm from the lockfile), README, BUILD-RECORD, MANIFEST |
| `tools/release-archive/verify.py` (**new**) | `check` (single root, manifest completeness and hashes, required layout, pinned helper / source identities, notices content, exe = record) and `selftest` (13 broken variants) |
| `crates/studio-api/src/commands.rs` | `bundled_defs(resource_dir)` split out of `resolve_defs` (same rule, now testable) |
| `crates/studio-api/tests/release_archive_layout.rs` (**new**) | extracted-root `defs` resolution + a title opened with them |
| `crates/tdb-snd/tests/ea_xma_real_vectors.rs` | with `NHLMS_RELEASE_ROOT`: the helper must come from the archive via `locate`, with the override variable unset |
| `docs/release/draft-download-page-attribution.md` (**new**) | page wording, obligations table, licence / EULA audit |

## 4. Checks (logs as in §2)

- `verify.py check` on a fresh extraction under `…\extract check with spaces\`: ok. The same check extracted the
  review bundle: ok.
- `verify.py selftest`: 13 / 13 refused.
  - Removed: the helper, the source, the helper notices, the FFmpeg licence, the runtime notices, the third-party
    notices, `defs`.
  - Changed: a file altered; a file added.
  - With a rewritten manifest: the helper removed, the source removed, the notices removed, the helper replaced.
- Test binaries run from `E:\tmp`, with `NHL_STUDIO_XMA2_HELPER` unset and `NHLMS_RELEASE_ROOT` = the extracted root:
  - `release_archive_layout` 1 / 1 (252 teams named through the archive's `defs`);
  - `ea_xma_real_vectors` 1 / 1: helper = `<root>\helpers\nhlms-xma2-helper.exe` (`dc3533ed…`), 83 vectors, all
    counts exact, all PCM = the research hashes.
- **Failed attempt (recorded):** the first assembly stopped because the libspeex notice path was wrong
  (`vendor\COPYING` instead of `vendor\speex-1.2.1\COPYING`). It was fixed and re-run, and nothing partial was
  kept.
- **Not done:** launching Studio from the archive (not permitted here; the desktop cards stay with the user).
  Tauri's resource directory being the executable's folder on this layout is the rule the r39 bundle already
  relied on. It is checked through `bundled_defs`, not through a running app.

## 5. Obligations and open release items

In the archive: FFmpeg source, configuration, scripts, licences, runtime notices and attribution (the draft's §2
table). Open:
- an in-app About / notices entry;
- Studio's own licence file (the Q46e / README conflict);
- 15 dependencies without a packaged licence file: alloc-stdlib, selectors, ts-rs, five unic-* crates,
  webview2-com(-sys), @mediapipe/tasks-vision, @react-three/fiber, draco3d, maath, stats-gl;
- the npm list is a superset of what the bundler keeps.

The helper keeps its name. The FFmpeg checklist's DLL-name point concerns libraries; this helper is a separate
executable, and its origin is disclosed in the notices, README and page draft.

## 6. Cleanup

Removed: the assembly stage and the check extraction (containment-checked under
`E:\nhlms-release-candidate-20260927\`), the self-test scratch (removed by the script), and scratch scripts.
**Kept:** the archive, the r43 review bundle, the r33 / r39 / 26 Sep bundles (untouched) and `C:\nhlms-toolchain\`.
