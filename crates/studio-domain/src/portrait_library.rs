//! The supplementary portrait library — the operator-compiled `graphical database/portraits` tree
//! that supplies selectable likenesses for historical (FHM corpus) people, independent of any game
//! install. Game-side portraits still come from the title's own files (`assets.rs`); this library is
//! the *source* a historical person can be matched against and, later, imported from.
//!
//! On disk the library is a two-level tree: `<root>/<xx>/<stem>_<nn>.png`, sharded by the first two
//! letters of the stem, where the stem follows the corpus person-slug convention
//! `last_first_dd_mm_yyyy` with every non-letter (apostrophe, hyphen, space) turned into `_`, and
//! `<nn>` numbers the variants of one person (`o_brien_jay_04_11_1999_02.png`). An optional CSV
//! beside the tree records each file's provenance (source URL / credit) and may carry an explicit
//! first/last split and aliases; when present it overrides the filename-derived identity.
//!
//! Identity matching never trusts a filename split of the name: `adam_charles_antoine` cannot be
//! split without knowing the person. Instead every entry keeps its stem *tokens* and matching is a
//! token-boundary prefix test against the corpus person's normalised last name (letters only, via
//! [`tdb_fhm::slug::normalize_name`]), so `o`+`brien` matches "O'Brien" and `lee` never matches
//! `leeman`. The four tiers the UI offers are [`MatchMode`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tdb_fhm::slug::normalize_name;

/// One portrait file in the library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortraitEntry {
    /// Forward-slash path relative to the library root (`aa/aab_vitalij_14_11_1979_01.png`).
    pub rel_path: String,
    /// The person stem — file stem minus the `_nn` variant suffix. Groups one person's variants.
    pub stem: String,
    /// Variant number (`01` → 1). `0` when the file carries no variant suffix.
    pub variant: u32,
    /// Name tokens in file order (`["o", "brien", "jay"]`), lowercase ASCII.
    pub tokens: Vec<String>,
    /// Birth date as `YYYY-MM-DD`, when the stem (or CSV) carries one.
    pub dob: Option<String>,
    /// CSV-supplied display split, when present. Filename-only entries leave both `None` and the
    /// UI shows the title-cased tokens.
    pub last_name: Option<String>,
    pub first_name: Option<String>,
    /// Provenance from the CSV (source URL / credit), when present.
    pub source: Option<String>,
    /// Extra searchable names from the CSV (already lowercased, letters only).
    pub aliases: Vec<String>,
    /// Index-only (`portrait_index.rs`): how the name boundary was established
    /// (`unambiguous_slug`, `structured_reference`, `ambiguous_name_boundary`, `malformed_identity`).
    pub identity_status: Option<String>,
    /// Index-only: `EA`, `Mod` or `Manual`.
    pub source_type: Option<String>,
    /// Index-only: the EA titles the portrait shipped in (`nhl09;nhl10;…`).
    pub games: Option<String>,
}

impl PortraitEntry {
    /// Every name letter concatenated (`obrienjay`) — the exact-match key.
    pub fn letters(&self) -> String {
        self.tokens.concat()
    }

    /// Whether the entry's leading tokens spell exactly `last_letters` at a token boundary.
    pub fn last_name_is(&self, last_letters: &str) -> bool {
        if last_letters.is_empty() {
            return false;
        }
        let mut acc = String::new();
        for t in &self.tokens {
            acc.push_str(t);
            if acc.len() >= last_letters.len() {
                return acc == last_letters;
            }
        }
        false
    }

