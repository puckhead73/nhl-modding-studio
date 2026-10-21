# TDB Id Reversal and Naming

**Evidence:** measured on every anchor; the rule is what makes the binary ids match the metadata and community names.

Every table and field in a TDB is identified by four ASCII characters. Modders know the player-bio table as `ubPc` and its team field as `dXSB`; the big-endian file and its metadata XML spell them `cPbu` and `BSXd`. Both are right. The four bytes are stored **byte-reversed on big-endian files**, and the two conventions are the same id read in opposite directions.

## The rule

| Where | Convention | Example |
|---|---|---|
| Xbox 360 and PS3 binary; `nhlng-meta.xml` | raw (on-disk) order | table `cPbu`, field `BSXd` |
| EA DB Editor XML, community spreadsheets, NHLViewNG, this wiki's prose | reversed "real" id | table `ubPc`, field `dXSB` |
| PS4 binary and `nhl_game-meta.xml` | un-reversed: little-endian files store the characters in the real order | `dSvy`, `GJCv` as written |

To convert, reverse the four characters. On little-endian files there is nothing to reverse, which is why the PS4 metadata reads "naturally".

## Why it happens

The id is a 32-bit integer written in the file's byte order. Reading it as four bytes on a big-endian file produces the characters backwards relative to the integer's natural (little-endian) spelling. The games do not care; tools do.

## Which name to use

This wiki uses the **real** ids in prose (`ubPc`, `eGlu`, `ZBac`, `kOtt`) because that is what the modding community writes, and gives the metadata **name** beside the id where it helps: `ubPc` is `exhibitionplayerbiotable`, `eGlu` is `exhibitionrostertable`, `ZBac` is `exhibitionplayers`, `kOtt` is `exhibitionteams`. The Studio's own screens show the metadata names and keep the four-letter ids behind a details affordance.

## Internal short names

The database and the games' own vocabulary also carry short names such as `emag` (`game` reversed) for the main database and `etir` for another; they appear in the metadata root element and are of no consequence to editing.

Last reviewed: 2026-10-20
