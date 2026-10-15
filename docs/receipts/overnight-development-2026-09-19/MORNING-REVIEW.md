# Morning review — developer lane, overnight 2026-09-19

Cards for the user, grouped **by title** so several operations can be reviewed in one pass. Everything below
is **developer-verified offline** (receipts 01–06); nothing is Codex-accepted or game-verified. The user is
the pass authority. Restore steps are given per card; Studio never wrote to an install, a disc image, an
anchor or a profile overnight. **NHL21-E1 is closed and is not reopened by any card.**

Conventions: *Source identity* = SHA-256 recorded in the receipt / fixture manifest; *Observe* = what to
look at; *Restore* = how to get back; *Limits* = what the card can and cannot show. Cards marked **withheld**
are write operations that are not ready for a live run and are listed only so the gap is visible.

---

## Desktop (no game) — visual review of the new surfaces

These need only the desktop app on any target listed; no install is written.

### V-1 Game Settings page (D2/D3/D4) — NHL 15 PS4 anchor, then NHL 18 PS4 install
- Open the NHL 15 PS4 anchor (`anchors/ps4/nhl15/databases/nhl_game.db`) → **Database → Game Settings**.
- *Observe:* 21 eligible rows with lineage names and 57 contexts; each row's detail card shows the stored
  defaults per context with the label arrays (choice text) and the exclusions with their reason; the
  four-site offset-binary fields decode to sensible ranges (`PeriodLength` 3..20 default 5, `GoalieAdvantage`
  −7..7); NHL 15 rows carry the **stored-default editor** (cell picker, code picker, Preview → held plan →
  Apply/Discard). Then open an NHL 16–21 project with its install root set: the page reads
  `databigs/inxp_00.big` `setting_csv` members (variant V1/V2/V3 named), the editor previews a CSV plan
  (member, line/cell, stored sizes, predicted placement fit-in-slot / append-and-retarget).
- *Limits:* what any screen shows at runtime is not asserted (see P1BS-1 / P1BS-CSV-1 below).

