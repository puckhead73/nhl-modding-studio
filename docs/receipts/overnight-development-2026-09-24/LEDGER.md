# League / Team completion ledger — 24 September 2026 (P1 of the overnight brief)

> **Current authority (27 Sep 2026, overnight after receipt 31):** the section "Refresh 27 Sep 2026 — current
> authority" at the end of this file supersedes the 26 Sep refresh where they differ. (26 Sep note, kept:) the section
> "Refresh 26 Sep 2026 — current authority" at the end of this file supersedes every earlier table
> where they differ. The 24 Sep tables and the 25 Sep refresh are kept as history; two of their
> citations are withdrawn research and are struck through in place. **Phase 1 and Phase 2 are not
> complete**: the concrete gaps are listed in that section.

Built from the [Phase 1 matrix](../../validation/phase-1-coverage-matrix-2026-09-18.md) (with its
R1–R4 addenda), the [Phase 2 matrix](../../validation/phase-2-coverage-matrix-2026-09-20.md) (with
its 21–23 Sep addenda), the research queue of 2026-09-22 pm (rows 0–13, R79–R81 corrections read
first) and the code as it is in the working tree at HEAD `8d60a6d` + uncommitted campaigns. Every
"done" below is **developer-verified only** (cargo / jsdom); nothing is Codex-accepted, desktop-
exercised or game-verified except NHL 21 E1 (closed). No percentages.

Vocabulary: **user-facing complete (offline)** = reader + editor/writer + API + UI + a meaningful
automated case per family, runtime acceptance pending; **internal only** = tested code with no API
or UI, or a listing/catalogue with no editor; **blocked** = the exact missing contract or fixture is
named; **n/a** = evidence-backed not applicable; **untested** = the code path exists for the family
but no case runs it there. Runtime-only blockers are listed once at the end and do not freeze the
offline rows.

## Phase 1 — Leagues

| Op | Family coverage | State | Concrete missing work |
|---|---|---|---|
| 1.1 League fields on the team row, league code → key, LOC-W1 rename, placements (loose / copied / archive member / disc image), save-reopen proof | all 27 targets; X360 07 through the stock-row selector (21 Sep) | **user-facing complete (offline)** | runtime cards only (which copy the console loads; disc images plan-only until reviewed) |
| 1.2 Conference / division names + validated moves | gen7 baked tables, PS4 per-title catalogues (77 / 78 derived), X360 / PS3 08–11 raw numbers | **complete (offline)**; 08–11 names **research unknown** | NHL 17 id mechanism, 77 / 78 names — research |
| 1.2 League / competition row creation, cloning, deletion | none | **not impl / blocked** — league tables carry no spare rows on the measured titles; engine tolerance of a grown table unmeasured; no reference / allocation contract | research contract: table growth + every consumer (`kOtt`, schedules, groups, dictionaries, art) |
| 1.2 Rules / season / playoff / tournament settings | gen7 10–15 / Legacy: **blocked** (compiled table in `default.xex`, refused with the reason); PS4 15 stored defaults (21 legacy ids, D3); PS4 16–21 CSV stored defaults incl. the 18–21 **paired** `setting.csv` + `_DEFAULT_` edit (D4, `settings_csv_route.rs`) | PS4 **complete (offline)**; gen7 **blocked** | gen7: locate a data-file copy or accept exe patching (research); PS4: which source wins at runtime (card) |
| 1.2 Schedules: browse / validate / edit / import / export, held-review apply | every family incl. NHL 07 variants | **complete (offline)** | — |
| 1.2 Schedule generation | Legacy SHL / `czechschedule` round-robins | **partial**: NHL / Swiss / AHL **blocked** — the stock division-weighted format is not established | research: the weighted format per title |
| 1.2 Tournament schedule ↔ league binding (`championsschedule`, `spengler`, `wc`, `worldchamp/junior`) | browse only | **research unknown** | research: slot semantics |
| 1.2 Season / Franchise copy of the schedule | none | **research unknown** (save side) | research |
| 1.3 Dictionary re-point / re-text / growth / rebuild, other locales, glyph static gate | all committed dictionaries + 100 loose locale files; X360 07 **blocked** (labels not in dictionary) | **complete (offline)** | PS3 08 / 09 / 10 / 12 glyph coverage unmeasured (fonts in-archive) — bounded developer measurement possible; runtime: which copy loads, rebuilt pool, glyph rendering |
| 1.4 League logos + imagery: listing, preview, same-size replace (loose), archive placement (EB / BIGF / BIG4 + `.bh`, PS4 `inxp_10.big`), dedup private copy, undo / restore | all families; the loose route re-swept today under the reviewed signature (5 real sweeps + 3 real GNF, `p0-f1-private-fixtures.log`) | **complete (offline)**; **P0 today**: the write is bound to the displayed review (receipt 01) | — |
| 1.4 Different dimensions | X360 12–Legacy XPR2 (64 / 256 wide), PS3 12–Legacy DDS, PS4 GNF (rule + real members), PS4 15 PNG: impl+tested; **X360 07–11 / PS3 08–11 RW4 packages: same-size only, refuse by name**; Xbox mipped loose rasters: partial (same-size) / refused (changed size) | **partial, explicit** | RW4-in-BIGF: whether the loose RW4 descriptor rule + the Apt quad rewrite apply inside the 07–11 package form is **unmeasured** (research); Xbox packed tail: H1 is a candidate (Q-R65-1), refusal stays |
| 1.5 League / cup audio | gen7 commentary catalogue listing (X360 / PS3 11–Legacy); PS4 `0XTC` contexts listing (15–21); league-keyed records located in the generic banks (researched absence of a separate bank, **not** n/a) | **internal-facing listing complete; replacement blocked** | P4: diagnostic decode (EA-MP3 PS4 15–17; gen7 EALayer3 v1 on the R80 tuples); replacement needs an encoder + event / bank mutation contract |

**Oldest actionable Phase 1 gap, offline:** none without a new contract. Every open Phase 1 row is
research-unknown, runtime-gated, or blocked on a named format fact. (The stale wording in matrix
row 1.4 "Replace with different dimensions — X360 12–15/Legacy: blocked (RW4xb2)" describes the
pre-C6 state; C6 / receipt 13 implemented XPR2 resizes there. Noted, not edited into the matrix by
this lane — flagged for the coordinator.)

## Phase 2 — Teams

