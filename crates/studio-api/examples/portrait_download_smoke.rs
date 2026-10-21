//! Live smoke of the in-app portrait-database downloader against the published Google Drive file:
//! start the download into `WORK_DIR` (no corpus open, so `WORK_DIR` stands in for the bundle
//! root), let it run for `SECONDS`, cancel, and confirm a resumable `.part` file grew. Then start
//! again and confirm the second request resumes (bytes continue from the partial size) before
//! cancelling once more. The partial file is deleted at the end unless `KEEP=1`. Nothing else is
//! written; the full 31.7 GB download is not attempted here.
//!
//! Usage: `portrait_download_smoke WORK_DIR [SECONDS]`
use std::path::PathBuf;
use std::time::{Duration, Instant};

use studio_api::Registry;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err("usage: portrait_download_smoke WORK_DIR [SECONDS]".into());
    }
    let work = PathBuf::from(&args[1]);
    let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    std::fs::create_dir_all(&work)?;
    let reg = Registry::default();

    let plan = reg.portrait_install_plan(&work)?;
    println!("PLAN download_dir={} install_root={} expected={} free={:?}", plan.download_dir, plan.install_root, plan.expected_bytes, plan.free_bytes);

    let run = |label: &str| -> Result<u64, Box<dyn std::error::Error>> {
        let started = reg.portrait_download_start(&work)?;
        println!("{label}: START phase={} message={}", started.phase, started.message);
        let t0 = Instant::now();
        let mut last = 0u64;
        while t0.elapsed() < Duration::from_secs(secs) {
            std::thread::sleep(Duration::from_millis(500));
            let s = reg.portrait_install_status()?;
            if s.phase != "downloading" {
                println!("{label}: phase changed to {} — {}", s.phase, s.message);
                if s.phase == "failed" {
                    return Err(format!("download failed: {}", s.error.unwrap_or_default()).into());
                }
                break;
            }
            last = s.bytes_done;
            println!("{label}: {} / {:?} bytes — {}", s.bytes_done, s.bytes_total, s.message);
        }
        let c = reg.portrait_install_cancel()?;
        println!("{label}: CANCEL requested (phase {})", c.phase);
        for _ in 0..60 {
            std::thread::sleep(Duration::from_millis(250));
            let s = reg.portrait_install_status()?;
            if s.phase != "downloading" {
                println!("{label}: final phase {} — {}", s.phase, s.message);
                break;
            }
        }
        Ok(last)
    };

    let first = run("first")?;
    let part = PathBuf::from(&plan.download_dir).join(format!("{}.part", plan.archive_name));
    let on_disk = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    println!("PART after first run: {on_disk} bytes on disk (last reported {first})");
    if on_disk == 0 {
        return Err("nothing was downloaded".into());
    }
    let second_plan = reg.portrait_install_plan(&work)?;
    if second_plan.partial_bytes != on_disk {
        return Err("the plan does not report the partial file".into());
    }
    let second = run("second")?;
    let after = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    println!("PART after second run: {after} bytes (reported {second})");
    if after < on_disk {
        return Err("the second run did not resume the partial file".into());
    }
    if std::env::var("KEEP").ok().as_deref() != Some("1") {
        let _ = std::fs::remove_file(&part);
        println!("CLEANED {}", part.display());
    }
    println!("OK resumable download verified");
    Ok(())
}
