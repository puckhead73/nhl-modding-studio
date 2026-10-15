//! Phase 1.4 — league logo assets: league code → family-wide **art index** → frontend package
//! paths, per title (research handoff P1.4-X, 2026-09-18).
//!
//! * NHL 09 → Legacy (Xbox 360; PS3 by identity of the file names): the executable holds a small
//!   table league code → logo index, baked in `defs/leagues/logos/<game>.toml`; the packages are
//!   `fe/ion/artassets/leaguelogos{,big,small,wide}/l<index>.big` (BIGF, RefPack'd XPR2 on
//!   12–Legacy / RW4 on 09–11 / DDS on PS3) — the same package shape Studio's portrait and team
//!   crest codecs already decode and re-encode.
//! * PS4 NHL 15 → 21: the index is stored natively as `stockleagues.artid`; the files are
//!   `nhl/ui/kettle_assets/leaguelogos/size{64,256,512}/l<index>.gnf` (+ `_fr`).
//! * NHL 07/08: no table found; by count the index equals the code (inferred, not decoded).
//!
//! This module only *resolves*: which index, which files, whether each exists under the given
//! install/extraction root, and how well the binding is established. Preview and replace go
//! through the existing texture surface (`texture_preview` / `import_texture`) on those paths.
//! Which screen draws which family, and the archive-vs-loose precedence at runtime, are
//! tomorrow's cards.

use std::path::Path;

use serde::Deserialize;

use crate::leagues::{LeagueCatalog, LeagueError, LeagueSource};

/// `defs/leagues/logos/<game>.toml`.
#[derive(Clone, Debug, Deserialize)]
pub struct LogoTable {
    pub game: String,
    pub applies_to: Vec<String>,
    pub witness: LogoWitness,
    pub codes: Vec<LogoCode>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LogoWitness {
    pub game: String,
    pub platform: String,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LogoCode {
    pub code: u64,
    pub key: String,
    pub logo_index: u64,
}

impl LogoTable {
    pub fn from_toml(text: &str) -> Result<Self, LeagueError> {
        toml::from_str(text).map_err(|e| LeagueError::Parse(e.to_string()))
    }

    pub fn load_dir(dir: &Path) -> Result<Vec<LogoTable>, LeagueError> {
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut paths: Vec<_> = std::fs::read_dir(dir)
            .map_err(LeagueError::Io)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        paths.sort();
        let mut out = Vec::new();
        for p in paths {
            let text = std::fs::read_to_string(&p).map_err(LeagueError::Io)?;
            out.push(Self::from_toml(&text)?);
        }
        Ok(out)
    }

    pub fn applies(&self, game_id: &str) -> bool {
        self.applies_to.iter().any(|g| g == game_id)
    }
}

/// One package family of a league logo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogoVariant {
    /// `leaguelogos` / `leaguelogosbig` / `leaguelogossmall` / `leaguelogoswide`; PS4 `size64` …
    pub family: String,
    /// Path relative to the install/extraction root (the argument `texture_preview` takes).
    pub rel_path: String,
    /// Whether the file exists loose under the given root (`None` when no root was given).
    pub exists: Option<bool>,
    /// The packed archive (relative to the install root) the member ships in, when the title reads
    /// it from an archive (PS4: `databigs/inxp10.big` on 15, `inxp_10.big` on 16–18/20/21,
    /// `inxp_10-010.big` on 19 — the fixture manifest's source archives). Feeds the archive-member
    /// replace route (D8). `None` for the gen7 loose packages.
    pub archive: Option<String>,
}

/// The packed **gen7** archive that carries the `fe/ion/artassets/**` league-logo / imagery
/// packages on the titles that ship them packed (research 16 / 02 §0): Xbox 360 NHL 11 `fe.big`,
/// NHL 12–15 / Legacy `cache.big` (Recompiled reads the same Legacy files), PS3 NHL 12–15 / Legacy
/// `cache.big` under `USRDIR` (those titles also ship a loose copy of every package — which copy the
/// console reads is research unknown; both are offered and the pair is disclosed); NHL 11 PS3 is
/// loose-only. Every member is stored raw (a plain BIGF payload) in these EB `0x00000400` archives.
/// NHL 07–10 (Xbox and PS3) keep the packages inside **BIG4** archives with a Viv4 `.bh` sidecar
/// (Xbox 07/08 and PS3 08 `data0.big`, Xbox/PS3 09/10 `fe.big`; the Xbox packages RefPack-wrapped
/// whole) — the BIG4 + `.bh` route (C4) patches both files as one change set.
pub fn gen7_logo_archive(game_id: Option<&str>, platform: Option<crate::Platform>) -> Option<String> {
    let g = game_id?;
    Some(
        match (platform?, g) {
            (crate::Platform::Xbox, "nhl07" | "nhl08") => "data0.big",
            (crate::Platform::Xbox, "nhl09" | "nhl10") => "fe.big",
            (crate::Platform::Ps3, "nhl08") => "data0.big",
            (crate::Platform::Ps3, "nhl09" | "nhl10") => "fe.big",
            (crate::Platform::Xbox, "nhl11") => "fe.big",
            (crate::Platform::Xbox | crate::Platform::Recomp, "nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy") => "cache.big",
            // Receipt 59: the catalogue id of Legacy Recompiled (its `cache.big` is the X360 Legacy archive, R128).
            (crate::Platform::Recomp, "legacyrecomp" | "nhllegacy-recompiled") => "cache.big",
            (crate::Platform::Ps3, "nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy") => "cache.big",
            _ => return None,
        }
        .to_string(),
    )
}

/// Whether `gen7_logo_archive`'s answer is a BIG4 + `.bh` pair (NHL 07–10) rather than an EB archive.
pub fn gen7_logo_archive_is_big4(game_id: Option<&str>) -> bool {
    matches!(game_id, Some("nhl07" | "nhl08" | "nhl09" | "nhl10"))
}

/// `gen7_logo_archive` narrowed to what exists: with a root, only when the archive file (and, for
/// a BIG4 archive, its `.bh` sidecar) is there.
pub fn gen7_logo_archive_at(game_id: Option<&str>, platform: Option<crate::Platform>, root: Option<&Path>) -> Option<String> {
    let a = gen7_logo_archive(game_id, platform)?;
    match root {
        Some(r) if !r.join(&a).is_file() => None,
        Some(r) if gen7_logo_archive_is_big4(game_id) && !r.join(crate::asset_member_edit::big4_sidecar_relative(&a)).is_file() => None,
        _ => Some(a),
    }
}

/// The PS4 archive the league-logo / imagery members ship in, per title.
pub fn ps4_logo_archive(game_id: Option<&str>) -> Option<String> {
    Some(match game_id? {
        "nhl15" => "databigs/inxp10.big",
        "nhl19" => "databigs/inxp_10-010.big",
        "nhl16" | "nhl17" | "nhl18" | "nhl20" | "nhl21" => "databigs/inxp_10.big",
        _ => return None,
    }
    .to_string())
}

/// A league's logo binding and its files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeagueLogo {
    pub code: u64,
    pub art_index: u64,
    /// `native` (PS4 `stockleagues.artid`), `traced` (this target's executable table),
    /// `by-reference` (the Xbox twin's table; same file names), `inferred` (index = code by count).
    pub evidence: String,
    pub witness: String,
    /// Other league codes of this title that share the same index (they show the same logo).
    pub shared_with: Vec<(u64, String)>,
    pub variants: Vec<LogoVariant>,
    /// Plain-language limits.
    pub notes: Vec<String>,
}

