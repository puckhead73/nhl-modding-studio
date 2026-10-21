# Historical Corpus and Rating Formulas

**Evidence:** describes the Studio's own data and algorithms (measured on the corpus; the rating formula's calibration is noted). It makes no claims about the games beyond those in [Field Encodings](Field-Encodings).

The Studio ships a **historical hockey database** covering seasons from 1909 to the present, hundreds of leagues, and the players, teams, rosters, contracts, jersey numbers, captaincies and drafts within them. It is the import source for the Historical and Legends sections. It is compiled independently and contains no EA data.

## Source and build

The corpus is built from season-partitioned CSV exports of a hockey management simulation: one folder per season (121 seasons), 23 semicolon-separated files each. Ids are namespaced per season, so a player's identity across seasons is established by name and date of birth, which is also the key used to match against an EA game (see [Roster Model](Roster-Model)). A build step ingests the CSVs into one SQLite file, roughly 800 MB, that ships with Studio as a data pack. About 190 names in the source lost their diacritics in export and are listed with the corpus; guesses at their correct spellings are not applied.

## Rating conversion

The source rates players on a 0 to 20 scale per attribute; EA games display 36 to 99 (or 50 to 99 on the earliest titles, 0 to 99 on PS4, see [Field Encodings](Field-Encodings)). The conversion is a **formula profile**: for each position (forward, defenceman, goalie) and each EA attribute, a weighted list of source attributes.

```
ea_value = clamp(lo + weighted_average(normalised components) × (hi − lo), lo, hi)
```

- Each component is normalised to 0..1 either against the nominal cap of 20 or against the **corpus-observed maximum** for that position and attribute (a profile-level choice).
- A missing source attribute contributes nothing to numerator or denominator; it is never a zero.
- A few EA attributes are **prebuilt** from statistics rather than attributes: shoot/pass bias and defence/offence bias from career goals, assists and games; pass/carry bias from puck handling, speed, passing and hockey sense; durability from games played against the team schedule; a shooting-percentage estimate using position-typical shots per game; goalie consistency from Vezina trophies. Bias and tier rows are written on their native 0..15 or 0..2 scale with no offset.

The Studio's EA attribute catalogue has 28 skater and 19 goalie attributes, in the order the games display them.

## Archetype classification

The EA player style (`QgFs`) is chosen by scoring weighted attribute sets per archetype and taking the maximum: sniper, playmaker, power forward, two-way forward and grinder for forwards, with an enforcer short-circuit when fighting and aggression are both at least 17; offensive, two-way and defensive for defencemen. Goalies get no style. Position comes from the source's own position grades: goalie if that grade tops all, defenceman if the better defence grade beats every forward grade, forward otherwise.

## Overall

EA stores no skater overall; Studio calculates one with EA's own regression, which was fitted on NHL 14 and 15 data. It is exact there and reads roughly 13 points low on NHL 10 to 13, so the Studio shows it only where the title's rating layout matches the fit.

## Legends

The Legends section ranks historical players on their computed overall and builds a best-ever roster per NHL franchise with curated strategies, cached in a sidecar database beside the corpus. The calculation must be rerun whenever the active formula profile changes; the app shows a banner until it is.

Last reviewed: 2026-10-20
