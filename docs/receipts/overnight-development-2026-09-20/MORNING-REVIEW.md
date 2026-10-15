# Morning review — developer lane, overnight 2026-09-20

Cards for the user, grouped **by title** so several operations can be reviewed in one pass. Everything
below is **developer-verified offline** (receipts 01–09); nothing is Codex-accepted or game-verified.
The user is the pass authority. Studio never wrote to an install, a disc image, an anchor or a profile
overnight; every write test ran on a disposable copy under `%TEMP%` and was restored byte-exact.
**NHL21-E1 is closed and is not reopened by any card.** The 19 September cards (V-1 … V-5, P1BS-1,
P1BS-CSV-1, R3-IMG-1/2, LOC-W3 R-A/R-B) remain valid; the cards below are the new or changed ones.

Conventions: *Source identity* = SHA-256 recorded in the receipt / fixture manifest; *Observe* = what to
look at; *Restore* = how to get back; *Limits* = what the card can and cannot show. Cards marked
**withheld** are write operations not ready for a live run.

---

## Desktop (no game) — the new surfaces

### V-6 Replace dialog: the resize note and the loose route (C1, C6)
- Any title with an install/extraction root: League logo → Replace… with a **differently sized** image.
- *Observe:* the amber note states where a resize is written (Xbox 12–Legacy 64/256-wide, PS3 12–Legacy)
  and that the display-crop families refuse; the write on an RW4 title (07–11) fails with "same-size
  only", on a trophies package with "not the texture's own size"; a same-size write shows the green
  "Wrote…" card with no amber note (an amber "rebuilt through the generic builder" note would mean a
  package shape Studio does not recognise — none of the 141 fixture packages triggers it).

### V-7 Archive route on gen7 titles (C3, C4)
- Xbox 360 NHL 12–Legacy or PS3 12–Legacy with the install root set (a copy!): League logo → "Replace in
  archive…" appears beside Replace…; NHL 07–10: the same, with the note that the `.bh` sidecar is patched
  together with the archive (NHL 07: the note says nothing is placed there).