fn gen7_variants(idx: u64, root: Option<&Path>, archive: Option<String>) -> Vec<LogoVariant> {
    ["leaguelogos", "leaguelogosbig", "leaguelogossmall", "leaguelogoswide"]
        .iter()
        .map(|fam| {
            let rel = format!("fe/ion/artassets/{fam}/l{idx}.big");
            LogoVariant {
                family: fam.to_string(),
                exists: root.map(|r| r.join(&rel).is_file()),
                rel_path: rel,
                archive: archive.clone(),
            }
        })
        .collect()
}

/// The PS4 member set per title (research 16 S-A/S-B, measured on the `l0` members of every
/// title): NHL 15 ships PNGs under `ui/assets` in two sizes; NHL 16 GNFs in the same two sizes;
/// NHL 17–21 add `size512` and a French `_fr` twin of every size.
/// Locate a loose logo without changing its canonical archive member name.
/// PS4 extraction may sit beside the databigs or under a separate Extracted tree.
pub fn loose_logo_path(root: &Path, member: &str) -> Option<String> {
    let direct = crate::assets::resolve_install_path(root, member)?;
    if direct.is_file() { return Some(member.to_string()); }
    let ui = member.strip_prefix("nhl/ui/")?;
    for prefix in ["databigs/Extracted", "databigs", "Extracted"] {
        let rel = format!("{prefix}/{member}");
        if root.join(&rel).is_file() { return Some(rel); }
    }
    // Reuse asset discovery for other supported nested extraction layouts, but keep
    // the returned path inside the selected root so texture writes stay contained.
    let roots = crate::assets::AssetRoots::resolve(root, crate::Platform::Ps4);
    let path = Path::new(&roots.fe).join(ui);
    if !path.is_file() { return None; }
    Some(path.strip_prefix(root).ok()?.to_string_lossy().replace('\\', "/"))
}