| Op | Family coverage | State | Concrete missing work |
|---|---|---|---|
| 2.1 Names / abbreviations / colours (profile cells, one undo step, draft ownership) | all; 07 / 08–10 no colour fields (n/a, measured) | **complete (offline)** | runtime cards T-1 / T-2 |
| 2.1 Index renumbering → refused with consumer counts; art id → swap between two existing teams, one undoable step; bare cell refused | Legacy X360, NHL 14 PS3, NHL 21 PS4 named cases; UI `TeamIdentityCard` (10 jsdom cases incl. ownership) | **complete (offline)** under a research **proposal** (handoff 10, not Codex-accepted) | title breadth of the named cases is 3 families — a per-family sweep over the anchors is **untested** elsewhere; the asset side of a swap is by construction (both art sets exist), stated |
| 2.2 Lines / strategies / draft picks / retired numbers / jerseys / equipment tabs | surfaces build on all 27 anchors (listing probe) | **listing + generic edit path exist; parity untested** — no named edit → save → reopen → restore case per family | **T1 (this campaign)**: named parity cases; side-effect scope shown before mutation |
| 2.2 Team → arena assignment (`kOtt.buDa` picker) | picker exists on titles with the link (not NHL 09 / 11); tenants join on the arena page | **editor exists; shared-arena disclosure on the assignment not established; no save / reopen / restore case per family** | **T1**: disclose tenants of the target arena and what the old arena keeps, before mutation; parity cases |
| 2.2 Team / league creation, cloning, deletion | none (jersey cloning exists as `clone_jersey`) | **not impl** — needs a complete reference / allocation contract (row capacity, `team_id` consumers, art id allocation, schedules, dictionaries, art files) | research contract; inventory kept honest here |
| 2.3 Team text through the project's own dictionary; shared strings disclosed on re-point / re-text | gen7 + PS4 anchors; X360 07 blocked | **complete (offline)** — `dictionary_edit.rs` names the other keys sharing a string | runtime: which screen reads which form |
| 2.4 Team logos: listing / preview / same-size replace / dimensions (12–Legacy, PS4) / archive placement / dedup refusal | all; 07–11 same-size only by name | **complete (offline)** | runtime card T-3 |
| 2.5 Venue: DB identity, venue model / jumbotron join (gen7), PS4 venue listing by vault level (R73, baked table + live folders), bounce-map candidate, no reassignment | gen7 + PS4 15–21 | **read-only complete**; centre-ice / rink `rendering/*` write exists with **loader acceptance not established** (R30-LZX-1) | no new offline work without a contract; reassignment not offered by decision |
| 2.6 Kits: gen7 join + Uniforms card (X360 / PS3 11–Legacy loose; PS3 12 / 13 from the render archives with member preview); loose component replace through the Uniforms outfit list (jersey / font / pant / sock / nameplate) | gen7; PS4 kit tables listing; PS4 two-pack selection = vault attribute | **inspection complete (gen7); replacement: loose complete, archive members read-only** | **T2 (this campaign)**: PS3 12 / 13 member replacement through the existing EB member-edit route — members are **`chunkzip`** (R76; receipt 04's "no chunklzx encoder" blocker does not describe them); NHL 11 / 12 sock pairs: both required, per-pair class **not** in the R74 data (aggregates only) → a single-sock replace cannot claim the mirror; PS4 duplicate pack triples stay unresolved |
| 2.7 Audio: gen7 name-call listing (15 / Legacy), commentary catalogue, PS4 song catalogue with dictionary titles, PS4 `goalhorn` name | listing only | **listing complete; decode / preview / replace blocked** | P4 diagnostic previews under R77 / R79 / R80 (gen7) and rev. 2 (PS4 EA-MP3); horn sources R78 / R81 |

## Runtime-only blockers (user cards, not offline work)

Which copy a console loads (loose vs archive vs image); dictionary rebuild / glyph rendering;
resized-texture acceptance (R31 / R32 / R33 cards); Xbox packed-tail partial replacement at
distance (R32-XB2-3); `chunklzx` store-route acceptance (R30-LZX-1); every membership / schedule /
settings change's in-game effect.

## Selected packages (in order)

1. **T1 — Teams 2.2 relationships parity**: arena assignment with tenant disclosure before
   mutation + named save / reopen / restore cases per family; team-tab edits (retired numbers,
   draft picks, strategies) parity cases; creation / cloning / deletion inventoried as not impl.
2. **T2 — Teams 2.6 PS3 NHL 12 / 13 kit member replacement** through the EB member-edit route
   (chunkzip, RW4ps3), with the receipt-04 blocker wording corrected by a dated note.
3. **P4 — Teams 2.7 diagnostic audio**: decoder dependency decision → PS4 EA-MP3 → gen7 EALayer3
   v1 on the R80 tuples → R78 / R81 horn sources → catalogue / preview integration.

## Outcome of the selected packages (end of campaign, 24 Sep 2026 — developer-verified only)

| Row | Before | After |
|---|---|---|
| 2.2 Team → arena assignment | editor without disclosure; no parity case | **user-facing complete (offline)**: tenant / vacancy disclosure before the write; 15 anchors save → reopen → undo; 07–12 measured without the link (receipt 02) |
| 2.2 Retired numbers / draft trades / strategies | listing + generic edit; parity untested | **impl+tested (dev)** on 26 / 18 / 27 anchors with save → reopen → undo; absences measured (NHL 07 `wcSL`, 07–10 draft) (receipt 02) |
| 2.2 Team / league creation, cloning, deletion | not impl | **not impl** — still no reference / allocation contract |
| 2.6 PS3 NHL 12 / 13 archived kit components | read-only (stale `chunklzx` blocker) | **replacement route implemented (dev)** through the archive-member route; synthetic two-archive proof + real member planned; loader acceptance = card R36-PS3-KIT-1 (receipt 03) |
| 2.6 loose kit components on the kit card | thumbnails only | **Replace…** through the reviewed loose route; NHL 11 / 12 sock pair deliberately not offered file-by-file (receipt 03) |
| 2.7 / 1.5 PS4 speech (NHL 15–21) | no reader | **bank listing + codec census + diagnostic EA-MP3 preview (dev)**; 24 / 24 research vectors; not listened to (receipt 04) |
| 2.7 / 1.5 gen7 speech (measured tuples) | listing only | **diagnostic EALayer3 v1 preview (dev)** on pbp / ann through the name-call chips; 116 / 116 research vectors incl. stereo music at the codec layer; archive-held streams and music banks not previewed (receipt 05) |
| 2.7 gen7 horns / chants on the arena row | numbers | **file / archive-member / candidate / no-binding readings + inspected-sources residual text (dev)**; banks not decoded (receipt 05) |
| 1.4 texture Replace (all families) | plan displayed, write unbound | **write bound to the displayed review** (receipt 01) |

Still open after this campaign (unchanged from the tables above): audio replacement / encoding / event and bank mutation; Xbox packed tail; creation / deletion contracts; schedule generation formats; gen7 settings; runtime and listening acceptance of everything.

**After the closeout (receipts 07 / 08):** the gen7 preview is reachable from every measured catalogue row (catalogue phrases, team and player name calls) and resolves archived banks / streams (PS3 09 / 10 / 12 / 13 banks) — the open audio rows are now: PS4 announcer / other banks (codec census unmeasured), gen7 music listing card (the bank previews through the API; no catalogue surface lists its samples), the `.abk` horn banks (S10A not read), and every replacement / mutation contract.

**Receipt 09:** gen7 banks (pbp / ann / music) list with header-measured codecs and preview on the measured tuples; the music listing card gap is closed. Open audio rows: PS4 announcer / other banks (codec census), `.abk` horn banks (S10A), all replacement / mutation contracts, listening acceptance.

## Refresh 25 Sep 2026 — against R88–R101 and the corrected blocker map (developer-verified only)

Read: research 2026-09-24 handoffs 06–08 + DEVELOPER-QUEUE (with its dated notes), 2026-09-25
DEVELOPER-QUEUE rows 1–12, R97 correction checkpoint (handoff 10), R98–R101 (handoffs 11–13), and
both 25 Sep direction audits. Corrections used as stated there (R88 counts, R95 → R97 chant wording,
R93 "near-silent" wording, R96 loud-frame scope).

| Row | Research now | Developer state after this refresh | Class |
|---|---|---|---|
| 2.7 / 1.5 gen7 `.abk` banks + `.ast` streams (loose) | R89 tree, R91 framing, R92 decode contract, R94 key split | **listing + tree + framing + PS3 diagnostic decode delivered** (receipt 22); X360 framed only | ready → done (dev) |
| same, archive-backed PS3 09 / 10 / 12 / 13, X360 09 | R98 framing holds; R100 decodes with exact counts | refused in words; **next package** (existing archive-member resolution from receipt 08) | ready → **done 26 Sep (receipt 24)**: PS3 09 / 10 / 12 / 13 listed / framed / previewed from own members, R98 census + R100 vectors reproduced; X360 09 still refused |
| cc 3 / 5 de-interleave, loop stub + body preview | structural evidence + R92 / R96 vectors | listed / framed, not decoded | ready, bounded → **done 26 Sep (receipt 25)**: per-pair and stub + body diagnostic previews; R100 / PREVIEW_REVIEW vectors reproduce |
| EA-Speex (R83) / EA-XMA (R84) previews | contracts measured; decoders = external implementations | **blocked on a decoder dependency decision** (Speex UWB; XMA2) — not taken without review | decision |
| chant wording on the arena page | R97 replaces R95: club banks 055–066 exist; only the Finnish arena's chant id equals a club number (61, as are its horn ids); no selecting consumer traced | current note says "candidate"; must not cite the withdrawn "only 61 reaches a club bank" sentence — **small wording package** | ready → **done 26 Sep (receipt 24)**: R97 §1.4 wording, chant ids never resolved or linked; gen7 goal / period horns link to their bank (PS3 09–13 archive members included) |
| 2.1 art-id re-key (R86 → R88 §F3, R99 / R101) | ~~archive entry = name + djb2 hash word~~ (**withdrawn**, 2026-09-25 handoff 14 §F1; see Refresh 26 Sep); kit / nameplate names `{style}_{teamartid}_{variant}`; in-payload references, PS3 12 socks, other titles' censuses open | swap-between-existing stays; re-key **not impl** (default refuse) | research-owned (R99 next task) |
| 1.2 / 2.2 creation (R87 + R88) | gen7 packed (no row slot); ~~PS4 18–21 non-NHL only, with a named row set~~ (**withdrawn**, 2026-09-24 handoff 06 §F2; see Refresh 26 Sep) | **not impl** — needs table growth (Q-R87-1) and a whole-graph contract | contract missing |
| 2.6 kit name keys (R101) | nameplates keyed like jerseys; rink logo per (style, variant) | the kit resolver already keys jerseys; nameplate keying is a small inspection gain | ready, low impact → **done 26 Sep (receipt 26)**: association shown on the six R103 trees, counts reproduced; not part of completeness |
| 1.4 / 2.4 Xbox packed tail | H1 candidate | mipped resize stays refused | unchanged |

**Order taken:** loose sound banks (done, receipt 22) → archive-backed banks → chant wording →
cc 3 / 5 and loop previews → nameplate keying. Speex / XMA previews wait on a dependency decision;
creation / re-key wait on research contracts.

## Refresh 26 Sep 2026 — current authority

Read: the coordinator's packages 23–26 review (`docs/validation/development-20260926-packages23-26-review.md`) and
R113–R118 review (`docs/validation/research-20260926-r113-r118-review.md`); research 2026-09-26 handoffs 03, 04,
05–09, 11 (R117, the research-side reconciliation of this ledger), 12 (R118), 13 (R119) and checkpoint 00c;
developer receipts through 27. Classes: **done (dev)** = developer-verified, runtime acceptance pending;
**implementation-ready** = a bounded contract exists for the named scope; **proposal** = research design, not an
implementation contract; **candidate** = an inferred reading kept with its competitors; **runtime-blocked** = only a
console observation decides; **contract missing** = a named research / writer contract is absent; **decision** = a
coordinator / dependency decision is required.

| Row | Evidence now | Class | Exact dependency / next step |
|---|---|---|---|
| 1.5 / 2.7 gen7 sound banks — loose (PS3 / X360 11–Legacy) and archive-backed PS3 09 / 10 / 12 / 13 | R89 / R91 / R92 / R98 / R100; 00c: every `.abk` / `.ast` member stored unwrapped; R119: R98 "records" = table slots, R82 = present records (difference = empty slots, mostly `fe.abk`) | **done (dev)** (receipts 22, 24, 25) | listening / runtime cards only (A-AST-1; V-64–V-67); optional display gain from R119: "N sounds (M empty slots)" |
| X360 NHL 09 banks | R98b header variant; R100 three vectors decode | **refused** in the listing | framing of the variant in Studio + the decoder decision below |
| EA-Speex previews (X360 11–Legacy ann / 11–14 pbp, PS3 14 pbp, PS4 15–21 rink announcer) | receipt 29: pinned libspeex 1.2.1 + C shim; 96 R83 vectors within 1 LSB of the libspeex oracle; 10 gen7 tuples + 14 PS4 banks through the domain | **done (dev)** incl. X360 NHL 14 `ann_eng` since receipt 31 (SBKR reader skips 8 empty slots); Recompiled not admitted (no vector) | card A-SPX-2 (listening) |
| EA-XMA (X360) previews | receipt 30: RIFF XMA2 + controlled helper runner; 83 R84 vectors exact through the runner (local oracle as helper); X360 NHL 10 → Legacy ram records wired, framed-only without a helper | Studio side **done (dev)**; ~~the helper binary blocked (install not authorised)~~ — superseded 27 Sep: **helper built (dev)**, receipt 32 (pinned LGPL FFmpeg 8.1.3, reproducible, 83 / 83 PCM bit-identical, internal bundle only); ~~`.ast` XMA (R92), SNS speech / music listing not wired~~ — superseded 27 Sep (receipt 33): **SNS speech / music listing + preview done (dev)** on X360 NHL 09 / 10 speech and 09–12 music (8 tuples, SNS + PCM = R84), **`.ast` XMA streams done (dev)** for the R92-measured shapes (non-loop mono / stereo on NHL 10 / 12 / 15 / Legacy; loop channel-config-3 pairs on NHL 15; WAV = R92), Cancel + missing-helper states; still refused: X360 NHL 09 banks (R98b / R100 header variant + archive source), NHL 11 / 13 / 14 `.ast` streams, looping / multi-segment ram, Recompiled | X360 NHL 09 banks (bounded, next audio item); cards A-XMA-2 / A-XMA-3 / V-75 on the r33 bundle; release obligations in receipt 32 §6 |
| Arena chant wording; horn → bank links | R97 §1.4; R81 / R98 member lists | **done (dev)** (receipt 24) | the chant selector is untraced — research / runtime |
| 2.6 nameplate association | R101 / R103 | **done (dev)** (receipt 26) | optional: the NHL 13 PS3 internal-name mismatch behind disclosure (card K-NAME-1) |
| 2.6 rink-logo keys | R103: positions 2–3 remain candidates | **candidate** | a static hypothesis (research) |
| 2.1 art-id re-key / archive rename | EB row = `block<<4, 0, size, djb2(path)`; the name lives in a fixed-width name-table slot (handoff 14 §F1, replacing "name + djb2 hash word"); six readiness layers; a same-length, same-directory outer rename is **designable offline only** (2026-09-26 handoff 03 as corrected: pool = absence from the enumerated sources, centre-ice logos excluded, CRC updates in the byte mask); loose copies are byte-identical aliases on Legacy / NHL 15 (R103b) | **contract missing** | a writer contract covering every layer and both copies; swap-between-existing stays the only route; re-key refused by default |
| 1.2 / 2.2 league / team creation, cloning, deletion | "PS4 18–21 non-NHL only, with a named row set" is **withdrawn** (handoff 06 §F2: stock rows are what a season ships, not requirements); gen7 main DB packed (no spare row); remaining: measured spare capacity + a proposed experiment | **contract missing** | a table-growth / whole-graph reference contract (Q-R87-1) and the runtime experiment; nothing implemented |
| 1.2 schedule generation — equal round robin | Studio generator (`schedule_edit.rs`) | **done (dev)** | — |
| 1.2 schedule generation — grouped European formats (Swiss, Finnish, Austrian, NHL 07 Swedish), stock-shaped or user quotas | R113 partitions + quotas + validator (19 sources); R114 formula (total = upper-triangle sum; zero diagonal, non-negative, symmetric — 00c), exact row capacity, 4-bit slots, date-order exceptions, same-day double bookings in two stock tables | **proposal** | coordinator acceptance of handoff 08 (with its refusal list) as an implementation contract; the Austrian two-phase `round = 1` block is **runtime-blocked** for meaning (measured tag, unverified phase semantics) — a generator must refuse the stock-shaped Austrian option or choose explicitly |
| 1.2 schedule generation — NHL division-weighted | R13 era quotas (NHL 18–21 confirmed on the real NHL 20): games per opponent class, **not** which pairs | **proposal** | a pair-assignment rule (research) + the same acceptance; the 24 Sep "format not established" is superseded — quotas are established, the pair rule is not |
| 1.2 schedule generation — AHL / CHL / ECHL | no stock rule | **proposal** (user-supplied quotas only) | as above |
| 1.2 schedule generation — **user-supplied pair quotas** (any league table with the game layout) | R114 §2 "user-supplied" (matrix arithmetic as corrected 03:43) | **done (dev)** 27 Sep (receipt 34): symmetric matrix, total = rows to fill, kept rows, stated home / away rule, no double booking among generated rows, deterministic, review / undo / redo / save-reopen; UI grid | card V-76; stock-shaped partitions and the Austrian two-phase writer stay **proposal** |
| 1.2 generation over round-tagged rows (PS4 18–21 Austrian `round = 1`) | R114 §3 | **fixed (dev)** 27 Sep (receipt 34): both generators refuse instead of rewriting `round` to 0; edit / import keep it | — |
| 1.2 tournament tables (`championsschedule`, `spenglerschedule`, NHL 17 `wcschedule`) — browse labels | R115: slot classes (participants / placeholders / seeds), WC slot = rank by `index` (consistent join), Spengler placeholder reading, CHL participant **candidate** (`ischampionsleague`, 32 clubs, PS4 18–21) | **done (dev), read-only** (receipt 27: `tournament_labels` 3 / 3; unbound, no editor / import / generation) — round names, "to be decided", seed labels, WC group stage by name, Spengler placeholders labelled as an interpretation | card V-70 (desktop); slot → club assignment for CHL / Spengler is **candidate / runtime-blocked** and not offered |
| 1.2 Season / Franchise copy of the schedule | 18 Sep P1-B handoff 21 §2: 44 / 46 day-1 NHL mode saves hold a copy corresponding to the stock NHL / AHL source (exceptions A–C); Franchise preseason rows in no source; no European-league specimen | **stock correspondence observed; edited-input behaviour runtime-blocked** (card P1B-3) | no longer "research unknown"; a Studio surface would be read-only correspondence, not a sync |
| 1.2 conference / division names NHL 08–11 | unchanged since 18 Sep | **research unknown** | a name source |
| 1.2 group names 77 / 78 | R36 §2 derived labels | **done (dev)** (per-title catalogues) | — |
| 1.2 gen7 rules / season settings | compiled in `default.xex` | **blocked** | a data-file copy or accepted exe patching |
| 1.4 / 2.4 RW4-in-BIGF resize (X360 07–11 / PS3 08–11); Xbox packed tail | unmeasured / H1 candidate | **refused by name** | research measurement |
| Player encoding (R116 observation) | declared ranges confirmed on 19 sources; birth month / day 1-based NHL 07–10, 0-based NHL 11+ = the contract | **no change** | the `assets.rs` doc comment "NHL 10 only" lags the code — wording only |
| Players 3.1 nationality labels (C16 / R120b) | national-team rosters give per-title codes (NHL 08 / 09, 10, 11 / 12, 13+); NHL 08 / 09 display already matched via the NHL 07 table (schema-shape override) | **done (dev)** (receipt 28: red NHL 10 raw 81 = "Croatia" → green on 25 anchor directories; import lookup and birth-state country region follow the title) | card V-72; unlabelled codes on NHL 08–12 (incl. 127) stay numbers until a title-owned source exists |
| Players 3.1 identity ranges on real NHL 20 / NHL 21 (C16 D2) | `intlcountry` / `state` declared 0..255 | **done (dev)** (receipt 28: edit / pending / refusal / import / save → reopen at 200; labelled dropdowns gained a stored-code box) | card V-73 |
| Players 3.1 zero-height placeholders (C16 D3) | 180–185 unnamed rows with height and weight 0; one named rostered NHL 12 player | **unresolved** — raw 0 = minimum encodable height, shown as 4'6"; not reinterpreted | research: a title-owned "missing" marker, or a user decision |
| Goalie Overall consistency input on PS4 | receipt 28 per-player comparison: +36 reproduces PS3 twins on PS4 NHL 15 / 16; mixed 17–19; worse 20 / 21 | **proposal** (not implemented) | coordinator review of the scoped +36 on PS4 15 / 16 |
| Equipment 4.5 goalie gear — database assignment (`maskid` / `masktype` / `maskpattern` / `maskrank` / `pads` / `blocker` / `trapper` / `stick` / `neckprotector`) | receipt 31: 25 anchor directories with the goalie companion + genuine NHL 20; stored numbers (no catalogue on any title) | **done (dev)** for database parity; catalogue / asset chain **contract missing** | research: which asset / catalogue each goalie id selects per title; card V-74 |
| Players style labels on the roster cell / Lines rail (R122 S1 / S2; research handoff 15) | three numbering eras (7 / 9 / 10 styles) | **fixed (dev)** 27 Sep (receipt 35): NHL 08 / 09 roster = NHL 07 vocabulary, NHL 07 / 10 numbers, NHL 11+ Legacy; rail from the domain | S3 (companion labels on NHL 11–13) = coordinator decision; card V-77 |
| Players 3.2 goalie `consistency` (R118 / C15) | stock 0 / 1 / 2 on every title; declared 0..3 / 0..63 / 0..99 | **done (dev)** (receipt 27) | meanings of the three values and the in-game effect of other declared values — research / runtime |

**Concrete League / Team gaps (why Phases 1–2 are not complete):** schedule generation beyond equal round robins
(proposal → contract acceptance); creation / cloning /
deletion (contract missing); re-key / archive rename (contract missing); rink-logo keys (candidate); NHL 08–11
conference names (research); gen7 rules (blocked); RW4-in-BIGF resize (unmeasured); X360 09 banks and the Speex /
~~XMA previews (decided 26 Sep, implementation pending)~~ XMA on X360 NHL 09 banks only (Speex and XMA previews delivered 26–27 Sep, receipts 29–33); every runtime card. Tournament browse labels are delivered
(receipt 27, read-only) and no longer a gap.

**Order from here (updated 26 Sep, after the receipt-27 review):** (1) the receipt-27 corrections and C16
nationality labels; (2) the stable user-review checkpoint; (3) Speex preview, then the XMA2 helper; (4) any new
implementation-ready League / Team contract, else goalie gear 4.5. Done since the previous order: the decoder
recommendation, tournament browse labels, gloves 4.2 / helmets 4.3 / skates 4.4 (receipt 27).


## Refresh 27 Sep 2026 — current authority (developer, overnight after receipt 31)

Supersedes the 26 Sep refresh where they differ; the 26 Sep table and its dated notes stay as history. Read: research
2026-09-26 handoffs 15–25, `BETA-HANDOFF.md` (B1–B15), `CHECKPOINT-2026-09-27.md`; developer receipts 32–38. Levels:
**dev** = developer-tested (cargo / jsdom, real trees read-only, disposable copies); nothing here is Codex-accepted,
desktop-observed, listened to or game-accepted unless it says so. "Bundle" = the review bundle that contains it.

| Row | Present behaviour (targets) | Level | Evidence | Owner / dependency | Next action |
|---|---|---|---|---|---|
| XMA2 helper | pinned LGPL FFmpeg 8.1.3, reproducible (`dc3533ed…`), 83 / 83 R84 vectors PCM bit-identical | **dev** | receipt 32 | developer; release: ~~distribution arrangement~~ **decided 27 Sep** (beta decisions 3): in the one release archive with its exact source, licences, notices and build record | archive assembly + verification (receipt 42) |
| XMA previews — speech / music banks | X360 NHL 09 / 10 speech, NHL 09–12 music (8 tuples) list + preview; missing-helper banner, row reasons, Cancel | **dev** | receipt 33 (SNS + PCM = R84) | developer | A-XMA-3, V-75 |
| XMA previews — sound banks | ram records NHL 10 → Legacy; `.ast` streams of R92 shapes on NHL 10 / 12 / 15 / Legacy (NHL 15 loop pairs); **X360 NHL 09 all banks** (471 streams + 1 901 records exact, 2 short refused) | **dev** | receipts 30, 33, 36 (WAV = R92; counts = R131) | developer | A-XMA-2 / A-XMA-4; two-segment records and NHL 11 / 13 / 14 `.ast` shapes need vectors (research) |
| EA-Speex previews | X360 NHL 11 → Legacy, PS3 14 pbp, PS4 15–21 rink announcer | **dev** | receipts 29, 31 | developer | A-SPX-2 |
| EALayer3 music | + PS3 NHL 09 / 10 / 12 / 13 (archive-backed) | **dev** | receipt 36 (12 R82 vectors) | developer | A-EAL3-2 |
| Legacy Recompiled audio | refused by name (no loose tree; R128 identity is database / archive bytes) | refused | receipts 29–36 | developer + research | a Recompiled source route (archive members) would need its own tuple regression |
| 1.2 schedule — equal round robin | unchanged | **dev** | — | — | — |
| 1.2 schedule — user-supplied pair quotas | any league table with the game layout; R123 Q1–Q4, Q6–Q8, **Q11 (host rule) / Q12**, R1, R3–R9; UI grid | **dev** | receipt 34 (+ addendum); **receipt 41** (27 Sep): R2 enforced in both generators — kept rows stay in place with every value, kept games block their clubs' dates, refusal otherwise | developer; ~~**coordinator**: accept R123 / decide R2 (kept-row position vs date merge)~~ R2 divergence corrected (coordinator dispatch after receipt 40 §2) | V-76 (recipe corrected, receipt 41 §5), P1-A-q |
| 1.2 schedule — stock-shaped partitions, Austrian two-phase | refused (round-tagged tables refused by both generators) | proposal | R123 (not accepted) | coordinator → developer | accept R123 Q9 / Q10 |
| 1.4 / 2.4 logo resize, old titles | X360 NHL 07 → 11 and PS3 NHL 08 → 11 package form: full-quad families resize; crops / quad-less / untagged / linear / mipped refuse by name | **dev** | receipt 37 (4 634 / 4 634 descriptors reproduced; fixture sweep 168 packages) | developer | T-3b (game); BIG4 re-placement for PS3 08–10 archive members is the archive route |
| Players — style labels (roster cell, Lines rail) | per title: 08 / 09 NHL 07 set, 07 / 10 numbers, 11+ Legacy | **dev** (defect fixed) | receipt 35 | coordinator for S3 (companion labels 11–13) | V-77 |
| Players 3.4 — portrait / head reassignment | guarded reassign to shipped assets: X360 09 → Legacy, PS3 15 / Legacy, PS4 portraits; PS4 heads / PS3 08–14 / Recompiled / 07–08 refused | **dev** (data-level contract only) | receipt 38 | research card P34-1 (game) | P34-1, V-78 |
| Equipment 4.5 goalie gear | numbers on 25 anchors + genuine NHL 20; NHL 08 / 09 paint = `maskcolor` (note + file check); NHL 12 pads file check fixed (receipt 40) | **dev** | receipts 31, 38, 40 | research: G2 team mask slots, G3 PS4 (maskid, masktype) pair validation | V-74, V-79 |
| Creation / cloning / deletion of leagues / teams | not implemented | contract missing | BETA B4 | ~~**user decision** (in / out of the beta) → one runtime experiment~~ — **decided 27 Sep** ([beta decisions](../../plans/beta-20261001-scope-decisions.md) 1 + clarification): league **and** team lifecycle operations are outside the October 1 beta; later scope, code and research preserved | not a beta blocker |
| gen7 rules / season settings | refused (compiled in `default.xex`) | blocked | BETA B6 | ~~**user decision** (exe patching in / out)~~ — **decided 27 Sep** (beta decisions 2): executable patching is **final-release** backlog, not a beta gate; settings in supported data files (PS4 15–21) stay in beta scope | final-release backlog |
| Art-id re-key / archive rename | not implemented; swap-between-existing only | contract missing | BETA B5 | research | — |
| Conference / division names NHL 08–11 | raw numbers | research unknown | BETA B7 | research | bounded exe trace |
| Board-ad replacement (gen7) | not implemented | unproven mapping | BETA B13 | research (decode) or a user gallery | — |

**Review bundles.** `E:\nhlms-review-build-2026-09-27` (receipts ≤ 28, 26 Sep); `E:\nhlms-review-build-r33-2026-09-27`
(receipts ≤ 33, with the helper); the final bundle of this run is recorded in the review plan §2 and in receipt 39.

**Order from here:** (1) coordinator review of receipts 32–39 and R123; (2) the user decisions B4 / B6; (3) research
G2 / G3 (goalie gear), two-segment / other `.ast` XMA shapes, Recompiled audio sources; (4) listening / desktop /
game cards on the final bundle.

## Refresh 27 Sep 2026 (afternoon) — current authority (developer, dispatch after receipt 40)

Supersedes the morning refresh where they differ (that table stays as history, with dated notes). Read: the
[beta decisions](../../plans/beta-20261001-scope-decisions.md), the
[dispatch after receipt 40](../../plans/development-20260927-after-receipt40.md), research R123 / R125 / R125b / R133;
developer receipts 41–44. Levels as above; nothing here is Codex-accepted, desktop-observed or game-accepted.

| Row | Present behaviour (targets) | Level | Evidence | Owner / dependency | Next action |
|---|---|---|---|---|---|
| 1.2 schedule generation — kept rows (both generators) | rows involving a non-participant stay in place with every value; generated rows keep their positions' indexes; kept games block their clubs' dates; refusal with nothing written otherwise | **dev** (defect fixed) | receipt 41 (red → green; synthetic NHL 07 fixtures; genuine NHL 20 unchanged path) | coordinator review | V-81, V-76 (recipe corrected) |
| 1.2 schedule — grouped / stock-shaped / Austrian phases | unchanged: refused (round-tagged) / not offered | proposal | R123 pending its own review | coordinator | — |
| 1.2 conference / division labels NHL 08–11 | divisions by member clubs (08–11); conferences inferred by position (10 / 11); NHL 07 header division defect fixed | **dev** | receipt 44 | — | no runtime claim |
| 4.5 goalie gear — team mask slots (G2) | Uniforms → Helmet card, NHL 10 → PS4 21; X360 file check (every stored paint names files on 10 → Legacy); PS4 numbers, 248 / 249 reported | **dev** | receipt 42 | research: how slots combine with `maskid` (runtime) | V-80; G3 (PS4 pairs) needs its own contract |
| Release archive (beta decision 3) | one zip: exe, defs, helper + exact source / licences / notices / build record, third-party notices, manifest; verified from a path with spaces | **dev** (local candidate, not published) | receipt 43 (check + 13 / 13 self-test + extracted-root defs and 83 R84 vectors) | user / coordinator: publication; Studio licence file (README vs Q46e); About entry; 15 licence-less dependencies | release-page draft in `docs/release/` |
| League / team creation, cloning, deletion | not implemented | **outside the beta** (decision 1 + clarification) | — | later scope | — |
| gen7 rules / season settings needing the executable | refused | **final-release backlog** (decision 2) | — | — | — |

**Review bundles:** `E:\nhlms-review-build-r43-2026-09-27\NHL Modding Studio rc-20260927-r43\` (receipts ≤ 42; the verified
extraction of the archive) is current; r39 / r33 / 26 Sep unchanged. Receipt 44 came after it.

**October 1 blockers now:** receipt 45 §6.


## Refresh 28 Sep 2026 — current authority (developer, dispatch after receipt 45)

Supersedes the 27 Sep afternoon refresh where they differ. Read: [dispatch after receipt 45](../../plans/development-20260928-after-receipt45.md),
the [coordinator review](../../validation/development-20260928-receipt45-review.md); receipts 46–48 (and later rows as added).
Nothing here is Codex-accepted beyond what that review accepted (receipt 41 preservation, receipt 43 archive integrity), desktop-observed
or game-accepted.

| Row | Present behaviour (targets) | Level | Evidence | Owner / dependency | Next action |
|---|---|---|---|---|---|
| Release: Studio licence (Q46e) | MIT declared in all package metadata and README; `LICENSE` withheld until the copyright holder is recorded | **dev**; one value missing | receipt 46 | **user**: the holder name | fill `tools/release-archive/studio-licence.json` from the recorded answer |
| Release: notices / About | in-app About & Licences; 0 missing third-party texts; measured bundle closure; WebView2 SDK notice; release gate + verifier | **dev** | receipt 46; r48 check + 19 self-test cases | coordinator review | V-82 |
| Release: embedded artwork | 17 box-art tiles, 3 platform tiles, 30 strategy GIFs, logo, backdrop, icon inside the exe; rights unrecorded | **blocked** (decision) | receipt 46 §1, draft §5 | **user / coordinator** | record rights or replace |
| Hockey Shop install (NHL 12 pads) | applied create / overwrite on a copy; project title; reviewed fingerprint; rollback; meshes no longer pads | **dev** (defect fixed) | receipt 47 | coordinator review | V-83 |
| 1.2 schedule generation — grouped single phase | user groups; shipped groups by exact identity; host rules; post-check | **dev** | receipt 48 | coordinator review | V-84 |
| 1.2 schedule — Austrian phased | refused (round-tagged) | withheld | observation 48 §2 | research / runtime: meaning of `round = 1`, phase-1 participants | — |
| 5.2 venue board art (gen7) | read-only gallery: board-ad / bench / jumbotron / ring / glass materials → their rasters; X360 previews, PS3 sizes | **dev** | receipt 49 (R127f agreement on 361 packages) | research: writer boundaries (handoff 03) | V-86; replacement withheld |
| Audio: X360 in-bank XMA residuals | intro + loop, looping, 4 / 6-channel pairs on NHL 10 → Legacy; (0, 0x7FFF) false refusal **fixed** | **dev** (defect fixed) | receipt 50 | user listening | A-XMA-5 |
| Audio: X360 `.ast` streams 11 / 13 / 14 | admitted per measured shape | **dev** | receipt 53 | user listening | A-XMA-6 |
| 4.5 goalie gear — PS4 mask pairs (G3) | pair classes against the title's own vault; team paints list helmet types | **dev** | receipt 51 (R135 figures exact on 7 titles) | runtime: G-MASK-1 | V-87 |
| 4.1–4.4 PS4 skater gear catalogue | helmet / gloves / skates / stick / visor vs Creation Zone lists, pictures | **dev** | receipt 52 (R136 figures exact) | dictionary route for names (research) | V-88 |
| Audio: X360 `.ast` streams 10 / 15 / Legacy | stereo loop, 4 ch ± loop, 6 ch (15 / Legacy) admitted | **dev** | receipt 56 (R140b inputs exact) | user listening | r56; A-XMA-7 |
| B10 / B14 portrait / head reassignment — PS3 09–14, Recompiled | checked against the attached install's archive members (EB; BIG4 + `.bh`); unlisted archives named; PS3 10–12 `head_<id>_cm` disclosed | **dev** | receipt 55 (real PS3 10 / 11 / 13 + Recompiled) | runtime: the game drawing it | r56; V-89 |
| Recompiled: player-row encodings | weight / ratings / birth dates / contract dollars editable (were refused: id mismatch) | **dev** (defect fixed, red → green) | receipt 59 | coordinator review | r59 |
| Audio: Recompiled sound banks | listed and previewed from its own archive members; PCM = X360 Legacy | **dev** | receipt 59 (R138) | user listening; loader precedence (runtime) | r59 |
| 5.2 venue board art — PS3 pictures, more titles | PS3 14 / 15 / Legacy previews (plain chain); X360 11 / 12 / 14 + PS3 14 listed | **dev** | receipt 58 (R137 counts exact; PS3 = X360 base on `adbd_WPG`) | research / coordinator: writer (withheld) | r59 |
| 4.1–4.5 PS4 gear / mask names | the title's own English text for Creation Zone keys (hash lookup); mask list-row names | **dev** | receipt 57 (R141 totals exact, NHL 17–21) | — | r59 |
| 4.5 PS4 mask pictures | Creation Zone list rows (17–21); name-rule candidates labelled (19–21); none on 15 / 16 | **dev** | receipt 54 (R135 §5 counts) | runtime | r56; V-90 |
| Hockey Shop install recovery (F1) | a failed undo keeps the original on disk (backup or `.nhlms-original`) and names it; unowned files never touched | **dev** (defect fixed) | receipt 61 (7 fault tests; NHL 12 applied regression) | coordinator review | r64 |
| 3.4 PS4 head reassignment | shipped heads from the open title's vault; NHL 17–21 headid + headtexture as one change; applies need the checked plan (all titles) | **dev** | receipt 63 (genuine NHL 20, NHL 21, NHL 15, foreign / missing-field cases) | user P34-1 / R15-HEAD-1 (runtime) | r64; V-94, V-95 |
| Recompiled encoded edits — public path | legacyrecomp: display / review / undo / redo / save / reopen / import; wrong title refused | **dev** (regression) | receipt 62 | coordinator review | r59+ |
| Release: embedded artwork | bundled for internal builds by the user's 28 Sep decision; rights unrecorded; per-group replacement backlog; external release refused | decision recorded; **blocked** for external release | receipt 64, `docs/release/artwork-replacement-backlog.md` | user: replace or document before external release | r64 |
| PS4 vault source at Apply (review F2) | every Check / Apply re-reads and hashes the vault members and identity definitions; parse cache keyed on the exact identity; fingerprint binds source + disclosures | **dev** (defect fixed) | receipt 66 (`ps4_vault_revalidation`: same-length / same-mtime change refused, definitions change refused) | coordinator review | r66; V-97 |
| Hockey Shop temporary-file ownership | `write_new` distinguishes not-created from created; only this call's own partial file is removed | **dev** | receipt 66 (2 deterministic fault tests) | coordinator review | r66 |
| 3.4 PS4 hair reassignment | `hairid` against the open title's `ge_nhl_skaterhairtexturepack` / `GenericHairStyle` values, NHL 17–21; `hairid` only; NHL 15 / 16 refused | **dev** | receipt 66 (genuine NHL 20, NHL 21; foreign / missing-field / range cases; synthetic ambiguity) | user R15-HAIR-1 (runtime) | r66; V-96 |
| Recompiled Hockey Shop guard | sync / plan / apply refused for legacyrecomp (title or platform); nothing read or written | **dev** (correctness guard) | receipt 67 (owned-tree public-path test; X360 control installs) | coordinator review; card R-RECOMP-LOOSE-1 | r67; V-98 |
| Recompiled read routes | containment via archive index (present / missing / unknown); speech catalogue, contexts, name calls, preview via members; dictionary attach disclosed | **dev** | receipt 67 (parity with X360 Legacy loose, both root layouts) | user V-99 | r67 |
| C37 PS3 board replacement | NHL 11–15 / Legacy, DXT1 / DXT3 same size, full declared chain; pair (loose + member) or archive-only in one copied-target transaction; DXT1 encoded opaque | **dev** | receipt 67 (29 vectors, census parity on 6 titles, Legacy + NHL 13 transaction / fault / stale / restore tests) | coordinator review; user V-100 / R37-BOARD-1 | r67 |
| Review build | r67 bundle (receipts ≤ 67), internal-review; r66 / r64 / r59 / r56 / r53 / r48 kept | **dev** | `review-build-r67/` | user cards | — |
| r67 verification freeze | r67 fixed as the internal verification baseline (zip `6297cda9…0b4a`, exe `60eee0ad…be65`); no further builds; bundles r33 → r66 were removed on 2026-09-29 at the user's request (supersedes the row above); the corpus data package was added | **dev** | receipt 68 | user cards on r67 | r67 |
| N21-001 PS4 onboarding paths (bounded correction under the freeze) | NHL 15-21 PS4: game folder picked directly and identified by its param.sfo; separate shadPS4 save-data folder (root / profile / title level; suggested %APPDATA%\shadPS4\savedata); full inventory with profile + title id; explicit roster choice; remembered + old-config reopen; distinct problems | **dev** | receipt 69 (synthetic layouts, genuine NHL 16 anchor end to end, read-only detection on the 7 real installs) | user focused reruns (receipt 69 §6) | r68 |
| N21-001-R1 (review correction) | stale remembered PS4 roster never replaced silently (explicit replacement, or explicit database-only when none is left); unreadable nested save / library folders reported as an incomplete inventory, never as "no saves" / "not found" | **dev** | receipt 70 (zero / one / several-alternative UI cases asserting open_workspace arguments and stored settings; coordinator probe 5 / 5; owned unlistable-folder cases) | user focused reruns (receipt 70 §4) | r69 |
| ONB-002 project folders (user-directed workflow revision) | Start fresh from the bundled, hashed baseline inventory (26 / 27 targets; NHL 20 PS4 refused as the NHL 19-base alias; gaps explicit) or Use my own files (copied, never written) into a project folder; every edit output saved into the project (copied-target transaction on `working/`); write guards + overlay for loose-asset routes; explicit reviewed deployment with backups and restore; hardened extraction service (preflight, markers, no overwrite, collision block, cancel) at onboarding and Settings → Tools; the install-side `.nhlmodproj` is no longer written | **dev** | receipt 71 (`project_workspace` 7 cases, `extract_service` 8, `project_store` 6, UI 7; 27-target creation sweep) | user: first screen, then NHL 21 (receipt 71 §4) | r70 |
| ONB-003 portable projects, resume, mod package builder + installer (user-directed) | any admitted input (roster save alone, main database alone, extracted assets alone, combinations; destinations optional until deployment, relinkable); manifest `nhlms-project/2` (v1 migrated) with lock, kept editable sources and semantic member records; Open existing project with resume warnings, last view, moved-project and missing-destination handling; `.nhlmod` builder (cells / dictionary texts / loose textures; members, native, structural blocked explicitly; measured same-title platform support, Legacy = Recompiled) and installer (validated container, requirements, staged replay with before-value conflicts, backups, restore) | **dev** | receipt 72 (`project_inputs` 4, `project_resume` 5, `mod_package` 4, domain `mod_package` 3, UI 6 new cases; Legacy X360→PS3/Recompiled/X360 on owned synthetic bases) | user: first pass (NHL 21 project → package → owned destination), then same-game multi-platform cards | r71 |
| ONB-003-R1 (r71 coordinator corrections, changes-required review) | package cells bound to row key values + baseline-row fingerprint (edits schema 2; r71 position-bound packages inspect but are refused by the installer); exact file-role routing (`main_db` never touches a save, `roster` never the database, `both` requires and checks each file on its own; explicit roster choice on every platform, stale choice refused); the save-index sidecar inside the reviewed plan and inside the restore step set, per-item recovery of split / interrupted installs and deployments (installer and project deployment share `registry/recovery.rs` over the in-place publisher's new coordinated removals); Save valid for assets-only projects; platform-correct asset roots (PS4 game folder, Recompiled `_compiled/`); Xbox 360 ↔ Recompiled loose textures by registered inheritance, PS3 refused with the missing-profile reason | **dev** | receipt 73 (`onb003_coordinator_probe` 6 / 6 at the coordinator's assertions; `onb003_corrections` 10; publisher removals unit test; UI 5 new cases) | coordinator review before the user package / install checklist | r72 |
| ONB-003-R2 (r72 coordinator correction R72-R1) | roster-only package installation from the save-data location alone: dependencies derived from the package before any path check, no game installation or main database required, schema metadata from the shipped baseline store verified by hash (`requirements[].role = "schema"`), explicit multi-save choice / stale-choice refusal kept on every platform incl. PS4 profile / title save folders, mixed packages still need their installation; receipt 73 + matrix wording corrected (deployment cards rerun; schema-1 refusal = packages with cell edits) | **dev** | receipt 74 (`r72_coordinator_probe` 1 / 1 verbatim; `onb003_corrections` +4 incl. store outside the checkout, schema failures, bad / wrong-title saves, PS4; UI 2 new cases) | coordinator review | r73 |