### V-2 Pending edits — CSV catalogue card (D4)
- After an NHL 16–21 CSV default Apply: **Pending edits** shows the catalogue card ("written by the next
  copied-target save") with "Drop the last catalogue edit"; the copied-target review lists the `archive-csv`
  artifact.
- *Observe:* the card text names the member, the cell and the placement; dropping it removes the artifact.

### V-3 Dictionary panel modes (D5)
- On a project with a dictionary attached (any Xbox 360 / PS3 anchor's `nhl_eng_us.db`; PS4 `eng_us.db`),
  the rename panel offers **re-point**, **re-text in place**, **free text (append, file grows)** and **new
  key**. The growth preview shows edited/inserted rows, order class (A / B with descent rows), appended
  bytes, sharers with the "this row only / all sharers" choice, and the runtime-gate warnings.
- *Observe:* NHL 08/09/10 Xbox, NHL 09/10 PS3 and `lng_lg` refuse **new key** with `baseline-order-anomalous`
  and list the descents; a text needing a glyph the tree lacks refuses with `needs-rebuild` naming the symbols.

### V-4 League logo card and the new "presentation imagery" page (D6)
- Any title with an install/extraction root: **Leagues → (a league) → League logo** lists the per-title
  variant set (NHL 15 PS4: two PNG sizes; NHL 16: two GNF sizes; 17–21: three sizes + French twins; gen7:
  four families) with thumbnails; **Leagues → presentation imagery** lists the league-keyed families with
  readings, consumer screens and thumbnails, Replace / Replace French buttons.
- *Observe:* thumbnails render for every family that exists loose (previously blank on PS3 08–11 RW4ps3,
  PS3 12–Legacy uncompressed DDS, PS4 15 PNG; the Xbox `leaguelogossmall` thumbnail used to be garbled).
- *Limits:* the readings ("d0 = NHL Western Conference" …) are the research handoff's inferred labels — this
  card is where they get confirmed or corrected.

---

## NHL 15 PS4 (CUSA00561)

### P1BS-1 — one stored default, NHL 15 (research 22 §card; Studio route from D3)
- *Source identity:* the install's `nhl_game.db` inside `patch_inxp00.big` (`abe36f58…`, the copy believed
  read first) and the base archive member (anchor identity `d9cf1be4…` meta). Work on **copies**.
- *Studio action:* open the copied DB → Game Settings → row *Playoff Series Length* (legacyid 72) → cell
  (context 3 `DISPLAY_FRANCHISE_CREATION_CLASIC`) → code 3 → 2 → Preview → Apply (ticket bound to the
  workspace revision; single-cell byte accounting; body CRC recomputed) → save the copy → place it yourself.
- *Controls first (rev. 3):* C0 stock install + fresh profile → Franchise creation → record the displayed
  option and a day-1 save; C1 same with the existing profile. A C0/C1 difference is a control result to be
  matched, not a located store.
- *Treatments:* A context row (3, 72) 3 → 2; B global row (72) 3 → 2 in a separate run; record the creation
  screen before touching the option and the day-1 save (`leaguestates` types 29–34, 41–43, 46–49).
- *Restore:* replace the archive member from the recorded hash; delete the test profile.
- *Limits:* shows whether the creation screen reads the stored default from that copy; does not locate
  where a chosen value is persisted.

---

## NHL 18 PS4 (first), then NHL 17 / 16 / 19 / 20 / 21 PS4

### P1BS-CSV-1 — one stored default through the copied target (research 01 §7; Studio route from D4)
- *Source identity:* `databigs/inxp_00.big` of the install (identities in the receipt 02/04 fixture
  manifest `ps4-setting-csv-manifest.json`; the 65 `setting_csv` members are census-equal).
- *Studio action:* open the copied install as a copied target → Game Settings → *Playoff Series Length*
  V1 `(3, 72)` 4 → 3 → Apply → **Pending edits** shows the catalogue card → copied-target review shows
  `archive-csv` (NHL 18: **fit-in-slot**, archive length and every offset identical; NHL 17: **append-and-
  retarget**, the archive grows) → Apply → the installed member decodes to the new text (Studio checks the
  native chunkzip framing) → copy the archive into place yourself.
- *Controls:* C0 / C1 as above. **NHL 17 needs its own D0 control** (retarget the *unchanged* member —
  Studio's byte-faithful re-encode makes it a pure placement test) before D1 (the edit) and D2.
- *Treatments:* A V1 (3, 72) 4 → 3; B V2 global pair 4 → 3; C V3 (3, 71) *Season Overtime Loss* 1 → 0.
- *Restore:* Studio's copied-target restore (byte-exact, tested) or the recorded archive hash.
- *Limits:* the loader's acceptance of a re-encoded member is the thing under test; shared-span (dedup)
  members are refused by Studio and are not part of this card.

### R3-IMG-2 — NHL 21 league logo vs scouting-region alias (research 03 §6; Studio route from D8)
- *Source identity:* the install's `databigs/inxp_10.big` (research 02 measured it: `l11.gnf` shares its
  block with `regionslogosbig/region-0.gnf` and its `_fr` twin). Work on a **copied target**.
- *Studio action:* Leagues → league with art index 11 → League logo → size256 → **Replace in archive…**
  with a same-size marked-up image → the dialog names the siblings and predicts a private copy → Apply →
  Pending edits shows the "Archive textures" card → copied-target review lists `archive-asset-0` → Apply
  → copy the archive into place yourself.
- *Observe:* the league logo **and** the scouting region-0 logo. Only the league logo changes → the loader
  honours per-row retargeting (the private copy worked); both change → the loader resolved the shared
  block another way; neither changes → the game did not read the rewritten archive (or the append is not
  accepted) — record which.
- *Restore:* Studio's copied-target restore (byte-exact, tested) or the recorded archive hash.
- *Limits:* the first PS4 archive write reaching a console; the appended-member acceptance (research 02
  runtime question) is exactly what this card measures. NHL21-E1 is not involved.

---

## NHL Legacy Xbox 360 (and NHL 12–15 Xbox 360 by the same route)

### LOC-W3 R-A — larger dictionary loads, appended text read through an existing key (research 20)
- *Source identity:* the install's `fe/loc/nhl_eng_us.db` (Legacy: anchor identity; class A).
- *Studio action:* attach the dictionary (a **copy**) → **free text (append)** on a key that is known to be
  displayed (e.g. the league name key of a league shown on the Play Now league picker) with a longer text →
  save in place (the file grows) → copy into place yourself.
- *Observe:* the title boots; that screen shows the longer text.
- *Restore:* the recorded hash / `.bak`.
- *Limits:* shows only that the larger file loads and the appended pool is read through an existing row.

### LOC-W3 R-B — a new key is looked up (research 20)
- *Studio action:* **new key** `NHLTeamName_<suffix>` (class-A file, glyphs in the tree) + change one team's
  `artabbr` to `<suffix>` (team detail) → save both → place.
- *Observe:* that team's name on a roster screen is the new text (new-key hash lookup at its sorted position).
- *Limits:* order tolerance on class B stays unknown (insertion is refused there, so no card).

### Glyph gate — **withheld**
- A text needing a symbol the tree lacks is refused (`needs-rebuild`); the canonical rebuild (O-G) is not
  built, so no run is possible.

### R3-IMG-1 — playoff-tree conference logo (research 03 §6)
- *Source identity:* `fe/ion/artassets/divisionlogos/d0.big` of the Legacy extraction (research 03 data:
  decoded-content SHA per member); work on a **copy of the extraction root**.
- *Studio action:* Leagues → presentation imagery → `divisionlogos` id 0 → Replace… with a same-size
  (256×256) marked-up image → `.bak` written → copy into place yourself.
- *Observe:* Season playoff tree: the Western Conference logo changes → confirms d0 ↔ West on this title;
  unchanged → the tree reads another family or the `cache.big` copy wins (loose-vs-archive precedence, atlas
  Q1).
- *Restore:* the `.bak` beside the file.
- *Limits:* same-dimension only; a changed-length package cannot be placed back into `cache.big` by Studio.

### League logo replace (P1-E) — Legacy Xbox 360, same-size
- *Studio action:* Leagues → a league → League logo → Replace… (XPR2 8_8_8_8 / DXT5 small) on the loose
  extraction; `.bak` written.
- *Observe:* the league picker / roster header logo.
- *Limits:* loose-vs-`cache.big` precedence unknown (same as above).

---

### V-5 Archive textures pending card and dialog (D8) — desktop only
- On any PS4 project with an install root: League logo → "Replace in archive…" → the plan dialog
  (container, stored sizes, placement, siblings, refusal text for a non-texture member) → Apply → the
  "Archive textures" card on Pending edits with "Undo the last texture replacement" (R1 F2: undoing a second replacement of the same member brings the first one back, not the shipped bytes).

## NHL 09 / 10 Xbox 360 and NHL 09 / 10 / 11 PS3 — league logo packages — **withheld**

The RW4 writers are proven offline on every shipped package (77/77 round trips; PS3 = Xbox pixels), but on
these titles the packages live inside `fe.big` (BIG4, RefPack'd on Xbox; PS3 09/10 `fe.big`) and Studio
cannot yet place a changed-length package back into the archive (receipt 06 §5). NHL 11 (loose BIGF on
both platforms) and NHL 08 PS3 (raw member, same length) are runnable **once the user has an extraction the
title reads loose** — precedence is research unknown, so no card is issued yet.

---

## Not carded (why)

- **NHL 07–09 Xbox settings, gen7/PS3 settings:** no data-side representation (D2 reason shown in Studio).
- **Audio:** blocked (format) — research 04; R4-AUD-1 is listen-only and needs no Studio change, so it is
  left to the research lane's list.
- **Different-dimension logo replace:** not built (Apt record floats + container resize; runtime unknown).
