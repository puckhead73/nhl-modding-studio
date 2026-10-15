//! Opt-in real-bank extraction, only into an owned test directory. No install writes.
use std::{path::PathBuf, sync::atomic::AtomicBool};
use studio_domain::{extract_service, game_manifest::hash, speech_legacy, Platform};

#[test]
#[ignore = "requires NHL07_EXTRACTED_INSTALL pointing at an already extracted installation; read-only"]
fn extracted_install_preserves_vanilla_boot_files() {
    let root = PathBuf::from(std::env::var("NHL07_EXTRACTED_INSTALL").unwrap());
    let pre = extract_service::preflight(&root, Platform::Xbox);
    let boot = pre.archives.iter().find(|a| a.path.file_name().unwrap() == "boot.big").unwrap();
    assert_eq!(boot.members.iter().filter(|m| m.state == "vanilla").count(), 17);
    assert_eq!(boot.state, "complete");
    assert_eq!(pre.archives.len(), 11);
    assert_eq!(pre.members_to_write, 0);
    assert_eq!(pre.state, "complete", "{}", pre.reason);
    println!("All 11 containers complete; 17 original loose boot files preserved. No installation writes.");
}

#[test]
#[ignore = "requires NHL07_SPEECH_FIXTURE pointing at the original audio/speech folder"]
fn real_banks_extract_and_verify_every_sample() -> Result<(), Box<dyn std::error::Error>> {
    let source = PathBuf::from(std::env::var("NHL07_SPEECH_FIXTURE")?);
    let install = source
        .parent()
        .and_then(|p| p.parent())
        .ok_or("expected audio/speech fixture folder")?;
    let install_preflight = extract_service::preflight(install, Platform::Xbox);
    assert_eq!(install_preflight.archives.len(), 11);
    assert!(
        install_preflight
            .archives
            .iter()
            .all(|a| a.parse_error.is_none()),
        "all 11 original NHL07 containers must parse"
    );
    println!("Read-only installation scan: all 11 containers parsed");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/nhl07-speech-qa")
        .join(format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
    std::fs::create_dir_all(&root)?;
    let mut identities = vec![];
    for stem in ["pbp_eng", "rink"] {
        for ext in ["big", "idx"] {
            let name = format!("{stem}.{ext}");
            let path = source.join(&name);
            identities.push((path.clone(), hash(std::fs::File::open(&path)?)?));
            std::fs::copy(path, root.join(name))?;
        }
    }
    let pre = extract_service::preflight(&root, Platform::Xbox);
    assert_eq!(pre.archives.len(), 2);
    assert!(pre.archives.iter().all(|a| a.parse_error.is_none()));
    assert_eq!(pre.members_total, 39318);
    let result =
        extract_service::extract(&root, Platform::Xbox, &AtomicBool::new(false), &mut |_| {})?;
    assert_eq!(
        (
            result.files_written,
            result.archives_failed,
            result.files_unsupported,
            result.files_conflicting
        ),
        (39318, 0, 0, 0)
    );
    let mut total = 0;
    for (stem, count) in [("pbp_eng", 12064), ("rink", 7595)] {
        let idx = std::fs::read(source.join(format!("{stem}.idx")))?;
        let mut bank = std::fs::File::open(source.join(format!("{stem}.big")))?;
        let samples = speech_legacy::parse(&idx, bank.metadata()?.len())?;
        assert_eq!(samples.len(), count);
        for sample in &samples {
            let prefix = sample.name(stem);
            assert_eq!(
                std::fs::read(root.join(format!("{prefix}.snr")))?,
                sample.snr
            );
            assert_eq!(
                std::fs::read(root.join(format!("{prefix}.sns")))?,
                speech_legacy::read_sample(&mut bank, sample)?
            );
            total += 1;
        }
    }
    assert_eq!(
        extract_service::preflight(&root, Platform::Xbox).state,
        "complete"
    );
    let repeat =
        extract_service::extract(&root, Platform::Xbox, &AtomicBool::new(false), &mut |_| {})?;
    assert_eq!((repeat.files_written, repeat.files_retained), (0, 39318));
    for (path, before) in identities {
        assert_eq!(hash(std::fs::File::open(path)?)?, before);
    }
    println!(
        "Verified {total} SNR/SNS pairs; outputs retained at {}",
        root.display()
    );
    Ok(())
}
