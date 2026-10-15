//! The SND10 `.sbr` bank (`SBle` little-endian on PS4, `SBbe` big-endian on Xbox 360 / PS3).
//!
//! Layout (research probe `snd10_sbr.py`, observed on the NHL 21 PS4 banks; the bit-exact fields
//! are reproduced here, the unknown words are kept as unknown):
//!
//! ```text
//! 0x00 magic 'SBle' | u32 file size | u16 version | u16 dset count | u32 bank id
//! 0x10 'MRAH' | u32 ? | u32 pointer-block offset (0x50) | u32 0x20
//! 0x20 u32 data chunk offset | u32 0x50 | zeros
//! 0x50 pointer block: dset_count × (u32 DSET offset, u32 pointer-slot address)
//! DSET 'TESD': u32 size | u64 dataset id | … | +0x38 u32 record count | u16 field count | u16 ? |
//!              u16 fields offset (0x48) | … ; field descriptors 24 bytes each at +fields offset:
//!              name (4 bytes, stored reversed) | u8 type | u8 storage | u8 u1 | u8 element size |
//!              u32 x | u32 0 | u32 array offset (absolute) | u32 ptr2
//! storage 0: every record = x           storage 1: record i = x + i · step (step = i16 from (u1, esize))
//! storage 2: array of nrec × esize at array offset, record = x + (elem << u1)
//! storage 3: table of x × esize at array offset, then nrec packed u1-bit indices, record = table[idx]
//! data chunk: 'data' [+ 0x104-byte stream path buffer ('\0\xC1' + path)] [+ embedded SNS payloads]
//! ```
//!
//! `.OFF` values are offsets into the `.sbs` stream file; `.RAM` values are relative to the data
//! chunk (embedded payloads). `.DUR` is an f32 bit pattern (seconds).

use std::collections::BTreeMap;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SbrError {
    #[error("not an SND10 bank: magic {0:?} (expected SBle or SBbe)")]
    Magic([u8; 4]),
    #[error("bank truncated: {what} needs {need} bytes at {at}, {len} available")]
    Truncated { what: &'static str, at: usize, need: usize, len: usize },
    #[error("DSET pointer {index} at {at} does not point at a 'TESD' record")]
    NotDset { index: usize, at: usize },
    #[error("field {name} uses storage {storage} with element size {esize}, which this reader does not decode")]
    Storage { name: String, storage: u8, esize: u8 },
}

/// One decoded field of a DSET: per-record integer values (`None` when the storage could not be
/// decoded — the field is still listed so the bank's shape is visible).
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: u8,
    pub storage: u8,
    pub elem_size: u8,
    pub values: Option<Vec<u64>>,
}

/// One DSET (record table).
#[derive(Debug, Clone)]
pub struct Dset {
    pub offset: usize,
    pub dataset_id: u64,
    pub records: usize,
    pub fields: Vec<Field>,
}

impl Dset {
    /// Record `i` as a field-name → value map (only decodable fields).
    pub fn record(&self, i: usize) -> BTreeMap<&str, u64> {
        self.fields
            .iter()
            .filter_map(|f| f.values.as_ref().and_then(|v| v.get(i)).map(|v| (f.name.as_str(), *v)))
            .collect()
    }
    /// The values of the field `name`, if decodable.
    pub fn column(&self, name: &str) -> Option<&[u64]> {
        self.fields.iter().find(|f| f.name == name).and_then(|f| f.values.as_deref())
    }
}

/// A parsed bank header + its DSETs (the bytes are not retained).
#[derive(Debug, Clone)]
pub struct Sbr {
    pub little_endian: bool,
    pub version: u16,
    pub bank_id: u32,
    pub data_offset: usize,
    /// The `.sbs` stream file name recorded in the data chunk's path buffer, when present.
    pub stream_path: Option<String>,
    pub dsets: Vec<Dset>,
}

fn need<'a>(b: &'a [u8], what: &'static str, at: usize, n: usize) -> Result<&'a [u8], SbrError> {
    b.get(at..at.checked_add(n).ok_or(SbrError::Truncated { what, at, need: n, len: b.len() })?)
        .ok_or(SbrError::Truncated { what, at, need: n, len: b.len() })
}

