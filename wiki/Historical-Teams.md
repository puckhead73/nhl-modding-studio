# Historical Teams

**Historical › Teams** browses the clubs of the historical database season by season and lets you send a whole team-season to the Roster Builder, aimed at the EA team it should replace.

## The list

Search runs on the server. The **League** picker defaults to **NHL**; choose **All leagues** or any other league abbreviation. NHL franchises are consolidated, so relocations and renames collapse into one entry; clubs outside the NHL stay standalone. Columns **Name** (with "(defunct)" where applicable), **Abbr**, **Lg** and **Seasons** (a span such as "1967–1993").

## The team page

The banner draws a crest in the club's colours (the corpus ships no logo art), the name and abbreviation, league, conference and division, the franchise name where it differs, parent club, notes and the roster count. The **Season** dropdown changes every tab; a **Club** dropdown appears when more than one club iced under the franchise that season.

Tabs: **Team Data**, **Depth Chart**, **Jersey Numbers & Captaincy**, **Contracts**, **Roster**.

- **Team Data**: identity, classification, colours (as swatches with hex values) and relations (parent club, affiliates). Read-only.
- **Depth Chart**: the five position blocks with jersey, name, captaincy, nationality and the source ability score ("FHM ability (0–20), not an EA overall"), sorted by ability.
- **Jersey Numbers & Captaincy**: the roster with numbers, a grid of the numbers worn that season ("The corpus records no retired-number list."), and the captain and alternates on record.
- **Contracts**: the season's cap sheet with average salary and clauses.
- **Roster**: the import entry point.

## Sending a team-season to the Builder

On the **Roster** tab with an EA project open:

1. Under **Replaces**, pick the EA team this roster should be placed onto.
2. Wait for "Calculating estimated OVR…" to finish; the table lists each member with estimated overall, age, nationality, ability and potential, sorted by overall.
3. Press **Send team-season to Builder (N)**. Every member is queued, already aimed at that team and carrying their historical jersey number; anyone already in the cart is skipped. Studio opens the [Roster Builder](Roster-Builder).

The button reads **All queued** once every member is in the cart.

Last reviewed: 2026-10-20
