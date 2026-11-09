//! Read-only, layer-explicit Frostbite head catalog extraction.
use crate::{blocks, install, invalid, manifest, mesh, oodle::Oodle, texture, toc, Result};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    path::Path,
};

#[derive(Default, Debug, Serialize)]
pub struct Summary {
    pub meshes_written: usize,
    pub textures_written: usize,
    pub failures: Vec<String>,
    pub warnings: Vec<String>,
    pub catalog_file: String,
}

fn head_resource(name: &str) -> bool {
    [
        "content/character/skater/shared/head/",
        "content/character/skater/referee/head/",
        "content/character/bench/shared/head/",
    ]
    .iter()
    .any(|p| name.starts_with(p))
}
fn texture_resource(name: &str) -> bool {
    head_resource(name) || name.starts_with("content/character/common/textures/")
}
fn file_base(asset: &manifest::Asset) -> String {
    let leaf = asset.name.rsplit('/').next().unwrap_or("asset");
    let name: String = leaf
        .chars()
        .take(80)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{name}_{}", asset.sha1)
}
fn output(path: &Path, data: &[u8], validate_only: bool) -> Result<()> {
    if validate_only {
        return Ok(());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(data)
}
fn decoded(root: &Path, span: &toc::Span, decoder: &Oodle) -> Result<Vec<u8>> {
    blocks::decode_with(
        &install::read_span(root, span)?,
        install::MAX_ASSET,
        |codec, bytes, size| decoder.decode(codec, bytes, size),
    )
}
fn reverse_guid(guid: &str) -> String {
    guid.as_bytes()
        .chunks_exact(2)
        .rev()
        .map(|p| std::str::from_utf8(p).unwrap())
        .collect()
}

/// `validate_only` runs the same serializers without writing game-derived assets.
/// Each selected layer stays separate; no guessed base/patch resource precedence.
pub fn export(
    root: &Path,
    layer: &str,
    game: &str,
    destination: &Path,
    decoder_path: &Path,
    validate_only: bool,
) -> Result<Summary> {
    if !matches!(game, "nhl22" | "nhl23" | "nhl24") {
        return Err(invalid("unsupported Frostbite game"));
    }
    let root = std::fs::canonicalize(root)?;
    let installation = install::inspect(&root)?;
    let source = installation
        .layers
        .iter()
        .find(|l| l.name == layer)
        .ok_or_else(|| invalid("layer not found"))?;
    if layer == "Patch" {
        let base = installation
            .layers
            .iter()
            .find(|l| l.name == "Data")
            .ok_or_else(|| invalid("Patch export requires its matching Data installation"))?;
        if source.base.is_none() || source.base != base.head {
            return Err(invalid("Patch base does not match Data head"));
        }
    }
    let destination_parent = std::fs::canonicalize(
        destination
            .parent()
            .ok_or_else(|| invalid("missing output parent"))?,
    )?;
    if destination_parent.starts_with(&root) {
        return Err(invalid("output must be outside the game"));
    }
    if destination.exists() {
        return Err(invalid("choose a new, empty catalog folder"));
    }
    std::fs::create_dir(destination)?;
    if !validate_only {
        std::fs::create_dir(destination.join("heads"))?;
        std::fs::create_dir(destination.join("textures"))?;
    }
    let decoder = Oodle::open(decoder_path)?;
    let mut summary = Summary::default();
    let mut tocs = Vec::new();
    let mut global_chunks: HashMap<String, Vec<toc::Span>> = HashMap::new();
    let mut base_chunks: HashMap<String, Vec<toc::Span>> = HashMap::new();
    if layer == "Patch" {
        let base = installation
            .layers
            .iter()
            .find(|l| l.name == "Data")
            .unwrap();
        for sb in &base.superbundles {
            for chunk in install::read_toc(&root, "Data", sb)?.chunks {
                let spans = base_chunks.entry(chunk.guid).or_default();
                if !spans.contains(&chunk.span) {
                    spans.push(chunk.span);
                }
            }
        }
    }
    for sb in &source.superbundles {
        if layer == "Patch" && !root.join(layer).join(format!("{sb}.toc")).is_file() {
            summary.warnings.push(format!(
                "{layer}/{sb}.toc is absent; no base-layer replacement was inferred."
            ));
            continue;
        }
        let parsed = install::read_toc(&root, layer, sb)?;
        for chunk in &parsed.chunks {
            let spans = global_chunks.entry(chunk.guid.clone()).or_default();
            if !spans.contains(&chunk.span) {
                spans.push(chunk.span.clone());
            }
        }
        tocs.push((sb.clone(), parsed));
    }
    let mut entries = Vec::<serde_json::Value>::new();
    let mut occurrences = Vec::<serde_json::Value>::new();
    let mut texture_occurrences = Vec::<serde_json::Value>::new();
    let mut textures = HashMap::<(String, String), String>::new();
    let mut meshes = HashMap::<(String, String), Vec<usize>>::new();
    let mut failed = HashSet::new();
    let chunk_provenance = std::cell::RefCell::new(Vec::<serde_json::Value>::new());
    for (sb, toc) in &tocs {
        let mut bundles: Vec<_> = toc
            .bundles
            .iter()
            .filter(|b| b.name.contains("/head/"))
            .collect();
        bundles.sort_by_key(|b| (b.spans[0].cas, b.spans[0].offset));
        for bundle in bundles {
            let assets = match manifest::parse(&install::read_span(&root, &bundle.spans[0])?) {
                Ok(a) if a.len() + 1 == bundle.spans.len() => a,
                _ => {
                    summary
                        .failures
                        .push(format!("{sb}/{}: invalid bundle manifest", bundle.name));
                    continue;
                }
            };
            let read_chunk = |guid: &str, full_texture: bool| -> Result<Vec<u8>> {
                let read = |span: &toc::Span, lookup: &str| {
                    chunk_provenance.borrow_mut().push(serde_json::json!({
                        "guid":guid,"resource_layer":layer,"superbundle":sb,"bundle":bundle.name,
                        "lookup":lookup,"span":span
                    }));
                    decoded(&root, span, &decoder)
                };
                let candidates: Vec<_> = assets
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| a.kind == "chunk" && a.name == guid)
                    .collect();
                if let Some((index, asset)) = candidates.first() {
                    if !full_texture || asset.logical_offset == Some(0) {
                        return read(&bundle.spans[index + 1], "bundle");
                    }
                }
                // TOC GUID bytes are stored in reverse order relative to RES/bundle GUIDs.
                let key = reverse_guid(guid);
                if let Some(chunk) = toc.chunks.iter().find(|c| c.guid == key) {
                    return read(&chunk.span, "selected-layer TOC");
                }
                match global_chunks.get(&key).map(Vec::as_slice) {
                    Some([span]) => read(span, "selected-layer TOC"),
                    Some(_) => Err(invalid("ambiguous streamed chunk across superbundles")),
                    None => match base_chunks.get(&key).map(Vec::as_slice) {
                        Some([span]) => read(span, "matching-base TOC"),
                        Some(_) => Err(invalid("ambiguous base streamed chunk")),
                        None => Err(invalid(format!("missing complete chunk {guid}"))),
                    },
                }
            };
            let mut maps = Vec::new();
            for (index, asset) in assets
                .iter()
                .enumerate()
                .filter(|(_, a)| a.resource_type == Some(0x6bde20ba) && texture_resource(&a.name))
            {
                texture_occurrences.push(serde_json::json!({
                    "resource":asset.name,"sha1":asset.sha1,"resource_metadata":asset.resource_meta,
                    "game":game,"layer":layer,"superbundle":sb,"bundle":bundle.name,
                    "span":bundle.spans[index+1],
                    "file":format!("textures/{}.dds",file_base(asset))
                }));
                let key = (asset.name.clone(), asset.sha1.clone());
                if let Some(file) = textures.get(&key) {
                    maps.push(file.clone());
                    continue;
                }
                if failed.contains(&key) {
                    continue;
                }
                let result = (|| {
                    let resource = decoded(&root, &bundle.spans[index + 1], &decoder)?;
                    let guid = texture::chunk_guid(&resource)?;
                    let pixels = read_chunk(&guid, true)?;
                    let dds = texture::dds(&resource, &pixels)?;
                    let file = format!("textures/{}.dds", file_base(asset));
                    output(&destination.join(&file), &dds, validate_only)?;
                    Ok::<_, std::io::Error>(file)
                })();
                match result {
                    Ok(file) => {
                        summary.textures_written += 1;
                        maps.push(file.clone());
                        textures.insert(key, file);
                    }
                    Err(e) => {
                        summary.failures.push(format!("{}: {e}", asset.name));
                        failed.insert(key);
                    }
                }
            }
            for (index, asset) in assets
                .iter()
                .enumerate()
                .filter(|(_, a)| a.resource_type == Some(0x49b156d4) && head_resource(&a.name))
            {
                let key = (asset.name.clone(), asset.sha1.clone());
                occurrences.push(serde_json::json!({"resource":asset.name,"sha1":asset.sha1,
                    "game":game,"layer":layer,"superbundle":sb,"bundle":bundle.name,
                    "span":bundle.spans[index+1],"textures":maps}));
                if let Some(indices) = meshes.get(&key) {
                    for &i in indices {
                        let existing = entries[i]["textures"].as_array_mut().unwrap();
                        for map in &maps {
                            if !existing.iter().any(|v| v.as_str() == Some(map)) {
                                existing.push(map.clone().into());
                            }
                        }
                    }
                    continue;
                }
                if failed.contains(&key) {
                    continue;
                }
                let result = (|| {
                    let data = decoded(&root, &bundle.spans[index + 1], &decoder)?;
                    let lods = mesh::parse(
                        &data,
                        asset.resource_meta.as_deref().unwrap_or(""),
                        |guid| read_chunk(guid, false),
                    )?;
                    let base = file_base(asset);
                    let origin = if asset.name.contains("/bench/") {
                        "coach"
                    } else if asset.name.contains("/referee/") {
                        "referee"
                    } else if asset.name.contains("/generic/") {
                        "generic"
                    } else {
                        "player"
                    };
                    let class = if asset.name.contains("/beard/") {
                        "facialhair"
                    } else if asset.name.contains("/hair/") || base.contains("hair_") {
                        "hair"
                    } else {
                        "head"
                    };
                    let mut written = Vec::new();
                    for lod in lods {
                        let file = format!("heads/{base}_lod{}_{}.glb", lod.level, game);
                        output(&destination.join(&file), &mesh::glb(&lod)?, validate_only)?;
                        written.push(serde_json::json!({"name":format!("{base}_lod{}_{}",lod.level,game),
                            "origin":origin,"class":class,"game":game,"mesh_file":file,"textures":maps,
                            "texture_sources":[],"source":asset.name,
                            "submeshes":lod.sections.iter().map(|s|s.name.clone()).collect::<Vec<_>>(),
                            "lod":lod.level,"chunk":lod.chunk,"resource_sha1":asset.sha1,"layer":layer}));
                    }
                    Ok::<_, std::io::Error>(written)
                })();
                match result {
                    Ok(written) => {
                        let indices = (entries.len()..entries.len() + written.len()).collect();
                        summary.meshes_written += written.len();
                        entries.extend(written);
                        meshes.insert(key, indices);
                    }
                    Err(e) => {
                        summary.failures.push(format!("{}: {e}", asset.name));
                        failed.insert(key);
                    }
                }
            }
        }
    }
    // Metadata is written even in qualification mode, never source pixel/mesh data.
    let catalog = serde_json::json!({"version":1,"game":game,
        "head_count":entries.iter().filter(|e|e["class"]=="head").count(),
        "hair_count":entries.iter().filter(|e|e["class"]=="hair").count(),
        "facial_hair_count":entries.iter().filter(|e|e["class"]=="facialhair").count(),
        "entries":entries,"layer":layer,"warnings":summary.warnings,
        "geometry_only":true,"texture_mips":"base","validate_only":validate_only});
    std::fs::write(
        destination.join("catalog.json"),
        serde_json::to_vec_pretty(&catalog)?,
    )?;
    std::fs::write(
        destination.join("provenance.json"),
        serde_json::to_vec_pretty(&occurrences)?,
    )?;
    std::fs::write(
        destination.join("texture-provenance.json"),
        serde_json::to_vec_pretty(&texture_occurrences)?,
    )?;
    std::fs::write(
        destination.join("chunk-provenance.json"),
        serde_json::to_vec_pretty(&chunk_provenance.into_inner())?,
    )?;
    summary.catalog_file = destination.join("catalog.json").to_string_lossy().into();
    std::fs::write(
        destination.join("summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    Ok(summary)
}
