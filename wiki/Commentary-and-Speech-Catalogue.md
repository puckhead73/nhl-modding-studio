# Commentary and Speech Catalogue

**Game Database › Leagues › commentary** is a read-only window onto the game's speech and sound data: which commentary events exist, which sentences and samples back them, which trigger speaks what, and the sound banks. It can decode individual samples for a diagnostic preview. Nothing here writes anything; audio replacement is not built (see [Known Limitations](Known-Limitations)).

The page is five cards. Each card shows its own refusal when the loaded title is outside its coverage, and most need the game's loose files, so extraction must have run.

## Commentary (read-only)

Xbox 360 and PS3, NHL 11 through Legacy. Search by event or sentence name; the checkbox **league / competition names only** is on by default. The subtitle counts events, sentences, phrases, graffiti tables, bank samples and keywords, and names the source folder.

- **Events** table: event, id, parameters and the sentences sharing the event's name.
- **Sentences sharing an event's name**: expand a row to see its bodies, sample groups, how the event selects among candidates, and the candidate sample ids. Click a sample id to select it for a preview.
- The preview panel decodes one sample ("Decode diagnostic preview", with **Cancel**) and shows a player plus an **Audio diagnostics** disclosure.

## Contexts (which trigger speaks what, read-only)

The context catalogue: contexts, triggers, condition names and speech items, searchable by any of them. Each row shows the trigger ("no trigger (evaluated by another path)" when none), pills for what it speaks or raises, and its conditions. Hidden with no install.

## PS4 speech banks (listing + diagnostic preview)

Lists each bank with its sample count and codec census. Only EA-MP3 samples (NHL 15 through 17) have a preview route; other codecs are listed, not decoded. A **Start** choice selects between two candidate decode starts.

## Gen7 sample banks (listing + diagnostic preview)

Play-by-play, announcer and music sample banks for the Xbox 360 and PS3 titles, listed and framed with per-row availability of a preview route.

## Gen7 sound banks (.abk listing, picker tree, diagnostic preview)

The `.abk` sound banks with their record counts, lists and matching `.ast` stream files. The picker tree shows each list's entries; multi-stream entries are offered as stereo pairs with a note that the channel split is an inference. Previews report a loop point where one exists. An arena's horn link opens this card directly on the matching bank.

For the formats behind these cards see [Audio](Audio), [Audio Banks and Streams](Audio-Banks-and-Streams), [Audio Codecs](Audio-Codecs) and [Commentary and Speech Data](Commentary-and-Speech-Data).

Last reviewed: 2026-10-20
