//! Lazy install access. Never read entire CAS files or modify a game directory.
use crate::{
    db, invalid, manifest,
    toc::{self, Span, Toc},
    Result,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
};
const MAX_INDEX: u64 = 128 * 1024 * 1024;
pub const MAX_ASSET: usize = 256 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct Layer {
    pub name: String,
    pub head: Option<i64>,
    pub base: Option<i64>,
    pub superbundles: Vec<String>,
    pub initfs: String,
    pub chunkmanifest: bool,
}
#[derive(Debug, Serialize)]
pub struct Install {
    pub root: String,
    pub layers: Vec<Layer>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct BundleRow {
    pub ordinal: usize,
    pub name: String,
    pub spans: usize,
}
#[derive(Debug, Serialize)]
pub struct BundlePage {
    pub total: usize,
    pub matched: usize,
    pub toc_chunks: usize,
    pub items: Vec<BundleRow>,
}
#[derive(Debug, Serialize)]
pub struct BundleAssets {
    pub name: String,
    pub assets: Vec<manifest::Asset>,
    pub spans: Vec<Span>,
}
#[derive(Debug, Serialize)]
pub struct ChunkPage {
    pub matched: usize,
    pub items: Vec<ChunkRow>,
}
#[derive(Debug, Serialize)]
pub struct ChunkRow {
    pub ordinal: usize,
    pub guid: String,
    pub span: Span,
}

pub fn chunks(
    root: &Path,
    layer: &str,
    sb: &str,
    search: &str,
    offset: usize,
) -> Result<ChunkPage> {
    let toc = read_toc(root, layer, sb)?;
    let q = search.to_lowercase();
    let rows: Vec<_> = toc
        .chunks
        .into_iter()
        .enumerate()
        .filter(|(_, c)| c.guid.contains(&q))
        .collect();
    Ok(ChunkPage {
        matched: rows.len(),
        items: rows
            .into_iter()
            .skip(offset)
            .take(100)
            .map(|(ordinal, c)| ChunkRow {
                ordinal,
                guid: c.guid,
                span: c.span,
            })
            .collect(),
    })
}
pub fn export_chunk(
    root: &Path,
    layer: &str,
    sb: &str,
    ordinal: usize,
    decoded: bool,
    destination: &Path,
) -> Result<usize> {
    let toc = read_toc(root, layer, sb)?;
    let c = toc
        .chunks
        .get(ordinal)
        .ok_or_else(|| invalid("chunk not found"))?;
    export_span(root, &c.span, decoded, destination)
}

pub fn read_index(path: &Path) -> Result<Vec<u8>> {
    let mut f = File::open(path)?;
    if f.metadata()?.len() > MAX_INDEX {
        return Err(invalid("index exceeds 128 MiB limit"));
    }
    let mut b = Vec::new();
    f.by_ref().take(MAX_INDEX + 1).read_to_end(&mut b)?;
    if b.len() as u64 > MAX_INDEX {
        return Err(invalid("index grew beyond limit"));
    }
    Ok(b)
}
fn relative(s: &str) -> Result<()> {
    if s.is_empty()
        || s.contains('\\')
        || s.contains(':')
        || s.split('/').any(|p| p.is_empty() || p == "." || p == "..")
        || Path::new(s)
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(invalid("unsafe relative path"));
    }
    Ok(())
}
fn contained(root: &Path, path: &Path) -> Result<PathBuf> {
    let base = root.canonicalize()?;
    let p = path.canonicalize()?;
    if !p.starts_with(&base) {
        return Err(invalid("file resolves outside install"));
    }
    Ok(p)
}
fn layer_path(root: &Path, layer: &str) -> Result<PathBuf> {
    if !matches!(layer, "Data" | "Patch") {
        return Err(invalid("layer must be Data or Patch"));
    }
    contained(root, &root.join(layer))
}
fn layout(root: &Path, layer: &str) -> Result<Value> {
    let dir = layer_path(root, layer)?;
    db::parse(&read_index(&contained(root, &dir.join("layout.toc"))?)?)
}
pub fn inspect(root: &Path) -> Result<Install> {
    let root = root.canonicalize()?;
    let mut layers = Vec::new();
    let mut warnings = Vec::new();
    if !root.join("Data/layout.toc").is_file() {
        return Err(invalid(
            "Choose the extracted game folder containing Data/layout.toc",
        ));
    }
    for name in ["Data", "Patch"] {
        if name == "Patch" && !root.join(name).exists() {
            continue;
        }
        let v = layout(&root, name)?;
        let sbs = v["superBundles"]
            .as_array()
            .ok_or_else(|| invalid("layout has no superbundles"))?;
        let mut superbundles = Vec::new();
        for sb in sbs {
            let s = sb["name"]
                .as_str()
                .ok_or_else(|| invalid("superbundle missing name"))?;
            relative(s)?;
            if !s.starts_with("Gen4b/") {
                return Err(invalid(
                    "This reader supports PS4 Gen4b only; PS5 is not qualified",
                ));
            }
            if !root.join(name).join(format!("{s}.toc")).is_file() {
                warnings.push(format!("{name}/{s}.toc is missing"));
            }
            superbundles.push(s.into());
        }
        let init = root.join(name).join("initfs_Gen4b");
        let initfs = if init.is_file() {
            let value = db::parse(&read_index(&contained(&root, &init)?)?)?;
            if value.get("encrypted").is_some() {
                "encrypted (opaque)"
            } else {
                "DbObject"
            }
        } else {
            "missing"
        }
        .into();
        let cm = root.join(name).join("chunkmanifest");
        let chunkmanifest = cm.is_file();
        if chunkmanifest {
            db::parse(&read_index(&contained(&root, &cm)?)?)?;
        }
        layers.push(Layer {
            name: name.into(),
            head: v["head"].as_i64(),
            base: v["base"].as_i64(),
            superbundles,
            initfs,
            chunkmanifest,
        });
    }
    if layers.len() == 2 && layers[1].base != layers[0].head {
        warnings.push("Patch base does not match Data head; do not combine these installs".into());
    }
    warnings.push("Read-only catalogue: roster editing and deployment are not enabled. Data and Patch are shown separately.".into());
    Ok(Install {
        root: root.to_string_lossy().into(),
        layers,
        warnings,
    })
}
pub fn read_toc(root: &Path, layer: &str, sb: &str) -> Result<Toc> {
    relative(sb)?;
    let v = layout(root, layer)?;
    if !v["superBundles"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v["name"].as_str() == Some(sb)))
    {
        return Err(invalid("superbundle is not declared by layout"));
    }
    if !sb.starts_with("Gen4b/") {
        return Err(invalid("only PS4 Gen4b is supported"));
    }
    let dir = layer_path(root, layer)?;
    toc::parse(&read_index(&contained(
        root,
        &dir.join(format!("{sb}.toc")),
    )?)?)
}
pub fn bundles(
    root: &Path,
    layer: &str,
    sb: &str,
    search: &str,
    offset: usize,
    limit: usize,
) -> Result<BundlePage> {
    let toc = read_toc(root, layer, sb)?;
    let q = search.to_lowercase();
    let rows: Vec<_> = toc
        .bundles
        .iter()
        .filter(|b| b.name.to_lowercase().contains(&q))
        .collect();
    Ok(BundlePage {
        total: toc.bundles.len(),
        matched: rows.len(),
        toc_chunks: toc.chunks.len(),
        items: rows
            .into_iter()
            .skip(offset)
            .take(limit.min(200))
            .map(|b| BundleRow {
                ordinal: b.ordinal,
                name: b.name.clone(),
                spans: b.spans.len(),
            })
            .collect(),
    })
}
pub fn cas_path(root: &Path, id: u32) -> Result<PathBuf> {
    if id >> 24 != 0 || (id >> 16) & 255 > 1 {
        return Err(invalid("unsupported PS4 CAS identifier"));
    }
    let layer = if id >> 16 == 0 { "Data" } else { "Patch" };
    let rel = format!(
        "{layer}/Gen4b/superbundlelayout/nhl_installpackage_{:02}/cas_{:02}.cas",
        (id >> 8) & 255,
        id & 255
    );
    contained(root, &root.join(rel))
}
pub fn read_span(root: &Path, s: &Span) -> Result<Vec<u8>> {
    if s.size as usize > MAX_ASSET {
        return Err(invalid("span exceeds 256 MiB limit"));
    }
    let mut f = File::open(cas_path(root, s.cas)?)?;
    if s.offset as u64 + s.size as u64 > f.metadata()?.len() {
        return Err(invalid("CAS span beyond end of file"));
    }
    f.seek(SeekFrom::Start(s.offset as u64))?;
    let mut b = vec![0; s.size as usize];
    f.read_exact(&mut b)?;
    Ok(b)
}
pub fn assets(root: &Path, layer: &str, sb: &str, ordinal: usize) -> Result<BundleAssets> {
    let toc = read_toc(root, layer, sb)?;
    let b = toc
        .bundles
        .get(ordinal)
        .ok_or_else(|| invalid("bundle not found"))?;
    let assets = manifest::parse(&read_span(root, &b.spans[0])?)?;
    if b.spans.len() != assets.len() + 1 {
        return Err(invalid("asset count does not match spans"));
    }
    Ok(BundleAssets {
        name: b.name.clone(),
        assets,
        spans: b.spans[1..].to_vec(),
    })
}
/// Create a new output only, outside the source install. Never overwrite existing exports.
pub fn export(
    root: &Path,
    layer: &str,
    sb: &str,
    bundle: usize,
    asset: usize,
    decoded: bool,
    destination: &Path,
) -> Result<usize> {
    let a = assets(root, layer, sb, bundle)?;
    let span = a
        .spans
        .get(asset)
        .ok_or_else(|| invalid("asset not found"))?;
    export_span(root, span, decoded, destination)
}
fn export_span(root: &Path, span: &Span, decoded: bool, destination: &Path) -> Result<usize> {
    let bytes = read_span(root, span)?;
    let bytes = if decoded {
        crate::blocks::decode_stored(&bytes, MAX_ASSET)?
    } else {
        bytes
    };
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("output requires a parent folder"))?
        .canonicalize()?;
    if parent.starts_with(root.canonicalize()?) {
        return Err(invalid("exports must be outside the source install"));
    }
    let file = destination
        .file_name()
        .ok_or_else(|| invalid("missing output file name"))?;
    let mut out = tempfile::NamedTempFile::new_in(&parent)?;
    use std::io::Write;
    out.write_all(&bytes)?;
    out.as_file().sync_all()?;
    out.persist_noclobber(parent.join(file))
        .map_err(|e| e.error)?;
    Ok(bytes.len())
}
