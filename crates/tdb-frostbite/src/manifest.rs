use crate::{be, cstr, hex, invalid, le, slice, Result};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct Asset {
    pub kind: String,
    pub name: String,
    pub size: u32,
    pub sha1: String,
    pub resource_type: Option<u32>,
    pub resource_id: Option<String>,
    pub resource_meta: Option<String>,
    pub logical_offset: Option<u32>,
}
pub fn parse(bytes: &[u8]) -> Result<Vec<Asset>> {
    let size = be(bytes, 0)? as usize;
    let b = slice(bytes, 4, size)?;
    if size + 4 != bytes.len() {
        return Err(invalid("manifest span length mismatch"));
    }
    if le(b, 0)? != 0x9d798ed6 {
        return Err(invalid("unsupported bundle manifest magic"));
    }
    let (total, ne, nr, nc) = (
        le(b, 4)? as usize,
        le(b, 8)? as usize,
        le(b, 12)? as usize,
        le(b, 16)? as usize,
    );
    if total != ne + nr + nc {
        return Err(invalid("manifest counts disagree"));
    }
    let strings = le(b, 20)? as usize;
    let meta = le(b, 24)? as usize;
    let meta_len = le(b, 28)? as usize;
    if meta_len != 0 {
        slice(b, meta, meta_len)?;
    }
    let hashes = 32;
    let ebx = hashes + total * 20;
    let res = ebx + ne * 8;
    let types = res + nr * 8;
    let metas = types + nr * 4;
    let ids = metas + nr * 16;
    let chunks = ids + nr * 8;
    let end = chunks + nc * 24;
    slice(b, 0, end)?;
    if total != 0 && (strings < end || strings >= b.len()) {
        return Err(invalid("invalid manifest string table"));
    }
    let mut out = Vec::with_capacity(total);
    for i in 0..total {
        let mut a = Asset {
            kind: String::new(),
            name: String::new(),
            size: 0,
            sha1: hex(slice(b, hashes + i * 20, 20)?),
            resource_type: None,
            resource_id: None,
            resource_meta: None,
            logical_offset: None,
        };
        if i < ne + nr {
            let is_res = i >= ne;
            let j = if is_res { i - ne } else { i };
            let p = if is_res { res } else { ebx } + j * 8;
            a.kind = if is_res { "res" } else { "ebx" }.into();
            a.name = cstr(b, strings + le(b, p)? as usize)?;
            a.size = le(b, p + 4)?;
            if is_res {
                a.resource_type = Some(le(b, types + j * 4)?);
                a.resource_id = Some(format!(
                    "{:016x}",
                    u64::from_le_bytes(slice(b, ids + j * 8, 8)?.try_into().unwrap())
                ));
                a.resource_meta = Some(hex(slice(b, metas + j * 16, 16)?));
            }
        } else {
            let p = chunks + (i - ne - nr) * 24;
            a.kind = "chunk".into();
            a.name = hex(slice(b, p, 16)?);
            a.logical_offset = Some(le(b, p + 16)?);
            a.size = le(b, p + 20)?;
        }
        out.push(a);
    }
    Ok(out)
}
