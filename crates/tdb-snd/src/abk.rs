//! Gen7 (PS3 / Xbox 360) `ABKC` sound banks and their `.ast` stream files — **read, list, locate,
//! frame** — on the research lane's measured contracts:
//!
//! * R82 (2026-09-24 handoff 01 §1): the `S10A` sample table and its records (ram segments, stream
//!   stubs, loop words);
//! * R89 / R94 (2026-09-25 `ABK_TREE_CONTRACT.md`): the header words, the pointer-fixup table (B),
//!   the name directory (C, name word = `u16 bank key | u16 name key` + cstring), sample lists
//!   (`u32 count + count × { u16 idx | u8 weight | u16 X | u24 S | u32 O }`, `0xFFFF` = empty slot),
//!   every record referenced exactly once, `O` = file-absolute `.ast` stream start;
//! * R91 / R98 (`AST_FRAMING.md` §1): `.ast` = `'S10S' | u32 id | zeros to 0x40 | streams` of blocks
//!   `{ u8 flags (0x00 / 0x80 terminal) | u24 size (incl. the 8 header bytes) | u32 samples | payload }`;
//!   loop records = a one-block stub `(samples, size) == (a, b)` + the following body.
//!
//! Every named refusal of the contract is a variant of [`AbkRefusal`] / [`AstRefusal`]; nothing is
//! guessed past a refusal. The unknown words `X` and `S` are carried as read, never interpreted, and
//! never rewritten (R94). Nothing here writes. The diagnostic **decode** of an EALayer3 stream
//! (PS3, codec `0x05`, one or two channels) reuses the R77 granule grammar in [`crate::ealayer3`];
//! EA-XMA (Xbox 360, codec `0x03`) is framed only — no XMA2 decoder is chosen.
use std::collections::{BTreeMap, HashMap};

use thiserror::Error;

use crate::sns::SnrHeader;

fn be32(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}
fn be16(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|s| u16::from_be_bytes([s[0], s[1]]))
}
fn be24(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 3)
        .map(|s| u32::from_be_bytes([0, s[0], s[1], s[2]]))
}

/// Refusals of the `ABKC` bank grammar (R82 / R89 / R94).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AbkRefusal {
    #[error("not an ABKC bank (magic {0:?})")]
    Magic([u8; 4]),
    #[error("bank header truncated ({0} bytes)")]
    Truncated(usize),
    #[error("platform code 0x{0:02x} is neither PS3 (0x0a) nor Xbox 360 (0x05)")]
    Platform(u8),
    #[error("header file size {declared} ≠ actual {actual}")]
    FileSize { declared: u32, actual: usize },
    #[error("section offsets out of order or beyond the file: tree {tree}, table {table}, data end {data_end}, A {a}, B {b}, C {c}, size {size}")]
    Sections {
        tree: u32,
        table: u32,
        data_end: u32,
        a: u32,
        b: u32,
        c: u32,
        size: usize,
    },
    #[error("sample table at {0}: magic is not S10A")]
    TableMagic(u32),
    #[error("sample table header length {found} ≠ 16 + 4·(count − 1) = {expected}")]
    TableHeader { found: u32, expected: u32 },
    #[error("record {index} at {at}: {reason}")]
    Record {
        index: usize,
        at: usize,
        reason: String,
    },
    #[error("record {index}: segment samples sum to {sum}, header says {num_samples}")]
    SegmentSum {
        index: usize,
        sum: u32,
        num_samples: u32,
    },
    #[error("record {index} ends at {end}, past the table data end {data_end}")]
    RecordOverrun {
        index: usize,
        end: usize,
        data_end: usize,
    },
    #[error("pointer {ptr} (slot at {slot}) is outside the tree region {tree}..{table}")]
    PointerOutsideTree {
        slot: u32,
        ptr: u32,
        tree: u32,
        table: u32,
    },
    #[error("name entry {index}: node {node} or name {name} outside the file / tree")]
    NameOutside { index: usize, node: u32, name: u32 },
    #[error("sample list at {list}: entry {entry} names record {index}, the table holds {count}")]
    IndexOutOfRange {
        list: u32,
        entry: usize,
        index: u16,
        count: usize,
    },
    #[error("record {index} is referenced by {times} list entries (every record is referenced exactly once)")]
    ReferencedTwice { index: u16, times: usize },
    #[error("{count} record(s) referenced by no list entry: {examples:?}")]
    Unreferenced { count: usize, examples: Vec<u16> },
    #[error("sample list at {list} ({count} entries) overruns the tree region")]
    ListOverrun { list: u32, count: u32 },
}

/// Refusals of the `.ast` stream grammar (R91 / R98).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AstRefusal {
    #[error("not an S10S stream file (magic {0:?})")]
    Magic([u8; 4]),
    #[error("stream file id 0x{ast:08x} ≠ bank header word 0x{bank:08x}")]
    IdMismatch { ast: u32, bank: u32 },
    #[error(
        "stream at {start}: block {block} at {at} has flag 0x{flag:02x} (only 0x00 / 0x80 exist)"
    )]
    Flag {
        start: u64,
        block: usize,
        at: u64,
        flag: u8,
    },
    #[error(
        "stream at {start}: block {block} at {at} has size {size} (< 8 or past the file end {len})"
    )]
    BlockSize {
        start: u64,
        block: usize,
        at: u64,
        size: u32,
        len: u64,
    },
    #[error("stream at {start}: no terminal (0x80) block before the file end")]
    NoTerminal { start: u64 },
    #[error("stream at {start}: block sample words sum to {sum}, the record says {num_samples}")]
    SampleSum {
        start: u64,
        sum: u64,
        num_samples: u32,
    },
    #[error("loop stub at {start}: (samples, size) = ({samples}, {size}), the record words are ({a}, {b})")]
    LoopStub {
        start: u64,
        samples: u32,
        size: u32,
        a: u32,
        b: u32,
    },
    #[error("stream start {start} is not 0x40-aligned")]
    Alignment { start: u64 },
}

