# Receipt 46 — MIT metadata, in-app About & Licences, the 15 missing licence texts, measured bundle closure, release gate

**Developer-verified; not Codex-accepted; nothing launched, published or uploaded.** HEAD `8d60a6d` (unchanged; nothing
staged or committed). Claim `dev-release-notices-mit`. Direction: [dispatch after receipt 45](../../plans/development-20260928-after-receipt45.md) §1.
The archive built from this work is recorded in the consolidated receipt, not here.

## 1. Plain-language summary

- **Studio's licence is MIT everywhere it is declared:** every workspace crate, the Tauri app and both npm manifests.
  `tdb-speex` says `MIT AND BSD-3-Clause`, because it carries the vendored libspeex. The README now says MIT.
  - **One value is missing:** the copyright holder. No answer is recorded (the user-results file is empty, and the
    decision records name none). `LICENSE` is therefore withheld from archives rather than filled with a placeholder.
  - The standard SPDX 3.29.0 MIT text is staged in `tools/release-archive/`, and the missing value sits in
    `studio-licence.json`.
- **About & Licences in the app:** under Settings, and as a link on the start screen, so it works before a project is
  open.
  - It reads the packaged `LICENSE`, `THIRD-PARTY-NOTICES.txt`, `BUILD-RECORD.json` and every licence file beside the
    executable, offline.
  - It shows whether the build is an internal review build or release-ready, and why.
  - Any file the notices name that is missing is reported by its path.
  - A development build says plainly that it has no packaged notices.
- **All 15 missing licence texts are resolved, by measuring rather than assuming.**
  - **4 of the 15 were never in the app.** A sourcemap build byte-identical to the embedded frontend shows it holds
    20 npm packages, not the 95 listed before; `@mediapipe/tasks-vision`, `draco3d`, `maath` and `stats-gl` are not
    among them.
  - **The other 11 now have texts** from their exact published source, checked against the crate's recorded commit,
    the npm `gitHead` or the NuGet package bytes.
  - `selectors` has no upstream licence file, so the official MPL-2.0 text and its source location are used.
- **Two things receipt 43 missed are now covered:**
  - Microsoft's WebView2 loader is statically linked into Studio; its licence and notice are now included.
  - Tailwind CSS is embedded in the stylesheet, and is now listed.
- **Found and disclosed, not decided:** the executable embeds 17 EA game box-art tiles, 3 console tiles and 30 strategy
  GIFs apparently captured from an EA game, plus a logo, a backdrop and the app icon of unrecorded origin. Studio's
  licence does not cover them. The archive cannot be marked release-ready until their rights are recorded or they are
  replaced.
- **The archive tools refuse a false "ready":** the assembler records `release_ready` and plain-language blockers. The
  verifier also refuses a broken notices link, a missing licence folder, a placeholder `LICENSE`, or a flipped
  status, even when the manifest was rewritten to match.
- **Prerequisites are stated correctly:**
  - Studio needs only Windows system libraries plus the WebView2 Runtime, measured from both executables' import
    tables.
  - Microsoft's guide says the WebView2 Runtime is part of Windows 11 but missing on a small number of Windows 10 PCs.
    The draft page, the README and a new startup message now say so. If the window cannot be created, a message box
    names the error and the WebView2 download, instead of a silent exit. This is unit-tested only; I launched nothing.

## 2. Files

