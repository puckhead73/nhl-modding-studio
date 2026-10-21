//! Thin asynchronous bridge to the portable head library.
use base64::Engine;
use serde_json::{json, Value};
use studio_domain::head_library as library;

async fn run(
    f: impl FnOnce() -> Result<Value, Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || f().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn head_library_status() -> Result<Value, String> {
    run(library::status).await
}
#[tauri::command]
pub async fn head_library_search(
    query: String,
    class: String,
    offset: u32,
) -> Result<Value, String> {
    run(move || library::search(&query, &class, offset)).await
}
#[tauri::command]
pub async fn head_library_detail(id: String) -> Result<Value, String> {
    run(move || library::detail(&id)).await
}
#[tauri::command]
pub async fn head_library_file(path: String) -> Result<Value, String> {
    run(move || {
        let bytes=library::file(&path)?;
        if path.ends_with(".dds") {
            let (width,height,rgba)=library::texture_rgba(&bytes)?;
            Ok(json!({"width":width,"height":height,"rgba_base64":base64::engine::general_purpose::STANDARD.encode(rgba)}))
        } else {Ok(json!({"base64":base64::engine::general_purpose::STANDARD.encode(bytes)}))}
    }).await
}
#[tauri::command]
pub async fn head_library_export(
    id: String,
    appearance: i64,
    destination: String,
    companions: Option<Vec<String>>,
) -> Result<Value, String> {
    run(move || {
        library::export_composition(
            &id,
            appearance,
            &companions.unwrap_or_default(),
            std::path::Path::new(&destination),
        )
    })
    .await
}
