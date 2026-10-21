# Transactions

**Game Database › Transactions** is the two-column trade screen familiar from earlier editors. "Move, copy, release, or delete players — and trade draft picks — between two teams. Every action is an audited, undoable edit; nothing touches disk until you save."

## Layout

Two columns, **Team A** and **Team B**. Each has mode buttons **Players**, **Free Agents** and **Draft Picks**, a league picker and a team picker. The roster view lists number, first and last name, position, overall, style and status; the picks view lists year, round, overall and origin ("via trade — X" or "Own pick").

## Actions

| Mode | Button | What it does |
|---|---|---|
| Players | **Move** | Moves the selected player to the other column's team. |
| Players | **Copy** | Clones the player onto the other team and keeps the original. |
| Players | **Free Agent** | Releases the player to the free-agent pool, after "Send player to Free Agents?". |
| Players | **Delete** | Removes the player's whole cluster (bio, ratings, roster slots) after "Delete player from the database?". |
| Free Agents | **Sign** | Signs the selected free agent to the other column's team. |
| Draft Picks | **Trade Pick** | Sends the pick to the other column's team; its origin stays on record. |
| centre | **⇄ Trade** | Swaps the two selected rostered players' slots. Needs a player on two different teams. |

Each button's tooltip names what is missing when it is disabled ("Select a player", "Select a team in the other column"). Changes apply immediately and show in the list; a failed operation rolls the list back and shows the error at the top.

## What happens underneath

A move relocates the player's occupant into a free slot of the destination team's block; a trade swaps the occupants of two slots; a release clears a slot; a signing places the player into a free slot, growing the team's block when it is full. None of these retag a slot's team, which is what older tools did and what corrupts rosters. See [Roster Model](Roster-Model).

Capacity is bounded by the game's own ceilings. A roster that would exceed its ceiling is refused with a message; the [Roster Builder](Roster-Builder) preview shows capacity bars before an import for the same reason.

## Undo

Every transaction is one undo step. **Undo** in the header reverses the last one; **Project Overview** lists them all with per-change revert.

Last reviewed: 2026-10-20