| Path | Change |
|---|---|
| `Cargo.toml`, `crates/*/Cargo.toml`, `src-tauri/Cargo.toml` | `license = "MIT"` (workspace) / `license.workspace = true`; `tdb-speex` `MIT AND BSD-3-Clause` |
| `package.json`, `app/package.json`, both `package-lock.json` root entries | `"license": "MIT"` |
| `README.md` | License section: MIT (Q46e), holder pending, third-party separation, artwork not covered |
| `crates/studio-api/src/about.rs` (**new**), `dto.rs` (`AboutDto`, `AboutLicenceFileDto`), `commands.rs` (`about_studio`, `about_licence_file`), `lib.rs`, `bindings.rs` (root 167) | reader: packaged status, notices, licence files (archive-relative, listed files only, canonical containment), missing files named |
| `app/src/pages/About.tsx` (+ `.test.tsx`), `routes.tsx` (`/about`, `/app/settings/about`), `nav.ts`, `lib/client.ts`, `pages/OpenProject.tsx` | the page; start-screen link |
| `app/src/bindings/AboutDto.ts`, `AboutLicenceFileDto.ts` | generated (`export-bindings --write`) |
| `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` (+ `Cargo.lock` one line) | startup failure message box (`windows` 0.61 — already in the build; no new crate) |
| `tools/release-archive/assemble.py` | measured npm closure (sourcemap build = embedded bundle), supplemental texts, WebView2 SDK texts, embedded-resource rights, Studio licence from config, PE import check, `release_ready` + blockers, refuses to overwrite a candidate |
| `tools/release-archive/verify.py` | notices-link / licence-folder checks, release-status consistency, import check, `check --release`, 6 new self-test cases + release-mode case |
| `tools/release-archive/pe_info.py`, `fetch_supplemental_licences.py`, `studio-licence.json`, `embedded-resources.json`, `LICENSE-MIT.spdx-3.29.0.txt`, `supplemental-licenses/**` (**new**) | as named; `PROVENANCE.json` records URL, basis, SHA-256 and date for each retrieved text |
| `tools/review-build/manifest.py` | build-input roots now include `app/public/` (bundled GIFs), `package-lock.json`, `tools/release-archive/`, `tools/review-build/` |
| `docs/release/draft-download-page-attribution.md` | rewritten: requirements, MIT, helper link table, prerequisites, artwork decision, audit status |
| `crates/studio-api/tests/about_notices.rs` (**new**) | below |

## 3. Evidence

- **Retrieval:** `fetch_supplemental_licences.py` wrote 13 files for 8 keys, then `--check` re-downloaded all of
  them: byte-identical. Sources:
  - crate commits from each crate's own `.cargo_vcs_info.json`; `unic-ucd-ident` was published from a different commit
    (`8a6ce830…`) than the other `unic` crates (`58786053…`), and each is fetched from its own;
  - `@react-three/fiber` from registry `gitHead` `f81bfe8c…` (= tag `v8.18.0`, integrity equal to the lockfile);
  - WebView2 SDK 1.0.3650.58, whose NuGet `WebView2LoaderStatic.lib` equals the crate's (`0659b741…`).
- **Frontend closure:** `npx vite build --sourcemap` into scratch. The JS map names 20 npm packages; the CSS carries
  the `tailwindcss v4.3.1 | MIT` banner; there are 22 image assets plus 30 GIFs.
- **Helper linkage:** packaged `config.h` has zlib, bzlib, lzma, iconv, TLS, SDL, D3D11VA / DXVA2, GPL, VERSION3 and
  NONFREE all 0. `pe_info`:
  - helper: KERNEL32, SHELL32, bcrypt, `api-ms-win-crt-*`, plus a manifest resource from MSYS2
    `windows-default-manifest` ("custom:Public Domain");
  - Studio: system DLLs plus the UCRT, no `VCRUNTIME`, no `WebView2Loader.dll`.
- **Tests:**
  - `studio-api --test about_notices` 3 / 3 (1 ignored real-root case, run at the archive checkpoint);
  - `studio-api --lib about` 1 / 1; `nhl-studio --lib` 1 / 1;
  - vitest `About.test.tsx` 3 / 3; `tsc --noEmit` clean;
  - `export-bindings --write`: 2 new, 286 unchanged.
- **Official sources consulted:**
  - Microsoft Learn, *Distribute your app and the WebView2 Runtime* (doc commit `4964cdf1`, read 28 Sep);
  - SPDX license-list-data v3.29.0 `MIT.txt`.

## 4. Limits and open items

- **The copyright holder** is the one missing input. It blocks `LICENSE` and release-ready status, nothing else.
- **The artwork rights** decision (draft §5) belongs to the user / coordinator.
- **Not verified:** desktop rendering of the About page, the startup message box and any behaviour without WebView2.
- The MPL-2.0 `selectors` entry relies on the notices naming its source location, because upstream ships no file.
- Final distribution review of the draft's tables stays with the user / coordinator. Publisher authenticity (code
  signing) is deferred by Q46.

## 5. Cleanup

The `vite-srcmap` scratch build and the patch scripts are in the session scratchpad, removed at the consolidated
checkpoint. There are no other temporary copies and no running processes.