    /// A human label: CSV names when present, else the stem tokens title-cased in `First Last`
    /// order is impossible to recover, so the file order (`Last First`) is kept.
    pub fn display_name(&self) -> String {
        match (&self.first_name, &self.last_name) {
            (Some(f), Some(l)) => format!("{f} {l}").trim().to_string(),
            _ => self
                .tokens
                .iter()
                .map(|t| {
                    let mut c = t.chars();
                    match c.next() {
                        Some(h) => h.to_uppercase().collect::<String>() + c.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

/// The identity tiers, loosest last. Exact is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    /// Last name + first name + DOB.
    Exact,
    /// Last name + DOB (ignores first-name variations).
    LastDob,
    /// Last name only (ignores first name and DOB).
    LastOnly,
    /// DOB only (ignores both names — covers surname spelling variations).
    DobOnly,
}

impl MatchMode {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "exact" => Self::Exact,
            "last_dob" => Self::LastDob,
            "last_only" | "last" => Self::LastOnly,
            "dob_only" | "dob" => Self::DobOnly,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::LastDob => "last_dob",
            Self::LastOnly => "last_only",
            Self::DobOnly => "dob_only",
        }
    }
}

/// A corpus person's identity as the library matches it.
#[derive(Debug, Clone)]
pub struct PersonKey {
    pub last_name: String,
    pub first_name: String,
    /// `YYYY-MM-DD`, or `None` / `"?"` when the corpus has no date.
    pub dob: Option<String>,
}

/// A loaded, indexed library.
#[derive(Debug)]
pub struct PortraitLibrary {
    root: PathBuf,
    entries: Vec<PortraitEntry>,
    /// Entries by the first two letters of the name (the shard key).
    by_prefix: HashMap<String, Vec<usize>>,
    by_dob: HashMap<String, Vec<usize>>,
    by_rel_path: HashMap<String, usize>,
    csv_path: Option<PathBuf>,
    csv_rows: usize,
    warnings: Vec<String>,
}

/// Summary for the status surface.
#[derive(Debug, Clone, Serialize)]
pub struct LibraryStats {
    pub root: String,
    pub portrait_count: usize,
    pub person_count: usize,
    pub csv_path: Option<String>,
    pub csv_rows: usize,
    pub warnings: Vec<String>,
}

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp"];

impl PortraitLibrary {
    /// Scan `root` (two levels deep) and apply the sidecar CSV if one sits in `root` or its parent.
    pub fn load(root: &Path) -> std::io::Result<Self> {
        let mut entries = Vec::new();
        let mut warnings = Vec::new();
        scan_dir(root, root, 0, &mut entries, &mut warnings)?;
        entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

        let mut lib = Self {
            root: root.to_path_buf(),
            entries,
            by_prefix: HashMap::new(),
            by_dob: HashMap::new(),
            by_rel_path: HashMap::new(),
            csv_path: None,
            csv_rows: 0,
            warnings,
        };
        if let Some(csv) = find_csv(root) {
            match lib.apply_csv(&csv) {
                Ok(rows) => {
                    lib.csv_rows = rows;
                    lib.csv_path = Some(csv);
                }
                Err(e) => lib.warnings.push(format!("{}: {e}", csv.display())),
            }
        }
        lib.rebuild_indexes();
        Ok(lib)
    }

    /// An empty library (no root found). Status reports it as unavailable.
    pub fn empty(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            entries: Vec::new(),
            by_prefix: HashMap::new(),
            by_dob: HashMap::new(),
            by_rel_path: HashMap::new(),
            csv_path: None,
            csv_rows: 0,
            warnings: Vec::new(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[PortraitEntry] {
        &self.entries
    }

    pub fn get(&self, rel_path: &str) -> Option<&PortraitEntry> {
        self.by_rel_path.get(rel_path).map(|&i| &self.entries[i])
    }

    /// Absolute path of an entry — refuses anything not in the index, so the UI can never read
    /// outside the library.
    pub fn abs_path(&self, rel_path: &str) -> Option<PathBuf> {
        self.get(rel_path).map(|e| self.root.join(&e.rel_path))
    }

    pub fn stats(&self) -> LibraryStats {
        let mut stems: Vec<&str> = self.entries.iter().map(|e| e.stem.as_str()).collect();
        stems.sort_unstable();
        stems.dedup();
        LibraryStats {
            root: self.root.to_string_lossy().into_owned(),
            portrait_count: self.entries.len(),
            person_count: stems.len(),
            csv_path: self
                .csv_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            csv_rows: self.csv_rows,
            warnings: self.warnings.clone(),
        }
    }

    /// Candidates for `person` under `mode`, in library order (stem, then variant).
    pub fn candidates(&self, person: &PersonKey, mode: MatchMode) -> Vec<&PortraitEntry> {
        let last = letters(&person.last_name);
        let first = letters(&person.first_name);
        let dob = person.dob.as_deref().filter(|d| !d.is_empty() && *d != "?");
        let mut out: Vec<usize> = match mode {
            MatchMode::Exact => {
                let Some(dob) = dob else { return Vec::new() };
                let full = format!("{last}{first}");
                self.pool_by_dob(dob)
                    .filter(|&i| {
                        let e = &self.entries[i];
                        e.letters() == full || e.aliases.iter().any(|a| *a == full)
                    })
                    .collect()
            }
            MatchMode::LastDob => {
                let Some(dob) = dob else { return Vec::new() };
                self.pool_by_dob(dob)
                    .filter(|&i| self.entries[i].last_name_is(&last))
                    .collect()
            }
            MatchMode::LastOnly => self
                .pool_by_prefix(&last)
                .filter(|&i| self.entries[i].last_name_is(&last))
                .collect(),
            MatchMode::DobOnly => {
                let Some(dob) = dob else { return Vec::new() };
                self.pool_by_dob(dob).collect()
            }
        };
        out.sort_unstable();
        out.dedup();
        out.into_iter().map(|i| &self.entries[i]).collect()
    }

    /// Manual lookup: every whitespace-separated query token must occur in the entry's name
    /// letters, stem, aliases or CSV names; `dob` (any of `YYYY-MM-DD`, `DD_MM_YYYY`, `DD.MM.YYYY`
    /// or a bare year) narrows further. Results are in library order, capped at `limit`.
    pub fn search(&self, query: &str, dob: Option<&str>, limit: usize) -> Vec<&PortraitEntry> {
        let needles: Vec<String> = query
            .split_whitespace()
            .map(letters)
            .filter(|s| !s.is_empty())
            .collect();
        let dob_filter = dob
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(normalise_dob_query);
        if needles.is_empty() && dob_filter.is_none() {
            return Vec::new();
        }
        // Start from the narrowest pool available.
        let pool: Box<dyn Iterator<Item = usize> + '_> = match &dob_filter {
            Some(DobQuery::Full(d)) => Box::new(self.pool_by_dob(d)),
            _ => match needles.first() {
                Some(n) if n.len() >= 2 && !needles.is_empty() && dob_filter.is_none() => {
                    // Only the first needle can be prefix-indexed when it is the leading token;
                    // a mid-name needle would miss, so fall back to the full scan.
                    Box::new(0..self.entries.len())
                }
                _ => Box::new(0..self.entries.len()),
            },
        };
        let mut out = Vec::new();
        for i in pool {
            let e = &self.entries[i];
            if let Some(f) = &dob_filter {
                match (f, e.dob.as_deref()) {
                    (DobQuery::Full(d), Some(ed)) if ed == d => {}
                    (DobQuery::Year(y), Some(ed)) if ed.starts_with(y.as_str()) => {}
                    _ => continue,
                }
            }
            let hay_letters = e.letters();
            let ok = needles.iter().all(|n| {
                hay_letters.contains(n.as_str())
                    || e.stem.contains(n.as_str())
                    || e.aliases.iter().any(|a| a.contains(n.as_str()))
                    || e.source
                        .as_deref()
                        .map_or(false, |s| letters(s).contains(n.as_str()))
            });
            if ok {
                out.push(e);
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }

    fn pool_by_dob(&self, dob: &str) -> impl Iterator<Item = usize> + '_ {
        self.by_dob.get(dob).into_iter().flatten().copied()
    }

    fn pool_by_prefix(&self, last_letters: &str) -> impl Iterator<Item = usize> + '_ {
        let key: String = last_letters.chars().take(2).collect();
        self.by_prefix.get(&key).into_iter().flatten().copied()
    }

    fn rebuild_indexes(&mut self) {
        self.by_prefix.clear();
        self.by_dob.clear();
        self.by_rel_path.clear();
        for (i, e) in self.entries.iter().enumerate() {
            let key: String = e.letters().chars().take(2).collect();
            self.by_prefix.entry(key).or_default().push(i);
            if let Some(d) = &e.dob {
                self.by_dob.entry(d.clone()).or_default().push(i);
            }
            self.by_rel_path.insert(e.rel_path.clone(), i);
        }
    }

    /// Merge a provenance CSV. Rows join to files by basename (with or without extension) or by
    /// relative path; a row for a file that is not in the tree is counted as a warning. Returns the
    /// number of rows applied.
    fn apply_csv(&mut self, path: &Path) -> Result<usize, String> {
        let raw = std::fs::read(path).map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&raw);
        let delimiter = sniff_delimiter(&text);
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(text.as_bytes());
        let headers: Vec<String> = rdr
            .headers()
            .map_err(|e| e.to_string())?
            .iter()
            .map(|h| h.trim().trim_start_matches('\u{feff}').to_ascii_lowercase())
            .collect();
        let col = |names: &[&str]| headers.iter().position(|h| names.contains(&h.as_str()));
        let c_file = col(&[
            "file",
            "filename",
            "file_name",
            "path",
            "image",
            "portrait",
            "rel_path",
        ]);
        let c_last = col(&["last", "last_name", "lastname", "surname"]);
        let c_first = col(&["first", "first_name", "firstname", "given", "given_name"]);
        let c_dob = col(&["dob", "birthdate", "birth_date", "born", "date_of_birth"]);
        let c_source = col(&[
            "source",
            "url",
            "src",
            "origin",
            "credit",
            "source_url",
            "link",
        ]);
        let c_alias = col(&["alias", "aliases", "aka", "alt_names", "also_known_as"]);
        let c_slug = col(&["slug", "key", "person", "player"]);
        let Some(c_file) = c_file.or(c_slug) else {
            return Err(format!(
                "no file/slug column recognised in header [{}]",
                headers.join(", ")
            ));
        };

        // Basename (without extension) → entry indices; a slug column addresses every variant.
        let mut by_base: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_stem: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, e) in self.entries.iter().enumerate() {
            let base = e.rel_path.rsplit('/').next().unwrap_or(&e.rel_path);
            let no_ext = base.rsplit_once('.').map(|(s, _)| s).unwrap_or(base);
            by_base
                .entry(no_ext.to_ascii_lowercase())
                .or_default()
                .push(i);
            by_stem.entry(e.stem.clone()).or_default().push(i);
        }

        let mut applied = 0usize;
        let mut unmatched = 0usize;
        for rec in rdr.records() {
            let rec = rec.map_err(|e| e.to_string())?;
            let get = |c: Option<usize>| {
                c.and_then(|c| rec.get(c))
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
            };
            let Some(file) = get(Some(c_file)) else {
                continue;
            };
            let norm = file.replace('\\', "/");
            let base = norm
                .rsplit('/')
                .next()
                .unwrap_or(&norm)
                .to_ascii_lowercase();
            let no_ext = base
                .rsplit_once('.')
                .filter(|(_, ext)| IMAGE_EXTS.contains(ext))
                .map(|(s, _)| s.to_string())
                .unwrap_or(base.clone());
            let targets: Vec<usize> = self
                .by_rel_path_lookup(&norm)
                .map(|i| vec![i])
                .or_else(|| by_base.get(&no_ext).cloned())
                .or_else(|| by_stem.get(&no_ext).cloned())
                .unwrap_or_default();
            if targets.is_empty() {
                unmatched += 1;
                continue;
            }
            let last = get(c_last).map(str::to_string);
            let first = get(c_first).map(str::to_string);
            let dob = get(c_dob).and_then(parse_any_dob);
            let source = get(c_source).map(str::to_string);
            let aliases: Vec<String> = get(c_alias)
                .map(|a| {
                    a.split([';', '|', ','])
                        .map(letters)
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            for i in targets {
                let e = &mut self.entries[i];
                if let (Some(l), Some(f)) = (&last, &first) {
                    // The CSV split is authoritative: rebuild the tokens as last-then-first words.
                    let mut toks: Vec<String> = normalize_name(l)
                        .split(' ')
                        .chain(normalize_name(f).split(' '))
                        .filter(|t| !t.is_empty())
                        .map(str::to_string)
                        .collect();
                    if toks.is_empty() {
                        toks = e.tokens.clone();
                    }
                    e.tokens = toks;
                    e.last_name = Some(l.clone());
                    e.first_name = Some(f.clone());
                }
                if dob.is_some() {
                    e.dob = dob.clone();
                }
                if source.is_some() {
                    e.source = source.clone();
                }
                for a in &aliases {
                    if !e.aliases.contains(a) {
                        e.aliases.push(a.clone());
                    }
                }
            }
            applied += 1;
        }
        if unmatched > 0 {
            self.warnings.push(format!(
                "{unmatched} CSV row(s) name a file that is not in the portrait tree"
            ));
        }
        Ok(applied)
    }

    fn by_rel_path_lookup(&self, rel: &str) -> Option<usize> {
        // Indexes are rebuilt after the CSV pass, so look up linearly only when the path form is
        // used (rare: most CSVs carry a basename).
        if !rel.contains('/') {
            return None;
        }
        let rel = rel.trim_start_matches("./");
        self.entries
            .iter()
            .position(|e| e.rel_path.eq_ignore_ascii_case(rel))
    }
}

/// Letters only (`O'Brien` → `obrien`), diacritics folded.
pub fn letters(s: &str) -> String {
    normalize_name(s)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn scan_dir(
    root: &Path,
    dir: &Path,
    depth: u32,
    out: &mut Vec<PortraitEntry>,
    warnings: &mut Vec<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warnings.push(format!("{}: {e}", dir.display()));
                continue;
            }
        };
        let path = entry.path();
        if path.is_dir() {
            if depth < 2 {
                scan_dir(root, &path, depth + 1, out, warnings)?;
            }
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !IMAGE_EXTS.contains(&ext.to_ascii_lowercase().as_str()) {
            continue;
        }
        let Some(stem_full) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel_path = rel.to_string_lossy().replace('\\', "/");
        if let Some(e) = parse_stem(stem_full, rel_path) {
            out.push(e);
        } else {
            warnings.push(format!("unrecognised portrait name: {}", path.display()));
        }
    }
    Ok(())
}

/// Parse `last_first_dd_mm_yyyy_nn` (variant and DOB both optional) into an entry.
fn parse_stem(stem_full: &str, rel_path: String) -> Option<PortraitEntry> {
    let lower = stem_full.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('_').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let is_num = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());

    // Peel the trailing variant, then a dd_mm_yyyy triple.
    let mut end = parts.len();
    let mut variant = 0u32;
    // A trailing 2-digit token is the variant only if a 4-digit year precedes the triple before it,
    // or if there is no DOB at all and it is the last token after at least one name token.
    let has_dob_at = |end: usize| {
        end >= 3
            && is_num(parts[end - 1], 4)
            && is_num(parts[end - 2], 2)
            && is_num(parts[end - 3], 2)
    };
    if end >= 2 && is_num(parts[end - 1], 2) && has_dob_at(end - 1) {
        variant = parts[end - 1].parse().unwrap_or(0);
        end -= 1;
    }
    let dob = if has_dob_at(end) {
        let (d, m, y) = (parts[end - 3], parts[end - 2], parts[end - 1]);
        end -= 3;
        Some(format!("{y}-{m}-{d}"))
    } else {
        None
    };
    if end == 0 {
        return None;
    }
    let tokens: Vec<String> = parts[..end]
        .iter()
        .map(|t| letters(t))
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return None;
    }
    let stem = if variant > 0 {
        lower
            .rsplit_once('_')
            .map(|(s, _)| s.to_string())
            .unwrap_or(lower.clone())
    } else {
        lower.clone()
    };
    Some(PortraitEntry {
        rel_path,
        stem,
        variant,
        tokens,
        dob,
        last_name: None,
        first_name: None,
        source: None,
        aliases: Vec::new(),
        identity_status: None,
        source_type: None,
        games: None,
    })
}

/// Find the provenance CSV: any `*.csv` directly in the root, else in the root's parent (the
/// `graphical database` folder). The first in name order wins.
fn find_csv(root: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for dir in [
        Some(root.to_path_buf()),
        root.parent().map(Path::to_path_buf),
    ]
    .into_iter()
    .flatten()
    {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            let mut found: Vec<PathBuf> = rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && p.extension()
                            .and_then(|e| e.to_str())
                            .map_or(false, |e| e.eq_ignore_ascii_case("csv"))
                })
                .collect();
            found.sort();
            candidates.extend(found);
        }
        if !candidates.is_empty() {
            break;
        }
    }
    candidates.into_iter().next()
}

