# Leagues

**Game Database › Leagues** lists the leagues the loaded game carries and opens each one for editing: its displayed name (through the language dictionary), its conferences and divisions, the teams that belong to it, its logo and its source schedule. What is editable depends heavily on the title, and the page says so wherever it cannot act.

## The list

Columns **Code**, **League** and **Teams**. A name shown in amber monospace is unresolved: a localization key or a raw code the game resolves at runtime. Search covers league, key and code.

The toolbar has two links, **presentation imagery** ([League Imagery](League-Imagery)) and **commentary** ([Commentary and Speech Catalogue](Commentary-and-Speech-Catalogue)), and the dictionary control:

- **With a dictionary attached:** "names via … dictionary (N rows)" and **detach** ("Detach the dictionary from this project. A pending rename planned on it is dropped; the file on disk is not touched.").
- **Without one:** "no dictionary" with the hint "On-screen league names live in this title's dictionary; attach it to see the game's text and to rename via the dictionary.", a path box and **attach**. Studio refuses files that are not dictionaries.

On PS4 titles the rows are league records whose names are localization keys; on older titles they are the league codes carried by teams.

## League detail

The banner shows the name, pills for the code, the name key and its binding evidence ("binding traced on this target", "binding hypothesis", "binding unstated"), and the team count. **Name source** explains that the on-screen name is looked up in the dictionary, not stored in the database.

### League details (not NHL 07)

Where the title has an editable league row: **Name key**, **League ID** (the primary key, not editable), **Artwork ID**, **Render ID**, **Display order**, **Primary colour**, **Secondary colour**. The note warns that the art, render and order ids and the colours are presentation values whose consumers are not yet traced, so they edit as plain numbers. Where there is no editable row the card says so.

### Rename via dictionary

Available with a dictionary attached (not in the Save Editor). Four modes, each with its own subtitle in the app:

| Mode | What it does |
|---|---|
| **text of another key** | Points this key at another existing key's text. No new text is added. |
| **free text (fits in place)** | Re-encodes this key's own text in place with the title's compression tree. It must fit the old string and use only characters the tree has. |
| **free text (append, file grows)** | Appends new text to the end of the string pool and points the key at it. "Whether the game reads a grown dictionary is a runtime question." |
| **new key** | Inserts a new key at its sorted position; offered only on titles whose rows are in hash order. |

**Preview** shows before and after, which other keys share the string, and warnings. **Hold rename** or **Hold growth** makes it the project's single pending dictionary change ("Only one dictionary change can be pending… Save each change before editing another string."). **Clear pending rename** drops it. If the text needs a symbol the tree lacks, the growing modes offer **Rebuild the dictionary**, an explicit choice with its own accounting, and the app notes "the game may still lack a font glyph for the symbol". See [TDB String Pools and Localization Database](TDB-String-Pools-and-Localization-Database) for why these limits exist.

### Competition structure

Where the title has a group table: the league's conferences, divisions and pools, each group with pills such as "inferred", "name unknown", "no stock team" and "key shared with N other group(s)", a **rename…** link into the dictionary panel, and disclosures for exceptional stock arrangements. A single-group league says so. Where no group table applies, the team rows' group ids are shown as numbers with the reason.

### League logo

"Logo art index N — one family-wide enum; the packages below are what the frontend loads for it." A table of family, file and preview with **Replace…** for loose files and **Replace in archive…** on PS4, where the game reads the packed archive. A shared logo warns which other league changes with it. Replacement flows are described under [File Browser](File-Browser).

### Source schedule

"Choose a season template. Schedule changes appear in both teams' calendars." Templates are named by games and league ("NHL · 82 games", "NHL · 58 games", "DEL season" and so on). The summary line gives games, dates and teams with a "consistent" pill or a findings count. Actions: **Export CSV**, **Export JSON**, **Import…**, **Generate…**, and per-game **edit** (home, away, month, day with **Preview** and **Apply**). A table whose teams cannot be named is read-only with the reason.

- **Import** takes a CSV you exported and edited; the summary reports games, rows, team keys and any that need an explicit destination, with a team-mapping disclosure for ambiguous or unmapped keys.
- **Generate** offers a balanced round-robin, pair quotas (you enter each pair's games; odd counts choose a host by one of four rules) or groups (games inside and across groups, optionally filled from the shipped table for the exact file it was recorded from). Every mode previews before **Apply generation** and applies as one undo step.

### Member teams

The teams carrying this league's code, each linked.

## NHL 07

NHL 07 has its own cards: **Identity** (name, teams, name source), **Competition names and presentation** (dictionary text per competition with **Edit text**; "Which game screens use these entries still needs verification."), and a **Competition structure** table where changing a team's league exchanges it with another team and both their schedule places. Adding or removing leagues or moving whole divisions is not available on NHL 07.

Last reviewed: 2026-10-20
