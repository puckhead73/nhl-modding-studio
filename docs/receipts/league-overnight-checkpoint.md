# League overnight checkpoint (2026-09-17) — closed

The run finished. Queue state, checks and cleanup are in the consolidated receipt
[league-overnight-2026-09-17](league-overnight-2026-09-17.md); per-target outcomes are in
[league-identity-offline-campaign](../validation/league-identity-offline-campaign.md).

| Package | State |
|---|---|
| O1 / L1.1-B | implemented + checked |
| O2 / L1.1-C | implemented + checked (backend); UI type-checked/built, not executed |
| O3 / L1.1-D | implemented + checked |
| O4 / L1.1-E | implemented + checked (API loop); UI type-checked/built, not executed |
| O5 / L1.1-F | implemented + checked (28 rows) |
| O6 / LOC | implemented + checked (LE + BE fixtures) |
| O7 / EB | implemented + checked (synthetic + real CUSA18182 archives) |
| O8 | done — awaiting Codex review |

Retained: `local/runs/league-overnight-2026-09-17/target/` (build cache for the review). Everything
else in that root was removed. Next action belongs to Codex (independent review).
