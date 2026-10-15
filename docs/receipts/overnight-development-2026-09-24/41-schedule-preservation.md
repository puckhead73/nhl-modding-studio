# Receipt 41 — schedule generation keeps retained rows in place; kept games block their clubs' dates; V-76 recipe corrected

**Developer-verified; not Codex-accepted; nothing desktop-exercised; nothing game-verified.** HEAD `8d60a6d`
(unchanged; nothing staged or committed). Claim `dev-schedule-preservation`. Direction:
[dispatch after receipt 40 §2](../../plans/development-20260927-after-receipt40.md). Research consumed: R123 R2 / R3 / R9
(2026-09-26 handoff 16), R114. Developer observation for research:
[observations/41-R123-preservation.md](observations/41-R123-preservation.md).

## 1. Plain-language summary

- **Defect fixed (both generators — round-robin and pair quotas):** a game that involves a club outside the
  chosen participants (the All-Star game, or games against a club you left out) used to be re-sorted into the
  new schedule by date. That moved it to a different row. It now **stays in its row with every stored value
  unchanged**: date, clubs, index, round and status. New games fill only the other rows, in date order, and each
  keeps its row's index.
- **Defect fixed:** a new game could land on a date when one of its clubs already plays a kept game. The old
  warning then said the generator had not caused it, which was wrong. Now a kept game blocks that club's date:
  the new matchday moves forward a day.
- **When the rule can't be met, the plan is refused and nothing is written.** The message names the kept row
  and the reason. Examples: the new games before a kept row can't all be dated by its date, or every such date
  is already taken for one of the clubs. Studio never moves a kept row to make room.
- Double bookings already present among the kept rows are reported as the source's own, not as new.
- Generations without kept rows work exactly as before.
- **Card V-76 arithmetic corrected.** Twelve clubs at four games per pair make 264 games, not 300. Raising four
  pairs to six makes 272. The corrected recipe is three groups of four with six games inside each group and four
  across, which makes exactly 300. It was checked on the real Legacy Swiss table.

## 2. Files

| Path | Change |
|---|---|
| `crates/studio-domain/src/schedule_edit.rs` | `merge_by_date` removed; new `place_generated` shared by both generators (retained rows in place; generated rows fill the replaceable positions in matchday order, keeping each position's index; dates held for date order around retained rows and moved off retained games' club-dates; refusal otherwise; introduced vs pre-existing double bookings); old "not created by the generator" warning removed; option / plan notes reworded |
| `crates/studio-domain/tests/schedule_preservation.rs` (**new**) | 4 cases, below |
| `crates/studio-domain/tests/schedule_edit.rs` | comment of the NHL 07 All-Star case only (its assertions pass unchanged) |

## 3. Rules as implemented

1. A row is **retained** unless both of its clubs are participants. A retained row is copied unchanged, and no
   write touches its record.
2. Generated games are produced as before (circle-method matchdays, or greedy quota matchdays). They are laid
   into the replaceable positions in order. Each position keeps its stored `index`. The index is the table's key,
   and an edit cannot write it.
3. Dates start from the user's date and spacing. A matchday part placed after a retained row is dated no
   earlier than that row. A matchday split by a retained row puts its later part on or after that row's date,
   and the two parts have no club in common. Nothing is moved earlier.
4. A generated matchday never shares a date with a retained game of one of its clubs. It moves forward one day
   at a time.
