//! `tdb-speex` — diagnostic Speex **ultra-wideband** (32 kHz) decoding for EA-Speex previews
//! (codec `0x09`, research R83; the receipt-27 review's decoder decision, 26 Sep 2026).
//!
//! The decoder is upstream **libspeex 1.2.1** (Xiph.Org, BSD-3-Clause), vendored unmodified from
//! the release tarball `speex-1.2.1.tar.gz` (SHA-256 `4b44d4f2b38a370a2d98a78329fefc56a0cf93d1c1be70029217baae6628feea`,
//! matching Xiph's published `SHA256SUMS`), behind a small C shim (`shim/`) compiled with `cc`.
//! The shim owns every libspeex struct and control call; Rust sees an opaque handle. There is no
//! bindgen and no libclang requirement. See `README.md` for the file list, hashes and build recipe.
//!
//! Guarantees of this wrapper: the handle is freed exactly once ([`Drop`]); every buffer length is
//! checked before the call; each libspeex result is mapped to a named [`SpeexError`]; libspeex's
//! internal assertion path (upstream `exit(1)`) is turned into [`SpeexError::Fatal`] by the shim and
//! the decoder then refuses further use ([`SpeexError::Poisoned`]). Decoding only — no encoder is
//! exposed and nothing is written.

use std::ffi::CStr;
use std::fmt;
use std::ptr::NonNull;

mod ffi {
    use std::os::raw::{c_char, c_int, c_short, c_uchar};

    #[repr(C)]
    pub struct NhlmsSpx {
        _private: [u8; 0],
    }

    pub const E_ARGS: c_int = -1;
    pub const E_END: c_int = -2;
    pub const E_CORRUPT: c_int = -3;
    pub const E_FATAL: c_int = -4;
    pub const E_POISONED: c_int = -5;
    pub const E_OVERSIZE: c_int = -6;

    extern "C" {
        pub fn nhlms_spx_new_uwb(enhancer: c_int) -> *mut NhlmsSpx;
        pub fn nhlms_spx_frame_size(h: *const NhlmsSpx) -> c_int;
        pub fn nhlms_spx_decode(h: *mut NhlmsSpx, data: *const c_uchar, len: c_int, out: *mut c_short, out_cap: c_int) -> c_int;
        pub fn nhlms_spx_bits_remaining(h: *mut NhlmsSpx) -> c_int;
        pub fn nhlms_spx_free(h: *mut NhlmsSpx);
        pub fn nhlms_spx_version() -> *const c_char;
        pub fn nhlms_spx_last_fatal() -> *const c_char;
    }
}

/// The largest packet the shim accepts (libspeex's default bit-buffer size).
pub const MAX_PACKET: usize = 2000;

/// A named decoder failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeexError {
    /// The decoder could not be created.
    Init,
    /// An empty packet, or a packet larger than [`MAX_PACKET`] bytes.
    PacketSize(usize),
    /// libspeex read an end-of-stream code in the packet.
    EndOfStream,
    /// libspeex reported a corrupt stream (e.g. an invalid mode).
    Corrupt,
    /// libspeex hit an internal assertion; the decoder is unusable from here on.
    Fatal(String),
    /// A previous call hit [`SpeexError::Fatal`].
    Poisoned,
    /// An unexpected code from the shim.
    Shim(i32),
}

impl fmt::Display for SpeexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpeexError::Init => write!(f, "the Speex decoder could not be created"),
            SpeexError::PacketSize(n) => write!(f, "a Speex packet of {n} bytes is outside 1..={MAX_PACKET}"),
            SpeexError::EndOfStream => write!(f, "the Speex packet carries an end-of-stream code"),
            SpeexError::Corrupt => write!(f, "libspeex reported a corrupt Speex frame"),
            SpeexError::Fatal(m) => write!(f, "libspeex internal error ({m}); decoding stopped"),
            SpeexError::Poisoned => write!(f, "the Speex decoder stopped after an internal error"),
            SpeexError::Shim(c) => write!(f, "unexpected Speex shim code {c}"),
        }
    }
}

impl std::error::Error for SpeexError {}

