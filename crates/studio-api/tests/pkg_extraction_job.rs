//! Opt-in real PS4 package job test. Fresh output only; originals are read-only.
use std::{path::PathBuf,time::{Duration,Instant}};
use studio_api::Registry;
#[test]
#[ignore="extracts a real package into NHLMS_PKG_QA_OUTPUT with the separately selected helper"]
fn real_package_job_validates_output_and_keeps_source() {
    let image=std::env::var("NHLMS_PKG_QA_IMAGE").unwrap();
    let out=PathBuf::from(std::env::var("NHLMS_PKG_QA_OUTPUT").unwrap());
    let extractor=std::env::var("NHLMS_PKG_QA_EXTRACTOR").unwrap();
    let game=std::env::var("NHLMS_PKG_QA_GAME").unwrap();
    assert!(!out.exists());
    let before=std::fs::metadata(&image).unwrap();
    let reg=Registry::default();
    let mut job=reg.pkg_extract_start(&image,&out.to_string_lossy(),&game,&extractor).unwrap();
    let until=Instant::now()+Duration::from_secs(1800);
    let mut log=Instant::now();
    while matches!(job.state.as_str(),"running"|"cancelling") {
        assert!(Instant::now()<until,"Package job timeout");
        std::thread::sleep(Duration::from_millis(250));
        job=reg.extract_status(job.job).unwrap();
        if log.elapsed()>Duration::from_secs(30) {eprintln!("{}",job.current);log=Instant::now();}
    }
    assert_eq!(job.state,"done","{:?}",job.error);
    assert!(job.files_written>20 && job.bytes_written>1_000_000);
    let after=std::fs::metadata(&image).unwrap();
    assert_eq!(before.len(),after.len());assert_eq!(before.modified().unwrap(),after.modified().unwrap());
    assert!(reg.pkg_extract_start(&image,&out.to_string_lossy(),&game,&extractor).is_err());
    std::fs::write(out.join("nhlms-package-job-receipt.json"),serde_json::to_vec_pretty(&job).unwrap()).unwrap();
    eprintln!("validated package job: {} files / {} bytes",job.files_written,job.bytes_written);
}
