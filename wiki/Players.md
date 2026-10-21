# Players

**Game Database › Players** lists every player in the database and opens each for editing: biography, roster spots and contract on **Player Data**, season rows on **Career Stats**, ratings and traits on **Attributes**, gear on **Equipment**. Tabs the loaded game cannot support are not shown.

## The list

Columns **ID**, **Name**, **Pos**, **Team** ("FA" for free agents, highlighted). A position filter and a search over name, team and id. A name shown with ⚠ contains a character that failed to decode; such rows are pinned to the bottom.

## The banner

Portrait (or "no portrait art found"), name, main team as a link or "Free Agent", position and style, date of birth, birthplace, handedness, weight and height, and the jersey number. Chips open a web search for the player's portrait, equipment or stats pages in your browser.

## Player Data

Groups appear only where the title carries the fields:

- **Biographical Data**: First Name, Last Name, Height, Weight, Birth Year, Birth Month, Birth Day, Birthplace, Birth State, Nationality, Position, Handedness.
- **Roster Data**: Jersey Number, Main Team, Junior Team, Active, Archetype (editable when a ratings record exists) and **Additional Teams**: extra roster spots (national teams, all-star rosters) shown as chips with ✕ to remove and **+ Add** to pick a team. "Adding or removing one never changes the main team."
- **Contract Data**: Contract Amount, Pro Team, Cap Hit (derived), Length Of Contract, Contract Status (derived), Year UFA, Two-Way, Multi-Year (derived).
- **Rookie Status**: NHL, AHL and CHL rookie flags.
- **Draft Data**: Draft Team, Draft Year, Draft Round, Draft Position.
- **Recorded name (commentary, read-only)** on Xbox 360 and PS3 NHL 11 through Legacy with an install: the play-by-play and announcer samples tagged with this player's name rows, with ids that open the diagnostic preview.
- **Misc bio fields**: everything else, to be changed only if you know what you are doing. Asset-pointer ids are deliberately excluded; use [Player Art](Player-Art) to reassign portraits and heads.

Values are click-to-edit. An enum value outside the known vocabulary shows "Unknown value (N)"; a field whose vocabularies belong to other titles shows "no catalogue for this title". Team pickers offer "(none — Free Agent)".

Weights, heights, birth dates and ratings are shown in the game's own units; the stored encodings per title are documented in [Field Encodings](Field-Encodings).

## Career Stats

Season rows by league with the stat columns the title stores, editable inline. ✕ deletes a row after a second confirming click. **+ Add Season** takes the season end year, the league (NHL or AHL) and, where the game stores one, a team code. Created players start without career data.

## Attributes

**Skater Ratings** or **Goalie Ratings**, captioned with the title's rating scale ("raw + 36 → 36..99", "raw + 50 → 50..99" or "stored 0..99"). The **Overall** tile is either *calculated* (EA's regression over the current ratings, so it follows your edits; "stored: N" shows the record's own value when it differs) or *stored*, with a tooltip explaining that the title's layout is not one the regression is calibrated for. Potential shows EA's growth tier and letter with a note that exact stars are provisional. **Traits** are checkboxes where the title stores them. Fields without a name in Studio's vocabulary appear under their raw names in a drawer.

Without a companion ratings record the tab is read-only and says so.

## Equipment

On most titles: a 3D gear preview rendered against the player's team kit (a free agent has no kit to render against; equipment fields stay editable), the slot fields (sticks, gloves, protective; or goalie gear), and **Art files on this install**, which checks which render files the assigned ids own ("an id without files has no art in the game"). PS4 adds a **Gear catalogue** check against the title's own Creation Zone lists and, for goalies, a **Mask pair** check of the stored mask and helmet type together ("Reported, not refused: whether the game rejects this pair is not known.").

Textures for a slot are authored in the [Hockey Shop](Hockey-Shop); this tab only changes which id a player wears.

On NHL 07 (Xbox 360) the tab is replaced by a picker that assigns an existing setup, appearance or portrait-and-commentary pairing from another player, with the note that in-game review is still needed.

## Adding, removing and moving players

Player creation, duplication and deletion, moves, trades and free-agent signings live on [Transactions](Transactions). Whole-roster imports from history come from the [Roster Builder](Roster-Builder).

Last reviewed: 2026-10-20
