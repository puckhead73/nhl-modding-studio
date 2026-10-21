# Evidence and Verification Conventions

**Evidence:** describes the conventions; makes no claims about the games.

Every page in the knowledge base carries an **Evidence** line near the top and labels individual claims. The labels are the project's own, and they are deliberately strict: nothing is promoted because it is plausible, and a document never makes something true by stating it.

## The three claim labels

| Label | Meaning | What it rests on |
|---|---|---|
| **measured** | The fact was reproduced on the game's own files: a reference database shipped with the game, a save file, an archive, or an extracted asset, for a named title, platform and build. | A parser or probe read the bytes and the result was checked, usually against a second source such as the metadata XML, a byte-identical round trip, or the game's own display. |
| **inferred** | The fact follows from measured facts but has not itself been reproduced. | Typical cases: a value that is consistent across titles, a mechanism that explains every observed file but was not observed directly, a reading of an undocumented word that fits all samples. |
| **open** | Unknown, under investigation, or contradicted by later work. | Listed so that nobody mistakes silence for certainty. |

A page may be mostly measured with a few open points; the labels are per claim, not per page.

## Reference data

Most measurements were made on **reference copies** of each title's shipped databases, metadata and sample saves, one set per title and platform, which the project calls *anchors*. Where a page says "on every anchor" it means the full set: ten Xbox 360 titles (NHL 07 through NHL 15 and NHL Legacy), ten PlayStation 3 titles (NHL 07 through NHL 15 and NHL Legacy, with NHL 07 a research copy only) and seven PlayStation 4 titles (NHL 15 through NHL 21). Measurements on installed games or patched builds are named by the build identifier the game exposes, such as a disc product code or a PlayStation 4 `APP_VER` string.

## How these relate to the Studio's support levels

The user guide uses a separate vocabulary for what the **application** can do: *implemented*, *automated-tested*, *game-verified*, *blocked* and *missing-fixture* (see [Contributing to the Wiki](Contributing-to-the-Wiki)). A format fact can be measured while the Studio feature that uses it is merely implemented. The two scales are kept apart on purpose.

**No title is game-verified.** No knowledge page claims that an edited file was loaded by a game and behaved correctly, unless the claim is explicitly attributed to an in-game observation. Where such observations exist they are rare and named.

## What the knowledge base leaves out

- Hashes of game files and paths on anyone's machine. Builds are identified by their product codes and version strings instead.
- Anything about the games' executables beyond what a player observes.
- Unreleased titles.
- Claims that were withdrawn by later measurement. Where a correction matters to understanding, the page says what was once believed and why it was wrong.

## Reading a table of per-title facts

Many pages tabulate a fact per title. Platforms are abbreviated X360 (Xbox 360), PS3 and PS4. "Legacy" means NHL Legacy Edition; the Legacy Recompilation shares its game data byte for byte with the Xbox 360 release, so a fact measured on Xbox 360 Legacy applies to it unless a page says otherwise. Where Xbox 360 and PlayStation 3 share a fact the table says "X360 = PS3"; that identity holds for the main database of every title from NHL 08 to NHL Legacy, but not for dictionaries, archives or saves.

Last reviewed: 2026-10-20
