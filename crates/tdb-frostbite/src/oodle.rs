//! Explicit, separately installed decoder provider. No DLL is redistributed.
use crate::{invalid, Result};
use std::{ffi::c_void, path::Path};
type Decompress = unsafe extern "C" fn(
    *const c_void,
    isize,
    *mut c_void,
    isize,
    i32,
    i32,
    i32,
    *mut c_void,
    isize,
    *mut c_void,
    *mut c_void,
    *mut c_void,
    isize,
    i32,
) -> isize;
pub struct Oodle {
    _library: libloading::Library,
    decompress: Decompress,
}
impl Oodle {
    /// The caller must obtain the path through explicit user selection.
    pub fn open(path: &Path) -> Result<Self> {
        let path = std::fs::canonicalize(path)?;
        // SAFETY: this is a user-selected native decoder, held until all calls finish.
        let library =
            unsafe { libloading::Library::new(&path) }.map_err(|e| invalid(e.to_string()))?;
        let decompress = *unsafe { library.get::<Decompress>(b"OodleLZ_Decompress\0") }
            .map_err(|e| invalid(e.to_string()))?;
        Ok(Self {
            _library: library,
            decompress,
        })
    }
    pub fn decode(&self, codec: u8, data: &[u8], size: usize) -> Result<Vec<u8>> {
        if codec != 0x11 || size > 0x100000 || data.len() > 0x100000 {
            return Err(invalid("unsupported Oodle block or size"));
        }
        let mut out = vec![0; size];
        // SAFETY: bounded input/output buffers stay alive; fuzz-safe decoder mode.
        let count = unsafe {
            (self.decompress)(
                data.as_ptr().cast(),
                data.len() as isize,
                out.as_mut_ptr().cast(),
                size as isize,
                1,
                0,
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                3,
            )
        };
        if count != size as isize {
            return Err(invalid(format!("Oodle decoded {count}, expected {size}")));
        }
        Ok(out)
    }
}
