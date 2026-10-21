# Historical Players

**Historical › Players** browses the players of the bundled historical hockey database, from 1909 to the present, and is the starting point for importing them into a game. Every player page mirrors the layout of an EA player, so that what you see is what the Roster Builder will write.

## The corpus gate

The Historical and Legends sections open the historical database automatically from the bundled data pack. If that fails, **Locate the historical data pack** asks for a path: leave it empty and press **Use bundled data**, or enter another database and press **Open**. A path that opens is remembered. See [Historical Corpus and Rating Formulas](Historical-Corpus-and-Rating-Formulas) for what the data is.

## The list

Search runs on the server across the whole corpus; once 500 rows are returned the placeholder changes to "Search rated players (top 500 shown)…". Filter **All**, **Skaters** or **Goalies**; columns **Name**, **Pos**, **Seasons**, **Nat**.

## The player page

The banner shows the selected portrait (if one was chosen on the Appearance tab), the name, the most recent club, a **Retired** pill where relevant, position and nickname, birth date and place, handedness, weight and height, the span of seasons, and the latest jersey number. Chips open web searches for the player's portrait, equipment and statistics pages.

Tabs: **Player Data**, **Career Stats**, **Attributes**, **Equipment**, **Appearance**.

### Player Data

Read-only groups: **Biographical Data**, **Roster Data** (jersey, main team, league, active, first and last season, seasons played, goaltender), **Contract Data**, **Draft Data** (with any other draft selections), **Awards**, and the **Season Timeline**: one row per season with team, league, ability, potential, height and weight.

With an EA project open, each timeline row has **Send to Builder** ("Send this season to the Roster Builder"), which becomes **Queued** once added. It queues that one player-season without choosing a team; you pick the team in the [Roster Builder](Roster-Builder). It does not navigate there.

### Career Stats

Skater and goalie tables for regular season and playoffs, by year, team and league.

### Attributes

EA ratings computed with the **active formula profile** ([Formulas](Formulas)). The **Season** dropdown offers **Full Career** (the mean of all played seasons), **Peak (3)** (the mean of the three best) and each season, the best marked ★. The card shows the overall, position and style, the position grades, the full EA rating grid, and a disclosure of the raw source attributes. Standard ratings are on EA's 36 to 99 scale; biases and goalie consistency keep their own scales.

### Equipment

A placeholder: "Gear is resolved on the EA side once a legend is placed on a roster."

### Appearance

A portrait picker for this person. Studio can index an optional portrait library you supply (a folder of images named by person, with an index or a provenance file; the **Library location** disclosure shows the folder in use and lets you point Studio at another). Matches are offered in tiers, **Exact match** (last name, first name and date of birth), **Last name + DOB**, **Last name only** and **DOB only**, and the library can be searched by name, alias or source with an optional date. Click an image to select it; **Clear selection** removes it. The selection is stored per person, appears in the banner, and is what a later import writes to the player's portrait where the target game supports it.

If no library is found: "No portrait library was found. Place the compiled tree at the folder above (or point Studio at it under “Library location”), then rescan."

Last reviewed: 2026-10-20
