//! Phase 1.4 / P1-G — the league-keyed presentation imagery beyond the four league-logo families
//! (research handoff 03): on Xbox 360 / PS3 the playoff-tree conference/league logos, the
//! championship and broadcast trophies and the MUD-mode league logos (`fe/ion/artassets/<family>/
//! <prefix><id>.big`, the same package chains as the league logos); on PS4 the league-shaped
//! asset-path registry types (`nhl/ui/definition[.kettle].assetpaths` templates with `#` = id and a
//! French tag). The support data is `defs/league_imagery.toml`, generated from the handoff's data
//! file; this module expands it per title into loose-file paths the shared texture surface
//! (preview / same-dimension replace) addresses. The id ↔ league mapping of the gen7 families is
//! computed in game code and stays unknown here — the listing is per title, not per league.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::{Platform, Result};

/// `defs/league_imagery.toml`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LeagueImageryDefs {
    #[serde(default)]
    pub source_sha256: String,
    #[serde(default)]
    pub gen7: Vec<Gen7Source>,
    #[serde(default)]
    pub ps4: Vec<Ps4Title>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Gen7Source {
    pub platform: String,
    pub title: String,
    #[serde(default)]
    pub families: Vec<Gen7Family>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Gen7Family {
    pub family: String,
    pub prefix: String,
    pub label: String,
    pub reading: String,
    #[serde(default)]
    pub consumers: Vec<String>,
    pub ids: Vec<u64>,
    /// `id:WxH:format` facts where the survey decoded the member (NHL 07–11 Xbox).
    #[serde(default)]
    pub texture_facts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ps4Title {
    pub title: String,
    pub registry: String,
    pub registry_sha256: String,
    #[serde(default)]
    pub types: Vec<Ps4Type>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ps4Type {
    #[serde(rename = "type")]
    pub kind: String,
    pub registry_key: String,
    pub label: String,
    pub template: String,
    #[serde(default)]
    pub consumers: Vec<String>,
    #[serde(default)]
    pub ids: Vec<u64>,
    #[serde(default)]
    pub fr_ids: Vec<u64>,
    #[serde(default)]
    pub present: Option<bool>,
    #[serde(default)]
    pub fr_present: Option<bool>,
}

impl LeagueImageryDefs {
    pub fn from_toml(text: &str) -> Result<Self> {
        toml::from_str(text).map_err(|e| DomainError::Validation(format!("league_imagery.toml: {e}")))
    }

    /// Load `<defs>/league_imagery.toml`; absent file = empty defs (projects built from parts).
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_toml(&text).map_err(|e| DomainError::Validation(format!("{}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(DomainError::Io { path: path.display().to_string(), source }),
        }
    }
}

/// One member of a family: the loose path under the install / extraction root.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageryMember {
    pub id: u64,
    pub rel_path: String,
    /// `None` when no root was given.
    pub exists: Option<bool>,
    /// The French twin (PS4 17–21 `_fr` files) when the registry declares one.
    pub fr_rel_path: Option<String>,
    pub fr_exists: Option<bool>,
    /// The packed archive the member ships in (PS4), for the archive-member replace route.
    pub archive: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageryFamily {
    /// Family folder (gen7) or registry type (PS4).
    pub family: String,
    pub label: String,
    /// The handoff's inferred reading of the members, for the user to confirm.
    pub reading: String,
    /// Observed consumer screens / layouts (empty = none referenced in UI text).
    pub consumers: Vec<String>,
    /// Texture facts per member where the survey decoded them (`id:WxH:format`).
    pub texture_facts: Vec<String>,
    pub members: Vec<ImageryMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeagueImagery {
    pub game_id: String,
    pub platform: String,
    /// Where the family ↔ path binding was read: the gen7 `generalhelpers` table or the PS4 registry.
    pub registry: String,
    pub families: Vec<ImageryFamily>,
    pub notes: Vec<String>,
}

fn exists(root: Option<&Path>, rel: &str) -> Option<bool> {
    root.map(|r| r.join(rel).is_file())
}

/// Expand a PS4 template for one id: `#` → id, the French tag → `""` (English) or `"_fr"`.
fn ps4_path(template: &str, id: Option<u64>, french: bool) -> String {
    let tag = if french { "_fr" } else { "" };
    let mut p = template.replace("LANG_TAG_FR", tag).replace("[loc FRE_FR]", tag);
    if let Some(id) = id {
        p = p.replace('#', &id.to_string());
    }
    format!("nhl/ui/{p}")
}

/// The league-keyed presentation imagery of this title, with loose-file existence under
/// `install_root`. `None` when the defs carry nothing for the title/platform.
pub fn league_imagery(
    defs: &LeagueImageryDefs,
    game_id: &str,
    platform: Platform,
    install_root: Option<&Path>,
) -> Option<LeagueImagery> {
    match platform {
        Platform::Ps4 => {
            let t = defs.ps4.iter().find(|t| t.title == game_id)?;
            let families = t
                .types
                .iter()
                .map(|ty| {
                    let members = if ty.ids.is_empty() && ty.present.is_some() {
                        let rel = ps4_path(&ty.template, None, false);
                        let fr = if ty.fr_present == Some(true) { Some(ps4_path(&ty.template, None, true)) } else { None };
                        vec![ImageryMember {
                            id: 0,
                            exists: exists(install_root, &rel),
                            fr_exists: fr.as_deref().and_then(|f| exists(install_root, f)),
                            rel_path: rel,
                            fr_rel_path: fr,
                            archive: crate::league_logos::ps4_logo_archive(Some(game_id)),
                        }]
                    } else {
                        ty.ids
                            .iter()
                            .map(|&id| {
                                let rel = ps4_path(&ty.template, Some(id), false);
                                let fr = ty.fr_ids.contains(&id).then(|| ps4_path(&ty.template, Some(id), true));
                                ImageryMember {
                                    id,
                                    exists: exists(install_root, &rel),
                                    fr_exists: fr.as_deref().and_then(|f| exists(install_root, f)),
                                    rel_path: rel,
                                    fr_rel_path: fr,
                                    archive: crate::league_logos::ps4_logo_archive(Some(game_id)),
                                }
                            })
                            .collect()
                    };
                    ImageryFamily {
                        family: ty.kind.clone(),
                        label: ty.label.clone(),
                        reading: String::new(),
                        consumers: ty.consumers.clone(),
                        texture_facts: Vec::new(),
                        members,
                    }
                })
                .collect();
            Some(LeagueImagery {
                game_id: game_id.to_string(),
                platform: platform.as_str().to_string(),
                registry: t.registry.clone(),
                families,
                notes: vec![
                    "PS4: paths come from the title's asset-path registry (`#` = id; the French tag expands to `_fr`); files are addressed as extracted copies under the install root (inside the EB archives on disc)".into(),
                    "Scouting region logos alias league logos (region 0 = l11, 1 = l12, 2 = l13, 7 = l3, 8 = l2 on every title); on NHL 16–21 they are shared archive spans, so replacing a league logo in place also changes the region art".into(),
                    "`l100` is a byte copy of `l5`; `league_logo` and `collection_league_logo` are one file".into(),
                ],
            })
        }
        // Recompiled ships the Xbox 360 Legacy files (research 01): same folders by identity.
        Platform::Xbox | Platform::Ps3 | Platform::Recomp => {
            let plat = if matches!(platform, Platform::Ps3) { "ps3" } else { "x360" };
            let s = defs.gen7.iter().find(|s| s.platform == plat && s.title == game_id)?;
            let archive = crate::league_logos::gen7_logo_archive_at(Some(game_id), Some(platform), install_root);
            let families = s
                .families
                .iter()
                .map(|f| ImageryFamily {
                    family: f.family.clone(),
                    label: f.label.clone(),
                    reading: f.reading.clone(),
                    consumers: f.consumers.clone(),
                    texture_facts: f.texture_facts.clone(),
                    members: f
                        .ids
                        .iter()
                        .map(|&id| {
                            let rel = format!("fe/ion/artassets/{}/{}{id}.big", f.family, f.prefix);
                            ImageryMember {
                                id,
                                exists: exists(install_root, &rel),
                                rel_path: rel,
                                fr_rel_path: None,
                                fr_exists: None,
                                archive: archive.clone(),
                            }
                        })
                        .collect(),
                })
                .collect();
            let traced = matches!(game_id, "nhl11" | "nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy");
            let mut notes = vec![
                "Same package chains as the league logos (BIGF package, RefPack'd on the packed titles); preview and same-dimension replace go through the shared texture surface; the id ↔ league mapping is computed in game code (unknown)".into(),
            ];
            if !traced {
                notes.push("NHL 07–10: no `generalhelpers` table in boot.big — the family folders exist but their screens were not traced (consumers listed by reference to NHL 11–Legacy)".into());
            }
            if matches!(platform, Platform::Ps3) {
                notes.push("PS3: registry and consumers read from the Xbox 360 `boot.big` of the same title (by reference); the PS3 packages were measured directly".into());
            }
            if let Some(a) = &archive {
                notes.push(if crate::league_logos::gen7_logo_archive_is_big4(Some(game_id)) {
                    format!("the console reads the packages from the BIG4 archive {a}, whose `.bh` index beside it is patched together with it; a replacement that does not fit its slot is appended and re-pointed (a shape no stock archive exhibits — card R2-BIG4-1)")
                } else if matches!(platform, Platform::Ps3) {
                    format!("PS3: the packages ship twice — loose under USRDIR/fe/ion/artassets and as members of {a}; which copy the console reads is research unknown, so a loose replace and an archive replace are separate operations and the other copy stays as shipped")
                } else {
                    format!("the console reads the packages from {a}; a loose file under the install root is an extraction copy")
                });
            }
            notes.push("No league-keyed selection-tile or banner family in the searched registries (`generalhelpers`); `.gfx`, executables and other consumers were not searched, so their absence is research unknown, not established; `lobbyimg` league banners are consumed only by the online lobby".into());
            Some(LeagueImagery {
                game_id: game_id.to_string(),
                platform: platform.as_str().to_string(),
                registry: if traced { "boot.big: fe/ion/game/pods/helpers/generalhelpers.big (getArtAssetPath)".into() } else { "family folders (screens not traced on this title)".into() },
                families,
                notes,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps4_templates_expand_ids_and_french_tags() {
        assert_eq!(ps4_path("kettle_assets/leaguelogos/size512/l#[loc FRE_FR].gnf", Some(7), false), "nhl/ui/kettle_assets/leaguelogos/size512/l7.gnf");
        assert_eq!(ps4_path("kettle_assets/leaguelogos/size512/l#[loc FRE_FR].gnf", Some(7), true), "nhl/ui/kettle_assets/leaguelogos/size512/l7_fr.gnf");
        assert_eq!(ps4_path("kettle_assets/leaguelogos/size256/memcupLANG_TAG_FR.gnf", None, true), "nhl/ui/kettle_assets/leaguelogos/size256/memcup_fr.gnf");
        assert_eq!(ps4_path("assets/gmconnected/scouting/regionsLogos/region-#.png", Some(0), false), "nhl/ui/assets/gmconnected/scouting/regionsLogos/region-0.png");
    }
}