/// The platform a bank was authored for (header byte 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BankPlatform {
    /// `0x0a`: 8-byte pointer slots, tree at 0x78, EALayer3 v1 payloads.
    Ps3,
    /// `0x05`: 4-byte pointer slots, tree at 0x5c, EA-XMA payloads.
    Xbox360,
}

impl BankPlatform {
    pub fn label(self) -> &'static str {
        match self {
            BankPlatform::Ps3 => "PS3",
            BankPlatform::Xbox360 => "Xbox 360",
        }
    }
}

/// One ram segment: the declared samples and the codec bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RamSegment {
    pub samples: u32,
    pub bytes: Vec<u8>,
}

/// What a record carries beyond its SNR header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordBody {
    /// Codec bytes inline in the bank.
    Ram {
        loop_start: Option<u32>,
        segments: Vec<RamSegment>,
    },
    /// The audio is in the sibling `.ast`; a loop record carries the stub words `(a, b)`.
    Stream { loop_words: Option<(u32, u32)> },
}

/// One `S10A` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub index: u16,
    pub at: usize,
    pub end: usize,
    pub header: SnrHeader,
    pub body: RecordBody,
}

impl Record {
    pub fn is_stream(&self) -> bool {
        matches!(self.body, RecordBody::Stream { .. })
    }
    pub fn is_loop(&self) -> bool {
        self.header.looped
    }
    /// "ram" / "ram-loop" / "stream" / "stream-loop".
    pub fn kind(&self) -> &'static str {
        match (&self.body, self.header.looped) {
            (RecordBody::Ram { .. }, false) => "ram",
            (RecordBody::Ram { .. }, true) => "ram-loop",
            (RecordBody::Stream { .. }, false) => "stream",
            (RecordBody::Stream { .. }, true) => "stream-loop",
        }
    }
}

/// One sample-list entry (R89): the record it names, its pick weight, the unknown words carried as
/// read, and the `.ast` start (`None` for ram records, whose word is `0xFFFFFFFF`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEntry {
    pub index: u16,
    pub weight: u8,
    pub x: u16,
    pub s: u32,
    pub stream_offset: Option<u32>,
}

/// A sample list and the named node(s) it is reached from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleList {
    pub at: u32,
    /// The pointer slot(s) that reach this list.
    pub slots: Vec<u32>,
    /// Names of the nearest named node at or before each slot (R89 "owner"); several when the node
    /// carries several names or several slots reach the list.
    pub owners: Vec<String>,
    pub owner_class: Option<u8>,
    pub entries: Vec<ListEntry>,
    pub empty_slots: usize,
}

/// One name-directory entry (tail section C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedNode {
    pub node: u32,
    pub name: String,
    /// `u16 bank key` (high) — an authoring-side identity, interpretation open (R94).
    pub bank_key: u16,
    /// `u16 name key` (low) — constant per name, algorithm unidentified (R94).
    pub name_key: u16,
    /// Class byte: 0 `_snd`, 1 `h_`, 2 `_msg`.
    pub class: u8,
}

/// A parsed bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbkBank {
    pub platform: BankPlatform,
    pub file_size: u32,
    pub tree_start: u32,
    pub table_offset: u32,
    pub table_data_size: u32,
    /// The sibling `.ast` id (word 0x2c); 0 when the bank has no stream side.
    pub ast_id: u32,
    pub handler_nodes: u16,
    pub records: Vec<Option<Record>>,
    pub names: Vec<NamedNode>,
    pub pointers: Vec<(u32, u32)>,
    pub lists: Vec<SampleList>,
}

impl AbkBank {
    pub fn record(&self, index: u16) -> Option<&Record> {
        self.records.get(index as usize).and_then(|r| r.as_ref())
    }
    pub fn record_count(&self) -> usize {
        self.records.iter().filter(|r| r.is_some()).count()
    }
    pub fn stream_records(&self) -> usize {
        self.records
            .iter()
            .flatten()
            .filter(|r| r.is_stream())
            .count()
    }
    pub fn ram_records(&self) -> usize {
        self.records
            .iter()
            .flatten()
            .filter(|r| !r.is_stream())
            .count()
    }
    pub fn distinct_names(&self) -> usize {
        let mut s: Vec<&str> = self.names.iter().map(|n| n.name.as_str()).collect();
        s.sort_unstable();
        s.dedup();
        s.len()
    }
    /// The list(s) whose entries name `index`.
    pub fn lists_naming(&self, index: u16) -> Vec<&SampleList> {
        self.lists
            .iter()
            .filter(|l| l.entries.iter().any(|e| e.index == index))
            .collect()
    }
}