fn ps4_variants(game_id: Option<&str>, idx: u64, root: Option<&Path>) -> Vec<LogoVariant> {
    let (dir, ext, sizes, suffixes): (&str, &str, &[u32], &[&str]) = match game_id {
        Some("nhl15") => ("assets", "png", &[64, 256], &[""]),
        Some("nhl16") => ("kettle_assets", "gnf", &[64, 256], &[""]),
        _ => ("kettle_assets", "gnf", &[64, 256, 512], &["", "_fr"]),
    };
    let mut out = Vec::new();
    for size in sizes {
        for suffix in suffixes {
            let rel = format!("nhl/ui/{dir}/leaguelogos/size{size}/l{idx}{suffix}.{ext}");
            out.push(LogoVariant {
                family: format!("size{size}{suffix}"),
                exists: root.map(|r| loose_logo_path(r, &rel).is_some()),
                rel_path: rel,
                archive: ps4_logo_archive(game_id),
            });
        }
    }
    out
}

/// Resolve one league's logo on this title. `install_root` is the install / extraction root the
/// texture surface addresses files against (loose files; archive-only members show `exists =
/// false` — the disc ships them packed on NHL 11+).
pub fn league_logo(
    catalog: &LeagueCatalog,
    tables: &[LogoTable],
    game_id: Option<&str>,
    platform: Option<crate::Platform>,
    code: u64,
    install_root: Option<&Path>,
) -> Option<LeagueLogo> {
    let league = catalog.league(code)?;
    let mut notes = Vec::new();
    if catalog.source == LeagueSource::StockLeagues {
        let idx = league.art_id?;
        let shared: Vec<(u64, String)> = catalog
            .leagues
            .iter()
            .filter(|l| l.code != code && l.art_id == Some(idx))
            .map(|l| (l.code, l.display()))
            .collect();
        notes.push(match game_id {
            Some("nhl15") => "PS4 NHL 15: the art index is the stockleagues row's own `artid`; the logos are plain PNGs in the ui/assets tree (inside the EB archive on disc — extracted copies are addressed here)".into(),
            _ => "PS4: the art index is the stockleagues row's own `artid`; files live in the kettle_assets tree (inside inxp_10.big on disc — extracted copies are addressed here)".into(),
        });
        return Some(LeagueLogo {
            code,
            art_index: idx,
            evidence: "native".into(),
            witness: "stockleagues.artid on this database".into(),
            shared_with: shared,
            variants: ps4_variants(game_id, idx, install_root),
            notes,
        });
    }
    let g = game_id?;
    let table = tables.iter().find(|t| t.applies(g));
    let (idx, evidence, witness) = match table {
        Some(t) => {
            let entry = t.codes.iter().find(|c| c.code == code)?;
            let traced = platform.is_some_and(|p| t.witness.game == g && t.witness.platform == p.as_str());
            (
                entry.logo_index,
                if traced { "traced" } else { "by-reference" },
                t.witness.source.clone(),
            )
        }
        None if matches!(g, "nhl07" | "nhl08") => {
            notes.push("no logo-index table was found in this executable; by count the index equals the league code (inferred, not decoded)".into());
            (code, "inferred", "P1.4-X §4: NHL 07/08 leaguelogos sets match the league table 1:1 by count".to_string())
        }
        None => return None,
    };
    let shared: Vec<(u64, String)> = table
        .map(|t| {
            t.codes
                .iter()
                .filter(|c| c.code != code && c.logo_index == idx)
                .filter_map(|c| catalog.league(c.code).map(|l| (c.code, l.display())))
                .collect()
        })
        .unwrap_or_default();
    if !shared.is_empty() {
        notes.push(format!(
            "index {idx} is shared: replacing this logo also changes {}",
            shared.iter().map(|(c, l)| format!("{l} (code {c})")).collect::<Vec<_>>().join(", ")
        ));
    }
    notes.push("packages are BIGF with a RefPack'd texture (XPR2 on NHL 12–Legacy, RW4 on 09–11, DDS on PS3); which screen uses which family is untraced".into());
    let archive = gen7_logo_archive_at(game_id, platform, install_root);
    if let Some(a) = &archive {
        if gen7_logo_archive_is_big4(game_id) {
            notes.push(format!("the console reads the packages from the BIG4 archive {a}, whose `.bh` index beside it is patched together with it; a replacement that does not fit its slot grows the archive by an ordered reflow (the stock shape: the slot grows, every later entry moves up; loader acceptance is card R2-BIG4-1){}", if matches!(game_id, Some("nhl07")) { "; on NHL 07 `data0.big`'s `.bh` is mirrored inside boot.big, so a changed index is placed into boot.big too and boot.bh follows — four files in one transaction" } else { "" }));
        } else if platform == Some(crate::Platform::Ps3) {
            notes.push(format!("PS3: the packages ship twice — loose under USRDIR/fe/ion/artassets and as members of {a}; which copy the console reads is research unknown, so a loose replace and an archive replace are separate operations and the other copy stays as shipped"));
        } else {
            notes.push(format!("the console reads the packages from {a}; a loose file under the install root is an extraction copy"));
        }
    }
    Some(LeagueLogo {
        code,
        art_index: idx,
        evidence: evidence.into(),
        witness,
        shared_with: shared,
        variants: gen7_variants(idx, install_root, archive),
        notes,
    })
}
