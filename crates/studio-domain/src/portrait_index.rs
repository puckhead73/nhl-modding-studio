//! The operator-built portrait index: `portraits.sqlite` beside the portrait tree (schema version
//! 1, see `portrait_index_README.txt` in the library folder). It is the primary backend of the
//! supplementary portrait library; the filename scan in [`crate::portrait_library`] is the fallback
//! for a tree that ships without an index.
//!
//! The index already resolves what a filename cannot: the compound-name boundary (`last_key` /
//! `first_key`, NULL when `identity_status = ambiguous_name_boundary`), the ISO `dob`, the recorded
//! spelling, provenance (`source_label`, `source_type`, `games`, `mods_json`) and an FTS5 table for
//! search. The four match modes map onto its indexes exactly as the README states; aliases feed
//! search only and never widen a match mode. Name keys reproduce the builder's `key()` —
//! casefold, explicit Latin transliteration, NFKD, keep alphanumerics — so a corpus person's
//! "Charles-Antoine" meets the index's `charlesantoine`.
//!
//! The database is opened read-only. User text never reaches SQL or FTS syntax unquoted.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Row};
use unicode_normalization::UnicodeNormalization;

use crate::portrait_library::{LibraryStats, MatchMode, PersonKey, PortraitEntry};

/// File name of the index inside the library root.
pub const INDEX_FILE: &str = "portraits.sqlite";
const SCHEMA_VERSION: &str = "1";

/// A read-only handle on the index.
pub struct PortraitIndex {
    root: PathBuf,
    path: PathBuf,
    conn: Mutex<Connection>,
    /// `relative_path` values start with `portraits/`; when the index sits *inside* that folder
    /// (the shipped layout), the prefix is stripped to reach the file.
    strip_prefix: bool,
    library_version: Option<String>,
    /// Entries whose compound-name boundary the index could not split (`last_key` NULL). Loaded
    /// once (a few hundred rows) so the name tiers can still reach them: a token-boundary test
    /// against the person being matched resolves the split the builder could not make blind
    /// (`o`+`brien` ↔ O'Brien), while `lee` never meets `leeman`.
    unresolved: Vec<PortraitEntry>,
}

impl std::fmt::Debug for PortraitIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PortraitIndex")
            .field("path", &self.path)
            .field("strip_prefix", &self.strip_prefix)
            .finish()
    }
}

const SELECT: &str =
    "SELECT p.relative_path, p.player_slug, p.recorded_name, p.last_name, p.first_name, \
     p.dob, p.portrait_number, p.identity_status, p.source_label, p.source_type, p.games \
     FROM portraits p";
const ORDER: &str = " ORDER BY p.last_key, p.first_key, p.dob, p.portrait_number, p.relative_path";

impl PortraitIndex {
    /// Whether `root` carries an index.
    pub fn exists_in(root: &Path) -> bool {
        root.join(INDEX_FILE).is_file()
    }

