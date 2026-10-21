//! Indexed, distribution-portable head assets. Native sources are provenance,
//! never required for browsing the compiled GLB/DDS library.
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn root() -> PathBuf {
    if let Some(path) = std::env::var_os("NHL_HEAD_LIBRARY") {
        return PathBuf::from(path);
    }
    let relative = Path::new("graphical database").join("headmeshes");
    for start in [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf)),
    ]
    .into_iter()
    .flatten()
    {
        for parent in start.ancestors() {
            let path = parent.join(&relative);
            if path.join("headmeshes.sqlite").is_file() {
                return path;
            }
        }
    }
    relative
}

fn connection(root: &Path) -> Result<Connection> {
    let db = Connection::open_with_flags(
        root.join("headmeshes.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let version: i32 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version != 1 {
        return Err("Unsupported head library version".into());
    }
    Ok(db)
}

pub fn status() -> Result<Value> {
    let root = root();
    if !root.join("headmeshes.sqlite").is_file() {
        return Ok(json!({"available": false, "root": root, "assets": 0}));
    }
    let db = connection(&root)?;
    let count: i64 = db.query_row("SELECT count(*) FROM assets", [], |r| r.get(0))?;
    Ok(json!({"available": true, "root": root, "assets": count}))
}

pub fn search(query: &str, class: &str, offset: u32) -> Result<Value> {
    let db = connection(&root())?;
    // Parameterized LIKE with literal wildcard escaping, bounded page size.
    let terms: Vec<_> = query
        .split_whitespace()
        .take(12)
        .map(|term| {
            format!(
                "%{}%",
                term.to_lowercase()
                    .replace('ø', "o")
                    .replace('ł', "l")
                    .replace('ß', "ss")
                    .replace('æ', "ae")
                    .replace('œ', "oe")
                    .replace('đ', "d")
                    .replace('ð', "d")
                    .replace('þ', "th")
                    .nfkd()
                    .filter(|c| !is_combining_mark(*c))
                    .collect::<String>()
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
        })
        .collect();
    let mut values = vec![
        rusqlite::types::Value::Text(class.to_owned()),
        rusqlite::types::Value::Integer(i64::from(offset)),
    ];
    let mut sql = String::from("SELECT a.id,a.class,a.stem,a.mesh_file,a.game,a.platform,a.origin,a.lod,
         (SELECT group_concat(DISTINCT game || '/' || platform) FROM appearances p WHERE p.asset_id=a.id)
         FROM assets a WHERE (?1='' OR a.class=?1)");
    for term in terms {
        values.push(rusqlite::types::Value::Text(term));
        let parameter = values.len();
        sql.push_str(&format!(" AND (a.stem LIKE ?{parameter} ESCAPE '\\' OR EXISTS
          (SELECT 1 FROM appearances p WHERE p.asset_id=a.id AND p.identity_slug LIKE ?{parameter} ESCAPE '\\'))"));
    }
    sql.push_str(" ORDER BY CASE WHEN lower(a.game) LIKE '%legacy%' THEN 2016 ELSE 2000 + CAST(substr(a.game,4) AS INTEGER) END DESC,a.platform,a.lod,a.id LIMIT 100 OFFSET ?2");
    let mut statement = db.prepare(&sql)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values), |r| {
            Ok(json!({
                "id":r.get::<_,String>(0)?, "class":r.get::<_,String>(1)?,
                "stem":r.get::<_,String>(2)?, "mesh_file":r.get::<_,String>(3)?,
                "game":r.get::<_,String>(4)?, "platform":r.get::<_,String>(5)?,
                "origin":r.get::<_,String>(6)?, "lod":r.get::<_,i64>(7)?,
                "sources":r.get::<_,String>(8)?
            }))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(json!(rows))
}

pub fn detail(id: &str) -> Result<Value> {
    let db = connection(&root())?;
    let mut statement = db.prepare("SELECT id,identity_slug,game,platform,source,textures_json,lod,entry_json FROM appearances WHERE asset_id=?1 ORDER BY id")?;
    let rows = statement
        .query_map([id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, String>(7)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut appearances = Vec::new();
    for (id, identity, game, platform, source, textures, lod, entry) in rows {
        appearances.push(
            json!({"id":id,"identity":identity,"game":game,"platform":platform,
            "source":source,"textures":serde_json::from_str::<Value>(&textures)?,"lod":lod,
            "original":serde_json::from_str::<Value>(&entry)?}),
        );
    }
    Ok(json!(appearances))
}

pub fn file(relative: &str) -> Result<Vec<u8>> {
    let root = std::fs::canonicalize(root())?;
    let path = std::fs::canonicalize(root.join(relative))?;
    if !path.starts_with(&root)
        || !matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("glb" | "dds")
        )
    {
        return Err("Not a library asset path".into());
    }
    if std::fs::metadata(&path)?.len() > 128 * 1024 * 1024 {
        return Err("Asset exceeds preview limit".into());
    }
    Ok(std::fs::read(path)?)
}

pub fn texture_rgba(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    if bytes.len() < 128 || &bytes[..4] != b"DDS " {
        return Err("Invalid DDS".into());
    }
    let word = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let (width, height) = (word(16), word(12));
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err("DDS dimensions exceed preview limits".into());
    }
    if &bytes[84..88] == b"DX10" {
        if bytes.len() < 148 {
            return Err("Truncated DX10 DDS".into());
        }
        let format = word(128);
        let payload = &bytes[148..];
        let block: usize = if matches!(format, 71 | 72 | 80) {
            8
        } else {
            16
        };
        let expected = if format == 28 {
            width as usize * height as usize * 4
        } else {
            width.div_ceil(4) as usize * height.div_ceil(4) as usize * block
        };
        if payload.len() < expected {
            return Err("Truncated DDS pixels".into());
        }
        let payload = &payload[..expected];
        let rgba = match format {
            28 => payload.to_vec(),
            80 => tdb_rx2::gnf::decode_bc4(payload, width, height),
            83 => tdb_rx2::gnf::decode_bc5(payload, width, height),
            98 | 99 => tdb_rx2::gnf::decode_bc7(payload, width, height),
            71 | 72 | 77 | 78 => {
                let format = if matches!(format, 71 | 72) {
                    tdb_archive_codec::DdsFormat::Dxt1
                } else {
                    tdb_archive_codec::DdsFormat::Dxt5
                };
                tdb_archive_codec::decode_dds_to_rgba(&tdb_archive_codec::encode_dds(
                    format, width, height, 1, payload,
                ))?
                .rgba
            }
            _ => return Err("Unsupported DXGI preview format".into()),
        };
        return Ok((width, height, rgba));
    }
    if &bytes[84..88] == b"\0\0\0\0"
        && word(88) == 32
        && word(92) == 255
        && word(96) == 65280
        && word(100) == 16711680
    {
        let end = 128 + width as usize * height as usize * 4;
        return Ok((
            width,
            height,
            bytes.get(128..end).ok_or("Truncated RGBA DDS")?.to_vec(),
        ));
    }
    let decoded = tdb_archive_codec::decode_dds_to_rgba(bytes)?;
    Ok((width, height, decoded.rgba))
}

pub fn export(id: &str, appearance: i64, destination: &Path) -> Result<Value> {
    let db = connection(&root())?;
    let mesh: String = db.query_row("SELECT mesh_file FROM assets WHERE id=?1", [id], |r| {
        r.get(0)
    })?;
    let provenance = detail(id)?;
    let chosen = provenance
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == appearance)
        .ok_or("Appearance does not belong to asset")?;
    if destination.exists() {
        return Err("Choose a new export folder".into());
    }
    let parent = std::fs::canonicalize(destination.parent().ok_or("Missing output parent")?)?;
    if parent.starts_with(std::fs::canonicalize(root())?) {
        return Err("Export outside the library".into());
    }
    std::fs::create_dir(destination)?;
    std::fs::write(
        destination.join(Path::new(&mesh).file_name().unwrap()),
        file(&mesh)?,
    )?;
    std::fs::create_dir(destination.join("textures"))?;
    for texture in chosen["textures"]
        .as_array()
        .ok_or("Invalid texture manifest")?
    {
        let path = texture["file"].as_str().ok_or("Missing texture path")?;
        let name = Path::new(path)
            .file_name()
            .ok_or("Missing texture filename")?;
        std::fs::write(destination.join("textures").join(name), file(path)?)?;
    }
    let mut portable = chosen.clone();
    for texture in portable["textures"]
        .as_array_mut()
        .ok_or("Invalid texture manifest")?
    {
        let name = Path::new(texture["file"].as_str().ok_or("Missing texture path")?)
            .file_name()
            .ok_or("Missing texture filename")?
            .to_string_lossy()
            .into_owned();
        texture["file"] = json!(format!("textures/{name}"));
    }
    std::fs::write(
        destination.join("provenance.json"),
        serde_json::to_vec_pretty(&json!({
        "asset":id,"mesh":Path::new(&mesh).file_name().unwrap().to_string_lossy(),
        "selected":portable,"library_mesh":mesh,"all_sources":provenance}))?,
    )?;
    Ok(json!({"destination":destination}))
}

pub fn export_composition(
    id: &str,
    appearance: i64,
    companions: &[String],
    destination: &Path,
) -> Result<Value> {
    if companions.is_empty() {
        return export(id, appearance, destination);
    }
    if companions.len() > 2 {
        return Err("Choose at most one hair and one facial-hair asset".into());
    }
    let db = connection(&root())?;
    let mut parts = Vec::new();
    let mut classes = std::collections::HashSet::new();
    // Validate all selections before creating any output.
    for companion in companions {
        let class: String =
            db.query_row("SELECT class FROM assets WHERE id=?1", [companion], |r| {
                r.get(0)
            })?;
        if !matches!(class.as_str(), "hair" | "facialhair") || !classes.insert(class.clone()) {
            return Err("Companions must be one hair and/or one facial-hair asset".into());
        }
        let variants = detail(companion)?;
        let selected = variants
            .as_array()
            .and_then(|a| a.first())
            .ok_or("Companion has no source appearance")?;
        parts.push((
            companion.clone(),
            class,
            selected["id"].as_i64().ok_or("Invalid appearance")?,
        ));
    }
    export(id, appearance, destination)?;
    for (asset, class, selected) in &parts {
        export(asset, *selected, &destination.join(class))?;
    }
    std::fs::write(
        destination.join("composition.json"),
        serde_json::to_vec_pretty(&json!({
            "version":1,"head":{"asset":id,"appearance":appearance,"package":"."},
            "companions":parts.iter().map(|(asset,class,selected)|json!({
                "asset":asset,"appearance":selected,"package":class})).collect::<Vec<_>>(),
            "coordinate_policy":"Native coordinates retained; cross-game fitting and native installation are separate."
        }))?,
    )?;
    Ok(json!({"destination":destination}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn texture_preview_preserves_classic_and_dx10_bc1_pixels() {
        let classic =
            tdb_archive_codec::encode_dds(tdb_archive_codec::DdsFormat::Dxt1, 4, 4, 1, &[0; 8]);
        let mut dx = classic[..128].to_vec();
        dx[84..88].copy_from_slice(b"DX10");
        for word in [72u32, 3, 0, 1, 0] {
            dx.extend_from_slice(&word.to_le_bytes());
        }
        dx.extend_from_slice(&classic[128..]);
        assert_eq!(texture_rgba(&classic).unwrap(), texture_rgba(&dx).unwrap());
        assert!(texture_rgba(&dx[..dx.len() - 1]).is_err());
        dx[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(texture_rgba(&dx).is_err());
    }
    #[test]
    fn uncompressed_rgba_stays_exact() {
        let pixels = [1, 2, 3, 4, 5, 6, 7, 8];
        let dds =
            tdb_archive_codec::encode_dds(tdb_archive_codec::DdsFormat::Rgba32, 2, 1, 1, &pixels);
        assert_eq!(texture_rgba(&dds).unwrap(), (2, 1, pixels.to_vec()));
    }
}
