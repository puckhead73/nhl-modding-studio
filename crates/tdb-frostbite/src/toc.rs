use crate::{be, cstr, hex, invalid, payload, slice, Result};
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Span {
    pub cas: u32,
    pub offset: u32,
    pub size: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct Bundle {
    pub ordinal: usize,
    pub name: String,
    pub name_word: u32,
    pub spans: Vec<Span>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Chunk {
    pub guid: String,
    pub span: Span,
}
#[derive(Debug, Serialize)]
pub struct Toc {
    pub flags: u32,
    pub bundles: Vec<Bundle>,
    pub chunks: Vec<Chunk>,
}

// The compressed-name table is an array of binary node pairs; negative values
// encode complemented characters, nonnegative values reference earlier pairs.
// Decode directly from the bounded table, without constructing a recursive tree.
fn name(b: &[u8], w: &[u32], start: u32) -> Result<String> {
    if w[11] & 4 == 0 {
        return cstr(b, w[8] as usize + start as usize);
    }
    let table = slice(b, w[14] as usize, w[13] as usize * 4)?;
    let data = slice(b, w[8] as usize, w[12] as usize * 4)?;
    if table.len() < 8 || table.len() % 8 != 0 {
        return Err(invalid("invalid name tree"));
    }
    let root = table.len() / 8 - 1;
    let mut bit = start as usize;
    let mut out = Vec::new();
    loop {
        let mut node = root;
        loop {
            if bit >= data.len() * 8 {
                return Err(invalid("name bit offset out of range"));
            }
            let branch = (be(data, bit / 32 * 4)? >> (bit % 32)) & 1;
            bit += 1;
            let v = be(table, node * 8 + branch as usize * 4)? as i32;
            if v < 0 {
                let ch = (!v) as u32;
                if ch == 0 {
                    return String::from_utf8(out).map_err(|_| invalid("invalid name UTF-8"));
                }
                if ch > 127 || out.len() >= 4096 {
                    return Err(invalid("unsupported/oversize bundle name"));
                }
                out.push(ch as u8);
                break;
            }
            if v as usize >= node {
                return Err(invalid("cyclic/forward name tree edge"));
            }
            node = v as usize;
        }
    }
}

pub fn parse(bytes: &[u8]) -> Result<Toc> {
    let b = payload(bytes)?;
    let w = (0..15).map(|i| be(b, i * 4)).collect::<Result<Vec<_>>>()?;
    if w[0] != 0x3c {
        return Err(invalid("not an NHL PS4 superbundle TOC"));
    }
    let (bd, bc, cg, cc, cd) = (
        w[1] as usize,
        w[2] as usize,
        w[4] as usize,
        w[5] as usize,
        w[9] as usize,
    );
    if w[10] as u64 != cc as u64 * 3 {
        return Err(invalid(
            "not PS4 3-word chunk records; PS5 requires a separate profile",
        ));
    }
    slice(b, bd, bc * 16)?;
    slice(b, cg, cc * 20)?;
    slice(b, cd, cc * 12)?;
    let mut chunks = Vec::with_capacity(cc);
    for i in 0..cc {
        let r = be(b, cg + i * 20 + 16)?;
        if r >> 24 != 1 {
            return Err(invalid(format!(
                "unsupported chunk reference flag {:#x}",
                r >> 24
            )));
        }
        let idx = (r & 0xffffff) as usize;
        if idx % 3 != 0 || idx + 3 > w[10] as usize {
            return Err(invalid("chunk reference outside data table"));
        }
        chunks.push(Chunk {
            guid: hex(slice(b, cg + i * 20, 16)?),
            span: Span {
                cas: be(b, cd + idx * 4)?,
                offset: be(b, cd + idx * 4 + 4)?,
                size: be(b, cd + idx * 4 + 8)?,
            },
        });
    }
    let mut bundles = Vec::with_capacity(bc);
    for i in 0..bc {
        let pos = bd + i * 16;
        let nw = be(b, pos)?;
        let sz = be(b, pos + 4)?;
        if sz >> 30 != 1 {
            return Err(invalid("unsupported external/deleted bundle record"));
        }
        let off = usize::try_from(u64::from_be_bytes(
            slice(b, pos + 8, 8)?.try_into().unwrap(),
        ))
        .map_err(|_| invalid("bundle offset overflow"))?;
        let record = slice(b, off, (sz & 0x3fffffff) as usize)?;
        let count = be(record, 12)? as usize;
        let flags_at = record
            .len()
            .checked_sub(count)
            .ok_or_else(|| invalid("bundle flags outside record"))?;
        if count == 0 || flags_at < 32 || count > (flags_at - 32) / 8 {
            return Err(invalid("invalid bundle span count"));
        }
        let flags = &record[flags_at..];
        let mut p = 32;
        let mut cas = None;
        let mut spans = Vec::with_capacity(count);
        for flag in flags {
            match flag {
                0 => {}
                1 => {
                    cas = Some(be(record, p)?);
                    p += 4;
                }
                _ => return Err(invalid("unsupported span identifier flag")),
            }
            let id = cas.ok_or_else(|| invalid("first span missing CAS id"))?;
            spans.push(Span {
                cas: id,
                offset: be(record, p)?,
                size: be(record, p + 4)?,
            });
            p += 8;
        }
        if p != flags_at {
            return Err(invalid("span stream does not meet flags"));
        }
        bundles.push(Bundle {
            ordinal: i,
            name: name(b, &w, nw)?,
            name_word: nw,
            spans,
        });
    }
    Ok(Toc {
        flags: w[11],
        bundles,
        chunks,
    })
}
