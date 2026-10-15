use studio_api::commands::resolve_corpus_path;

#[test]
fn bundled_corpus_resolution_is_relocatable_and_missing_data_is_actionable() {
    let root = std::env::temp_dir().join(format!("nhlms-portable-corpus-{}", std::process::id()));
    std::fs::create_dir_all(root.join("corpus")).unwrap();
    assert!(resolve_corpus_path(&root, "").unwrap_err().contains("Extract the whole"));
    std::fs::write(root.join("corpus/historical.db"), b"path resolution fixture").unwrap();
    assert_eq!(resolve_corpus_path(&root, "").unwrap(), root.join("corpus/historical.db"));
    assert_eq!(resolve_corpus_path(&root, "custom.db").unwrap(), std::path::PathBuf::from("custom.db"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "set NHLMS_RELEASE_ROOT to an owned extracted playtester bundle; opens its copied corpus"]
fn extracted_historical_data_opens_and_serves_filters() {
    let root = std::path::PathBuf::from(std::env::var_os("NHLMS_RELEASE_ROOT").expect("bundle root"));
    let path = resolve_corpus_path(&root, "").unwrap();
    let registry = studio_api::Registry::default();
    assert!(registry.open_corpus(&path.to_string_lossy()).unwrap() > 190000);
    registry.historical_filters().unwrap();
    assert!(registry.corpus_open());
}