/// Parse a whole bank (R82 records + R89 / R94 tree). Refuses on the first contract violation.
pub fn parse_bank(b: &[u8]) -> Result<AbkBank, AbkRefusal> {
    if b.len() < 0x40 {
        return Err(AbkRefusal::Truncated(b.len()));
    }
    if &b[0..4] != b"ABKC" {
        return Err(AbkRefusal::Magic([b[0], b[1], b[2], b[3]]));
    }
    let platform = match b[8] {
        0x0a => BankPlatform::Ps3,
        0x05 => BankPlatform::Xbox360,
        other => return Err(AbkRefusal::Platform(other)),
    };
    let handler_nodes = be16(b, 0x0a).unwrap_or(0);
    let file_size = be32(b, 0x14).unwrap();
    if file_size as usize != b.len() {
        return Err(AbkRefusal::FileSize {
            declared: file_size,
            actual: b.len(),
        });
    }
    let table_offset = be32(b, 0x18).unwrap();
    let tree_start = be32(b, 0x1c).unwrap();
    let table_data_size = be32(b, 0x24).unwrap();
    let ast_id = be32(b, 0x2c).unwrap();
    let (a, bsec, c) = (
        be32(b, 0x30).unwrap(),
        be32(b, 0x34).unwrap(),
        be32(b, 0x38).unwrap(),
    );
    let data_end = table_offset.saturating_add(table_data_size);
    let ordered = tree_start >= 0x40
        && tree_start <= table_offset
        && data_end <= a
        && a <= bsec
        && bsec <= c
        && (c as usize) + 4 <= b.len();
    if !ordered {
        return Err(AbkRefusal::Sections {
            tree: tree_start,
            table: table_offset,
            data_end,
            a,
            b: bsec,
            c,
            size: b.len(),
        });
    }

    // ---- S10A table + records (R82) ----
    let t = table_offset as usize;
    if b.get(t..t + 4) != Some(b"S10A") {
        return Err(AbkRefusal::TableMagic(table_offset));
    }
    let count = be32(b, t + 8).ok_or(AbkRefusal::Truncated(b.len()))? as usize;
    let header_len = be32(b, t + 12).ok_or(AbkRefusal::Truncated(b.len()))?;
    let expected_header = if count == 0 {
        0
    } else {
        16 + 4 * (count as u32 - 1)
    };
    if header_len != expected_header {
        return Err(AbkRefusal::TableHeader {
            found: header_len,
            expected: expected_header,
        });
    }
    let mut starts: Vec<Option<usize>> = Vec::with_capacity(count);
    if count > 0 {
        starts.push(Some(t + header_len as usize));
        for i in 1..count {
            let off = be32(b, t + 16 + 4 * (i - 1)).ok_or(AbkRefusal::Truncated(b.len()))?;
            starts.push(if off == 0xFFFF_FFFF {
                None
            } else {
                Some(t + off as usize)
            });
        }
    }
    let data_end_usize = data_end as usize;
    let mut records: Vec<Option<Record>> = Vec::with_capacity(count);
    for (i, start) in starts.iter().enumerate() {
        let Some(at) = *start else {
            records.push(None);
            continue;
        };
        let hb = b.get(at..at + 8).ok_or(AbkRefusal::Record {
            index: i,
            at,
            reason: "SNR header past the file end".into(),
        })?;
        let header = SnrHeader::parse(hb).ok_or(AbkRefusal::Record {
            index: i,
            at,
            reason: "SNR header unreadable".into(),
        })?;
        let mut p = at + 8;
        let body = if header.stream_type == 1 {
            let loop_words = if header.looped {
                let a = be32(b, p).ok_or(AbkRefusal::Record {
                    index: i,
                    at,
                    reason: "loop word a missing".into(),
                })?;
                let bb = be32(b, p + 4).ok_or(AbkRefusal::Record {
                    index: i,
                    at,
                    reason: "loop word b missing".into(),
                })?;
                p += 8;
                Some((a, bb))
            } else {
                None
            };
            RecordBody::Stream { loop_words }
        } else if header.stream_type == 0 {
            let loop_start = if header.looped {
                let ls = be32(b, p).ok_or(AbkRefusal::Record {
                    index: i,
                    at,
                    reason: "loop_start missing".into(),
                })?;
                p += 4;
                Some(ls)
            } else {
                None
            };
            let mut segments = Vec::new();
            let mut sum: u32 = 0;
            loop {
                let size = be32(b, p).ok_or(AbkRefusal::Record {
                    index: i,
                    at,
                    reason: "segment size missing".into(),
                })?;
                let samples = be32(b, p + 4).ok_or(AbkRefusal::Record {
                    index: i,
                    at,
                    reason: "segment samples missing".into(),
                })?;
                if size < 8 {
                    return Err(AbkRefusal::Record {
                        index: i,
                        at,
                        reason: format!("segment size {size} < 8"),
                    });
                }
                let end = p + size as usize;
                if end > data_end_usize {
                    return Err(AbkRefusal::RecordOverrun {
                        index: i,
                        end,
                        data_end: data_end_usize,
                    });
                }
                segments.push(RamSegment {
                    samples,
                    bytes: b[p + 8..end].to_vec(),
                });
                sum = sum.saturating_add(samples);
                p = end;
                if sum >= header.num_samples || segments.len() == 2 || !header.looped {
                    break;
                }
            }
            if sum != header.num_samples {
                return Err(AbkRefusal::SegmentSum {
                    index: i,
                    sum,
                    num_samples: header.num_samples,
                });
            }
            RecordBody::Ram {
                loop_start,
                segments,
            }
        } else {
            return Err(AbkRefusal::Record {
                index: i,
                at,
                reason: format!(
                    "stream type {} is neither ram (0) nor stream (1)",
                    header.stream_type
                ),
            });
        };
        if p > data_end_usize {
            return Err(AbkRefusal::RecordOverrun {
                index: i,
                end: p,
                data_end: data_end_usize,
            });
        }
        records.push(Some(Record {
            index: i as u16,
            at,
            end: p,
            header,
            body,
        }));
    }

    // ---- tail C: name directory (R89 / R94) ----
    let mut names = Vec::new();
    let c_count = be32(b, c as usize).unwrap() as usize;
    for i in 0..c_count {
        let e = c as usize + 4 + 12 * i;
        let (node, name_off, flags) = match (be32(b, e), be32(b, e + 4), be32(b, e + 8)) {
            (Some(n), Some(m), Some(f)) => (n, m, f),
            _ => {
                return Err(AbkRefusal::NameOutside {
                    index: i,
                    node: 0,
                    name: 0,
                })
            }
        };
        if node < tree_start || node >= table_offset || (name_off as usize) + 4 >= b.len() {
            return Err(AbkRefusal::NameOutside {
                index: i,
                node,
                name: name_off,
            });
        }
        let keys = be32(b, name_off as usize).unwrap();
        let s = &b[name_off as usize + 4..];
        let end = s.iter().position(|&x| x == 0).unwrap_or(s.len());
        names.push(NamedNode {
            node,
            name: String::from_utf8_lossy(&s[..end]).into_owned(),
            bank_key: (keys >> 16) as u16,
            name_key: (keys & 0xffff) as u16,
            // Byte 0 of the big-endian flags word is its high byte (R89: 0 `_snd` / 1 `h_` / 2 `_msg`).
            class: (flags >> 24) as u8,
        });
    }
    let mut nodes_sorted: Vec<(u32, usize)> =
        names.iter().enumerate().map(|(i, n)| (n.node, i)).collect();
    nodes_sorted.sort_unstable();

    // ---- tail B: pointer-fixup table → sample lists (R89) ----
    let mut pointers = Vec::new();
    let b_count = be32(b, bsec as usize).unwrap() as usize;
    let slot_len = match platform {
        BankPlatform::Ps3 => 4,
        BankPlatform::Xbox360 => 0,
    };
    let mut lists: BTreeMap<u32, SampleList> = BTreeMap::new();
    for i in 0..b_count {
        let loc = be32(b, bsec as usize + 4 + 4 * i).ok_or(AbkRefusal::Truncated(b.len()))?;
        let ptr = be32(b, loc as usize + slot_len).ok_or(AbkRefusal::PointerOutsideTree {
            slot: loc,
            ptr: 0,
            tree: tree_start,
            table: table_offset,
        })?;
        if ptr < tree_start || ptr >= table_offset {
            return Err(AbkRefusal::PointerOutsideTree {
                slot: loc,
                ptr,
                tree: tree_start,
                table: table_offset,
            });
        }
        pointers.push((loc, ptr));
        if b[ptr as usize] != 0 {
            continue; // node classes 1 / 2 / 4 — not decoded (R89)
        }
        let n = be32(b, ptr as usize).unwrap();
        if n == 0 {
            continue; // an empty list (record-less banks such as `100_goal.abk`) — R89 counts none
        }
        let list_end = ptr as usize + 4 + 12 * n as usize;
        if list_end > table_offset as usize {
            return Err(AbkRefusal::ListOverrun {
                list: ptr,
                count: n,
            });
        }
        // Owner: the nearest named node at or before the slot (the list is reached from inside that
        // node's extent — R89 §1); every name attached to that node counts.
        let owner_node = nodes_sorted
            .iter()
            .rev()
            .find(|(off, _)| *off <= loc)
            .map(|(off, _)| *off);
        let owners: Vec<String> = owner_node
            .map(|off| {
                names
                    .iter()
                    .filter(|nm| nm.node == off)
                    .map(|nm| nm.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let owner_class =
            owner_node.and_then(|off| names.iter().find(|nm| nm.node == off).map(|nm| nm.class));
        let list = lists.entry(ptr).or_insert_with(|| SampleList {
            at: ptr,
            slots: Vec::new(),
            owners: Vec::new(),
            owner_class,
            entries: Vec::new(),
            empty_slots: 0,
        });
        list.slots.push(loc);
        for o in owners {
            if !list.owners.contains(&o) {
                list.owners.push(o);
            }
        }
        if !list.entries.is_empty() || list.empty_slots > 0 {
            continue; // the same list reached from another slot: entries already read
        }
        for k in 0..n as usize {
            let e = ptr as usize + 4 + 12 * k;
            let idx = be16(b, e).unwrap();
            if idx == 0xFFFF {
                list.empty_slots += 1;
                continue;
            }
            if idx as usize >= count {
                return Err(AbkRefusal::IndexOutOfRange {
                    list: ptr,
                    entry: k,
                    index: idx,
                    count,
                });
            }
            let weight = b[e + 2];
            let x = be16(b, e + 3).unwrap();
            let s = be24(b, e + 5).unwrap();
            let o = be32(b, e + 8).unwrap();
            list.entries.push(ListEntry {
                index: idx,
                weight,
                x,
                s,
                stream_offset: (o != 0xFFFF_FFFF).then_some(o),
            });
        }
    }
    let lists: Vec<SampleList> = lists.into_values().collect();

    // ---- T1: every present record referenced exactly once ----
    let mut refs: HashMap<u16, usize> = HashMap::new();
    for l in &lists {
        for e in &l.entries {
            *refs.entry(e.index).or_default() += 1;
        }
    }
    if let Some((idx, times)) = refs.iter().find(|(_, t)| **t > 1) {
        return Err(AbkRefusal::ReferencedTwice {
            index: *idx,
            times: *times,
        });
    }
    let unreferenced: Vec<u16> = records
        .iter()
        .flatten()
        .filter(|r| !refs.contains_key(&r.index))
        .map(|r| r.index)
        .collect();
    if !unreferenced.is_empty() {
        return Err(AbkRefusal::Unreferenced {
            count: unreferenced.len(),
            examples: unreferenced.iter().take(8).copied().collect(),
        });
    }

    Ok(AbkBank {
        platform,
        file_size,
        tree_start,
        table_offset,
        table_data_size,
        ast_id,
        handler_nodes,
        records,
        names,
        pointers,
        lists,
    })
}

/// One `.ast` block as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstBlock {
    pub at: u64,
    pub terminal: bool,
    pub size: u32,
    pub samples: u32,
    pub payload: Vec<u8>,
}

/// One framed stream: its blocks, byte extent and sample sum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstStream {
    pub start: u64,
    pub end: u64,
    pub blocks: Vec<AstBlock>,
    pub samples: u64,
}

/// The `.ast` id word (offset 4) after the `S10S` magic.
pub fn ast_id(b: &[u8]) -> Result<u32, AstRefusal> {
    if b.len() < 8 || &b[0..4] != b"S10S" {
        let mut m = [0u8; 4];
        m.copy_from_slice(
            &b[..4.min(b.len())]
                .iter()
                .chain([0u8; 4].iter())
                .take(4)
                .copied()
                .collect::<Vec<u8>>(),
        );
        return Err(AstRefusal::Magic(m));
    }
    Ok(be32(b, 4).unwrap())
}

/// Walk one stream from `start` to its terminal block (R91 §1). `b` may be the whole file or a
/// window starting at file offset `base`; `start` is file-absolute.
pub fn walk_stream(b: &[u8], base: u64, start: u64) -> Result<AstStream, AstRefusal> {
    if start % 0x40 != 0 {
        return Err(AstRefusal::Alignment { start });
    }
    let len = base + b.len() as u64;
    let mut at = start;
    let mut blocks = Vec::new();
    let mut samples: u64 = 0;
    loop {
        let rel = (at - base) as usize;
        let Some(h) = b.get(rel..rel + 8) else {
            return Err(AstRefusal::NoTerminal { start });
        };
        let flag = h[0];
        if flag != 0x00 && flag != 0x80 {
            return Err(AstRefusal::Flag {
                start,
                block: blocks.len(),
                at,
                flag,
            });
        }
        let size = u32::from_be_bytes([0, h[1], h[2], h[3]]);
        let declared = u32::from_be_bytes([h[4], h[5], h[6], h[7]]);
        if size < 8 || at + size as u64 > len {
            return Err(AstRefusal::BlockSize {
                start,
                block: blocks.len(),
                at,
                size,
                len,
            });
        }
        let payload = b[rel + 8..rel + size as usize].to_vec();
        samples += declared as u64;
        let terminal = flag == 0x80;
        blocks.push(AstBlock {
            at,
            terminal,
            size,
            samples: declared,
            payload,
        });
        at += size as u64;
        if terminal {
            break;
        }
    }
    Ok(AstStream {
        start,
        end: at,
        blocks,
        samples,
    })
}

/// [`walk_stream`] over a seekable reader, block by block, bounded to one member: the stream file
/// occupies `[base, base + len)` of the reader (a loose file: `base` 0, `len` its size; a stored
/// archive member: its offset and size), and `start` is relative to the stream file. No block may
/// cross the member's end — the wrapper boundary is a refusal, not a read into the next member.
pub fn walk_stream_in<R: std::io::Read + std::io::Seek>(
    r: &mut R,
    base: u64,
    len: u64,
    start: u64,
) -> Result<AstStream, AstRefusal> {
    use std::io::SeekFrom;
    if start % 0x40 != 0 {
        return Err(AstRefusal::Alignment { start });
    }
    let mut at = start;
    let mut blocks = Vec::new();
    let mut samples: u64 = 0;
    loop {
        if at + 8 > len {
            return Err(AstRefusal::NoTerminal { start });
        }
        let mut h = [0u8; 8];
        if r.seek(SeekFrom::Start(base + at)).is_err() || r.read_exact(&mut h).is_err() {
            return Err(AstRefusal::NoTerminal { start });
        }
        let flag = h[0];
        if flag != 0x00 && flag != 0x80 {
            return Err(AstRefusal::Flag { start, block: blocks.len(), at, flag });
        }
        let size = u32::from_be_bytes([0, h[1], h[2], h[3]]);
        let declared = u32::from_be_bytes([h[4], h[5], h[6], h[7]]);
        if size < 8 || at + size as u64 > len {
            return Err(AstRefusal::BlockSize { start, block: blocks.len(), at, size, len });
        }
        let mut payload = vec![0u8; size as usize - 8];
        if r.read_exact(&mut payload).is_err() {
            return Err(AstRefusal::BlockSize { start, block: blocks.len(), at, size, len });
        }
        samples += declared as u64;
        let terminal = flag == 0x80;
        blocks.push(AstBlock { at, terminal, size, samples: declared, payload });
        at += size as u64;
        if terminal {
            break;
        }
    }
    Ok(AstStream { start, end: at, blocks, samples })
}

/// The number of interleaved sub-streams a channel configuration word names (R82 / R91):
/// 0 mono and 1 stereo = one; 3 = two stereo streams; 5 = three. `None` for an unmeasured value.
pub fn sub_streams(channel_config: u8) -> Option<usize> {
    match channel_config {
        0 | 1 => Some(1),
        3 => Some(2),
        5 => Some(3),
        _ => None,
    }
}

/// The R92 candidate construction of an EALayer3 v1 stream or ram record with **one** sub-stream
/// (channel config 0 / 1): the 47-sample PCM frame (planar for stereo) followed by the decoder's
/// output from index 1 152, trimmed to `num_samples` frames. The label a preview must carry is
/// [`EAL3_AST_LABEL`].
pub struct Eal3Diagnostic {
    pub sample_rate: u32,
    pub channels: u32,
    pub granules: usize,
    pub rebuilt_frames: usize,
    pub decoded_frames_per_channel: usize,
    /// Interleaved i16, exactly `num_samples × channels`.
    pub pcm: Vec<i16>,
}

pub const EAL3_AST_LABEL: &str = "diagnostic preview — R92 candidate construction (PCM frame + decoder output from sample 1 152, trimmed to the declared count); not game-verified";

/// The terminal block of a PS3 stream ends in zero padding to the next 0x40 boundary (R91 §1);
/// an EA frame cannot start with a zero common byte, so the padding is the shortest zero tail
/// whose removal leaves a payload the granule grammar walks to its exact end. Returns the payload
/// unchanged when it has no zero tail (ram segments, interior blocks).
pub fn trim_terminal_padding(pl: &[u8]) -> &[u8] {
    let zeros = pl.iter().rev().take_while(|&&x| x == 0).count();
    if zeros == 0 {
        return pl;
    }
    for keep in (pl.len() - zeros)..=pl.len() {
        if let Ok(gr) = crate::ealayer3::parse_block(&pl[..keep], 0) {
            if gr.last().map(|g| g.end) == Some(keep) {
                return &pl[..keep];
            }
        }
    }
    pl
}

pub fn eal3_diagnostic(
    blocks: &[(u32, &[u8])],
    num_samples: u32,
) -> Result<Eal3Diagnostic, String> {
    // The last block's zero padding (PS3 `.ast` terminal blocks) is not frame data.
    let trimmed: Vec<(u32, &[u8])> = blocks
        .iter()
        .enumerate()
        .map(|(i, (n, pl))| {
            (
                *n,
                if i + 1 == blocks.len() {
                    trim_terminal_padding(pl)
                } else {
                    *pl
                },
            )
        })
        .collect();
    let blocks = trimmed.as_slice();
    let parsed = crate::ealayer3::contract_parse(blocks, num_samples)
        .map_err(|e| format!("EALayer3 v1 grammar refused: {e}"))?;
    let rebuilt =
        crate::ealayer3::rebuild(&parsed.granules).map_err(|e| format!("rebuild refused: {e}"))?;
    let decoded =
        crate::decode_layer3(&rebuilt.mpeg).map_err(|e| format!("decoder refused: {e}"))?;
    let ch = decoded.channels as usize;
    let pcm_frame = &parsed.pcm.samples;
    let n = parsed.pcm.pcm_samples as usize;
    // Planar PCM frame (all left, then all right — R91 §6) → interleaved.
    let mut out: Vec<i16> = Vec::with_capacity(num_samples as usize * ch);
    if pcm_frame.len() >= n * ch {
        for i in 0..n {
            for c in 0..ch {
                out.push(pcm_frame[c * n + i]);
            }
        }
    }
    let from = 1152 * ch;
    if decoded.pcm.len() > from {
        out.extend_from_slice(&decoded.pcm[from..]);
    }
    out.truncate(num_samples as usize * ch);
    Ok(Eal3Diagnostic {
        sample_rate: decoded.sample_rate,
        channels: decoded.channels,
        granules: parsed.granules.len(),
        rebuilt_frames: rebuilt.frames,
        decoded_frames_per_channel: decoded.pcm.len() / ch.max(1),
        pcm: out,
    })
}

/// The label of a multi-part construction (26 Sep 2026): loop stub + body, and / or one sub-stream
/// of a channel-config 3 / 5 entry. Both rules are the research's (R91 §1 loops; R91 / R92 inferred
/// de-interleave), neither is reference-validated.
pub const EAL3_PART_LABEL: &str = "diagnostic preview — R92 candidate construction per part (loop: the stub's PCM frame, then the body; channel config 3 / 5: one stereo sub-stream by the inferred de-interleave, EA frame i → sub-stream i mod N); not game-verified, de-interleave not reference-validated";

/// Split each block of a channel-config 3 / 5 stream into its `n` sub-streams: EA frame (granule
/// record) `i` of a block goes to sub-stream `i mod n` (R91 §1, inferred); the block's sample word
/// is per sub-stream. The last block's zero padding is trimmed first. Refuses a block whose frame
/// count is not a multiple of `n`.
pub fn split_sub_streams(blocks: &[(u32, &[u8])], n: usize) -> Result<Vec<Vec<(u32, Vec<u8>)>>, String> {
    if n == 0 {
        return Err("zero sub-streams".into());
    }
    let mut out: Vec<Vec<(u32, Vec<u8>)>> = vec![Vec::with_capacity(blocks.len()); n];
    for (bi, (declared, pl)) in blocks.iter().enumerate() {
        let pl = if bi + 1 == blocks.len() { trim_terminal_padding(pl) } else { *pl };
        let gr = crate::ealayer3::parse_block(pl, bi).map_err(|e| format!("block {bi}: {e}"))?;
        if gr.len() % n != 0 {
            return Err(format!("block {bi}: {} EA frames is not a multiple of the {n} sub-streams", gr.len()));
        }
        if gr.last().map(|g| g.end) != Some(pl.len()) {
            return Err(format!("block {bi}: EA frames do not tile the payload"));
        }
        let mut parts: Vec<Vec<u8>> = vec![Vec::new(); n];
        for (i, g) in gr.iter().enumerate() {
            parts[i % n].extend_from_slice(&pl[g.at..g.end]);
        }
        for (k, part) in parts.into_iter().enumerate() {
            out[k].push((*declared, part));
        }
    }
    Ok(out)
}

/// A loop stub's output: its PCM frame only (R91 §1 / R96 V6: the stub's granules are warm-up and
/// flush). Exactly one PCM frame whose sample count equals the record's word `a`; planar → interleaved.
pub fn stub_pcm(stub: &[u8], a: u32) -> Result<(u32, Vec<i16>), String> {
    let stub = trim_terminal_padding(stub);
    let gr = crate::ealayer3::parse_block(stub, 0).map_err(|e| format!("loop stub: {e}"))?;
    let frames: Vec<&crate::ealayer3::Granule> = gr.iter().filter(|g| g.pcm.is_some()).collect();
    let [g] = frames.as_slice() else {
        return Err(format!("loop stub: {} PCM frames (one expected)", frames.len()));
    };
    let p = g.pcm.as_ref().unwrap();
    if p.pcm_samples as u32 != a {
        return Err(format!("loop stub: PCM frame holds {} samples, the record's word a is {a}", p.pcm_samples));
    }
    let ch = g.nch();
    let n = p.pcm_samples as usize;
    if p.samples.len() < n * ch {
        return Err("loop stub: PCM frame shorter than declared".into());
    }
    let mut out = Vec::with_capacity(n * ch);
    for i in 0..n {
        for c in 0..ch {
            out.push(p.samples[c * n + i]);
        }
    }
    Ok((ch as u32, out))
}

/// The loop construction of one (sub-)stream: the stub's PCM (`a` samples), then the body's R92
/// construction trimmed to `body_samples`. Played once — the loop point is `a` (R82 / R91), not
/// repeated here.
pub fn eal3_loop_diagnostic(stub: &[u8], a: u32, body: &[(u32, &[u8])], body_samples: u32) -> Result<Eal3Diagnostic, String> {
    let (ch, mut pcm) = stub_pcm(stub, a)?;
    let d = eal3_diagnostic(body, body_samples)?;
    if d.channels != ch {
        return Err(format!("loop: stub has {ch} channel(s), body {}", d.channels));
    }
    pcm.extend_from_slice(&d.pcm);
    Ok(Eal3Diagnostic { pcm, ..d })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w32(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_be_bytes());
    }
    fn w16(v: &mut Vec<u8>, x: u16) {
        v.extend_from_slice(&x.to_be_bytes());
    }

    /// A tiny PS3-shaped bank: one named `_msg` node owning a pointer slot that reaches one sample
    /// list with a stream record (index 0) and a ram record (index 1); one absent slot (index 2).
    fn synthetic_bank(platform: u8, ast_id: u32, tamper: &str) -> Vec<u8> {
        let ps3 = platform == 0x0a;
        let tree_start: u32 = if ps3 { 0x78 } else { 0x5c };
        let mut b = vec![0u8; tree_start as usize];
        b[0..4].copy_from_slice(b"ABKC");
        b[4..8].copy_from_slice(&0x01010300u32.to_be_bytes());
        b[8] = platform;
        b[0x0a..0x0c].copy_from_slice(&1u16.to_be_bytes());
        // tree region: u32 + zeros, one handler node (0x68 bytes) then the node graph
        w32(&mut b, 0x2a1);
        w32(&mut b, 0);
        b.extend(std::iter::repeat(0u8).take(0x68)); // handler node
        let node_off = b.len() as u32; // the named _msg node
        b.extend_from_slice(&[0x02, 0, 0, 0, 0, 0, 0, 0]);
        let slot_loc = b.len() as u32;
        let ptr_pos = b.len() + if ps3 { 4 } else { 0 };
        if ps3 {
            w32(&mut b, 0);
        }
        w32(&mut b, 0); // pointer placeholder
        let list_off = b.len() as u32;
        let mut ptr = list_off;
        if tamper == "pointer-outside" {
            ptr = 0xFFFF_0000;
        }
        b[ptr_pos..ptr_pos + 4].copy_from_slice(&ptr.to_be_bytes());
        // sample list: 3 entries (record 0 stream @0x40, record 1 ram, one empty slot)
        let n_entries: u32 = if tamper == "duplicate" { 3 } else { 3 };
        w32(&mut b, n_entries);
        let mut entry = |b: &mut Vec<u8>, idx: u16, weight: u8, o: u32| {
            w16(b, idx);
            b.push(weight);
            w16(b, 0xdf1f);
            b.extend_from_slice(&[0, 0, 4]);
            w32(b, o);
        };
        entry(&mut b, 0, 100, 0x40);
        entry(
            &mut b,
            if tamper == "duplicate" { 0 } else { 1 },
            90,
            0xFFFF_FFFF,
        );
        if tamper == "index-range" {
            entry(&mut b, 7, 80, 0xFFFF_FFFF);
        } else {
            entry(&mut b, 0xFFFF, 0, 0);
        }
        while b.len() % 4 != 0 {
            b.push(0);
        }
        // S10A table: 3 slots, the third absent
        let table_off = b.len() as u32;
        b.extend_from_slice(b"S10A");
        w32(&mut b, 0);
        w32(&mut b, 3);
        w32(&mut b, 16 + 4 * 2);
        let rec1_slot = b.len();
        w32(&mut b, 0);
        w32(&mut b, 0xFFFF_FFFF);
        // record 0: stream, no loop: SNR h1 (version 1, codec 5, cc 0, 22050 Hz) h2 (type 1, 100 samples)
        let h1: u32 = (1 << 28) | (5 << 24) | (0 << 18) | 22_050;
        w32(&mut b, h1);
        w32(&mut b, (1 << 30) | 100);
        // record 1: ram, no loop, 2 codec bytes, 10 samples
        let rec1_off = b.len() as u32 - table_off;
        b[rec1_slot..rec1_slot + 4].copy_from_slice(&rec1_off.to_be_bytes());
        w32(&mut b, h1);
        w32(&mut b, 10);
        w32(&mut b, 10);
        w32(&mut b, 10);
        b.extend_from_slice(&[0xAA, 0xBB]);
        while b.len() % 4 != 0 {
            b.push(0);
        }
        let data_size = b.len() as u32 - table_off;
        // tail A (0), B (1 slot), C (1 name)
        let a = b.len() as u32;
        w32(&mut b, 0);
        let bs = b.len() as u32;
        w32(&mut b, 1);
        w32(&mut b, slot_loc);
        let c = b.len() as u32;
        let name_off = c + 4 + 12;
        w32(&mut b, 1);
        w32(&mut b, node_off);
        w32(&mut b, name_off);
        w32(&mut b, 0x0200_0000);
        w32(&mut b, 0x2266_0cd6);
        b.extend_from_slice(b"frolundaSamplePicker_msg\0\0\0\0");
        let size = b.len() as u32;
        b[0x14..0x18].copy_from_slice(&size.to_be_bytes());
        b[0x18..0x1c].copy_from_slice(&table_off.to_be_bytes());
        b[0x1c..0x20].copy_from_slice(&tree_start.to_be_bytes());
        b[0x20..0x24].copy_from_slice(&table_off.to_be_bytes());
        b[0x24..0x28].copy_from_slice(&data_size.to_be_bytes());
        b[0x2c..0x30].copy_from_slice(&ast_id.to_be_bytes());
        b[0x30..0x34].copy_from_slice(&a.to_be_bytes());
        b[0x34..0x38].copy_from_slice(&bs.to_be_bytes());
        b[0x38..0x3c].copy_from_slice(&c.to_be_bytes());
        b
    }

    fn synthetic_ast(id: u32, flag: u8) -> Vec<u8> {
        let mut a = vec![0u8; 0x40];
        a[0..4].copy_from_slice(b"S10S");
        a[4..8].copy_from_slice(&id.to_be_bytes());
        // one stream of two blocks: 60 + 40 samples
        a.push(0x00);
        a.extend_from_slice(&[0, 0, 8 + 6]);
        w32(&mut a, 60);
        a.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
        a.push(flag);
        a.extend_from_slice(&[0, 0, 8 + 2]);
        w32(&mut a, 40);
        a.extend_from_slice(&[7, 8]);
        while a.len() % 0x40 != 0 {
            a.push(0);
        }
        a
    }

    #[test]
    fn a_synthetic_bank_parses_on_both_platforms() {
        for (code, plat) in [(0x0au8, BankPlatform::Ps3), (0x05, BankPlatform::Xbox360)] {
            let bank = parse_bank(&synthetic_bank(code, 0x460ed680, "")).unwrap();
            assert_eq!(bank.platform, plat);
            assert_eq!(bank.records.len(), 3);
            assert_eq!(bank.record_count(), 2);
            assert_eq!((bank.stream_records(), bank.ram_records()), (1, 1));
            assert_eq!(bank.ast_id, 0x460ed680);
            assert_eq!(bank.names.len(), 1);
            assert_eq!(bank.names[0].name, "frolundaSamplePicker_msg");
            assert_eq!(
                (
                    bank.names[0].bank_key,
                    bank.names[0].name_key,
                    bank.names[0].class
                ),
                (0x2266, 0x0cd6, 2)
            );
            assert_eq!(bank.lists.len(), 1);
            let l = &bank.lists[0];
            assert_eq!(l.owners, vec!["frolundaSamplePicker_msg".to_string()]);
            assert_eq!(l.entries.len(), 2);
            assert_eq!(l.empty_slots, 1);
            assert_eq!(l.entries[0].stream_offset, Some(0x40));
            assert_eq!(l.entries[1].stream_offset, None);
            assert_eq!(l.entries[0].weight, 100);
            let r1 = bank.record(1).unwrap();
            assert_eq!(r1.kind(), "ram");
            match &r1.body {
                RecordBody::Ram { segments, .. } => assert_eq!(segments[0].bytes, vec![0xAA, 0xBB]),
                _ => panic!(),
            }
            assert_eq!(bank.record(0).unwrap().kind(), "stream");
            assert!(bank.record(2).is_none());
        }
    }

    #[test]
    fn the_contract_refusals_fire() {
        assert!(matches!(
            parse_bank(&synthetic_bank(0x0a, 1, "pointer-outside")),
            Err(AbkRefusal::PointerOutsideTree { .. })
        ));
        assert!(matches!(
            parse_bank(&synthetic_bank(0x0a, 1, "duplicate")),
            Err(AbkRefusal::ReferencedTwice { index: 0, times: 2 })
                | Err(AbkRefusal::Unreferenced { .. })
        ));
        assert!(matches!(
            parse_bank(&synthetic_bank(0x0a, 1, "index-range")),
            Err(AbkRefusal::IndexOutOfRange { index: 7, .. })
        ));
        let mut bad = synthetic_bank(0x0a, 1, "");
        bad[8] = 0x07;
        assert!(matches!(parse_bank(&bad), Err(AbkRefusal::Platform(0x07))));
        let mut short = synthetic_bank(0x0a, 1, "");
        short.pop();
        assert!(matches!(
            parse_bank(&short),
            Err(AbkRefusal::FileSize { .. })
        ));
    }

    #[test]
    fn streams_are_framed_and_the_id_is_checked() {
        let ast = synthetic_ast(0x460ed680, 0x80);
        assert_eq!(ast_id(&ast).unwrap(), 0x460ed680);
        let s = walk_stream(&ast, 0, 0x40).unwrap();
        assert_eq!(s.blocks.len(), 2);
        assert_eq!(s.samples, 100);
        assert!(s.blocks[1].terminal && !s.blocks[0].terminal);
        assert_eq!(s.blocks[0].payload, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(s.end, 0x40 + 14 + 10);
        // A window that starts at the stream works the same with a base.
        let s2 = walk_stream(&ast[0x40..], 0x40, 0x40).unwrap();
        assert_eq!(s2.samples, 100);
        // No terminal flag: the walk runs into the zero padding (size 0 < 8) or off the file end.
        assert!(matches!(
            walk_stream(&synthetic_ast(1, 0x00), 0, 0x40),
            Err(AstRefusal::NoTerminal { .. } | AstRefusal::BlockSize { .. })
        ));
        assert!(matches!(
            walk_stream(&synthetic_ast(1, 0x40), 0, 0x40),
            Err(AstRefusal::Flag { flag: 0x40, .. })
        ));
        assert!(matches!(
            walk_stream(&ast, 0, 0x44),
            Err(AstRefusal::Alignment { .. })
        ));
        assert!(matches!(ast_id(b"S10A\0\0\0\0"), Err(AstRefusal::Magic(_))));
        // The reader walk agrees with the slice walk, also inside a larger container at an offset,
        // and refuses a stream whose blocks would cross the member's end.
        let mut container = vec![0xAAu8; 0x100];
        container.extend_from_slice(&ast);
        container.extend(std::iter::repeat(0xBBu8).take(64));
        let mut cur = std::io::Cursor::new(&container);
        let r = walk_stream_in(&mut cur, 0x100, ast.len() as u64, 0x40).unwrap();
        assert_eq!((r.samples, r.blocks.len(), r.end), (s.samples, s.blocks.len(), s.end));
        assert_eq!(r.blocks[0].payload, s.blocks[0].payload);
        let mut cur = std::io::Cursor::new(&container);
        assert!(matches!(walk_stream_in(&mut cur, 0x100, 0x40 + 22, 0x40), Err(AstRefusal::BlockSize { .. })));
        let mut cur = std::io::Cursor::new(&container);
        assert!(matches!(walk_stream_in(&mut cur, 0x100, 0x40 + 20, 0x40), Err(AstRefusal::NoTerminal { .. })));
        assert_eq!(sub_streams(0), Some(1));
        assert_eq!(sub_streams(3), Some(2));
        assert_eq!(sub_streams(7), None);
    }
}