struct Rd {
    le: bool,
}
impl Rd {
    fn u16(&self, b: &[u8]) -> u16 {
        let a = [b[0], b[1]];
        if self.le { u16::from_le_bytes(a) } else { u16::from_be_bytes(a) }
    }
    fn u32(&self, b: &[u8]) -> u32 {
        let a = [b[0], b[1], b[2], b[3]];
        if self.le { u32::from_le_bytes(a) } else { u32::from_be_bytes(a) }
    }
    fn u64(&self, b: &[u8]) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[..8]);
        if self.le { u64::from_le_bytes(a) } else { u64::from_be_bytes(a) }
    }
    fn elem(&self, b: &[u8], esize: u8) -> Option<u64> {
        Some(match esize {
            1 => b[0] as u64,
            2 => self.u16(b) as u64,
            4 => self.u32(b) as u64,
            8 => self.u64(b),
            _ => return None,
        })
    }
}

impl Sbr {
    pub fn parse(b: &[u8]) -> Result<Sbr, SbrError> {
        let magic = need(b, "magic", 0, 4)?;
        let le = match magic {
            b"SBle" => true,
            b"SBbe" => false,
            m => return Err(SbrError::Magic([m[0], m[1], m[2], m[3]])),
        };
        let r = Rd { le };
        let head = need(b, "header", 0, 0x24)?;
        let version = r.u16(&head[8..10]);
        let dset_count = r.u16(&head[10..12]) as usize;
        let bank_id = r.u32(&head[12..16]);
        let ptr_block = r.u32(&head[0x18..0x1c]) as usize;
        let data_offset = r.u32(&head[0x20..0x24]) as usize;
        let stream_path = match b.get(data_offset + 4..data_offset + 6) {
            // The NHL 15–17 banks carry the placeholder text "[[Stream File Path Buffer …]]" here: no
            // path is recorded, and the `.sbs` is the bank's own stem (as the research probes resolve it).
            Some([0x00, 0xc1]) => b
                .get(data_offset + 4..(data_offset + 4 + 0x100).min(b.len()))
                .map(|s| s.iter().map(|&c| c as char).collect::<String>().trim_matches(|c| c == '\0' || c == ' ' || c == '\u{c1}').to_string())
                .filter(|s| !s.is_empty() && !s.starts_with("[[")),
            _ => None,
        };
        let mut dsets = Vec::with_capacity(dset_count);
        for i in 0..dset_count {
            let q = ptr_block + 8 * i;
            let p = r.u32(need(b, "DSET pointer", q, 4)?) as usize;
            if need(b, "DSET", p, 8).is_err() || &b[p..p + 4] != b"TESD" {
                return Err(SbrError::NotDset { index: i, at: p });
            }
            dsets.push(Self::dset(b, &r, p)?);
        }
        Ok(Sbr { little_endian: le, version, bank_id, data_offset, stream_path, dsets })
    }

    fn dset(b: &[u8], r: &Rd, p: usize) -> Result<Dset, SbrError> {
        let h = need(b, "DSET header", p, 0x48)?;
        let dataset_id = r.u64(&h[8..16]);
        let nrec = r.u32(&h[0x38..0x3c]) as usize;
        let nfld = r.u16(&h[0x3c..0x3e]) as usize;
        let foff = r.u16(&h[0x40..0x42]) as usize;
        let mut fields = Vec::with_capacity(nfld);
        for i in 0..nfld {
            let q = p + foff + 24 * i;
            let d = need(b, "field descriptor", q, 24)?;
            let name: String = d[..4].iter().rev().map(|&c| c as char).collect();
            let (ty, storage, u1, esize) = (d[4], d[5], d[6], d[7]);
            let x = r.u32(&d[8..12]) as u64;
            let aoff = r.u32(&d[16..20]) as usize;
            let values = Self::field_values(b, r, storage, u1, esize, x, aoff, nrec);
            fields.push(Field { name, ty, storage, elem_size: esize, values });
        }
        Ok(Dset { offset: p, dataset_id, records: nrec, fields })
    }

    fn field_values(b: &[u8], r: &Rd, storage: u8, u1: u8, esize: u8, x: u64, aoff: usize, nrec: usize) -> Option<Vec<u64>> {
        match storage {
            0 => Some(vec![x; nrec]),
            1 => {
                let step = (u1 as u16 | ((esize as u16) << 8)) as i16 as i64;
                Some((0..nrec as i64).map(|i| (x as i64 + i * step) as u64).collect())
            }
            2 => {
                let es = esize as usize;
                let arr = b.get(aoff..aoff.checked_add(es.checked_mul(nrec)?)?)?;
                arr.chunks_exact(es).map(|c| r.elem(c, esize).map(|v| x.wrapping_add(v << u1))).collect()
            }
            3 => {
                if u1 == 0 {
                    return None;
                }
                let es = esize as usize;
                let n = x as usize;
                let tab = b.get(aoff..aoff.checked_add(es.checked_mul(n)?)?)?;
                let table: Option<Vec<u64>> = tab.chunks_exact(es).map(|c| r.elem(c, esize)).collect();
                let table = table?;
                let bits = b.get(aoff + es * n..)?;
                let mut out = Vec::with_capacity(nrec);
                let mut bp = 0usize;
                for _ in 0..nrec {
                    let mut v = 0usize;
                    for k in 0..u1 as usize {
                        let byte = *bits.get((bp + k) >> 3)?;
                        v |= (((byte >> ((bp + k) & 7)) & 1) as usize) << k;
                    }
                    bp += u1 as usize;
                    out.push(*table.get(v)?);
                }
                Some(out)
            }
            _ => None,
        }
    }

