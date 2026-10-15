//! Misc 5.4 / Teams 2.7 (PS4, 22 Sep 2026) — the **song catalogue**: what a team's goal / power-play
//! / win song can be chosen from on NHL 17 → 21, as a read-only listing.
//!
//! Research 2026-09-21 pm handoff 12 (R54) measured `nhl/data/common/gamemodes/creation_zone_csv/
//! songs.csv` on NHL 17–21 (191 / 245 / 637 / 356 / 422 rows: identifier, a `$TXT_SONG_…` title
//! key, composer, a genre id named in `songgenres.csv`, an art id, an `audioid`, and the goal /
//! power-play / win — from NHL 19 also intro — eligibility flags). NHL 15 / 16 ship no catalogue.
//!
//! Two negatives are carried as stated, never bridged: the database's `goalsongid` / `winsongid` /
//! `powerplaysongid` (/ `introsongid`) cells are **not** catalogue `audioid`s nor `index`es (0 of
//! the non-zero cells join on every title), and no sound bank carries the catalogue `audioid`s as
//! sample ids (R54 §2). So this is "selectable songs (catalogue)", not "the team's song", and the
//! team's cells stay raw ids with "no catalogue join". No preview, no writer.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::leagues::AttachedDictionary;
use crate::project::Platform;
use crate::text_keys::{resolve_key_text, resolution_note, stamp, DictionaryStamp, KeyLookup};
use crate::{DomainError, Result};

/// One catalogue row, as the CSV carries it (the title key is left as the key; a dictionary
/// resolution is the caller's business when one is attached).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongRow {
    pub index: i64,
    pub identifier: String,
    pub artid: i64,
    pub audioid: i64,
    pub composer: String,
    pub genre: i64,
    /// The genre's `$TXT_…` key from `songgenres.csv`, when the title ships it.
    pub genre_key: Option<String>,
    pub is_goal: bool,
    /// `None` on the titles whose CSV has no `isintrosong` column (NHL 17 / 18).
    pub is_intro: Option<bool>,
    pub is_powerplay: bool,
    pub is_win: bool,
    /// `$TXT_SONG_<IDENTIFIER>` — a dictionary key, kept raw.
    pub name: String,
    /// `name` resolved through the project's own attached dictionary; `None` when it did not
    /// resolve (the card shows `name` then). Filled by [`song_listing`], never by the parser.
    pub title: Option<String>,
    /// `exact` / `by-hash` / `absent` / `no-dictionary` / `not-a-key` — see [`KeyLookup`].
    pub title_lookup: String,
    /// `genre_key` resolved the same way.
    pub genre_label: Option<String>,
    pub genre_lookup: Option<String>,
}

/// One genre of the catalogue with its row count and its key resolved through the dictionary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongGenreCount {
    pub genre: i64,
    /// The `$TXT_…` key from `songgenres.csv`, or `genre <n>` when the title ships none.
    pub key: String,
    pub label: Option<String>,
    pub lookup: String,
    pub rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongCatalogue {
    pub rows: Vec<SongRow>,
    /// `genre id → $TXT_… key`.
    pub genres: BTreeMap<i64, String>,
    pub source: String,
    /// NHL 19 repeats its header inside the file; each repeat is skipped and counted.
    pub header_repeats: usize,
    /// The file carried bytes that are not UTF-8 (a composer name on some titles); they were
    /// replaced, never dropped silently.
    pub decoded_lossy: bool,
}

impl SongCatalogue {
    /// The catalogue path under a PS4 install root (the same directory as the goal horns).
    pub fn path_for(install_root: &Path) -> PathBuf {
        Self::csv_dir(install_root).join("songs.csv")
    }

    pub fn genres_path_for(install_root: &Path) -> PathBuf {
        Self::csv_dir(install_root).join("songgenres.csv")
    }

    fn csv_dir(install_root: &Path) -> PathBuf {
        crate::assets::ps4_kettle_root(install_root)
            .parent()
            .map(|nhl_data| nhl_data.join("common").join("gamemodes").join("creation_zone_csv"))
            .unwrap_or_else(|| install_root.to_path_buf())
    }

