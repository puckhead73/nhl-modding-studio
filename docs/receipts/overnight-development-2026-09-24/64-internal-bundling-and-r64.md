# Receipt 64 — artwork kept for internal builds (recorded, not cleared); NHL 11–13 style decision locked; review build r64

**Developer-verified; not Codex-accepted; nothing launched, uploaded or sent.** HEAD `8d60a6d` (unchanged). Claim
`dev-artwork-internal-bundling`. Dispatch after receipt 60 §4 and the S3 addendum.

## 1. Plain-language summary

- **Your 28 September artwork decision is now written into the release tooling.** The existing images stay in internal builds and
  are to be replaced before any external release.
  - The build record, the notices and the verifier all say this build is an **internal review** build.
  - Each image group has a replacement backlog.
  - The decision **records no rights.** Every image stays "rights unrecorded", so external-release mode is still refused.
- **Copyright holder:** no answer is recorded anywhere I can find. `LICENSE` stays withheld (never a placeholder). This is still the
  single missing licence value.
- **NHL 11–13 companion style:** the coordinator decided these values stay numbers. Studio already does that. A test now locks it,
  while the roster cell's accepted labels and NHL 14+ are unchanged.
- **New review bundle r64**, with receipts through 64.

## 2. Files

| Path | Change |
|---|---|
| `tools/release-archive/embedded-resources.json` | `internal_bundling` (the decision, its record, its scope: "a packaging permission, not a provenance or external-distribution-rights finding"); each entry gets `group` and `replacement`; **every `rights` value unchanged** |
| `tools/release-archive/assemble.py` | notices §7 quote the decision and list the backlog per group; the artwork blocker names the decision; BUILD-RECORD gains `distribution` (`internal-review` unless release-ready), `internal_bundling` and `artwork_replacement_backlog` |
| `tools/release-archive/verify.py` | a not-release-ready archive whose `distribution` is anything but `internal-review` is refused; a new self-test case ("distribution relabelled for release") |
| `docs/release/artwork-replacement-backlog.md` (**new**) | six groups, counts from r59 / r64 (17 + 3 + 30 + 1 + 1 + 1 = 53), the action for each; originals untouched |
| `crates/studio-domain/tests/style_label_scoping.rs` | `nhl11_to_13_companion_style_stays_numeric` (X360 NHL 11 / 12 / 13, PS3 11 / 13: the skater-AI companion `p_style` code 3 has no label and no options) |

## 3. r64 — build and verification

| Item | Value |
|---|---|
| Bundle | `E:\nhlms-review-build-r64-2026-09-28\NHL Modding Studio rc-20260928-r64\nhl-studio.exe`, SHA-256 `b7ab3ce5d5a03cb1255a0a1b78d76a04a4608511a5935c997146206fd73577ba` (50 621 952 bytes) |
| Archive | `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r64.zip`, SHA-256 `6f88b5f7adeb36fbdcb324c20389c48ab8c847cc3dd283cbf73a5a4e690c56f5`, 620 files |
| Inputs | HEAD `8d60a6d` + 952 dirty inputs (`80bb06f6…cd65`), identical after the build; built 13:39:55 → 13:42:36 (session clock) |
| defs / frontend / helper | `20c3b183…cf8b` / `f0833e6e…2386` / `dc3533ed…8c21` |
| Contents | receipts through 64: r59 + 61 (Hockey Shop rollback), 63 (PS4 heads, reviewed applies), 64. Receipt 62 is a test only |
| Checks | `verify.py check` passes. `selftest` refuses all 21 cases, including the new distribution case and release mode. The three bundle tests pass from `E:\tmp` (defs, notices, R84 XMA vectors through the bundled helper). Record: `distribution: internal-review`, `release_ready: false`, backlog `{app-icon 1, header-logo 1, platform-tiles 3, rink-backdrop 1, strategy-diagrams 30, title-tiles 17}` |

Gates before the build: bindings in sync · `studio-api --lib` 40 / 40 · `style_label_scoping` 4 / 4.

## 4. Limits

- `release_ready` is packaging eligibility only. It is not acceptance of any feature, test or user observation.
- The About page shows the blockers from BUILD-RECORD, but it has no dedicated view of the backlog.
