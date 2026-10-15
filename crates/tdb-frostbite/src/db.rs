//! Strict DbObject decoding, with bounded nesting and exact container consumption.
use crate::{hex, invalid, payload, slice, Result};
use serde_json::{Map, Value};

struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = slice(self.b, self.p, n)?;
        self.p += n;
        Ok(s)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn len(&mut self) -> Result<usize> {
        let mut n = 0u32;
        for shift in (0..35).step_by(7) {
            let v = self.byte()?;
            if shift == 28 && v > 15 {
                return Err(invalid("LEB128 overflow"));
            }
            n |= ((v & 127) as u32) << shift;
            if v & 128 == 0 {
                return Ok(n as usize);
            }
        }
        Err(invalid("unterminated LEB128"))
    }
    fn entry(&mut self, depth: usize) -> Result<Option<(String, Value)>> {
        if depth > 64 {
            return Err(invalid("DbObject nesting limit"));
        }
        let t = self.byte()?;
        if t == 0 {
            return Ok(None);
        }
        let name = if t & 128 == 0 {
            let s = crate::cstr(self.b, self.p)?;
            self.p += s.len() + 1;
            s
        } else {
            String::new()
        };
        let value = match t & 31 {
            1 | 2 => {
                let n = self.len()?;
                let mut sub = Reader {
                    b: self.take(n)?,
                    p: 0,
                };
                let mut list = Vec::new();
                let mut map = Map::new();
                let mut terminated = false;
                while sub.p < sub.b.len() {
                    match sub.entry(depth + 1)? {
                        None => {
                            terminated = true;
                            break;
                        }
                        Some((k, v)) => {
                            if t & 31 == 1 {
                                list.push(v)
                            } else if map.insert(k, v).is_some() {
                                return Err(invalid("duplicate DbObject key"));
                            }
                        }
                    }
                }
                if !terminated || sub.p != sub.b.len() {
                    return Err(invalid("DbObject container length/terminator mismatch"));
                }
                if t & 31 == 1 {
                    Value::Array(list)
                } else {
                    Value::Object(map)
                }
            }
            6 => {
                let v = self.byte()?;
                if v > 1 {
                    return Err(invalid("invalid boolean"));
                }
                Value::Bool(v != 0)
            }
            7 => {
                let n = self.len()?;
                let bytes = self.take(n)?;
                if bytes.last() != Some(&0) {
                    return Err(invalid("string missing terminator"));
                }
                Value::String(
                    String::from_utf8(bytes[..n - 1].to_vec())
                        .map_err(|_| invalid("invalid UTF-8"))?,
                )
            }
            8 => Value::from(i32::from_le_bytes(self.take(4)?.try_into().unwrap())),
            9 => Value::from(i64::from_le_bytes(self.take(8)?.try_into().unwrap())),
            11 | 12 => {
                let f = f64::from_le_bytes(self.take(8)?.try_into().unwrap());
                if !f.is_finite() {
                    return Err(invalid("nonfinite double"));
                }
                Value::from(f)
            }
            15 => Value::String(hex(self.take(16)?)),
            16 => Value::String(hex(self.take(20)?)),
            // Preserve opaque bytes without interpreting encrypted initfs contents.
            19 => {
                let n = self.len()?;
                serde_json::json!({"blob_hex":hex(self.take(n)?)})
            }
            k => return Err(invalid(format!("unsupported DbObject type {k:#x}"))),
        };
        Ok(Some((name, value)))
    }
}
pub fn parse(bytes: &[u8]) -> Result<Value> {
    let mut r = Reader {
        b: payload(bytes)?,
        p: 0,
    };
    let (_, v) = r.entry(0)?.ok_or_else(|| invalid("empty DbObject"))?;
    if r.p != r.b.len() {
        return Err(invalid("trailing DbObject bytes"));
    }
    Ok(v)
}