fn sniff_delimiter(text: &str) -> u8 {
    let first = text.lines().next().unwrap_or("");
    let semis = first.matches(';').count();
    let commas = first.matches(',').count();
    let tabs = first.matches('\t').count();
    if tabs > semis && tabs > commas {
        b'\t'
    } else if semis > commas {
        b';'
    } else {
        b','
    }
}

/// Accept `YYYY-MM-DD`, `YYYY/MM/DD`, `DD_MM_YYYY`, `DD.MM.YYYY`, `DD/MM/YYYY` → `YYYY-MM-DD`.
pub fn parse_any_dob(s: &str) -> Option<String> {
    let parts: Vec<&str> = s
        .trim()
        .split(['-', '_', '.', '/', ' '])
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() != 3 || !parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let (y, m, d) = if parts[0].len() == 4 {
        (parts[0], parts[1], parts[2])
    } else if parts[2].len() == 4 {
        (parts[2], parts[1], parts[0])
    } else {
        return None;
    };
    let (m, d) = (m.parse::<u32>().ok()?, d.parse::<u32>().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(format!("{y}-{m:02}-{d:02}"))
}

enum DobQuery {
    Full(String),
    Year(String),
}

fn normalise_dob_query(s: &str) -> DobQuery {
    if let Some(full) = parse_any_dob(s) {
        return DobQuery::Full(full);
    }
    DobQuery::Year(s.chars().filter(|c| c.is_ascii_digit()).take(4).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib_with(files: &[&str], csv: Option<&str>) -> (tempfile::TempDir, PortraitLibrary) {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("portraits");
        for f in files {
            let shard = &f[..2];
            let dir = root.join(shard);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(f), b"\x89PNG").unwrap();
        }
        if let Some(c) = csv {
            std::fs::write(tmp.path().join("portraits.csv"), c).unwrap();
        }
        let lib = PortraitLibrary::load(&root).unwrap();
        (tmp, lib)
    }

    const FILES: &[&str] = &[
        "aalto_antti_04_03_1975_01.png",
        "aalto_antti_04_03_1975_02.png",
        "aalto_teemu_30_03_1978_01.png",
        "o_brien_jay_04_11_1999_01.png",
        "obrian_jay_04_11_1999_01.png",
        "lee_anders_03_07_1990_01.png",
        "leeman_gary_19_02_1964_01.png",
        "carse_bob_19_07_1919.png",
        "adam_charles_antoine_13_11_2006_01.png",
    ];

    fn key(last: &str, first: &str, dob: &str) -> PersonKey {
        PersonKey {
            last_name: last.into(),
            first_name: first.into(),
            dob: Some(dob.into()),
        }
    }

    fn paths(v: Vec<&PortraitEntry>) -> Vec<String> {
        v.into_iter().map(|e| e.rel_path.clone()).collect()
    }

    #[test]
    fn stem_parsing_handles_variants_dob_and_bare_names() {
        let (_t, lib) = lib_with(FILES, None);
        assert_eq!(lib.len(), FILES.len());
        let e = lib.get("o_/o_brien_jay_04_11_1999_01.png").unwrap();
        assert_eq!(e.tokens, vec!["o", "brien", "jay"]);
        assert_eq!(e.dob.as_deref(), Some("1999-11-04"));
        assert_eq!(e.variant, 1);
        assert_eq!(e.stem, "o_brien_jay_04_11_1999");
        let bob = lib
            .entries()
            .iter()
            .find(|e| e.stem.starts_with("carse"))
            .unwrap();
        assert_eq!(bob.variant, 0);
        assert_eq!(bob.dob.as_deref(), Some("1919-07-19"));
        assert_eq!(bob.stem, "carse_bob_19_07_1919");
    }

    #[test]
    fn exact_tier_matches_apostrophe_names_and_all_variants() {
        let (_t, lib) = lib_with(FILES, None);
        let got = paths(lib.candidates(&key("O'Brien", "Jay", "1999-11-04"), MatchMode::Exact));
        assert_eq!(got, vec!["o_/o_brien_jay_04_11_1999_01.png"]);
        let antti = lib.candidates(&key("Aalto", "Antti", "1975-03-04"), MatchMode::Exact);
        assert_eq!(antti.len(), 2);
        // Multi-part first names do not need a split.
        let ca = lib.candidates(
            &key("Adam", "Charles-Antoine", "2006-11-13"),
            MatchMode::Exact,
        );
        assert_eq!(ca.len(), 1);
    }

    #[test]
    fn looser_tiers_widen_in_order() {
        let (_t, lib) = lib_with(FILES, None);
        let p = key("Aalto", "Anttii", "1975-03-04");
        assert!(lib.candidates(&p, MatchMode::Exact).is_empty());
        assert_eq!(lib.candidates(&p, MatchMode::LastDob).len(), 2);
        assert_eq!(lib.candidates(&p, MatchMode::LastOnly).len(), 3);
        // Surname spelled differently in the library → DOB tier finds it.
        let q = key("O'Brien", "Jay", "1999-11-04");
        assert_eq!(lib.candidates(&q, MatchMode::DobOnly).len(), 2);
        // Token-boundary prefix: "Lee" must not match "Leeman".
        let lee = key("Lee", "Anders", "1990-07-03");
        assert_eq!(lib.candidates(&lee, MatchMode::LastOnly).len(), 1);
        // No DOB on the person → the DOB tiers yield nothing rather than everything.
        let nodob = PersonKey {
            last_name: "Aalto".into(),
            first_name: "Antti".into(),
            dob: None,
        };
        assert!(lib.candidates(&nodob, MatchMode::Exact).is_empty());
        assert_eq!(lib.candidates(&nodob, MatchMode::LastOnly).len(), 3);
    }

    #[test]
    fn search_matches_tokens_and_dob_forms() {
        let (_t, lib) = lib_with(FILES, None);
        assert_eq!(lib.search("aalto", None, 50).len(), 3);
        assert_eq!(lib.search("aalto teemu", None, 50).len(), 1);
        assert_eq!(lib.search("brien", None, 50).len(), 1);
        assert_eq!(lib.search("", Some("04_11_1999"), 50).len(), 2);
        assert_eq!(lib.search("", Some("1999"), 50).len(), 2);
        assert_eq!(lib.search("jay", Some("1999-11-04"), 50).len(), 2);
        assert_eq!(lib.search("aalto", None, 2).len(), 2);
        assert!(lib.search("", None, 50).is_empty());
    }

    #[test]
    fn csv_supplies_split_source_and_aliases() {
        let csv = "file;first_name;last_name;dob;source;aliases\n\
                   adam_charles_antoine_13_11_2006_01.png;Charles-Antoine;Adam;13.11.2006;https://example.org/ca;C.A. Adam\n\
                   obrian_jay_04_11_1999_01.png;Jay;O'Brien;1999-11-04;eliteprospects;\n\
                   missing_file_01.png;X;Y;;;\n";
        let (_t, lib) = lib_with(FILES, Some(csv));
        let st = lib.stats();
        assert_eq!(st.csv_rows, 2);
        assert!(st.warnings.iter().any(|w| w.contains("1 CSV row")));
        let ca = lib
            .entries()
            .iter()
            .find(|e| e.stem.starts_with("adam_charles"))
            .unwrap();
        assert_eq!(ca.display_name(), "Charles-Antoine Adam");
        assert_eq!(ca.source.as_deref(), Some("https://example.org/ca"));
        assert_eq!(ca.tokens, vec!["adam", "charles", "antoine"]);
        assert_eq!(ca.aliases, vec!["caadam"]);
        // The CSV names override the misspelled filename, so Exact now finds both O'Brien files.
        let got = lib.candidates(&key("O'Brien", "Jay", "1999-11-04"), MatchMode::Exact);
        assert_eq!(got.len(), 2);
        assert_eq!(lib.search("eliteprospects", None, 10).len(), 1);
    }

    #[test]
    fn abs_path_refuses_unknown_entries() {
        let (_t, lib) = lib_with(FILES, None);
        assert!(lib.abs_path("../../etc/passwd").is_none());
        assert!(lib.abs_path("aa/aalto_antti_04_03_1975_01.png").is_some());
    }

    /// Smoke the real operator library: `NHL_PORTRAIT_LIBRARY_ROOT=<dir> cargo test -p studio-domain
    /// --lib portrait_library -- --ignored --nocapture`. Fails (not skips) when the root is unset.
    #[test]
    #[ignore]
    fn real_library_smoke() {
        let root = std::env::var("NHL_PORTRAIT_LIBRARY_ROOT")
            .expect("set NHL_PORTRAIT_LIBRARY_ROOT to the portraits folder");
        let t0 = std::time::Instant::now();
        let lib = PortraitLibrary::load(Path::new(&root)).unwrap();
        let st = lib.stats();
        eprintln!(
            "loaded {} portraits / {} people in {:?}; csv={:?} rows={}; warnings={}",
            st.portrait_count,
            st.person_count,
            t0.elapsed(),
            st.csv_path,
            st.csv_rows,
            st.warnings.len()
        );
        for w in st.warnings.iter().take(10) {
            eprintln!("  warn: {w}");
        }
        assert!(st.portrait_count > 0);
        for (last, first, dob) in [
            ("O'Brien", "Jay", "1999-11-04"),
            ("Aalto", "Antti", "1975-03-04"),
            ("Adam", "Charles-Antoine", "2006-11-13"),
            ("Carse", "Bob", "1919-07-19"),
            ("Apps", "Syl", "1915-01-18"),
        ] {
            let k = key(last, first, dob);
            let t = std::time::Instant::now();
            let n: Vec<usize> = [
                MatchMode::Exact,
                MatchMode::LastDob,
                MatchMode::LastOnly,
                MatchMode::DobOnly,
            ]
            .iter()
            .map(|m| lib.candidates(&k, *m).len())
            .collect();
            eprintln!(
                "{first} {last} {dob}: exact/last_dob/last/dob = {n:?} in {:?}",
                t.elapsed()
            );
        }
        let t = std::time::Instant::now();
        let hits = lib.search("gretzky", None, 50);
        eprintln!("search gretzky: {} hits in {:?}", hits.len(), t.elapsed());
    }

    #[test]
    fn dob_parsing_accepts_common_forms() {
        assert_eq!(parse_any_dob("1999-11-04").as_deref(), Some("1999-11-04"));
        assert_eq!(parse_any_dob("04_11_1999").as_deref(), Some("1999-11-04"));
        assert_eq!(parse_any_dob("4.11.1999").as_deref(), Some("1999-11-04"));
        assert_eq!(parse_any_dob("1999").as_deref(), None);
        assert_eq!(parse_any_dob("99-13-01").as_deref(), None);
    }
}
