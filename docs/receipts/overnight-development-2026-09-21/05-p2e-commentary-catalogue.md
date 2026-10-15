# Receipt 05 — Priority 2-E: the read-only gen7 commentary catalogue (Phase 1.5 foundation)

**Developer-verified; not Codex-accepted; nothing game-verified; nothing played or decoded.** Branch
`feat/phase5-asset-tooling`, baseline `8d60a6d`. Real installs on `X:` read only. Claim:
`dev-p2e-commentary-catalogue`.

## 1. Plain-language summary

The commentary data of the Xbox 360 / PS3 titles (NHL 11 through Legacy) can now be read inside
Studio: every speech event with its parameters, the sentences that carry an event's name with their
bodies and phrases, and — for each phrase — the sample group it draws from, which event parameters
pick among that group's candidates, how many candidate recordings the group lists and how many the
sound bank actually holds. A new page under Leagues ("commentary") lists it with a name search and a
league/cup filter. The wording is the research contract's: a sentence "shares this event's name",
an event without one reads "no sentence carries this event name", and samples are candidates that
the parameters select among at runtime. Nothing plays, decodes or writes any sound; the titles
whose speech tables live only inside archives (PS3 NHL 12 / 13), the XML titles (NHL 09 / 10) and
PS4 refuse with the exact reason rather than showing an empty list.

## 2. What users can now do

- Leagues → **commentary**: search the title's speech events / sentences by name, filter to
  league / competition names (AHL, CHL, LEAGUE, CUP, SPENGLER, MEMORIAL, CALDER, KELLY,
  INTERNATIONAL, TOURNAMENT), see per event its parameters, whether other names share its 16-bit
  id, and how many sentences share its name; expand a sentence to its bodies and phrases with the
  sample group, the selecting parameters, the candidate count, the bank-resolved count, the
  unresolved ids and the repetition pool.
- Read the exact source files (path, length, SHA-256) the listing was built from.

## 3. Implementation

| Component | Change |
|---|---|
| **new crate** `crates/tdb-speech` (`src/lib.rs` `0a1cfbccd8e12373`) | read-only, bounded parsers: `EVT0` v3.3 / v1.2 incl. the NHL 11 4-byte-magic form (events, parameter table with enum values, secondary lists), `MOIR` (id → names, layout auto-detected by the measured rule, every candidate kept), `SPC0` v5.3 / v4.4 / v4.2 (the FNV-1 slot rule checked per sentence; bodies; phrases as one fixed `(tag, value)` + event-ref tags + pool; the measured v5 shapes enforced — a broken ref head refuses), `GRF0` v3.3 / v2.1 (named NHL 11 entries; `(tag, value, num_tags)` index; malformed entries counted), `SBKR` (sample id → `.sbs` offset), `KWD0` (keyword → sample ids, both endian forms). 3 unit cases (hash, refusals, a synthetic v5 file incl. a corrupted ref). |
| `crates/tdb-speech/tests/research_model.rs` (`9cc29bbbbf8333ce`, `--ignored`) | the research model (`r27-gen7-speech-model.json`) on the **10 loose binary trees** (X360 11 / 12 / 13 / 14 / 15 / Legacy; PS3 11 / 14 / 15 / Legacy): events, parameters, MOIR = events, version, sentences, FNV slot ok, phrases, refs, one fixed pair per phrase, secret-tag census, graffiti table per phrase, refs at the event's index, sentence-has-event, graffiti tables / malformed / distinct refs, bank count / id range — **all equal**. |
| `crates/studio-domain/src/speech_catalogue.rs` (`7ecbb9275c3cdff2`) | coverage gate per title / platform with the exact refusal reasons (archive-only PS3 12 / 13, XML 09 / 10, NHL 07 / 08's older system, PS4); loose `audio/speech` sources with identities; the catalogue (events with parameter names + other names for the id + sentence count; sentences with event id, slot, bodies, phrases resolved to `(group tag, value, name)`, selecting parameter names — a ref whose tag is not the event's parameter at that index is labelled, not hidden —, `table_found`, candidate ids, bank-resolved, unresolved ids); MOIR ambiguities; keyword sample counts; stats; notes; `view()` = query / league-only / cap. |
| `crates/studio-api` (`dto.rs` `d41733e0374e0e67`, `registry.rs` `349a1b5f47fa49d6`, `commands.rs` `af590e984a279d4d`, `lib.rs`, `bindings.rs`) | `SpeechCatalogueDto` (+ source / event / sentence / body / phrase / stats DTOs; candidate ids capped at 64 per phrase with the exact `candidate_count`); `Registry::speech_catalogue(project, root, query, league_only, limit)`; command `speech_catalogue`; 7 new bindings exported. |
| `app/src/components/SpeechCatalogueCard.tsx` (`ad2e0c7509f99a25`), `pages/SpeechCatalogue.tsx`, `routes.tsx` (`leagues/commentary`), `pages/Leagues.tsx` (link), `lib/client.ts` | the page; 3 jsdom cases (`SpeechCatalogueCard.test.tsx` `2da85a6831a69a55`): request args, the contract wording, expansion with candidates / unresolved ids, query + toggle re-requests, refusal text. |
| `crates/studio-api/tests/speech_catalogue.rs` (`3b3cabc47b1752c7`, `--ignored`) | registry seam on the real X360 NHL 15 + Legacy trees: totals (3 714 / 1 489 / 9 829 · 3 982 / 1 718 / 11 465), every sentence on its slot, every phrase with a table, keywords loaded, candidate accounting; the league filter reproduces the research trace exactly (83 / 27 and 88 / 28 league events without / with a sentence; the cup events with a sentence are exactly `CUP_END_CUP_NHL` and `CUP_END_CUP_PRESENTATION_NHL_CONFERENCE`; AHL / CHL / OHL / QMJHL / WHL / TOURNAMENT cups have none); `CUP_END_CUP_NHL` = 29 243; query + cap; NHL 10 refuses with "XML". |

