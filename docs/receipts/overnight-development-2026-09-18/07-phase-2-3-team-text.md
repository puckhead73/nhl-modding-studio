# Receipt 07 — Phase 2.3 (team text) opening slice; 2.1 disposition

Overnight developer lane, 18 September 2026. Consumes research handoff P2.3-X
(`docs/research/overnight-2026-09-18/23-phase2-team-localization-keys.md`; claim
`p2.3-team-localization-keys`). Status: **implementation complete / automated verified / user
review pending.** Phase 1 implementation coverage was closed before this (receipts 01–06); this
is the first Phase 2 slice, built because its prerequisite (free text in place, receipt 06) had
just landed and the handoff was ready.

## Plain-language summary

A team's on-screen text lives in two places: the database row (`fullname` / `shortname` /
`abbrname`, already editable in Studio's Team Data tab) and the localization dictionary, where up
to nine per-team keys (full name, city, three-letter abbreviation, two-line name, nicknames,
ticker) are all keyed by the team row's `artabbr` — not its display abbreviation. The Team Data
tab now shows those keys with their current text beside the database strings, and each key edits
through the same dictionary panel as league names: point it at another key's text, or type new
text that is re-encoded in place when it fits ("Leafs" → "Buds" holds as a pending change and
saves through every existing route). Which screen reads which of the two sources is exactly the
runtime question the handoff queues; the card says so instead of guessing.

## Implementation

- `crates/studio-domain/src/team_text.rs`: the nine family templates (P2.3-X), `team_text(...)`
  resolving each key exactly or by case-folded hash on the attached dictionary (`absent` when not
  present or no dictionary), with the row's database strings; `EditSession::team_text`.
- `studio-api`: `Registry::team_text`, command `team_text`, `TeamTextDto` (97 export roots).
- App: `TeamTextCard` on the Team Data tab; "edit…" opens the shared `DictionaryRenamePanel`
  (both modes) for that key, gated on an attached dictionary.

## Per-target disposition (2.3 / 2.1 text)

| Family | Team keys | Edit |
|---|---|---|
| NHL 07 → Legacy (Xbox 360 / PS3 / Recomp) with its dictionary attached | all nine templates tried; presence per title as the census says (nicknames 14+, ticker 09+) | LOC-W1 / LOC-W2 in place |
| PS4 NHL 15 → 21 | templates tried against the PS4 dictionary — **join not established by research for PS4** (P2.3-X observed the Xbox family); shown as resolved/absent per key with the same disclosure | same routes; treat resolved keys as a hypothesis until a screen confirms |
| NHL 07 | keys exist (`NHLTeamName/…`) per the census; thin team index still carries `artabbr`? — **not verified** (card lists what resolves) | — |

Not claimed: which screens use dictionary keys vs. database strings; changing `artabbr` itself
(re-keys every family and may drive asset lookups); longer texts (growth path).

## Checks

| Check | Result |
|---|---|
| `cargo test -p studio-api --test membership team_text` | 1 passed (Legacy TOR: name/city/abbr/line 2/nicknames/ticker resolve; in-place edit "Leafs"→"Buds" pending as `retext`; EASHL placeholders absent) |
| `tsc --noEmit` | clean |

## Changed paths

New: `crates/studio-domain/src/team_text.rs`, `app/src/components/TeamTextCard.tsx`, bindings
`TeamTextDto.ts`, `TeamTextKeyDto.ts`. Modified: `crates/studio-domain/src/{lib.rs,session.rs}`,
`crates/studio-api/src/{dto.rs,registry.rs,commands.rs,lib.rs,bindings.rs}`,
`crates/studio-api/tests/membership.rs`, `app/src/lib/client.ts`, `app/src/pages/TeamDetail.tsx`.
