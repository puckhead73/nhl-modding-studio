# Window overflow correction — r75

User reported clipped onboarding in r74 with a long PS4 detection report. The fixed, vertically centered OpenProject overlay did not constrain or scroll its panel, allowing its top and bottom to extend beyond the viewport.

The panel now has `max-h-full`, `min-w-0` and `overflow-auto`. All onboarding steps share this panel. Content retains its natural size until it exceeds available height; then the panel scrolls within the window. No data handling or project creation behavior changed.

Validation: nine OpenProject tests pass; TypeScript passes. Real Edge browser layout with synthetic discovery containing fifteen installation candidates passes at 1200×800, 900×600 and 640×480. Each case asserts the panel stays inside the viewport, overflow is scrollable, and the bottom Next button can be brought into view. Screenshots of the top and bottom and measured rectangles are retained under `../overnight-20261002/scroll-*`; `scroll-check.cjs` reproduces the check. Browser rendering is not a native desktop/backend qualification.

The previous rendering sweep missed the long fresh-project setup state. This check covers that state explicitly. Earlier r74 artifacts and its running review process are left intact. This correction does not complete the remaining roadmap items listed in the r74 handoff.

r75 production build and extracted archive verification passed. Before/after dirty-input manifests are identical (1,052 inputs; SHA-256 `5387a222ce26f6b697c27174ea270991bf0a6948cabd80fa121d1321eef7f0bb`). Executable SHA-256: `8b0c886da90aaa9bf3b3fa0a77e72542ab323b73b241191c7c067a347d41cd43`. Archive hash is in `archive-hash.json`; bundled resource hashes are in `build-record-input.json`. r74 ZIP hash remains `c1b615dd6bf35bd854fc21a989f78a788f9558e7f1c8306ed5aac46f0b86383f`. The temporary Vite server was stopped. r75 is frozen for review; the running r74 window was not closed or modified.