    /// Read and parse the catalogue (and the genre names when present). A missing songs file is
    /// an error the caller turns into "no catalogue on this install".
    pub fn load(install_root: &Path) -> Result<Self> {
        let path = Self::path_for(install_root);
        let bytes = std::fs::read(&path).map_err(|e| DomainError::Validation(format!("{}: {e}", path.display())))?;
        let lossy = String::from_utf8_lossy(&bytes);
        let decoded_lossy = matches!(lossy, std::borrow::Cow::Owned(_));
        let genres = std::fs::read(Self::genres_path_for(install_root)).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
        let mut c = Self::parse(&lossy, genres.as_deref(), &path.display().to_string())?;
        c.decoded_lossy = decoded_lossy;
        Ok(c)
    }

    /// Parse the CSV text. Columns are looked up by name (NHL 17 / 18 lack `isintrosong`); a
    /// repeated header line is skipped and counted; a row that is not a record is skipped.
    pub fn parse(text: &str, genres_text: Option<&str>, source: &str) -> Result<Self> {
        let mut genres = BTreeMap::new();
        if let Some(g) = genres_text {
            let mut lines = g.lines().map(str::trim).filter(|l| !l.is_empty());
            if let Some(h) = lines.next() {
                let cols: Vec<&str> = h.split(',').map(str::trim).collect();
                let (ci, cn) = (cols.iter().position(|c| *c == "genre"), cols.iter().position(|c| *c == "name"));
                if let (Some(ci), Some(cn)) = (ci, cn) {
                    for l in lines {
                        let f: Vec<&str> = l.split(',').map(str::trim).collect();
                        if let (Some(id), Some(name)) = (f.get(ci).and_then(|v| v.parse::<i64>().ok()), f.get(cn)) {
                            genres.insert(id, name.to_string());
                        }
                    }
                }
            }
        }
        let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
        let header = lines.next().ok_or_else(|| DomainError::Validation(format!("{source}: empty catalogue")))?;
        let cols: Vec<&str> = header.split(',').map(str::trim).collect();
        let col = |n: &str| cols.iter().position(|c| *c == n);
        let (Some(c_index), Some(c_ident), Some(c_artid), Some(c_audio), Some(c_composer), Some(c_genre), Some(c_goal), Some(c_pp), Some(c_win), Some(c_name)) = (
            col("index"),
            col("identifier"),
            col("artid"),
            col("audioid"),
            col("composer"),
            col("genre"),
            col("isgoalsong"),
            col("ispowerplaysong"),
            col("iswinsong"),
            col("name"),
        ) else {
            return Err(DomainError::Validation(format!("{source}: the catalogue header lacks one of index / identifier / artid / audioid / composer / genre / isgoalsong / ispowerplaysong / iswinsong / name (found: {header})")));
        };
        let c_intro = col("isintrosong");
        let mut rows = Vec::new();
        let mut header_repeats = 0usize;
        for line in lines {
            if line == header || line.starts_with("index,identifier") {
                header_repeats += 1;
                continue;
            }
            let f: Vec<&str> = line.split(',').map(str::trim).collect();
            let get = |i: usize| f.get(i).copied().unwrap_or("");
            let num = |i: usize| -> Option<i64> { get(i).parse().ok() };
            let Some(index) = num(c_index) else { continue };
            let genre = num(c_genre).unwrap_or(0);
            rows.push(SongRow {
                index,
                identifier: get(c_ident).to_string(),
                artid: num(c_artid).unwrap_or(0),
                audioid: num(c_audio).unwrap_or(0),
                composer: get(c_composer).to_string(),
                genre,
                genre_key: genres.get(&genre).cloned(),
                is_goal: num(c_goal) == Some(1),
                is_intro: c_intro.map(|c| num(c) == Some(1)),
                is_powerplay: num(c_pp) == Some(1),
                is_win: num(c_win) == Some(1),
                name: get(c_name).to_string(),
                title: None,
                title_lookup: KeyLookup::NoDictionary.as_str().to_string(),
                genre_label: None,
                genre_lookup: None,
            });
        }
        if rows.is_empty() {
            return Err(DomainError::Validation(format!("{source}: the catalogue has a header but no rows")));
        }
        Ok(Self { rows, genres, source: source.to_string(), header_repeats, decoded_lossy: false })
    }
}

