# MS-02 NHL 07 Xbox 360 disc file reference

Scope: compare the linked game installation with an original-disc file manifest before optional game-asset extraction. Container parser repairs are explicitly deferred to the next user-assigned task.

## Reference and region

Source supplied by the user: `X:\Games\Emulation\Roms\XBOX360\ISOs\NHL 07 (USA).iso`. Region label: USA, from the supplied source's filename/identification; this is not an independent certification of the disc's authenticity. The manifest records the ISO SHA-256 and each original file's relative path, size and SHA-256. It lives alongside database/save anchors at `anchors/xbox360/nhl07/game-files.json` and is embedded in Studio so installed copies do not depend on the repository or the user's ISO path.

The reference distinguishes BIG containers from other files shipped loose. Optional `$SystemUpdate` entries are recorded but excluded from installation comparison, matching the disc extractor's skip behavior. Anchor hashes and exact disc-file matches are recorded separately. An empty `disc_matches` list means no byte-identical loose disc file; it does not prove save incompatibility or assert a relationship to a packed member. Save schema compatibility remains the separate MS-03 issue.

Other supplied Xbox 360 ISO filenames, under the same ISOs folder: NHL 08/09/10/15 (USA), NHL 11 (USA, En/Fr), NHL 12 (USA/Europe, En/Fr/De/Sv/Fi/Ru/Cs), NHL 13 (USA/Europe), NHL 14 (USA/Europe, En/Fr/De/Sv/Fi/Ru/Cs), and Legacy (USA/Europe, En/Fr/De/Ru). These are source-location notes only; no vanilla manifests or qualification for those games are claimed by this task.

## Behavior

The read-only checker hashes original files and reports missing/changed/unreadable paths. Additional files are reported separately because they can be extracted assets or mods; their presence does not prove modification of original disc files. Studio extraction receipts are ignored as bookkeeping. Symlinks/reparse points inside the installation are not followed and produce findings. No files are repaired, removed or overwritten.

Matching original files without extras pass automatically. Differences, extras, unavailable references or failed scans produce a warning with an acknowledgement checkbox. Extraction, Skip and Continue wait for the result and acknowledgement, then permit continuation. Acknowledgement resets for another source or a repeated scan. The UI names the reference region and explains that another region/edition may differ without implying it is modified. A mismatch does not identify the actual region. Only NHL 07 Xbox 360 currently has a bundled reference.

The asset extraction screen uses “existing files detected” instead of presenting an unqualified “partial” label. Counts explicitly describe container member paths, separately from the manifest's original loose-file/container counts. Existing loose paths alone do not prove that extraction started. Existing archive parsing and conflict protections remain unchanged.

## Reproduction

Generate a new reference (output must not already exist):

```powershell
cargo run -p studio-domain --bin game-file-manifest --offline -- 'X:\Games\Emulation\Roms\XBOX360\ISOs\NHL 07 (USA).iso' nhl07 xbox 'NHL 07 (USA) Xbox 360 — user-supplied original ISO' anchors/xbox360/nhl07 anchors/xbox360/nhl07/game-files.json USA
```

Read-only installation check:

```powershell
cargo run -p studio-domain --bin game-file-check --offline -- anchors/xbox360/nhl07/game-files.json 'F:\NHL Installations\Xbox 360\NHL07\NHL 07 (USA)'
```

The original ISO is opened read-only. Generation is an explicit maintainer operation; Studio never learns a vanilla reference from a user's selected installation.

## Executed checks

- Generated reference: 281 loose files, 11 BIG containers, one optional dashboard-update file. Source ISO SHA-256: `dca27bd7c3c0e6358c9bd35a08cef5de6d8d640e39b56695f5fe318be806cc91`.
- Exact disc/anchor matches: `db/NHL07NG.db`, `db/NHL07NG-meta.xml`, `fe/loc/nhl_eng_us.db`. This does not resolve the incompatible ROSTER1 save schema.
- `cargo test -p studio-domain --lib game_manifest --offline`: 1 scenario passed, covering unchanged original loose files, optional updates, additional files, same-length changed bytes, missing files, wrong target, unsafe manifest paths and source preservation.
- `cargo test -p studio-domain --lib extract_service::tests --offline`: all 10 existing extraction regressions passed.
- `npm test -- src/components/VanillaCheck.test.tsx src/components/ExtractionPanel.test.tsx src/pages/OpenProject.test.tsx`: 31 tests passed, including separate Skip/Continue acknowledgement gates, automatic pass for matching originals, different/additional/unverified outcomes, failed checks, source changes and rechecks.
- `npm run build`: TypeScript and Vite production build passed. Studio's Tauri development build also completed with the embedded reference and relaunched. Existing warnings remain unrelated.
- Real read-only check of `F:\NHL Installations\Xbox 360\NHL07\NHL 07 (USA)`: `status: vanilla`, 292 required files matched, 281 loose files and 11 BIG containers, zero findings. The optional dashboard update was absent as expected. Receipt: `target/nhl07-vanilla-check.json` (local generated artifact); the result is retained here independently of that temporary build directory.

This feature's manual desktop acceptance remains for user review. The previous onboarding acceptance does not automatically qualify these new screens. No game file was modified or container re-extracted during verification.

## Verification speed refinement

The development profile previously compiled SHA-256 without optimization. Optimizing only the `sha2` dependency in development/test profiles reduced the same read-only NHL 07 installation check from 72.228 seconds to 2.807 seconds (about 26 times faster). Both runs matched all 292 required files with zero findings. Measurements are consecutive local runs and include normal OS file caching; other drives and cold reads may take longer. No sampling, metadata-only shortcut or cached validation result was introduced: every original file is still read and hashed. Release builds already use optimization. Local reports: `target/nhl07-check-before.json` and `target/nhl07-check-after.json`.
