# Agent 2 — H2.3 accepted; H2.4 reproducible export-tool build

## Acceptance

H2.3, including H2.3a/b, is accepted for the demonstrated Windows NHL21 export route. The previously reproduced completion rollback, unowned launch, and exception-during-descendant-drain defects are closed. The unconditional release path closes the job even when confirmation fails; the receipt distinguishes measured cleanup from unconfirmed cleanup. Terminal-event delivery now distinguishes a claim from a successful write. H2.2 remains accepted.

Independent test run: 50 selected lifecycle/native-export tests passed in 59.524 seconds, no selected test skipped. Excluded only `test_spawn_failure_is_a_failed_receipt`, whose invalid-EXE launch stalled in an earlier review. Deterministic process-creation-failure coverage passed. The run includes all 20 native-export regressions, the new exceptional cleanup cases, commit-boundary cases, ownership failures, real Blender snapshot mutation and fresh Turgeon/Fedorov parity. Used Blender's bundled Python 3.11, redirecting test scratch directories into the permitted review workspace without source changes.

Independently rehashed the two preserved `-h23b-01` runs against phase 11: MCD, GNF and final-audit bytes match for each; both receipts are completed and both event streams contain 14 events. Broad worker tests remain prior/reported evidence, not rerun in this review.

Reviewed SHA-256s:

- `src/application/export.py`: `e79dd264125bf3f2ab90cbc862f586598e4394bcd7e783b6f630153c563f4510`
- `src/application/lifecycle.py`: `c969554dc5692c21963b8bdc6ab645dd448a0797f6ea1df940b525ec4c17fbfe`
- `tests/adapters/test_export_lifecycle.py`: `6f75c6e950ca21b1f74313c5025b44cce0048a499fda18ee37ed7b837ba9c253`

This is a bounded Windows lifecycle acceptance, not POSIX qualification, universal converter readiness or new runtime acceptance. The intentionally untested POSIX branch cannot confirm tree emptiness. No source, accepted package or game was changed by this review. Legacy eye-evidence correction remains a separate pending Codex review; S7 is unauthorized.

## Prompt for Claude — H2.4

You are Agent 2, the sole converter implementation developer. Codex has accepted H2.2 and the H2.3 Windows export lifecycle. Your sole active package is H2.4: establish a reproducible source/dependency build for the demonstrated NHL21 export toolchain and prove the fresh native executable preserves output.

Work only in `E:\Projects\Repositories\nhl-head-mesh-converter`. Confirm `codex/research` and read AGENTS.md, current statuses, the accepted H2.2/H2.3 reports, `experiments/codex/nhl21_package/Cargo.toml` and Cargo.lock, its native source, the Python pack/audit modules and their imports. Current Cargo.toml points directly to Studio's `tdb-rx2` crate; the exporter defaults to an executable under an old experimental build directory. This package makes those dependencies explicit and reproducible without reimplementing native conversion.

Preserve the unborn checkout and all untracked work. No reset, clean, branch switch, staging, commit or push. Do not modify Studio, original assets, accepted packages or games. Studio crate dependencies are read-only inputs. Do not build into Studio's target directory or overwrite the existing accepted packer executable.

### Required result

1. Define the exact source set and dependencies needed to build and execute this demonstrated export route. Include transitive local Rust dependencies, Cargo manifests/locks, relevant Python helper modules, interpreter/NumPy/Blender versions, and the native compiler toolchain. Produce a machine-readable manifest of relative source paths and SHA-256s plus an explicit external-dependency manifest. Record dirty dependency contents by hash; a repository commit alone is insufficient. Keep licensed inputs separate as local path/hash references.
2. Add one runnable, documented build/check entry point accepting explicit dependency locations and a fresh output/build root. Reuse the existing native packer implementation. Remove the need for the developer's fixed Studio drive path in this build route; a generated build manifest or isolated source/dependency snapshot under a fresh converter-local run is acceptable. Do not vendor all of Studio, create another converter implementation, or mutate its working tree. Use a dedicated Cargo target directory and the locked dependency graph. Missing or incompatible dependencies must produce actionable errors rather than silent substitutions.
3. Record the actual tool versions, source/dependency hashes, build command, lockfile and produced executable hash in a build receipt. Detect input drift during the build or build from captured inputs. Record licenses for dependencies this route would redistribute; do not bundle Blender or game assets in this package. Do not add signing, installers, CI, an SBOM platform or a general build framework.
4. Build a fresh native executable from the recorded source and use it through the accepted exporter via `--packer-exe`, with the other inputs explicit. Re-export Turgeon and Fedorov into fresh directories. Require MCD/GNF/final-audit parity with phase 11. Executable byte identity across builds is not required unless proven; distinguish reproducible native output from a bit-identical compiler product. Explain legitimate implementation/toolchain/path receipt changes.
5. Deliver a concise reproduction command sequence and a focused automated check for the build/input contract. Test missing dependencies, wrong declared source hashes and existing-output refusal without touching accepted outputs. Preserve existing native export/lifecycle behavior and rerun relevant regressions if shared execution code changes. Actual fresh-build exports are mandatory when the local prerequisites are available; if a concrete toolchain/dependency blocker prevents them, identify it precisely and stop without claiming a reproducible build.

Writable scope: converter-local build/check tooling, narrowly scoped native Cargo/build configuration, dependency/source manifests, focused tests and Claude documentation. Do not alter native geometry/texture algorithms or lifecycle behavior to make the build pass. Preserve CRLF where established. Record full starting/final hashes of changed files.

Non-goals: full head-profile schema implementation, expanding donors/titles, GUI/distribution, Studio integration, Legacy S7 or new fitting experiments. Versioned profile consolidation is the next application package after this build baseline; Legacy remains independently gated on Codex's review decision.

Return changed files, exact build/test commands and versions, build/source manifest paths, fresh executable hash, both fixture parity results, remaining external dependencies and the smallest next useful action. Update Claude status/handoff with H2.3 acceptance. Stop for Codex review before committing or expanding scope.