    /// Open `<root>/portraits.sqlite` read-only and verify its schema version.
    pub fn open(root: &Path) -> Result<Self, String> {
        let path = root.join(INDEX_FILE);
        let conn = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("open {}: {e}", path.display()))?;
        let meta = |k: &str| -> Result<Option<String>, String> {
            conn.query_row(
                "SELECT value FROM metadata WHERE key = ?1",
                params![k],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("{}: metadata.{k}: {e}", path.display()))
        };
        let schema = meta("schema_version")?.unwrap_or_default();
        if schema != SCHEMA_VERSION {
            return Err(format!(
                "{}: schema version {schema:?} is not {SCHEMA_VERSION:?}",
                path.display()
            ));
        }
        let library_version = meta("library_version")?;
        let image_folder = meta("image_folder")?.unwrap_or_else(|| "portraits".into());
        // Shipped layout: the index sits inside the image folder, so `portraits/aa/x.png` resolves
        // to `<root>/aa/x.png`. If a sibling `<root>/portraits/` exists the paths are literal.
        let strip_prefix = !root.join(&image_folder).is_dir()
            && root.file_name().map_or(false, |n| {
                n.to_string_lossy().eq_ignore_ascii_case(&image_folder)
            });
        let mut this = Self {
            root: root.to_path_buf(),
            path,
            conn: Mutex::new(conn),
            strip_prefix,
            library_version,
            unresolved: Vec::new(),
        };
        this.unresolved = this.load_unresolved()?;
        Ok(this)
    }

    /// The NULL-key rows, with their recorded-name tokens re-keyed for the boundary test.
    fn load_unresolved(&self) -> Result<Vec<PortraitEntry>, String> {
        let conn = self.conn.lock().unwrap();
        let mut st = conn
            .prepare(&format!("{SELECT} WHERE p.last_key IS NULL{ORDER}"))
            .map_err(|e| e.to_string())?;
        let rows = st.query_map([], row_entry).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            let mut e = r.map_err(|e| e.to_string())?;
            e.tokens = e
                .tokens
                .iter()
                .map(|t| key(t))
                .filter(|t| !t.is_empty())
                .collect();
            if !e.tokens.is_empty() {
                out.push(e);
            }
        }
        Ok(out)
    }

    /// Unresolved entries the person reaches under `mode` by the token-boundary rule. The DOB
    /// tier needs nothing here: those rows carry a `dob` and the SQL path already returns them.
    fn unresolved_matches(
        &self,
        last: &str,
        first: &str,
        dob: Option<&str>,
        mode: MatchMode,
    ) -> Vec<PortraitEntry> {
        if last.is_empty() || matches!(mode, MatchMode::DobOnly) {
            return Vec::new();
        }
        let full = format!("{last}{first}");
        self.unresolved
            .iter()
            .filter(|e| match mode {
                MatchMode::Exact => {
                    !first.is_empty() && e.dob.as_deref() == dob && e.letters() == full
                }
                MatchMode::LastDob => e.dob.as_deref() == dob && e.last_name_is(last),
                MatchMode::LastOnly => e.last_name_is(last),
                MatchMode::DobOnly => false,
            })
            .cloned()
            .collect()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn library_version(&self) -> Option<&str> {
        self.library_version.as_deref()
    }

    /// Absolute path of an indexed portrait; `None` for anything not in the index.
    pub fn abs_path(&self, rel_path: &str) -> Option<PathBuf> {
        let e = self.get(rel_path)?;
        Some(self.resolve(&e.rel_path))
    }

    fn resolve(&self, rel: &str) -> PathBuf {
        let rel = if self.strip_prefix {
            rel.split_once('/').map(|(_, rest)| rest).unwrap_or(rel)
        } else {
            rel
        };
        self.root.join(rel)
    }

    pub fn get(&self, rel_path: &str) -> Option<PortraitEntry> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            &format!("{SELECT} WHERE p.relative_path = ?1"),
            params![rel_path],
            row_entry,
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn stats(&self) -> Result<LibraryStats, String> {
        let conn = self.conn.lock().unwrap();
        let one = |sql: &str| -> Result<i64, String> {
            conn.query_row(sql, [], |r| r.get(0))
                .map_err(|e| e.to_string())
        };
        let portrait_count = one("SELECT count(*) FROM portraits")? as usize;
        let person_count = one("SELECT count(DISTINCT player_slug) FROM portraits")? as usize;
        let mut warnings = Vec::new();
        let mut st = conn
            .prepare("SELECT kind, count(*) FROM issues GROUP BY kind ORDER BY kind")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map_err(|e| e.to_string())?;
        for r in rows.flatten() {
            warnings.push(format!("index issue {}: {} portrait(s)", r.0, r.1));
        }
        Ok(LibraryStats {
            root: self.root.to_string_lossy().into_owned(),
            portrait_count,
            person_count,
            csv_path: Some(self.path.to_string_lossy().into_owned()),
            csv_rows: portrait_count,
            warnings,
        })
    }

    /// Candidates for `person` under `mode`: the index's keyed rows in its canonical order, then
    /// any ambiguous-boundary rows (NULL keys) the person reaches by the token-boundary rule — so
    /// the name tiers meet `o_brien_jay` for O'Brien, Jay even though the builder could not split
    /// that filename on its own.
    pub fn candidates(
        &self,
        person: &PersonKey,
        mode: MatchMode,
        limit: usize,
    ) -> Result<(Vec<PortraitEntry>, usize), String> {
        let last = key(&person.last_name);
        let first = key(&person.first_name);
        let dob = person
            .dob
            .as_deref()
            .filter(|d| !d.is_empty() && *d != "?")
            .map(str::to_string);
        let (where_sql, binds): (&str, Vec<String>) = match mode {
            MatchMode::Exact => match &dob {
                Some(d) if !last.is_empty() && !first.is_empty() => (
                    "p.last_key = ?1 AND p.first_key = ?2 AND p.dob = ?3",
                    vec![last, first, d.clone()],
                ),
                _ => return Ok((Vec::new(), 0)),
            },
            MatchMode::LastDob => match &dob {
                Some(d) if !last.is_empty() => {
                    ("p.last_key = ?1 AND p.dob = ?2", vec![last, d.clone()])
                }
                _ => return Ok((Vec::new(), 0)),
            },
            MatchMode::LastOnly => {
                if last.is_empty() {
                    return Ok((Vec::new(), 0));
                }
                ("p.last_key = ?1", vec![last])
            }
            MatchMode::DobOnly => match &dob {
                Some(d) => ("p.dob = ?1", vec![d.clone()]),
                None => return Ok((Vec::new(), 0)),
            },
        };
        let (mut page, mut total) = self.run(where_sql, "", &binds, limit)?;
        // Resolve the compound-name boundary the index left open, against this person.
        let extra = self.unresolved_matches(
            &key(&person.last_name),
            &key(&person.first_name),
            dob.as_deref(),
            mode,
        );
        total += extra.len();
        page.extend(extra);
        page.truncate(limit.max(1));
        Ok((page, total))
    }

    /// FTS5 prefix-token AND search across recorded names, aliases, slug and DOB, optionally
    /// narrowed to an exact ISO birthdate or a bare birth year.
    pub fn search(
        &self,
        query: &str,
        dob: Option<&str>,
        limit: usize,
    ) -> Result<(Vec<PortraitEntry>, usize), String> {
        let tokens = words(query);
        let dob = dob.map(str::trim).filter(|d| !d.is_empty());
        if tokens.is_empty() && dob.is_none() {
            return Ok((Vec::new(), 0));
        }
        let mut conds: Vec<String> = Vec::new();
        let mut binds: Vec<String> = Vec::new();
        let mut join = "";
        if !tokens.is_empty() {
            // Quote every token: user input is never interpreted as FTS operators.
            let expr = tokens
                .iter()
                .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(" AND ");
            join = " JOIN portrait_search ON portrait_search.rowid = p.id";
            binds.push(expr);
            conds.push(format!("portrait_search MATCH ?{}", binds.len()));
        }
        if let Some(d) = dob {
            match crate::portrait_library::parse_any_dob(d) {
                Some(full) => {
                    binds.push(full);
                    conds.push(format!("p.dob = ?{}", binds.len()));
                }
                None => {
                    let year: String = d.chars().filter(|c| c.is_ascii_digit()).take(4).collect();
                    if year.len() == 4 {
                        binds.push(format!("{year}-%"));
                        conds.push(format!("p.dob LIKE ?{}", binds.len()));
                    } else {
                        return Err(format!(
                            "{d:?} is not a date (YYYY-MM-DD, DD_MM_YYYY) or a year"
                        ));
                    }
                }
            }
        }
        let where_sql = conds.join(" AND ");
        self.run(&where_sql, join, &binds, limit)
    }

    /// Execute a filtered select, returning the page and the unpaged total.
    fn run(
        &self,
        where_sql: &str,
        join: &str,
        binds: &[String],
        limit: usize,
    ) -> Result<(Vec<PortraitEntry>, usize), String> {
        let conn = self.conn.lock().unwrap();
        let params: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|b| b as &dyn rusqlite::ToSql).collect();
        let total: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM portraits p{join} WHERE {where_sql}"),
                params.as_slice(),
                |r| r.get(0),
            )
            .map_err(|e| format!("portrait index count: {e}"))?;
        let mut st = conn
            .prepare(&format!(
                "{SELECT}{join} WHERE {where_sql}{ORDER} LIMIT {}",
                limit.max(1)
            ))
            .map_err(|e| format!("portrait index query: {e}"))?;
        let rows = st
            .query_map(params.as_slice(), row_entry)
            .map_err(|e| format!("portrait index query: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("portrait index row: {e}"))?);
        }
        Ok((out, total as usize))
    }
}

