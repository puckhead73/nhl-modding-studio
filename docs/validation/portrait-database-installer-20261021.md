# In-app portrait database downloader and installer (21 Oct 2026)

**Summary.** The supplementary portrait library (about 110,000 player portraits with its
`portraits.sqlite` index) is published as one 7z archive on Google Drive. The Studio can now
download it in the background while the operator works and install it where the historical
pages already look for it, or install it from an archive the operator downloaded themselves.
Evidence level: **implemented + automated-tested**, with a live download smoke against the
published file; no game involvement.

## What ships

- **Onboarding offer.** The Mod Studio entry card carries a checkbox, "Download the portrait
  database in the background", shown until it is answered once (checked or not); afterwards the
  offer lives in Settings → General only. Entering the Studio with it checked starts the job.
- **Progress bar.** `PortraitInstallBanner` sits under the primary tabs and above the contextual
  subtabs in the Studio shell (and on the onboarding card): bytes and percent while downloading,
  files extracted while installing, Cancel while running, Dismiss afterwards.
- **Settings → General → Portrait database.** Status (installed / downloaded / paused with the
  partial size / absent), the install location, the archive size against free space, and three
  actions: Download and install (or Resume / Install the downloaded archive), Install from
  archive… (file picker, 7z only), and the download page link.
- **Backend** (`crates/studio-api/src/registry/portrait_install.rs`, one job at a time):
  - download into `<base>/downloads/<archive>.part` in bounded 32 MiB `Range` chunks, each
    retried with backoff; a dropped connection or a cancel keeps the partial file and the next
    start resumes from its size; free space is checked once the size is known;
  - install by extracting the 7z (`sevenz-rust2`) into `<base>/graphical database/portraits.installing`,
    refusing member paths that could escape it, locating the folder that holds
    `portraits.sqlite` wherever the archive nests it, setting aside an existing library as
    `portraits.replaced-<timestamp>`, and moving the new one to `<base>/graphical database/portraits`;
    a downloaded archive is deleted after a successful install, a manually chosen one is kept;
  - `<base>` is the folder beside the open historical corpus, else the bundle's resource root, so
    the result is found by the shipped-layout rule without an override (any explicit root override
    in the selection sidecar is cleared and the cached backend reopened on the next status poll).

Google Drive specifics measured on 21 Oct 2026: the file is
`NHL Modding Studio - Portrait Database.7z`, 31,737,672,102 bytes; open-ended requests may be
answered with Drive's "Quota exceeded" or "can't scan for viruses" page, while bounded `Range`
requests keep returning bytes, which is why the downloader is chunked. The interstitial form, when
present, is followed; the quota page is retried with growing waits and then reported with the
advice to download in a browser and use Install from archive.

## Checks

- `cargo test -p studio-api --lib portrait_install` — 5 passed: interstitial form parsing,
  member-path escape refusal, index-folder discovery, plan reporting + non-7z refusal, and a real
  7z round trip (archive built in the test with a nested top folder; existing library set aside;
  cancelled extraction leaves nothing).
- Live smoke `crates/studio-api/examples/portrait_download_smoke.rs` against the published file:
  first run reached 68.5 MB in 10 s and cancelled; second run resumed from the partial size and
  continued to 93.3 MB; partial file removed afterwards. The full 29.6 GiB download was not run.
- Frontend: `portraitInstall.test.ts` (phase and size helpers); the onboarding suite's read-only
  call filter now also allows the two status polls; full `vitest` suite green before packaging.

## Not covered

- A full-size download and install end to end (hours and ~60 GB of disk at peak). The first tester
  run is the evidence for that; the banner and Settings card expose every failure message verbatim.
- The downloader has no bandwidth limit and no pause other than cancel-and-resume.
