# Roster Builder

**Historical › Roster Builder** imports historical player-seasons into the open project. "Send players here from the Historical browse, choose a target team, then Preview → Commit." The preview is a dry run; the commit writes exactly what the preview showed, as one undo step.

Without a project open the page says "Open an EA project to build a roster into it." An empty cart says "Open a Historical player and use “Send to Builder” on a season row to queue an import."

## The cart

"Cart — N player(s)", grouped into Forwards, Defensemen and Goalies, each group sorted by estimated overall. Per row:

- **Player** and **Season**.
- **Est. OVR**: the overall the active formula profile computes for that season.
- **Target team**: the EA team to place the player on. Changing it resets the identity choice and triggers a new suggestion.
- **Identity**: how the player enters the database.
  - **Add as a new player** creates a new EA player.
  - Under **Same person (suggested)**: "Update <name> (<team>)", when Studio found the same person already in the database, by name and date of birth or by name alone (marked "— name only").
  - Under **Replace on <team>**: the target team's current players of the same position group, highest overall first. Studio pre-selects the highest-rated unused one. Choices already taken by another cart row are disabled.
  - If no compatible player is left the row turns amber: "No existing slot available for this position. Select another destination."
- **Jersey**: optional; pre-filled when the player came from a team-season.
- **Remove**.

Any edit to the cart discards the current preview. **Clear cart** empties it.

## Preview

Enabled once every row has a target team, ratings are estimated, the EA roster has loaded and every replacement is valid. The **Preview** card ("A dry-run — nothing is written until you Commit. Commit produces byte-identical edits.") shows:

- a net change pill ("+N players") and a blockers pill;
- one row per item with its outcome (**Create**, **Update**, or a blocker such as **Person not found**), the destination team, an **already imported** pill where the import ledger has seen this player-season go to this target before (advisory, never blocking), and any fields skipped because the title lacks them or the value is out of range;
- **Roster capacity**: one bar per affected roster table as "after / ceiling", with **over capacity** where the import would exceed the game's own ceiling ([Roster Model](Roster-Model)); an unknown ceiling is marked and not enforced;
- the validation audit on the previewed state: clean, or counts of errors, warnings and infos with the first findings.

## Commit

Enabled when a current preview has no blockers and no capacity overflow; otherwise the tooltip says "Resolve blockers before committing" or "The roster would exceed its capacity — free space or split the import". Afterwards the **Imported** card shows how many were created and updated, the number of cell edits, and the note: "The import is a single undo entry — reverse it from the pending-edits history. Save the project to write it to disk." The cart clears.

Ratings are written with the target title's own encoding (offset and scale), never a fixed constant; see [Field Encodings](Field-Encodings).

Last reviewed: 2026-10-20