fn row_entry(r: &Row<'_>) -> rusqlite::Result<PortraitEntry> {
    let recorded: String = r.get(2)?;
    let last_name: Option<String> = r.get(3)?;
    let first_name: Option<String> = r.get(4)?;
    let number: Option<i64> = r.get(6)?;
    Ok(PortraitEntry {
        rel_path: r.get(0)?,
        stem: r.get(1)?,
        variant: number.unwrap_or(0).max(0) as u32,
        tokens: recorded.split_whitespace().map(str::to_string).collect(),
        dob: r.get(5)?,
        last_name,
        first_name,
        source: r.get(8)?,
        aliases: Vec::new(),
        identity_status: r.get(7)?,
        source_type: r.get(9)?,
        games: r.get::<_, Option<String>>(10)?.filter(|g| !g.is_empty()),
    })
}

/// The builder's `key()`: casefold, explicit Latin transliteration, NFKD, keep alphanumerics.
pub fn key(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars().flat_map(|c| c.to_lowercase()) {
        let translit: &str = match ch {
            'ø' => "o",
            'ł' => "l",
            'ß' => "ss",
            'æ' => "ae",
            'œ' => "oe",
            'đ' | 'ð' => "d",
            'þ' => "th",
            _ => {
                for d in ch.nfkd() {
                    if d.is_alphanumeric() {
                        out.push(d);
                    }
                }
                continue;
            }
        };
        out.push_str(translit);
    }
    out
}