/// The upstream version string of the linked libspeex (`"speex-1.2.1"`).
pub fn version() -> String {
    // SAFETY: the shim returns a pointer to a static NUL-terminated upstream string (or NULL).
    unsafe {
        let p = ffi::nhlms_spx_version();
        if p.is_null() {
            String::new()
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

/// One Speex ultra-wideband decoder (32 kHz, 640 samples per frame). Not `Sync`; `Send` is sound
/// because the handle is owned exclusively and libspeex keeps no thread-bound state in it.
pub struct UwbDecoder {
    h: NonNull<ffi::NhlmsSpx>,
    frame_size: usize,
}

// SAFETY: the handle is heap state owned solely by this value; the shim's fatal-error slot is
// thread-local and only consulted during the call that set it.
unsafe impl Send for UwbDecoder {}

impl UwbDecoder {
    /// A new decoder. `enhancer` sets libspeex's perceptual enhancer (`SPEEX_SET_ENH`); the R83
    /// reference path decodes with it **on**.
    pub fn new(enhancer: bool) -> Result<Self, SpeexError> {
        // SAFETY: plain constructor; NULL is handled.
        let raw = unsafe { ffi::nhlms_spx_new_uwb(enhancer as i32) };
        let h = NonNull::new(raw).ok_or(SpeexError::Init)?;
        // SAFETY: `h` is a live handle.
        let fs = unsafe { ffi::nhlms_spx_frame_size(h.as_ptr()) };
        let d = UwbDecoder { h, frame_size: fs.max(0) as usize };
        if fs <= 0 {
            return Err(SpeexError::Init);
        }
        Ok(d)
    }

    /// Samples per frame (640 for ultra-wideband).
    pub fn frame_size(&self) -> usize {
        self.frame_size
    }

    /// Decode exactly one packet (one Speex frame) into `out`, which must hold at least
    /// [`frame_size`](Self::frame_size) samples; returns the number of samples written.
    pub fn decode_into(&mut self, packet: &[u8], out: &mut [i16]) -> Result<usize, SpeexError> {
        if packet.is_empty() || packet.len() > MAX_PACKET {
            return Err(SpeexError::PacketSize(packet.len()));
        }
        if out.len() < self.frame_size {
            return Err(SpeexError::Shim(ffi::E_ARGS));
        }
        let cap = out.len().min(i32::MAX as usize) as i32;
        // SAFETY: the handle is live; `packet` and `out` are valid for the lengths passed, which
        // the shim checks again; the shim does not retain either pointer.
        let rc = unsafe { ffi::nhlms_spx_decode(self.h.as_ptr(), packet.as_ptr(), packet.len() as i32, out.as_mut_ptr(), cap) };
        match rc {
            n if n > 0 => Ok(n as usize),
            ffi::E_END => Err(SpeexError::EndOfStream),
            ffi::E_CORRUPT => Err(SpeexError::Corrupt),
            ffi::E_POISONED => Err(SpeexError::Poisoned),
            ffi::E_OVERSIZE => Err(SpeexError::PacketSize(packet.len())),
            ffi::E_FATAL => {
                // SAFETY: static upstream message or NULL, set by the call that just returned.
                let m = unsafe {
                    let p = ffi::nhlms_spx_last_fatal();
                    if p.is_null() {
                        String::from("unknown")
                    } else {
                        CStr::from_ptr(p).to_string_lossy().into_owned()
                    }
                };
                Err(SpeexError::Fatal(m))
            }
            other => Err(SpeexError::Shim(other)),
        }
    }

    /// Decode one packet into a new frame.
    pub fn decode(&mut self, packet: &[u8]) -> Result<Vec<i16>, SpeexError> {
        let mut out = vec![0i16; self.frame_size];
        let n = self.decode_into(packet, &mut out)?;
        out.truncate(n);
        Ok(out)
    }

    /// Bits left unread in the last packet (diagnostic).
    pub fn bits_remaining(&mut self) -> Result<usize, SpeexError> {
        // SAFETY: live handle.
        let r = unsafe { ffi::nhlms_spx_bits_remaining(self.h.as_ptr()) };
        match r {
            n if n >= 0 => Ok(n as usize),
            ffi::E_POISONED => Err(SpeexError::Poisoned),
            other => Err(SpeexError::Shim(other)),
        }
    }
}

impl Drop for UwbDecoder {
    fn drop(&mut self) {
        // SAFETY: the handle came from `nhlms_spx_new_uwb` and is freed exactly once here.
        unsafe { ffi::nhlms_spx_free(self.h.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_and_frame_size() {
        assert_eq!(version(), "speex-1.2.1");
        let d = UwbDecoder::new(true).unwrap();
        assert_eq!(d.frame_size(), 640);
    }

    #[test]
    fn packet_bounds_are_checked_before_the_call() {
        let mut d = UwbDecoder::new(true).unwrap();
        assert_eq!(d.decode(&[]), Err(SpeexError::PacketSize(0)));
        assert_eq!(d.decode(&vec![0u8; MAX_PACKET + 1]), Err(SpeexError::PacketSize(MAX_PACKET + 1)));
        let mut small = [0i16; 10];
        assert!(d.decode_into(&[0x30; 20], &mut small).is_err(), "output smaller than a frame");
    }

    /// Arbitrary bytes never take the process down: each packet decodes to a frame or refuses by
    /// name; a fatal assertion (if one is reached) poisons that decoder only, and a fresh decoder
    /// works afterwards. Deterministic pseudo-random input (xorshift).
    #[test]
    fn arbitrary_packets_decode_or_refuse_without_aborting() {
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        let (mut ok, mut refused, mut fatal) = (0, 0, 0);
        let mut d = UwbDecoder::new(true).unwrap();
        for _ in 0..3000 {
            let len = 1 + (next() % 255) as usize;
            let pkt: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            match d.decode(&pkt) {
                Ok(f) => {
                    assert_eq!(f.len(), 640);
                    ok += 1;
                }
                Err(SpeexError::Fatal(_)) => {
                    fatal += 1;
                    assert_eq!(d.decode(&pkt), Err(SpeexError::Poisoned));
                    d = UwbDecoder::new(true).unwrap();
                }
                Err(_) => refused += 1,
            }
        }
        eprintln!("arbitrary packets: {ok} decoded, {refused} refused, {fatal} internal errors contained");
        assert!(ok + refused + fatal == 3000);
    }
}