    /// The DSET with the most records (the sample table of a `pbp_samples*.sbr` bank).
    pub fn largest_dset(&self) -> Option<&Dset> {
        self.dsets.iter().max_by_key(|d| d.records)
    }
}

/// `.DUR` as seconds (an f32 bit pattern in the bank).
pub fn duration_seconds(dur: u64) -> f32 {
    f32::from_bits(dur as u32)
}

/// Synthetic `SBle` banks for tests and fixtures (not a game writer): one DSET whose records carry
/// `.SID` / `.OFF` / `.DUR` as storage-2 arrays (u32, shift 0), and a data chunk with the
/// placeholder path buffer the shipped banks carry.
pub mod synth {
    /// Build a little-endian bank with the given `(sid, off, dur_bits)` records.
    pub fn build_bank(records: &[(u32, u32, u32)]) -> Vec<u8> {
        let nrec = records.len();
        let p = 0x60usize;
        let nfld = 3usize;
        let arrays = p + 0x48 + 24 * nfld;
        let arrays_end = arrays + 4 * nrec * nfld;
        let data_off = (arrays_end + 15) & !15;
        let total = data_off + 4 + 0x104;
        let mut b = vec![0u8; total];
        let w16 = |b: &mut [u8], at: usize, v: u16| b[at..at + 2].copy_from_slice(&v.to_le_bytes());
        let w32 = |b: &mut [u8], at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        b[..4].copy_from_slice(b"SBle");
        w32(&mut b, 4, total as u32);
        w16(&mut b, 8, 1);
        w16(&mut b, 10, 1);
        w32(&mut b, 12, 0x5EED);
        b[0x10..0x14].copy_from_slice(b"MRAH");
        w32(&mut b, 0x18, 0x50);
        w32(&mut b, 0x20, data_off as u32);
        w32(&mut b, 0x50, p as u32);
        b[p..p + 4].copy_from_slice(b"TESD");
        w32(&mut b, p + 4, (0x48 + 24 * nfld) as u32);
        w32(&mut b, p + 0x38, nrec as u32);
        w16(&mut b, p + 0x3c, nfld as u16);
        w16(&mut b, p + 0x40, 0x48);
        for (i, name) in [b".SID", b".OFF", b".DUR"].iter().enumerate() {
            let q = p + 0x48 + 24 * i;
            let rev: Vec<u8> = name.iter().rev().copied().collect();
            b[q..q + 4].copy_from_slice(&rev);
            b[q + 4] = 1;
            b[q + 5] = 2;
            b[q + 6] = 0;
            b[q + 7] = 4;
            w32(&mut b, q + 16, (arrays + 4 * nrec * i) as u32);
            for (r, rec) in records.iter().enumerate() {
                let v = [rec.0, rec.1, rec.2][i];
                w32(&mut b, arrays + 4 * nrec * i + 4 * r, v);
            }
        }
        b[data_off..data_off + 4].copy_from_slice(b"data");
        b[data_off + 4] = 0;
        b[data_off + 5] = 0xc1;
        let ph = b"[[Stream File Path Buffer]]";
        b[data_off + 6..data_off + 6 + ph.len()].copy_from_slice(ph);
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-DSET little-endian bank with the four storages.
    fn bank(le: bool) -> Vec<u8> {
        let mut b = vec![0u8; 0x400];
        b[..4].copy_from_slice(if le { b"SBle" } else { b"SBbe" });
        let w16 = |b: &mut [u8], at: usize, v: u16| b[at..at + 2].copy_from_slice(&if le { v.to_le_bytes() } else { v.to_be_bytes() });
        let w32 = |b: &mut [u8], at: usize, v: u32| b[at..at + 4].copy_from_slice(&if le { v.to_le_bytes() } else { v.to_be_bytes() });
        w32(&mut b, 4, 0x400);
        w16(&mut b, 8, 1);
        w16(&mut b, 10, 1);
        w32(&mut b, 12, 0xABCD);
        w32(&mut b, 0x18, 0x50);
        w32(&mut b, 0x20, 0x300);
        // pointer block → DSET at 0x60
        w32(&mut b, 0x50, 0x60);
        let p = 0x60;
        b[p..p + 4].copy_from_slice(b"TESD");
        w32(&mut b, p + 4, 0x48 + 24 * 4);
        w32(&mut b, p + 0x38, 4); // 4 records
        w16(&mut b, p + 0x3c, 4); // 4 fields
        w16(&mut b, p + 0x40, 0x48);
        let arrays = p + 0x48 + 24 * 4; // 0x108
        let mut fld = |i: usize, name: &[u8; 4], storage: u8, u1: u8, esize: u8, x: u32, aoff: u32| {
            let q = p + 0x48 + 24 * i;
            let rev: Vec<u8> = name.iter().rev().copied().collect();
            b[q..q + 4].copy_from_slice(&rev);
            b[q + 4] = 1;
            b[q + 5] = storage;
            b[q + 6] = u1;
            b[q + 7] = esize;
            w32(&mut b, q + 8, x);
            w32(&mut b, q + 16, aoff);
        };
        fld(0, b".SID", 0, 0, 4, 7, 0); // constant 7
        fld(1, b".OFF", 1, 0x10, 0x00, 100, 0); // progression step 0x0010 = 16
        fld(2, b".RAM", 2, 4, 2, 1000, arrays as u32); // array u16 << 4 + 1000
        fld(3, b".DUR", 3, 2, 4, 3, (arrays + 8) as u32); // table of 3 u32, 2-bit indices
        // storage-2 array: 4 × u16
        for (i, v) in [1u16, 2, 3, 4].iter().enumerate() {
            w16(&mut b, arrays + 2 * i, *v);
        }
        // storage-3 table: 3 × u32 then packed indices 2,0,1,2 → bits (LSB first): 10 00 01 10 → byte 0b10010010? compute below
        let t = arrays + 8;
        for (i, v) in [0x3f80_0000u32, 0x4000_0000, 0x4040_0000].iter().enumerate() {
            w32(&mut b, t + 4 * i, *v);
        }
        let idx = [2u8, 0, 1, 2];
        let mut byte = 0u8;
        for (i, v) in idx.iter().enumerate() {
            byte |= v << (2 * i);
        }
        b[t + 12] = byte;
        // data chunk with a path buffer
        b[0x300..0x304].copy_from_slice(b"data");
        b[0x304] = 0;
        b[0x305] = 0xc1;
        b[0x306..0x306 + 15].copy_from_slice(b"pbp_samples.sbs");
        b
    }

    #[test]
    fn parses_both_endiannesses_and_the_four_storages() {
        for le in [true, false] {
            let s = Sbr::parse(&bank(le)).unwrap();
            assert_eq!((s.little_endian, s.version, s.bank_id, s.data_offset), (le, 1, 0xABCD, 0x300));
            assert_eq!(s.stream_path.as_deref(), Some("pbp_samples.sbs"));
            let d = s.largest_dset().unwrap();
            assert_eq!(d.records, 4);
            assert_eq!(d.column(".SID").unwrap(), &[7, 7, 7, 7]);
            assert_eq!(d.column(".OFF").unwrap(), &[100, 116, 132, 148]);
            assert_eq!(d.column(".RAM").unwrap(), &[1016, 1032, 1048, 1064]);
            assert_eq!(d.column(".DUR").unwrap(), &[0x4040_0000, 0x3f80_0000, 0x4000_0000, 0x4040_0000]);
            assert_eq!(duration_seconds(d.column(".DUR").unwrap()[1]), 1.0);
            assert_eq!(d.record(2)[".OFF"], 132);
        }
    }

    #[test]
    fn refuses_bad_magic_truncation_and_a_non_dset_pointer() {
        assert!(matches!(Sbr::parse(b"XXXX0000"), Err(SbrError::Magic(_))));
        let b = bank(true);
        assert!(matches!(Sbr::parse(&b[..0x20]), Err(SbrError::Truncated { .. })));
        let mut bad = b.clone();
        bad[0x60..0x64].copy_from_slice(b"XXXX");
        assert!(matches!(Sbr::parse(&bad), Err(SbrError::NotDset { index: 0, at: 0x60 })));
        // A storage-2 array past the end of the bank is an undecodable field, not a panic.
        let mut cut = b.clone();
        cut[0x60 + 0x48 + 24 * 2 + 16..0x60 + 0x48 + 24 * 2 + 20].copy_from_slice(&0x3ff0u32.to_le_bytes());
        let s = Sbr::parse(&cut).unwrap();
        assert!(s.dsets[0].column(".RAM").is_none());
    }
}
