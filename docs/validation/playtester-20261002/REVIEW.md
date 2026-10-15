# Self-contained playtester archive — r76

User authorized a single archive containing Studio and all required data/runtime files for extraction and launch by playtesters. This supersedes the earlier separate-corpus provisioning choice for this package. No sending, upload or publication performed. Earlier numbered candidates remain untouched.

Built from r75 functionality plus two portability changes:

- `open_corpus` accepts an empty selection as `corpus/historical.db` beside the running executable. Historical browsing tries this automatically on a clean profile. Explicit user selections remain supported, and a stale selection can be cleared to recover to the bundled database. No machine-specific corpus path is saved for the bundled choice.
- Studio selects `webview2/` beside its executable before creating Tauri/WebView threads. Microsoft's documented AppContainer read/execute permissions are applied only to that runtime directory, allowing unpackaged Windows 10 startup without administrator installation. Normal builds without that directory still use an installed runtime. No renderer sandbox is disabled.

Payload:

- Verified fresh-project inventory and 151 baseline/anchor files, with 27 listed targets and NHL20 still unavailable because its source is an NHL19 alias.
- Historical database snapshot: schema 6, 190,303 people, 117 seasons, 1910–2026. Read-only source connection plus SQLite backup creates a consistent owned snapshot; quick_check passed. No personal formula profiles, import ledger, settings or generated Legends caches are bundled. Default profiles/caches are produced on demand.
- Complete Microsoft Fixed Version WebView2 154.0.4258.53 x64 runtime, 257 files, downloaded from the official Microsoft page. The runtime executable's Authenticode status is Valid and signer is Microsoft Corporation. Original runtime files and embedded third-party-notice viewer are retained. Download URL, CAB hash and signature evidence are alongside this receipt.
- Existing definitions, XMA helper, exact helper source/build configuration, third-party license texts and notices; embedded frontend/artwork; START-HERE.txt instructions; manifest and build record.

Microsoft deployment reference: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution (fixed-version selection, full payload, Windows 10 AppContainer grants, local-folder requirement). This runtime does not auto-update; update it with subsequent playtester packages.

Validation so far: 308 frontend tests pass (54 files), TypeScript clean, 348 bindings in sync, corpus path resolver test passes, native shell test passes, production build passes. The packaged historical snapshot also opens via the real Registry and serves historical filters in an owned disposable copy. Final archive/extraction/startup evidence is recorded below when complete.

Limits: binary is unsigned; existing public-release readiness blockers remain recorded rather than relabeled as resolved. User-authorized playtester bundling is not a new license/right determination. Original game installations, personal saves and game asset archives are not included. Supported optional asset operations still require those matching source files. This package does not finish remaining roadmap features or qualify every game runtime. No user visual/game check is promoted.

## Final evidence

ZIP: `target/internal-candidates/nhlms-release-candidate-20261002-r76/NHL Modding Studio Playtest r76.zip` — 581,874,942 bytes; SHA-256 `876ea83c2f03f1d0add0722b3f19b09cd98fada8f00919e5d8160bdf3584d6b0`.

Extraction and verification passed all 1,031 manifest entries, historical/runtime identities, notices, helper/source identities and baseline inventory. Historical snapshot SHA-256: `d00414d144b98fd1a7a9a434b04659d07618dd4e9f970b10140658ea4e497caa`. Executable SHA-256: `932a05c6b4050c093aaca58bd17869bb7a448ce79d2f7ac64710a7dd1c08e42c`.

Final source manifests match before/after: 1,056 dirty inputs, SHA-256 `fe7959cfa9307fd296749e5d88037d210dc0264a1dd5f2bdb6faa34ced437099`, HEAD `8d60a6d582c1bf208e0a5bc590b28d5add7d5051`. The packaging import generated one new Python bytecode cache; that session-created file was removed before final source comparison.

Native smoke test launched the extracted executable from `H:\TEMP` with a new isolated WebView profile. PID 278068 had a responsive window; its WebView parent/renderer processes came from this extracted bundle's `webview2/msedgewebview2.exe`. This confirms actual bundled-runtime startup; it is not a clean Windows 10 machine qualification or a full interactive desktop acceptance pass. The smoke instance and its children exited, and the isolated profile was removed. The existing user review process was not closed. The disposable corpus-test copy was removed; the archive, stage/extracted trees, official runtime download and development evidence remain.

Archive frozen for the requested playtest distribution. No upload or recipient message was sent.
