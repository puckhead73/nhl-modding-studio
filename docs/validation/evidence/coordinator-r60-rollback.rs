use std::path::{Path, PathBuf};
#[derive(Debug)] enum ApiError { Workspace(String) }
#[derive(Default)]
struct WriteTxn {
    /// (path, prior bytes — `None` when the install created the file, backup written by this install).
    done: Vec<(PathBuf, Option<Vec<u8>>, Option<PathBuf>)>,
}

impl WriteTxn {
    fn write(&mut self, path: &Path, bytes: &[u8], backup: bool) -> Result<(), ApiError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ApiError::Workspace(format!("creating {}: {e}", parent.display())))?;
        }
        if path.exists() && !path.is_file() {
            return Err(ApiError::Workspace(format!("{} exists and is not a file; nothing was replaced", path.display())));
        }
        let prior = if path.is_file() {
            Some(std::fs::read(path).map_err(|e| ApiError::Workspace(format!("reading {}: {e}", path.display())))?)
        } else {
            None
        };
        let mut new_backup = None;
        if backup && prior.is_some() {
            let bak = bak_path(path);
            if !bak.exists() {
                backup_once(path)?;
                new_backup = Some(bak);
            }
        }
        // Record before touching the file, so a failed write is also restored.
        self.done.push((path.to_path_buf(), prior, new_backup));
        let tmp = tmp_path(path);
        std::fs::write(&tmp, bytes).map_err(|e| ApiError::Workspace(format!("writing {}: {e}", tmp.display())))?;
        std::fs::rename(&tmp, path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            ApiError::Workspace(format!("replacing {}: {e}", path.display()))
        })
    }

    /// Undo in reverse order; `Err` lists what could not be restored.
    fn rollback(self) -> Result<(), String> {
        let mut left = Vec::new();
        for (path, prior, backup) in self.done.into_iter().rev() {
            let r = match prior {
                Some(b) => std::fs::write(&path, b),
                None if path.exists() => std::fs::remove_file(&path),
                None => Ok(()),
            };
            if let Err(e) = r {
                left.push(format!("{} ({e})", path.display()));
            }
            if let Some(bak) = backup {
                let _ = std::fs::remove_file(bak);
            }
        }
        if left.is_empty() {
            Ok(())
        } else {
            Err(left.join("; "))
        }
    }
}

fn bak_path(path: &Path) -> PathBuf {
    let mut b = path.as_os_str().to_owned();
    b.push(".bak");
    PathBuf::from(b)
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut b = path.as_os_str().to_owned();
    b.push(".nhlms-tmp");
    PathBuf::from(b)
}

fn backup_once(path: &Path) -> Result<(), ApiError> {
    if !path.is_file() {
        return Ok(());
    }
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    let bak = PathBuf::from(bak);
    if !bak.exists() {
        std::fs::copy(path, &bak)
            .map_err(|e| ApiError::Workspace(format!("backing up {}: {e}", path.display())))?;
    }
    Ok(())
}


fn main() {
 let root=std::env::args().nth(1).unwrap(); let root=PathBuf::from(root);
 std::fs::create_dir_all(&root).unwrap(); let p=root.join("asset.rx2");
 std::fs::write(&p,b"original").unwrap();
 let mut txn=WriteTxn::default(); txn.write(&p,b"replacement",true).unwrap();
 assert_eq!(std::fs::read(bak_path(&p)).unwrap(),b"original");
 std::fs::remove_file(&p).unwrap(); std::fs::create_dir(&p).unwrap();
 let r=txn.rollback();
 println!("rollback_failed={} backup_survives={}",r.is_err(),bak_path(&p).exists());
 assert!(r.is_err()); assert!(!bak_path(&p).exists());
}
