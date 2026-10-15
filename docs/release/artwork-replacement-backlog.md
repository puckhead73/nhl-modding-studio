# Artwork replacement backlog (before any external release)

**Status 28 Sep 2026: not started, by decision.** The user allows the existing artwork in internal research / team builds: "for now we
will bundle the assets with it, to be replaced before external release" (recorded in
[beta scope decisions](../plans/beta-20261001-scope-decisions.md), 28 September clarification). That is a packaging permission. It is
**not** a provenance or distribution-rights finding. Every row below keeps `rights: unrecorded` in
[`tools/release-archive/embedded-resources.json`](../../tools/release-archive/embedded-resources.json). External release mode
(`verify.py check --release`) stays refused until each group is replaced, or its rights are separately documented.

| Group | What | Source in the repository | Count in r59 | Action before external release |
|---|---|---|---|---|
| `title-tiles` | title picker tiles: EA NHL box art, user-edited to a matched set (commit 086e0f0) | `app/src/assets/ui/nhl07…nhl21.png`, `nhlleg.png`, `legacyrecomp.png` | 17 | replace, or record rights |
| `platform-tiles` | platform picker tiles: console artwork, user-edited (086e0f0) | `app/src/assets/ui/360.png`, `PS3.png`, `ps4.png` | 3 | replace, or record rights |
| `strategy-diagrams` | line strategy diagrams that appear to be captured from an EA NHL game | `app/public/strategies/*.gif` | 30 | replace, or record rights |
| `header-logo` | Studio header logo | `app/src/assets/ui/NHL Modding Studio Header.png` | 1 | record its provenance (may be first-party), or replace |
| `rink-backdrop` | rink backdrop behind the 3D viewers | `app/src/assets/rink-background.png` | 1 | record its provenance, or replace |
| `app-icon` | application icon inside `nhl-studio.exe` | `src-tauri/icons/icon.ico` | 1 | record its provenance, or replace |

The originals stay in the repository unchanged. Nothing here is deleted, renamed or marked first-party without a recorded answer.
