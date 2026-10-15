//! NHL08 PS3 speech extraction: original banks remain read-only.
use std::{fs, path::PathBuf, sync::atomic::{AtomicBool, Ordering}};
use studio_domain::{extract_service, game_manifest::hash, speech_legacy, Platform};

#[test]
#[ignore = "requires NHL08_PS3_INSTALL; exports only into an owned target directory"]
fn ps3_banks_extract_resume_and_verify_every_stream() {
    let install = PathBuf::from(std::env::var("NHL08_PS3_INSTALL").unwrap());
    let source = install.join("audio/speech");
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/nhl08-ps3-speech-qa")
        .join(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().to_string());
    fs::create_dir_all(&output).unwrap();
    let installation = extract_service::preflight(&install, Platform::Ps3);
    assert_eq!(installation.archives.len(), 8);
    for archive in &installation.archives { assert!(archive.parse_error.is_none(), "{}: {:?}", archive.path.display(), archive.parse_error); }
    let identities: Vec<_> = ["pbp_eng.big", "pbp_eng.idx", "rink.big", "rink.idx"].into_iter().map(|name| {
        let path = source.join(name);
        let digest = hash(fs::File::open(&path).unwrap()).unwrap();
        (path, digest)
    }).collect();
    let before = extract_service::preflight_to(&source, &output, Platform::Ps3);
    assert_eq!(before.members_total, 76426);
    // Interrupt after real output, then resume using the same destination.
    let cancel = AtomicBool::new(false);
    let partial = extract_service::extract_to(&source, &output, Platform::Ps3, &cancel, &mut |p| {
        if p.files_written >= 8 { cancel.store(true, Ordering::Relaxed); }
    }).unwrap();
    assert!(partial.cancelled);
    assert!(partial.files_written >= 8 && partial.files_written < 76426);
    let result = extract_service::extract_to(&source, &output, Platform::Ps3, &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.files_written + result.files_retained, 76426);
    assert_eq!((result.archives_failed, result.files_unsupported, result.files_conflicting), (0, 0, 0));
    for (stem, count) in [("pbp_eng", 29750), ("rink", 8463)] {
        let raw = fs::read(source.join(format!("{stem}.idx"))).unwrap();
        let idx = tdb_archive_codec::refpack_decompress(&raw).unwrap();
        let mut bank = fs::File::open(source.join(format!("{stem}.big"))).unwrap();
        let samples = speech_legacy::parse(&idx, bank.metadata().unwrap().len()).unwrap();
        assert_eq!(samples.len(), count);
        for sample in &samples {
            assert_eq!(tdb_snd::SnrHeader::parse(&sample.snr).unwrap().codec, 5);
            let name = sample.name(stem);
            assert_eq!(fs::read(output.join(format!("{name}.snr"))).unwrap(), sample.snr);
            assert_eq!(fs::read(output.join(format!("{name}.sns"))).unwrap(), speech_legacy::read_sample(&mut bank, sample).unwrap());
        }
        println!("{stem}: {count} encoded SNR/SNS pairs verified byte-for-byte");
    }
    assert_eq!(extract_service::preflight_to(&source, &output, Platform::Ps3).state, "complete");
    let repeat = extract_service::extract_to(&source, &output, Platform::Ps3, &AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!((repeat.files_written, repeat.files_retained), (0, 76426));
    for (path, before) in identities { assert_eq!(hash(fs::File::open(path).unwrap()).unwrap(), before); }
    println!("Extraction complete; cancellation/resume and repeat verified. Outputs: {}", output.display());
}