/// The listing a page shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SongListing {
    pub available: bool,
    /// Why nothing is listed, when `available` is false.
    pub reason: Option<String>,
    pub source: Option<String>,
    pub rows_total: usize,
    pub composers: usize,
    /// Genres in descending row-count order, keys resolved through the dictionary.
    pub genre_counts: Vec<SongGenreCount>,
    pub query: String,
    pub matching: usize,
    pub truncated: bool,
    pub rows: Vec<SongRow>,
    pub notes: Vec<String>,
    /// The dictionary the keys were resolved through (this project's attachment), or `None`.
    pub dictionary: Option<DictionaryStamp>,
}

/// Build the listing for a title. Not-a-PS4-title and NHL 15 / 16 are "unavailable, with the
/// reason"; a missing file on 17–21 likewise; a parse failure is an error.
pub fn song_listing(game_id: Option<&str>, platform: Platform, install_root: &Path, query: &str, limit: usize, dictionary: Option<&AttachedDictionary>) -> Result<SongListing> {
    let unavailable = |reason: String| SongListing {
        available: false,
        reason: Some(reason),
        source: None,
        rows_total: 0,
        composers: 0,
        genre_counts: Vec::new(),
        query: query.to_string(),
        matching: 0,
        truncated: false,
        rows: Vec::new(),
        notes: Vec::new(),
        dictionary: stamp(dictionary),
    };
    if platform != Platform::Ps4 {
        return Ok(unavailable("the song catalogue is a PS4 NHL 17–21 install file; on the Xbox 360 / PS3 titles the music tables and banks of research 2026-09-20 handoff 10 apply and no catalogue exists".into()));
    }
    match game_id {
        Some("nhl15" | "nhl16") => return Ok(unavailable("NHL 15 / 16 ship no song catalogue (research 2026-09-21 pm handoff R54 §1)".into())),
        Some("nhl17" | "nhl18" | "nhl19" | "nhl20" | "nhl21") => {}
        Some(other) => return Ok(unavailable(format!("no song catalogue is measured for `{other}` (R54 covers NHL 17–21)"))),
        None => return Ok(unavailable("the open project has no recorded title".into())),
    }
    let path = SongCatalogue::path_for(install_root);
    if !path.is_file() {
        return Ok(unavailable(format!("no `songs.csv` at {} — the attached install root has no extracted catalogue", path.display())));
    }
    let c = SongCatalogue::load(install_root)?;
    // Title and genre keys through this project's own dictionary (item 2 of the coordinator
    // review of receipts 10–17): resolved before filtering so a query can match the resolved text
    // as well as the key.
    let (mut titles_resolved, mut titles_unresolved) = (0usize, 0usize);
    let rows_resolved: Vec<SongRow> = c
        .rows
        .iter()
        .map(|r| {
            let t = resolve_key_text(dictionary, &r.name);
            if t.resolved() {
                titles_resolved += 1;
            } else if t.lookup != KeyLookup::NotAKey {
                titles_unresolved += 1;
            }
            let g = r.genre_key.as_deref().map(|k| resolve_key_text(dictionary, k));
            SongRow {
                title: t.text.clone(),
                title_lookup: t.lookup.as_str().to_string(),
                genre_label: g.as_ref().and_then(|g| g.text.clone()),
                genre_lookup: g.as_ref().map(|g| g.lookup.as_str().to_string()),
                ..r.clone()
            }
        })
        .collect();
    let q = query.trim().to_ascii_lowercase();
    let matches = |r: &SongRow| {
        q.is_empty()
            || r.identifier.to_ascii_lowercase().contains(&q)
            || r.composer.to_ascii_lowercase().contains(&q)
            || r.name.to_ascii_lowercase().contains(&q)
            || r.title.as_deref().is_some_and(|t| t.to_ascii_lowercase().contains(&q))
            || r.genre_key.as_deref().is_some_and(|g| g.to_ascii_lowercase().contains(&q))
            || r.genre_label.as_deref().is_some_and(|g| g.to_ascii_lowercase().contains(&q))
    };
    let matching_rows: Vec<&SongRow> = rows_resolved.iter().filter(|r| matches(r)).collect();
    let matching = matching_rows.len();
    let limit = limit.max(1);
    let rows: Vec<SongRow> = matching_rows.into_iter().take(limit).cloned().collect();
    let composers = c.rows.iter().map(|r| r.composer.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()).collect::<std::collections::BTreeSet<_>>().len();
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for r in &c.rows {
        *counts.entry(r.genre).or_default() += 1;
    }
    let mut genre_counts: Vec<SongGenreCount> = counts
        .into_iter()
        .map(|(g, n)| match c.genres.get(&g) {
            Some(key) => {
                let k = resolve_key_text(dictionary, key);
                SongGenreCount { genre: g, key: key.clone(), label: k.text.clone(), lookup: k.lookup.as_str().to_string(), rows: n }
            }
            None => SongGenreCount { genre: g, key: format!("genre {g}"), label: None, lookup: KeyLookup::NotAKey.as_str().to_string(), rows: n },
        })
        .collect();
    genre_counts.sort_by(|a, b| b.rows.cmp(&a.rows).then(a.genre.cmp(&b.genre)));
    let mut notes = vec![
        resolution_note(dictionary, "song title", titles_resolved, titles_unresolved),
        "selectable songs (catalogue) — not the team's song: the team's goal / win / power-play (/ intro) song cells are not catalogue `audioid`s nor `index`es (0 of the non-zero cells join on every title, R54 §2) and stay raw ids".into(),
        "no sound bank carries the catalogue `audioid`s as sample ids (R54 §2): which file a row plays is not established, so there is no preview and no replacement".into(),
    ];
    if c.header_repeats > 0 {
        notes.push(format!("the file repeats its header {} time{} (NHL 19's shipped form); the repeats were skipped", c.header_repeats, if c.header_repeats == 1 { "" } else { "s" }));
    }
    if c.decoded_lossy {
        notes.push("the file carries bytes that are not UTF-8 (a composer name); they were replaced for display, not dropped".into());
    }
    if c.genres.is_empty() {
        notes.push("no `songgenres.csv` beside the catalogue: genres are shown as ids".into());
    }
    Ok(SongListing {
        available: true,
        reason: None,
        source: Some(c.source.clone()),
        rows_total: c.rows.len(),
        composers,
        genre_counts,
        query: query.to_string(),
        matching,
        truncated: matching > rows.len(),
        rows,
        notes,
        dictionary: stamp(dictionary),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENRES: &str = "genre,name\n11,$TXT_ROCK\n4,$TXT_DUBSTEP\n0,$TXT_NONE\n";

    #[test]
    fn parses_the_nhl21_shape_with_intro_flags_genres_and_a_repeated_header() {
        let csv = "index,identifier,artid,audioid,composer,genre,isgoalsong,isintrosong,ispowerplaysong,iswinsong,name\n\
                   -1,None,30,0,,0,1,0,1,1,$None\n\
                   30,AlienDance,53,195704,Bob Bradley,4,1,0,1,1,$TXT_SONG_ALIEN_DANCE\n\
                   index,identifier,artid,audioid,composer,genre,isgoalsong,isintrosong,ispowerplaysong,iswinsong,name\n\
                   31,AllEyesOnMe,56,212602,William Davies,11,1,1,0,1,$TXT_SONG_ALL_EYES_ON_ME\n";
        let c = SongCatalogue::parse(csv, Some(GENRES), "test").unwrap();
        assert_eq!(c.rows.len(), 3);
        assert_eq!(c.header_repeats, 1);
        let r = &c.rows[1];
        assert_eq!((r.index, r.identifier.as_str(), r.audioid, r.composer.as_str(), r.genre), (30, "AlienDance", 195704, "Bob Bradley", 4));
        assert_eq!(r.genre_key.as_deref(), Some("$TXT_DUBSTEP"));
        assert_eq!((r.is_goal, r.is_intro, r.is_powerplay, r.is_win), (true, Some(false), true, true));
        assert_eq!((c.rows[2].is_intro, c.rows[2].is_powerplay), (Some(true), false));
        assert_eq!(c.rows[0].genre_key.as_deref(), Some("$TXT_NONE"));
    }

    #[test]
    fn parses_the_nhl17_shape_without_intro_and_refuses_a_wrong_header() {
        let csv = "index,identifier,artid,audioid,composer,genre,isgoalsong,ispowerplaysong,iswinsong,name\n32,AllTheRage,57,178505,Bob Bradley/Chris Egan,1,1,1,1,$TXT_SONG_ALL_THE_RAGE\n";
        let c = SongCatalogue::parse(csv, None, "test").unwrap();
        assert_eq!(c.rows[0].is_intro, None);
        assert!(c.rows[0].genre_key.is_none() && c.genres.is_empty());
        assert!(SongCatalogue::parse("index,identifier,name\n1,A,$A\n", None, "t").is_err());
        assert!(SongCatalogue::parse("index,identifier,artid,audioid,composer,genre,isgoalsong,ispowerplaysong,iswinsong,name\n", None, "t").is_err());
    }

    #[test]
    fn the_listing_states_why_it_is_unavailable_and_filters_when_it_is() {
        let d = std::env::temp_dir().join(format!("nhlms-songs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let dir = d.join("nhl").join("data").join("common").join("gamemodes").join("creation_zone_csv");
        std::fs::create_dir_all(&dir).unwrap();
        // Not PS4, NHL 15, an unmeasured title, no title, and a PS4 17+ root without the file.
        assert!(!song_listing(Some("nhllegacy"), Platform::Xbox, &d, "", 50, None).unwrap().available);
        assert_eq!(song_listing(Some("nhl15"), Platform::Ps4, &d, "", 50, None).unwrap().reason.as_deref().map(|r| r.contains("NHL 15 / 16")), Some(true));
        assert!(song_listing(Some("nhl22"), Platform::Ps4, &d, "", 50, None).unwrap().reason.as_deref().unwrap().contains("not measured") || !song_listing(Some("nhl22"), Platform::Ps4, &d, "", 50, None).unwrap().available);
        assert!(!song_listing(None, Platform::Ps4, &d, "", 50, None).unwrap().available);
        let missing = song_listing(Some("nhl21"), Platform::Ps4, &d, "", 50, None).unwrap();
        assert!(!missing.available && missing.reason.as_deref().unwrap().contains("songs.csv"));
        // With the file: counts, genres, filter, cap, notes.
        std::fs::write(dir.join("songgenres.csv"), GENRES).unwrap();
        let mut csv = String::from("index,identifier,artid,audioid,composer,genre,isgoalsong,isintrosong,ispowerplaysong,iswinsong,name\n");
        for i in 0..5 {
            csv.push_str(&format!("{i},Song{i},53,{},Composer {},{},1,0,1,1,$TXT_SONG_{i}\n", 1000 + i, i % 2, if i < 3 { 11 } else { 4 }));
        }
        std::fs::write(dir.join("songs.csv"), csv.as_bytes()).unwrap();
        let l = song_listing(Some("nhl21"), Platform::Ps4, &d, "", 2, None).unwrap();
        assert!(l.available);
        assert_eq!((l.rows_total, l.composers, l.matching, l.rows.len(), l.truncated), (5, 2, 5, 2, true));
        assert_eq!((l.genre_counts[0].genre, l.genre_counts[0].key.as_str(), l.genre_counts[0].rows, l.genre_counts[0].lookup.as_str()), (11, "$TXT_ROCK", 3, "no-dictionary"));
        assert!(l.notes.iter().any(|n| n.contains("not the team's song")));
        let l = song_listing(Some("nhl21"), Platform::Ps4, &d, "composer 1", 50, None).unwrap();
        assert_eq!((l.matching, l.rows.len(), l.truncated), (2, 2, false));
        let l = song_listing(Some("nhl21"), Platform::Ps4, &d, "dubstep", 50, None).unwrap();
        assert_eq!(l.matching, 2);
        // A non-UTF-8 byte is replaced and noted.
        let mut bytes = csv.clone().into_bytes();
        bytes.extend_from_slice(b"9,Bad,53,9,Comp\xe9,11,1,0,1,1,$TXT_SONG_9\n");
        std::fs::write(dir.join("songs.csv"), bytes).unwrap();
        let l = song_listing(Some("nhl21"), Platform::Ps4, &d, "", 50, None).unwrap();
        assert_eq!(l.rows_total, 6);
        assert!(l.notes.iter().any(|n| n.contains("not UTF-8")));
        let _ = std::fs::remove_dir_all(&d);
    }
}
