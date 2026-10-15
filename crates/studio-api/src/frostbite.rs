//! Standalone read-only asset onboarding; does not manufacture a TDB workspace.
use std::path::Path;
use tdb_frostbite::install;

#[tauri::command]
pub async fn frostbite_chunks(
    root: String,
    layer: String,
    superbundle: String,
    search: String,
    offset: usize,
) -> Result<install::ChunkPage, String> {
    run(move || install::chunks(Path::new(&root), &layer, &superbundle, &search, offset)).await
}
#[tauri::command]
pub async fn frostbite_export_chunk(
    root: String,
    layer: String,
    superbundle: String,
    chunk: usize,
    decoded: bool,
    destination: String,
) -> Result<usize, String> {
    run(move || {
        install::export_chunk(
            Path::new(&root),
            &layer,
            &superbundle,
            chunk,
            decoded,
            Path::new(&destination),
        )
    })
    .await
}

async fn run<T: Send + 'static>(
    f: impl FnOnce() -> std::io::Result<T> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn frostbite_inspect(root: String) -> Result<install::Install, String> {
    run(move || install::inspect(Path::new(&root))).await
}
#[tauri::command]
pub async fn frostbite_bundles(
    root: String,
    layer: String,
    superbundle: String,
    search: String,
    offset: usize,
) -> Result<install::BundlePage, String> {
    run(move || install::bundles(Path::new(&root), &layer, &superbundle, &search, offset, 100))
        .await
}
#[tauri::command]
pub async fn frostbite_assets(
    root: String,
    layer: String,
    superbundle: String,
    bundle: usize,
) -> Result<install::BundleAssets, String> {
    run(move || install::assets(Path::new(&root), &layer, &superbundle, bundle)).await
}
#[tauri::command]
pub async fn frostbite_export(
    root: String,
    layer: String,
    superbundle: String,
    bundle: usize,
    asset: usize,
    decoded: bool,
    destination: String,
) -> Result<usize, String> {
    run(move || {
        install::export(
            Path::new(&root),
            &layer,
            &superbundle,
            bundle,
            asset,
            decoded,
            Path::new(&destination),
        )
    })
    .await
}