/// The builder's `words()` tokeniser: casefold + transliteration + NFKD without combining marks,
/// then runs of alphanumerics.
pub fn words(value: &str) -> Vec<String> {
    let mut folded = String::with_capacity(value.len());
    for ch in value.chars().flat_map(|c| c.to_lowercase()) {
        match ch {
            'ø' => folded.push('o'),
            'ł' => folded.push('l'),
            'ß' => folded.push_str("ss"),
            'æ' => folded.push_str("ae"),
            'œ' => folded.push_str("oe"),
            'đ' | 'ð' => folded.push('d'),
            'þ' => folded.push_str("th"),
            _ => {
                for d in ch.nfkd() {
                    if !unicode_normalization::char::is_combining_mark(d) {
                        folded.push(d);
                    }
                }
            }
        }
    }
    folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A miniature index with the shipped schema (the FTS5 table included) and the shipped layout:
    /// `portraits.sqlite` inside `<root>` = `.../portraits`, paths prefixed `portraits/`.
    fn fixture() -> (tempfile::TempDir, PortraitIndex) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("portraits");
        std::fs::create_dir_all(root.join("zh")).unwrap();
        std::fs::create_dir_all(root.join("ad")).unwrap();
        std::fs::write(
            root.join("zh/zherdev_nikolai_05_11_1984_01.png"),
            b"\x89PNG",
        )
        .unwrap();
        let conn = Connection::open(root.join(INDEX_FILE)).unwrap();
        conn.execute_batch(
            "CREATE TABLE metadata(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO metadata VALUES ('schema_version','1'),('library_version','test'),('image_folder','portraits');
             CREATE TABLE portraits(id INTEGER PRIMARY KEY, relative_path TEXT NOT NULL UNIQUE, player_slug TEXT NOT NULL,
               recorded_name TEXT NOT NULL, last_name TEXT, first_name TEXT, last_key TEXT, first_key TEXT, dob TEXT,
               recorded_dob TEXT, portrait_number INTEGER, identity_status TEXT NOT NULL, identity_evidence TEXT,
               source_label TEXT, source_type TEXT, games TEXT, mods_json TEXT, file_bytes INTEGER NOT NULL,
               file_mtime_ns INTEGER NOT NULL, manifest_sha256 TEXT, width INTEGER, height INTEGER, suggested_shard TEXT);
             CREATE TABLE aliases(portrait_id INTEGER, alias TEXT NOT NULL, PRIMARY KEY(portrait_id, alias));
             CREATE TABLE issues(id INTEGER PRIMARY KEY, portrait_id INTEGER, relative_path TEXT, kind TEXT NOT NULL, detail TEXT NOT NULL);
             CREATE VIRTUAL TABLE portrait_search USING fts5(recorded_name, aliases, slug, dob, tokenized_names,
               tokenize='unicode61 remove_diacritics 2', prefix='2 3 4');
             INSERT INTO portraits VALUES
              (1,'portraits/zh/zherdev_nikolai_05_11_1984_01.png','zherdev_nikolai_05_11_1984','zherdev nikolai','zherdev','nikolai','zherdev','nikolai','1984-11-05','05_11_1984',1,'unambiguous_slug',NULL,'nhl09 - nhl14','EA','nhl09;nhl10',NULL,4,0,NULL,512,512,'zh'),
              (2,'portraits/zh/zherdev_nikolay_05_11_1984_01.png','zherdev_nikolay_05_11_1984','zherdev nikolay','zherdev','nikolay','zherdev','nikolay','1984-11-05','05_11_1984',1,'unambiguous_slug',NULL,'EHM Portraits','Mod','',NULL,4,0,NULL,512,512,'zh'),
              (3,'portraits/ad/adam_charles_antoine_13_11_2006_01.png','adam_charles_antoine_13_11_2006','adam charles antoine','adam','charles antoine','adam','charlesantoine','2006-11-13','13_11_2006',1,'structured_reference','players.json','nhl26','EA','nhl26',NULL,4,0,NULL,512,512,'ad'),
              (4,'portraits/ae/aeschlimann_jean_jacques_30_05_1967_01.png','aeschlimann_jean_jacques_30_05_1967','aeschlimann jean jacques',NULL,NULL,NULL,NULL,'1967-05-30','30_05_1967',1,'ambiguous_name_boundary','boundary',NULL,'Mod','',NULL,4,0,NULL,512,512,'ae'),
              (5,'portraits/en/enstrom_tobias_05_11_1984_01.png','enstrom_tobias_05_11_1984','enstrom tobias','enstrom','tobias','enstrom','tobias','1984-11-05','05_11_1984',1,'unambiguous_slug',NULL,'nhl12','EA','nhl12',NULL,4,0,NULL,512,512,'en'),
              (6,'portraits/o/o_brien_jay_04_11_1999_01.png','o_brien_jay_04_11_1999','o brien jay',NULL,NULL,NULL,NULL,'1999-11-04','04_11_1999',1,'ambiguous_name_boundary','boundary','nhl20','EA','nhl20',NULL,4,0,NULL,512,512,'o');
             INSERT INTO aliases VALUES (2,'zherdev_nikolai_05_11_1984');
             INSERT INTO issues VALUES (1,4,'portraits/ae/aeschlimann_jean_jacques_30_05_1967_01.png','ambiguous_name_boundary','boundary');
             INSERT INTO portrait_search(rowid, recorded_name, aliases, slug, dob, tokenized_names) VALUES
              (1,'zherdev nikolai','','zherdev_nikolai_05_11_1984','1984-11-05','zherdev nikolai zherdevnikolai'),
              (2,'zherdev nikolay','zherdev_nikolai_05_11_1984','zherdev_nikolay_05_11_1984','1984-11-05','zherdev nikolay zherdevnikolay'),
              (3,'adam charles antoine','','adam_charles_antoine_13_11_2006','2006-11-13','adam charles antoine adamcharlesantoine'),
              (4,'aeschlimann jean jacques','','aeschlimann_jean_jacques_30_05_1967','1967-05-30','aeschlimann jean jacques'),
              (5,'enstrom tobias','','enstrom_tobias_05_11_1984','1984-11-05','enstrom tobias enstromtobias'),
              (6,'o brien jay','','o_brien_jay_04_11_1999','1999-11-04','o brien jay obrienjay');",
        )
        .unwrap();
        drop(conn);
        let idx = PortraitIndex::open(&root).unwrap();
        (tmp, idx)
    }

    fn person(last: &str, first: &str, dob: Option<&str>) -> PersonKey {
        PersonKey {
            last_name: last.into(),
            first_name: first.into(),
            dob: dob.map(str::to_string),
        }
    }

    #[test]
    fn key_reproduces_the_builder() {
        assert_eq!(key("Charles-Antoine"), "charlesantoine");
        assert_eq!(key("O'Brien"), "obrien");
        assert_eq!(key("Sundström"), "sundstrom");
        assert_eq!(key("Bjørn Ødegaard"), "bjornodegaard");
        assert_eq!(key("Müller-Straße"), "mullerstrasse");
        assert_eq!(words("Beniers, Mat!"), vec!["beniers", "mat"]);
        assert_eq!(words("Ødegaard_Bjørn"), vec!["odegaard", "bjorn"]);
    }

    #[test]
    fn tiers_follow_the_index_contract() {
        let (_t, idx) = fixture();
        let p = person("Zherdev", "Nikolai", Some("1984-11-05"));
        assert_eq!(idx.candidates(&p, MatchMode::Exact, 50).unwrap().1, 1);
        assert_eq!(idx.candidates(&p, MatchMode::LastDob, 50).unwrap().1, 2);
        assert_eq!(idx.candidates(&p, MatchMode::LastOnly, 50).unwrap().1, 2);
        assert_eq!(idx.candidates(&p, MatchMode::DobOnly, 50).unwrap().1, 3);
        // Hyphenated first name meets the index key.
        let ca = person("Adam", "Charles-Antoine", Some("2006-11-13"));
        let (rows, n) = idx.candidates(&ca, MatchMode::Exact, 50).unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            rows[0].identity_status.as_deref(),
            Some("structured_reference")
        );
        assert_eq!(rows[0].display_name(), "charles antoine adam");
        // Ambiguous boundary in the index: Studio resolves it against the person by token
        // boundary, so every tier reaches it and it is reported once.
        let jj = person("Aeschlimann", "Jean-Jacques", Some("1967-05-30"));
        assert_eq!(idx.candidates(&jj, MatchMode::Exact, 50).unwrap().1, 1);
        assert_eq!(idx.candidates(&jj, MatchMode::LastDob, 50).unwrap().1, 1);
        assert_eq!(idx.candidates(&jj, MatchMode::LastOnly, 50).unwrap().1, 1);
        // The split follows the person: "O'Brien" takes o+brien, "Brien" alone does not.
        let ob = person("O'Brien", "Jay", Some("1999-11-04"));
        assert_eq!(idx.candidates(&ob, MatchMode::Exact, 50).unwrap().1, 1);
        assert_eq!(
            idx.candidates(
                &person("Brien", "Jay", Some("1999-11-04")),
                MatchMode::LastOnly,
                50
            )
            .unwrap()
            .1,
            0
        );
        assert_eq!(
            idx.candidates(
                &person("O", "Brien Jay", Some("1999-11-04")),
                MatchMode::LastDob,
                50
            )
            .unwrap()
            .1,
            1
        );
        assert_eq!(
            idx.candidates(
                &person("O'Brien", "Jay", Some("1999-11-05")),
                MatchMode::Exact,
                50
            )
            .unwrap()
            .1,
            0
        );
        let (rows, n) = idx.candidates(&jj, MatchMode::DobOnly, 50).unwrap();
        assert_eq!(n, 1);
        assert_eq!(
            rows[0].identity_status.as_deref(),
            Some("ambiguous_name_boundary")
        );
        // No DOB on the person → DOB tiers yield nothing, surname still works.
        let nod = person("Zherdev", "Nikolai", None);
        assert_eq!(idx.candidates(&nod, MatchMode::Exact, 50).unwrap().1, 0);
        assert_eq!(idx.candidates(&nod, MatchMode::LastOnly, 50).unwrap().1, 2);
        // Paging: limit applies to the page, not the total.
        let (rows, n) = idx.candidates(&p, MatchMode::DobOnly, 1).unwrap();
        assert_eq!((rows.len(), n), (1, 3));
    }

    #[test]
    fn search_uses_fts_prefixes_and_dob_filters() {
        let (_t, idx) = fixture();
        assert_eq!(idx.search("zherdev", None, 50).unwrap().1, 2);
        assert_eq!(idx.search("zher nikola", None, 50).unwrap().1, 2);
        // Aliases feed search.
        assert_eq!(
            idx.search("zherdev_nikolai_05_11_1984", None, 50)
                .unwrap()
                .1,
            2
        );
        assert_eq!(idx.search("", Some("1984-11-05"), 50).unwrap().1, 3);
        assert_eq!(idx.search("", Some("05_11_1984"), 50).unwrap().1, 3);
        assert_eq!(idx.search("", Some("1984"), 50).unwrap().1, 3);
        assert_eq!(idx.search("enstrom", Some("1984"), 50).unwrap().1, 1);
        assert!(idx.search("", Some("nope"), 50).is_err());
        // Operators in user text are literal, never FTS syntax.
        assert_eq!(idx.search("zherdev OR \"", None, 50).unwrap().1, 0);
        assert_eq!(idx.search("", None, 50).unwrap().1, 0);
    }

    #[test]
    fn paths_resolve_through_the_shipped_layout() {
        let (_t, idx) = fixture();
        let abs = idx
            .abs_path("portraits/zh/zherdev_nikolai_05_11_1984_01.png")
            .unwrap();
        assert!(abs.is_file(), "{}", abs.display());
        assert!(idx
            .abs_path("zh/zherdev_nikolai_05_11_1984_01.png")
            .is_none());
        assert!(idx.abs_path("../portraits.sqlite").is_none());
        let st = idx.stats().unwrap();
        assert_eq!((st.portrait_count, st.person_count), (6, 6));
        assert!(st
            .warnings
            .iter()
            .any(|w| w.contains("ambiguous_name_boundary: 1")));
        assert_eq!(idx.library_version(), Some("test"));
    }

    /// `NHL_PORTRAIT_LIBRARY_ROOT=<dir> cargo test -p studio-domain --lib portrait_index -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_index_smoke() {
        let root =
            std::env::var("NHL_PORTRAIT_LIBRARY_ROOT").expect("set NHL_PORTRAIT_LIBRARY_ROOT");
        let t0 = std::time::Instant::now();
        let idx = PortraitIndex::open(Path::new(&root)).unwrap();
        let st = idx.stats().unwrap();
        eprintln!(
            "opened in {:?}: {} portraits / {} people; version {:?}; {:?}",
            t0.elapsed(),
            st.portrait_count,
            st.person_count,
            idx.library_version(),
            st.warnings
        );
        for (l, f, d) in [
            ("Zherdev", "Nikolai", "1984-11-05"),
            ("O'Brien", "Jay", "1999-11-04"),
            ("Adam", "Charles-Antoine", "2006-11-13"),
            ("Apps", "Syl", "1915-01-18"),
        ] {
            let p = person(l, f, Some(d));
            let t = std::time::Instant::now();
            let n: Vec<usize> = [
                MatchMode::Exact,
                MatchMode::LastDob,
                MatchMode::LastOnly,
                MatchMode::DobOnly,
            ]
            .iter()
            .map(|m| idx.candidates(&p, *m, 50).unwrap().1)
            .collect();
            eprintln!(
                "{f} {l} {d}: exact/last_dob/last/dob = {n:?} in {:?}",
                t.elapsed()
            );
        }
        let t = std::time::Instant::now();
        let (rows, n) = idx.search("beniers mat", None, 50).unwrap();
        eprintln!(
            "search 'beniers mat': {n} in {:?}; first {:?}",
            t.elapsed(),
            rows.first().map(|r| &r.rel_path)
        );
        let first = rows.first().expect("a hit");
        assert!(idx.abs_path(&first.rel_path).unwrap().is_file());
    }
}