- *Observe:* the plan dialog names the archive, the stored form (raw / RefPack), the placement (fit-in-slot
  or appended); on a BIG4 title the copied-target review lists **two** artifacts ("Archive with replaced
  texture member(s)" and "Archive index sidecar … patched together with its archive").

### V-8 Whole-group scope (C7) — PS4 16–21
- League logo → "Replace in archive…" on a shared member (most `size256/l*` logos): the dialog shows the
  radio choice *This member only* / *Whole group (N paths)*; switching re-previews (the placement text
  changes from "private copy" to "whole group: … in place / appended once").

### V-9 Dictionary rebuild (C5)
- Any project with a dictionary attached: rename panel → free text (append) with a `~` in the text →
  Preview shows the `needs-rebuild` refusal and the amber offer; "Turn on the rebuild and preview again"
  → the preview shows the rebuild facts (symbols, then after Hold the accounting lines); the pending card
  lists the growth with `serializer canonical-rebuild`.

---

## NHL 15–21 PS4 (CUSA00561 … CUSA18182) — R2-EB-2: archive placement (C2)

- *Source identity:* the install's `databigs/inxp_10*.big` (NHL 15: `inxp10.big`; 19: `inxp_10-010.big`),
  lengths/entry counts in receipt 03 §4. Work on a **copied target** of the install.
- *Studio action:* Leagues → NHL logo → size256 → Replace in archive… with a same-size marked image →
  (16–21) the dialog names the `_fr` twin and predicts a private copy → Apply → copied-target review
  `archive-asset-0` → Apply → copy the archive into place yourself.
- *Observe:* the league logo in the Play Now / roster header. Changes → the console honoured the
  private copy (16–21) or the in-place rewrite (15). The `_fr` twin (French UI) must be unchanged.
- *Treatment B (16–21):* the same with **Whole group**: both the logo and its French twin change.
- *Restore:* Studio's copied-target restore (byte-exact, tested on every title) or the recorded hash.
- *Limits:* loader acceptance of an appended/re-pointed row (research Q-R2-3) is exactly what this
  measures; a 3 GB archive save takes ~35 minutes in Studio (receipt 03 §4).

## NHL Legacy Xbox 360 / PS3 — R2-EB-1: `cache.big` placement (C3)

- *Source identity:* `cache.big` (Xbox `312 439 568` B; PS3 `USRDIR/cache.big` `308 267 648` B).
- *Studio action:* as above on a copied install: League logo → Replace in archive… (fit-in-slot with a
  marked logo; the small DXT5 logo appends) → save → place.
- *Observe:* the league logo on the league picker. PS3: the loose `USRDIR/fe/ion/artassets/…` copy is
  untouched by this card — if the logo does **not** change, the loose copy wins (precedence, atlas Q1);
  then run the loose Replace… instead and observe again.
- *Restore:* copied-target restore / recorded hash.

## NHL 10 Xbox 360 — R2-BIG4-1 (rev. 2 wording, now runnable): `fe.big` + `fe.bh` (C4)

- *Source identity:* `fe.big` `247 494 615` B + `fe.bh`. Work on a **copy** of both files.
- *Treatment A (fit-in-slot):* League logo → Replace in archive… with a marked logo (compresses smaller:
  the dialog says "fit the member's slot"); save; place both files.
- *Treatment B (append-and-retarget):* the small logo with a noisy image (the dialog says "appended at the
  end … the directory's offset order is broken … R2-BIG4-1"); save; place both files.
- *Treatment B′:* B but copy only `fe.big`, leaving the stock `fe.bh`.
- *Observe (bounded readings, research 02 rev. 2):* A changes → the patched archive is read at all; B
  changes → the appended entry was consumed by this screen; B stock → append not consumed (placement,
  caching or rejection — not separated); B′ working shows only that this load did not need the sidecar.
- *Restore:* copied-target restore restores both files; or the two recorded hashes.
- Same card applies to NHL 09 Xbox (`fe.big`), NHL 08 Xbox (`data0.big`, 2 GB), PS3 09/10 (`fe.big`) and
  PS3 08 (`data0.big`, raw members: A only). **NHL 07: nothing is placed (withheld by policy).**
  PS3 NHL 10's `fe.big` needed the C10 binder correction (receipt 09 §3) before it could be opened
  at all — its card is valid only with that fix in the build.

## Any title with a dictionary — LOC-W3 R-C: rebuilt pool + new glyph (C5)

- *Studio action:* attach the dictionary (copy) → free text (append) on a displayed key with a text that
  needs `~` → accept the rebuild → Hold → save → place.
- *Observe:* the screen shows the text; the `~` is drawn (glyph gate) or a box/blank (the font lacks it —
  the dictionary encodes it either way); the title boots at all (rebuilt pool + tree accepted).
- *Restore:* the recorded hash / `.bak`.
- *Limits:* R-A/R-B (grown file, new key) are separate cards (19 Sep).

## Xbox 360 NHL 12–Legacy / PS3 12–Legacy — R6-DIM-1: resized league logo (C6)

- *Studio action:* League logo → Replace… (loose) with a **256×256** image on a 64×64 logo (Xbox) or any
  larger size (PS3) → Write; place (loose) or go through the archive route.
- *Observe:* the logo on the league picker is drawn at the new size (the Apt quad was rewritten) — or
  clipped / unchanged / broken (the loader ignores or rejects the rewritten geometry).
- *Restore:* `.bak` / copied-target restore.

---

## Phase 2 opening — Team identity (C8, receipt 07)

### V-10 Desktop: the Colours section and split names
- Open any team on NHL 11–15 / Legacy / PS4 15 (two colours), PS4 16 (six), PS4 17–21 (nine); NHL 08–10
  show **no** Colours section (measured: no colour fields on the team row).
- *Observe:* each colour is a swatch + `#rrggbb` box; typing a new value and pressing Enter stages **one**
  pending change with all three bytes (11–16) or the one word (17–21); one Undo restores it; PS4 17–21
  show City / Team Name / Nickname, PS4 16 Short Name / Nickname.

### T-1 Team colour on screen — NHL Legacy Xbox 360 / PS3, PS4 15–21 (copied DB; roster save on gen7)
- *Source identity:* the title's `nhlng.db` / `nhl_game.db` (anchor hash) or a fresh roster save.
- *Studio action:* one club → Colours → set **Front-end primary** (17–21) / **Primary** (≤ 16) to a
  loud value (e.g. `#ff00ff`) → save → place.
- *Observe:* the team-select / roster header colour bar, the scoreboard (presentation), and the
  overlay (17–21). Which of the three families each screen consumes is what this measures; a stock
  colour everywhere means the screen reads a different source (`forceteamcolors`, the vault packs, or
  a build-time asset) — record which screen changed.
- *Restore:* the recorded hash / Studio undo before save.
- *Limits:* the byte order of the packed word is proven on the anchors (receipt 07 §3); this card only
  measures **consumption**, not encoding.

### T-2 Split names — PS4 17–21
- Change **City** only, then **Team Name** only, then **Nickname** only (three separate saves).
- *Observe:* which screens compose city + team name (menus, scoreboard) and where the nickname is
  used (commentary text / social feed). A screen that keeps the old text reads the localization
  dictionary (`locstring`) instead — that is the 2.3 operation, not a defect of this one.

## Phase 2.4 — Team logo families (C9, receipt 08)

### V-11 Desktop: the Team logos card
- Open a team on any title with an install/extraction root: the card lists the measured families
  (4 on NHL 07 … 6 on 13–Legacy; 5 on PS4 15/16, 7 on 17, 8 on 18, 13 on 19, 14 on 20, 15 on 21) with
  thumbnails; NHL 07–10 / 12–Legacy show "Replace in archive…" beside "Replace…", PS3 12–Legacy both
  (loose and `cache.big`), PS3 11 loose only, PS4 the archive route.

### T-3 Which screen draws which family — every title (copied target)
- *Studio action:* one club → Team logos → replace **one** family at a time with a marked image (the
  magenta band) → save → place (loose on PS3 11; the archive route elsewhere).
- *Observe:* team select, roster header, scoreboard/overlay, calendar (dynasty), playoff tree, HUT
  card. Record which screen shows the mark for each family; a family that never shows is unused by the
  screens visited or read from another source (PS4 17+: the `img://LogoCro*` processor path is the
  same file).
- *Restore:* `.bak` / copied-target restore.
- *Limits:* one family per run keeps the reading unambiguous; the placement mechanics (fit / append)
  are the Phase 1 cards R2-EB-1 / R2-BIG4-1 / R2-EB-2.

---

## Withheld / not carded

- **NHL 07 archive placement** — policy: nothing is placed (`boot.big` cascade).
- **RW4 (07–11), RW4ps3 (08–11), GNF (16–21) resizes** — not built (descriptors); no card.
- **Audio** — blocked (format), unchanged.
- **Disc images** — same-length only, unchanged.
- **Team `artid` / `index` re-identification** — not built (cascade contract per research handoff 07 §1
  not accepted); no card.
- **Rink logo / centre-ice / puck textures (`rendering/…`)** — not in C9 (2.5); the small PS4 HUT /
  nation / Spengler sets — not offered (key not established).
