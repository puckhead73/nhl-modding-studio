# r72 coordinator review — original defects fixed; one dependency gap remains

2026-10-02. Independently verified all six preserved r71 probes and the ten new correction cases. The five original r71 defects are closed at the tested automated scope. **ONB-003 correction acceptance remains incomplete:** a roster-only package still requires a game installation and installation-side metadata. This is a safe refusal, not an observed integrity defect. No user/game acceptance is implied.

## Verified

- Archive SHA-256: `e56f6a0f58d690f159a94e18ff234a8e30ff3ca6b00cff2d229a7f7f05421f0b`.
- Executable SHA-256: `13c5786b937d1eb1d8387dc5ff997984ff2f83577e28dbf7c1a3cecb7f0200c4`.
- All **771 manifest entries** in the extracted r72 bundle hash correctly.
- The permanent `onb003_coordinator_probe.rs` is byte-identical to the coordinator's preserved r71 reproduction (`a1e8516551791ebb53fb4b77b0a51d33bf8409d68be83c20bc8d1ee102156bab`).
- Ran `cargo test -p studio-api --test onb003_coordinator_probe --test onb003_corrections`: **10/10 + 10/10 passed**. The first binary includes the original six probes and four existing package tests; these are 20 tests total, not 26.
- Reviewed package/project/recovery/domain publisher paths and the installer UI. Their files and both test files match r72's `dirty-inputs-before.txt`.
- Confirmed code independently stages file roles, binds keyed rows, handles sidecar removals in the publisher and uses shared recovery classification. These results cover the existing owned-fixture/fault tests, not an actual process crash or game run.

No full suite/frontend/CI rerun, archive selftest rerun or launch was performed by this review. Developer receipt evidence remains separately attributed. No original game/save or implementation file was changed. All write tests used owned temporary copies. Prior bundles were untouched; their complete trees were not independently rehashed this turn.

## R72-R1 — P2: roster-only installation still requires unrelated game resources

This requirement was explicit in the r71 correction handoff, item 2: “Roster-only must work without an unrelated main DB using verified bundled metadata.” It follows ONB-003's dependency-specific installer behavior.

`crates/studio-api/src/registry/package.rs:775` unconditionally refuses an empty game root. It then takes metadata only from `report.db_paths.meta_xml`; staging requires that path even when every operation targets the roster. The installer page's `canPreflight` similarly requires a non-empty game root. The package preflight API has no baseline-store argument to supply the already-bundled schema.

Owned-copy reproduction:

1. Import only the genuine Legacy Xbox 360 roster into an owned project, using the bundled metadata store. No game installation is linked.
2. Edit two fields, save and build a package. Inspect and assert all operations have `file == "roster"`.
3. Copy the genuine roster into an independent recipient save folder. Pass that explicit save and save location to preflight with no game root.
4. Result: **`choose the destination game installation first`**.
5. Try the save folder as root instead. Result: **`no nhlng-meta.xml found`**.
6. Recipient hash stays unchanged; the assertion that this supported roster-only package is installable fails.

Command: `cargo test -p studio-api --test r72_coordinator_probe coordinator_ -- --nocapture`.
Result: **0 passed, 1 failed**, four inherited tests filtered out. Complete reproduction: [r72_coordinator_probe.rs.txt](r72_coordinator_probe.rs.txt). Restore to the matching tests path to run; coordinator removed the temporary sibling after preserving it. The permanent developer tests are unchanged.

Required correction: determine resources from package contents before requiring paths. For roster-only packages, accept the actual selected save independently of an installation, resolve and verify title/platform metadata from packaged resources, and retain explicit multi-save/stale-choice rules. Do not ask the user to fabricate an install folder or copy schema files manually. Wrong or missing schemas must refuse with a precise reason and no writes. Test native save output and exact restore, not just an enabled button.

## Scope and documentation

Legacy Xbox 360/Recompiled texture qualification is automated and uses synthetic native texture fixtures. Legacy PS3 cross-platform texture placement remains unsupported; that limitation is disclosed and remains a blocker to the full requested multi-platform graphical workflow. Cross-game/year conversion stays deferred. Other package exclusions remain as listed in receipt 73.

Receipt 73 says deployment cards are not invalidated despite changes to project deployment planning, header publication and shared restore. Preserve historical results but rerun those changed paths. Its blanket statement that every r71 package is refused is also broader than the implementation: the explicit schema-1 refusal applies to packages with cell edits. Update those descriptions without promoting user results.

Disposition: accept the five original fixes at their tested scope, retain r72 as historical evidence, close R72-R1 before calling the complete correction delivered. The remaining portability gaps are separate from this small dependency correction. [Development handoff](CORRECTION-HANDOFF.md).