5. **Refusal, with nothing written**, in three cases: a matchday placed before a retained row ends up dated after
   it (because of spacing, or because every date up to it is taken by a club's retained game); the dates cross
   the September season boundary; or, as a safety net, a club would play twice on one date because of a
   generated game.
6. Notes list each matchday moved and why. Retained-to-retained double bookings are counted as pre-existing.

This is a conservative preservation rule. It does not claim the game needs date order or cares about index
continuity (R123 R3 describes the stock tables, where index equals position).

## 4. Evidence (`docs/validation/evidence/overnight-development-2026-09-24/logs/r41-*`)

**Red run first** (`r41-schedule-preservation-red.log`, the new tests against the old merge): all 3 cases failed.
- The synthetic retained All-Star row at row 60 was replaced by a generated game (`Oct 17 …`).
- The kept "club v All-Star" row at row 0 was replaced.
- On the real table, the kept game at row 21 (Jan 5, slot 29 v 16) was overwritten by an Oct 7 generated game.

**Green** (`r41-schedule-preservation-green.log`), 4 / 4:
- **Fixture A** (synthetic, from the committed NHL 07 X360 `nhlschedulesmall`: the All-Star row moved to row 60,
  dated like its neighbours, round 1, status 2; written through the edit path, saved to a temporary copy at a path
  with spaces, and reopened as the source):
  - round-robin (30 clubs, Oct 1 every 4 days) and pair quotas (every pair once, balanced hosts) both keep row 60
    unchanged, and no write touches it;
  - generated rows keep their positions' indexes; the table stays in date order; nothing is introduced on an
    occupied club-date;
  - one reviewed apply equals one undo entry; undo restores the source bytes; redo restores the applied bytes;
    the saved copy reopens with identical rows;
  - a preview made before an unrelated edit is refused on apply, with bytes unchanged.
  - The round-robin note reads "1 matchday(s) moved later … Oct 17 → Jan 13 (after row 60 …)".
- **Fixture B** (synthetic: row 0 = Minnesota Wild v the All-Star squad on Oct 1, the start date; quota matrix
  with the replaced pair at 0; 434 generated + 2 kept):
  - Minnesota plays only its kept game on Oct 1, and the first generated date is Oct 2;
  - the note names the move; the old warning is gone; same apply, undo, redo and reopen checks.
- **Fixture C** (the kept game at row 1): refused with nothing written. The message: "…every such date is taken
  for a club of that matchday — Minnesota Wild® already plays a kept game on Oct 1…".
- **Real stock table, 29 of 30 clubs** (its 29 kept games each involve a selected club): refused in all five tried
  configurations (Oct 5 every 2 / 4 days; Jan 2 every 1 / 2 / 3 days), each naming the row and the reason.
  Refusal is the correct outcome under the rule above. A smarter date packing might fit; it is not attempted.
- **V-76 recipe** on the committed X360 Legacy anchor `swissschedule` (12 default members, 300 rows, 0 kept): the
  three-groups matrix previews valid with 300 generated rows and writes nothing.

**Regressions** (`r41-schedule-regressions.log`):
- `schedule_edit` 14 / 14, plus the ignored genuine NHL 20 install-database case 1 / 1
  (`NHL_STUDIO_FIXTURES=E:/nhl-studio-fixtures`; czechschedule 14 × 4 = 364, 0 kept);
- studio-domain lib `schedule` 3; studio-api `schedule_edit` 1.
- No DTO change; bindings not affected.
- Anchor versus genuine NHL 20: the preservation cases run on the NHL 07 X360 and Legacy X360 anchors only. The
  NHL 20 case has no kept rows, so it exercises the unchanged path.

**Failed attempt (recorded):** the first synthetic fixture tried to give the retained row index 777. The edit
path refuses it, because `vfEq` (the index field) is not updatable. The fixture keeps positional indexes.

## 5. Card V-76 (corrected; replaces receipt 34 §5's recipe)

X360 / PS3 NHL Legacy → Leagues → Swiss league → Source schedule → Generate… → pair quotas → Fill 4.
Twelve clubs × four games per pair = 264; the table has 300 rows, so 36 more games are needed.
Set the six pairs inside each group to **6**:
- HC Ambri-Piotta, SC Bern, EHC Biel, HC Davos;
- HC Fribourg-Gottéron, Genève-Servette HC, Kloten Flyers, HC Lausanne;
- HC Lugano, SCL Tigers, EV Zug, ZSC Lions.

That is 18 pairs × 2 more games = 36, so the total reads "… of 300 games". Then Preview → Apply → Save → reopen.
The table shows your counts; Undo restores it. The PS4 Austrian refusal part of the card is unchanged.
(Raising only four pairs to six gives 272 and is refused, naming the difference.)

## 6. Limits

- The greedy date rule refuses some tables a smarter packing could fill. The dense case is a club left out whose
  games are spread through the season.
- R123 grouped / stock-shaped / phased generation stays pending its review. Round-tagged generation stays refused.
- Nothing about runtime tolerance of unsorted or re-indexed tables is claimed.

## 7. Cleanup

The tests' temporary copies ("nhlms schedule preservation …" under the temp root) are removed by their guard.
The scratch patch scripts were deleted. **Disk:** E: was full (408 KB free) and the linker failed. The
regenerable `target\debug\incremental` build cache (≈ 32 GB) was deleted, and later builds ran with
`CARGO_INCREMENTAL=0`.
