# Teams

**Game Database › Teams** is where most roster work happens. The list shows every team by id, name and abbreviation; a team's detail page has up to nine tabs covering identity, lines, strategies, numbers, contracts, draft picks and uniforms. Tabs the loaded game cannot support are simply not shown.

## The banner

The team's logo (or "no logo art found for this title"), name, abbreviation, an **Inactive** pill where relevant, the league, conference and division (unnamed groups show as "Conference group N"), the home arena linked to [Arenas](Arenas), and the roster count.

## Tabs

| Tab | Shown when |
|---|---|
| **Team Data** | the title has a team table (always, in practice) |
| **Schedule** | NHL 07 only |
| **Depth Chart** | always |
| **Lines** | the title has line data |
| **Strategies** | the title has a strategy table |
| **Jersey Numbers & Captaincy** | the title has captaincy data |
| **Contracts** | the title has contract data |
| **Draft Picks** | the title has draft-pick data |
| **Uniforms** | the project has an install tree or a jersey registry |

### Team Data

Fields are grouped into **Identity** (Full Name, City, Team Name, Nickname, Abbreviation, Short Name, Art Abbr, State / Region, Coach ID, Art ID, Audio ID), **Classification** (League, League group, Conference, Division, Play Style, Active), **Venue** (Arena), **Relations** (AHL Affiliate, Rival Team) and **Colours** (primary, secondary and the front-end, presentation and overlay sets the title carries). Only fields the game carries appear. Every value is click-to-edit: Enter commits, Escape cancels, and text over the field's byte limit is refused with the limit stated.

- **Art ID** is a dropdown of "<art> — <team>". Choosing another team's id swaps the two teams' art ids, including dependent rows, as one undoable step.
- **Arena** links to the arena page; **change** opens a picker and a disclosure that says who else plays there and what the old arena is left with before **Assign**.
- **League membership** (not NHL 07): move the team to another league with the conference and division from the title's own competition table; a single-group league fills all three automatically. **Move team** applies as one undo step.
- **Team logos** (hidden in the Save Editor): every logo family keyed by the team's art id, with **Replace…** and **Replace in archive…**. The subtitle notes that the same id also names the rink logo, centre-ice logo and the jerseys' middle component.
- **Misc fields**: everything not presented above, with the warning "only change if you know what you are doing".

On NHL 07 the classification cells move into a **League and division** card where changing the league swaps two teams' league, conference, division and schedule places.

### Schedule (NHL 07)

A month calendar of the team's games. Click a day to add or change a game (opponent, home or away, date). "The year controls the calendar display; NHL 07 stores only month and day." A link opens the league schedule's CSV import and export.

### Depth Chart

Five blocks (Left Wing, Center, Right Wing, Defense, Goalie) listing the roster, then **In the system**: players under this team's contract or rights who play elsewhere, plus the affiliate's roster, each with their current team.

### Lines

Situation tabs with counts: **Even Strength**, **Power Play**, **Penalty Kill**, **4-on-4 / Overtime**, **Shootout**, **Extra Attacker**, **Scratched**, showing only the units the title carries. Drag a card from the roster rail onto a slot to place a player; drag a card onto another card to swap. **Best lines** fills every unit by overall, excluding scratched players. The rail filters by position and lets you set each player **Dressed** or **Scratched** and their per-slot archetype. Captain and alternate badges show on cards. Archetype labels depend on the title; NHL 07 and NHL 10 show numbers.

### Strategies

**Team Strategies** with rink diagrams: forecheck formation, neutral-zone formation and pressure, offensive and defensive pressure, defensive strategy, power-play and penalty-kill strategy, and a carry-or-dump slider. **Forward Lines' Strategies** per offence line (style plus four sliders) and **Defensive Pairing Strategies** per pair. Line assignments are read-only here. Edits apply to the exhibition sheet and are mirrored to the career-mode table when a dynasty save has populated it.

### Jersey Numbers & Captaincy

Drag a number from the **Numbers** grid onto a player in **Roster**; × clears a number. Numbers already worn by a teammate must be cleared first. Where the title has a retired-numbers table, a checkbox marks a number retired. **Captaincy** sets Captain, Alternate 1 and Alternate 2; only dressed players can hold a role.

### Contracts and Draft Picks

Both read-only summaries: contracts with cap hit, term and status (edit them on the player's page); future picks grouped by year with their origin (trade them on [Transactions](Transactions)).

### Uniforms

The kit viewer, jersey registry, kit assignments and the nameplate editor. See [Uniforms and Nameplates](Uniforms-and-Nameplates).

Last reviewed: 2026-10-20
