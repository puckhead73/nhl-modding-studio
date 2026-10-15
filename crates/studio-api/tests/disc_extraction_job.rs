//! The full-disc copier through the cancellable job API; all data is synthetic.
use std::{path::PathBuf, sync::atomic::{AtomicUsize, Ordering}, time::{Duration, Instant}};
use studio_api::Registry;
use tdb_xiso::{Layout, synth::{image, SynthFile}};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("nhlms-discjob-{}-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&p).unwrap(); Self(p)
    }
}
impl Drop for Temp { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
fn s(p: &std::path::Path) -> String { p.to_string_lossy().into_owned() }
fn finish(reg: &Registry, mut job: studio_api::dto::ExtractJobDto) -> studio_api::dto::ExtractJobDto {
    let deadline = Instant::now() + Duration::from_secs(15);
    while matches!(job.state.as_str(), "running" | "cancelling") {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
        job = reg.extract_status(job.job).unwrap();
    }
    job
}

#[test]
fn iso9660_job_is_accepted_and_invalid_disc_leaves_no_output() {
    // Minimal ISO9660 fixture, independent of the production parser.
    fn record(name: &[u8], sector: u32, size: u32, directory: bool) -> Vec<u8> {
        let mut r = vec![0; 33 + name.len() + usize::from(name.len() % 2 == 0)];
        r[0] = r.len() as u8;
        r[2..6].copy_from_slice(&sector.to_le_bytes());
        r[6..10].copy_from_slice(&sector.to_be_bytes());
        r[10..14].copy_from_slice(&size.to_le_bytes());
        r[14..18].copy_from_slice(&size.to_be_bytes());
        r[25] = if directory { 2 } else { 0 };
        r[28..30].copy_from_slice(&1u16.to_le_bytes());
        r[30..32].copy_from_slice(&1u16.to_be_bytes());
        r[32] = name.len() as u8;
        r[33..33 + name.len()].copy_from_slice(name);
        r
    }
    let tmp = Temp::new();
    let mut bytes = vec![0; 36 * 2048];
    let pvd = &mut bytes[16 * 2048..17 * 2048];
    pvd[0] = 1; pvd[1..6].copy_from_slice(b"CD001"); pvd[6] = 1;
    pvd[120..124].copy_from_slice(&[1,0,0,1]);
    pvd[128..132].copy_from_slice(&[0,8,8,0]);
    let root = record(&[0],20,2048,true);
    pvd[156..156+root.len()].copy_from_slice(&root);
    let file = record(b"TEST.DB;1",21,5,false);
    bytes[20*2048..20*2048+file.len()].copy_from_slice(&file);
    bytes[21*2048..21*2048+5].copy_from_slice(b"hello");
    let image = tmp.0.join("ps3.iso");
    std::fs::write(&image,&bytes).unwrap();
    for valid in [true,false] {
        if !valid { std::fs::write(&image,b"not a disc").unwrap(); }
        let out = tmp.0.join(if valid { "copied" } else { "refused" });
        let reg = Registry::default();
        let mut job = reg.disc_extract_start(&s(&image),&s(&out)).unwrap();
        let deadline = Instant::now()+Duration::from_secs(10);
        while matches!(job.state.as_str(),"running"|"cancelling") {
            assert!(Instant::now()<deadline);
            std::thread::sleep(Duration::from_millis(10));
            job = reg.extract_status(job.job).unwrap();
        }
        assert_eq!(job.state,if valid { "done" } else { "failed" },"{:?}",job.error);
        if valid {
            assert_eq!(std::fs::read(out.join("TEST.DB")).unwrap(),b"hello");
            assert_eq!(std::fs::read(&image).unwrap(),bytes);
        } else { assert!(!out.exists()); }
    }
}

#[test]
fn disc_job_reports_complete_copy_and_refuses_existing_output() {
    let tmp = Temp::new();
    let source = tmp.0.join("disc.iso");
    let bytes = image(Layout::Xsf,&[SynthFile {path:"db/example.db",bytes:b"test database"},SynthFile {path:"default.xex",bytes:b"boot"}],None);
    std::fs::write(&source,&bytes).unwrap();
    let out = tmp.0.join("new game folder");
    let reg = Registry::default();
    let mut job = reg.disc_extract_start(&s(&source),&s(&out)).unwrap();
    let deadline = Instant::now()+Duration::from_secs(10);
    while matches!(job.state.as_str(),"running"|"cancelling") {
        assert!(Instant::now()<deadline,"job did not finish");
        std::thread::sleep(Duration::from_millis(10));
        job = reg.extract_status(job.job).unwrap();
    }
    assert_eq!(job.state,"done","{:?}",job.error);
    assert_eq!(job.files_written,2);
    assert_eq!(job.bytes_written,17);
    assert_eq!(std::fs::read(out.join("db/example.db")).unwrap(),b"test database");
    assert_eq!(std::fs::read(&source).unwrap(),bytes);
    let refused = finish(&reg, reg.disc_extract_start(&s(&source), &s(&out)).unwrap());
    assert_eq!(refused.state, "failed");
    assert!(refused.error.unwrap().contains("contains files"));
    assert!(reg.extract_running_for(&s(&out)).is_none());
}

#[test]
fn empty_destinations_work_and_invalid_sources_are_not_reported_as_collisions() {
    let tmp = Temp::new();
    let source = tmp.0.join("disc.iso");
    let out = tmp.0.join("selected folder");
    std::fs::create_dir(&out).unwrap();
    let reg = Registry::default();
    std::fs::write(&source, b"not an ISO").unwrap();
    std::fs::write(out.join("KEEP-ME.txt"), b"keep").unwrap();
    let failed = finish(&reg, reg.disc_extract_start(&s(&source), &s(&out)).unwrap());
    assert_eq!(failed.state, "failed");
    let error = failed.error.unwrap();
    assert!(error.contains("ISO"), "{error}");
    assert!(!error.contains("contains files"), "{error}");
    assert_eq!(std::fs::read(out.join("KEEP-ME.txt")).unwrap(), b"keep");
    std::fs::remove_file(out.join("KEEP-ME.txt")).unwrap();
    std::fs::write(&source, image(Layout::Xsf, &[SynthFile { path: "default.xex", bytes: b"boot" }], None)).unwrap();
    let done = finish(&reg, reg.disc_extract_start(&s(&source), &s(&out)).unwrap());
    assert_eq!(done.state, "done", "{:?}", done.error);
    assert_eq!(std::fs::read(out.join("default.xex")).unwrap(), b"boot");
    let missing_parent = tmp.0.join("missing/game");
    let failed = finish(&reg, reg.disc_extract_start(&s(&source), &s(&missing_parent)).unwrap());
    assert!(failed.error.unwrap().contains("parent folder is unavailable"));
}