Research consumed (identities): 2026-09-21 handoff 03 rev. 2a (`research-r24-ctx0-structure`,
`ready`) — the operations, wording and refusals; 2026-09-21 handoff 04 (`research-r27-gen7-speech-model`)
— the phrase / graffiti model and layouts; 2026-09-20 handoff 13 rev. 4 (`EVT0` / `MOIR` / hash /
`SBKR`) and handoff 07 §3 (`KWD0` as the team / arena audioid space); the R22/R23 review's
requirements (record bounds parsed, resolution states explicit, `287` not shown as a control —
it is shown as the secret tag id it is). **Not consumed tonight:** `CTX0` context records (handoff
01 — the trigger → conditions → items listing), the NHL 09 / 10 XML form, the archive-member
route for PS3 12 / 13, sample preview (needs a decoder; none exists).

## 4. Checks executed (developer)

| Command | Result | Log |
|---|---|---|
| `cargo test -p tdb-speech` | 3 passed | — |
| `cargo test -p tdb-speech --test research_model -- --ignored --nocapture` | **1 passed — 10/10 loose trees equal the research model** | [p2e-speech-research-model.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2e-speech-research-model.log) |
| `cargo test -p studio-api --test speech_catalogue -- --ignored --nocapture` | 1 passed (NHL 15 + Legacy X360; NHL 10 refusal) | [p2e-speech-catalogue-api.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2e-speech-catalogue-api.log) |
| `cargo run -p studio-api --bin export-bindings -- --write` then `--check` | 7 files written; "Bindings are in sync" | [p2e-bindings-check.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2e-bindings-check.log) |
| `cd app && npx tsc --noEmit -p .` | clean | — |
| `cd app && npx vitest run` (full) | **21 files, 159 passed** | [p2e-frontend-full.log](../../validation/evidence/overnight-development-2026-09-21/logs/p2e-frontend-full.log) |

Not run: PS3 trees through the registry (the crate-level oracle covers PS3 11 / 14 / 15 / Legacy);
any launch; the desktop page (user review). Two research-model figures were **not** re-derived
tonight: the R22/R23 review's "keyword mapped-sample" counts (superseded by the phrase model) and
the CTX0 record counts (not consumed).

## 5. Coverage (matrix 1.5 → this changes the "blocked (format)" cells)

| Operation | X360 11–Legacy | PS3 11 / 14 / 15 / Legacy | PS3 12 / 13 | X360 / PS3 09 / 10 | 07 / 08 | PS4 |
|---|---|---|---|---|---|---|
| List events + parameters, sentences sharing an event's name, phrases → candidate samples → bank | **impl+tested, dev** (crate oracle on the real trees; API seam on 15 / Legacy) | **impl+tested, dev** (crate oracle; API by identity — same files) | blocked — archive-only sources; the archive route is not built (refusal names it) | blocked — XML form (refusal names it) | n/a (evidence) — older system, research model does not cover it | blocked — separate PS4 chain (refusal names it) |
| Contexts (trigger → conditions → items) | not impl (handoff 01 ready) | ← | ← | — | — | — |
| Sample preview / playback | blocked — no decoder | ← | ← | ← | ← | ← |
| Replace / reassign a cue | not impl — no encoder / preservation contract entered | ← | ← | ← | ← | ← |

## 6. Cleanup

Scratchpad `speech_api.py` deleted at checkpoint; no temp roots; no processes.

## 7. Next

P3 Teams (2.7): the team / arena name-call listing — graffiti `(team_ID, audioid)` and the keyword
index join on the Team page (read-only), then the `artid` / `index` re-identification contract.
