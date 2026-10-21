# Imported head-converter components

These are the user's existing converter components, imported from
`E:\Projects\Repositories\nhl-head-mesh-converter`. Original files were not
changed. `upstream-source-manifest.json` records their original hashes.

This is **not a universal game-to-game exporter**. The geometry/anatomy modules
and qualified NHL21 profile route are available here; destination-specific
format, skeleton, UV, material and runtime qualification still apply.
The Studio library browser exports portable GLB/DDS packages. Selecting an
appearance there does not install a native game modification.

## Dependencies and checks

Python with NumPy and Pillow; Blender 4.5 for extraction/baking; Rust/Cargo for
the native packer. Licensed meshes, calibrated profiles and source images are
explicit private inputs, not bundled here.

From this directory:

```powershell
python -m unittest tests.conversion.test_conversion tests.conversion.test_worker tests.conversion.test_app_interface tests.conversion.test_legacy_quant tests.conversion.test_legacy_qualify -v
python experiments/claude/worker/run_job.py path/to/job.json --dry-run
python experiments/claude/worker/run_job.py path/to/job.json --events
```

The worker consumes the existing `nhl-head-job/1` contract. Its implemented
anatomy route is CC5 head-only → calibrated NHL21 donor, with mouth, eye, neck,
collision and atlas stages. The two example jobs retain their original source
fingerprints; relocate inputs only after verifying identical content.

Build a native packer against this Studio checkout, using an existing run root
and a new output directory:

```powershell
python -m src.application.native_build --tdb-rx2 ../../crates/tdb-rx2 --run-root C:/HeadRuns --out C:/HeadRuns/native-build --offline
python -m src.application.export --help
```

Export requires a completed worker manifest, a qualified native profile or
handoff, matching import preconditions and a validated build receipt. The
exporter writes a new offline package and rechecks protected bytes and native
layout. It does not write to game installations. Missing inputs or failed
qualification remain failures.

## Studio integration changes, 2026-10-20

- Fixed scalar NumPy half-float quantization while retaining array behavior.
- Removed deprecated bitwise inversion of a Python boolean in neck masking.
- Included the worker, bake/render drivers and native packer source dependencies.
- Resolved the current Studio crate's inherited workspace license when creating
  a standalone native build snapshot.

The current build and a byte-identical NHL21 donor round trip passed. Replaying
the old Turgeon anatomy export stopped because its original
`S:\Downloads\Mogilny Test\Turgeon\TurgeonTest.Fbx` is absent. That check was not
disabled. See the overnight validation receipt for the distinction between
automated structural checks and in-game qualification.
