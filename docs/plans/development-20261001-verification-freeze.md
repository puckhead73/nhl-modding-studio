# r67 verification freeze — user direction, 1 October 2026

Continue the same developer session only to record and honor this freeze, then stop. This supersedes the unattended feature-continuation queues, including Recompiled portrait/head preview, remaining read routes and expanded board work. The user is beginning the initial beta verification pass and wants a stable baseline. Do not message other sessions.

Read `docs/validation/development-20261001-receipt67-review.md`. r67 is the fixed internal candidate:

- ZIP: `E:\nhlms-release-candidate-20260928\NHL Modding Studio rc-20260928-r67.zip`
- ZIP SHA-256: `6297cda973d2b9368d5809c086d71eb9f72c0ec14aa463493083f73fdfaa0b4a`
- Executable: `E:\nhlms-review-build-r67-2026-09-28\NHL Modding Studio rc-20260928-r67\nhl-studio.exe`
- Executable SHA-256: `60eee0adbd16df95692c4249f9926d7aff998855cf04958ba897fa12d37bbe65`

Record a short developer-owned freeze receipt: exact baseline, pending changes preserved, no further builds scheduled, outstanding limitations and pointers to existing cards. Do not create another executable, relabel r67, change defs/resources, commit, deploy, install or launch anything. Preserve old bundles, the dirty checkout and recovery evidence. No routine refactor, cache migration, extra platform support, dependency update, asset replacement or cleanup while the user tests.

Carry forward these limitations explicitly: speech unreadable/duplicate/partial-tree behavior from the review; PS3 NHL 11–13 board UI absent; writes tested on Legacy/NHL 13 only; outstanding Recompiled routes and guarded Hockey Shop install; Austrian phases/X360 boards/re-key gates; unresolved runtime cards. Do not remove them from scope to call the beta complete. DXT1 opacity is accepted for this pass with its existing disclosure; make no further encoder change.

On a user-reported defect, triage reproduction/build/title/platform and severity. Only resume implementation when the user or coordinator explicitly dispatches the bounded correction. Treat data loss, wrong-target writes, failed recovery, unusable startup/core save or a reproducible blocker of the agreed pass as urgent. Cosmetic issues and new functionality remain queued. Stop the affected workflow while an integrity defect is unresolved; do not require the user to abandon unrelated safe verification.

If a correction is authorized: reproduce on an owned copy, add the relevant regression, fix only that issue, run affected checks, produce a separately numbered candidate with exact hashes/change list and state which previous results need repeating. Never silently replace r67 or make the user restart the entire pass for unrelated changes. Freeze again after delivery.

No claim of external-release readiness: internal artwork approval is unchanged, holder remains pending, and all agreed capability/validation gaps remain separate from packaging gates. Do not execute the uncorrected Recompiled loose-loader card. Stop after the freeze receipt and wait for verification results.
