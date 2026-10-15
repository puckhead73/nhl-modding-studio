//! Span-preserving member replacement inside a small BIGF / BIG4 **package** (the `l{n}.big`
//! league-logo packages, portrait packages, …) — research handoff 02 (2026-09-19) §2 / §4.2.
//!
//! EA's packages have a fixed shape the generic [`crate::ArchiveBuilder`] does not reproduce:
//! members sit on 64-byte lines, a zero-length marker may or may not occupy a line of its own
//! (the per-era "stride"), the table is followed by an opaque 8-byte trailer (`L277 15 05 00 00`,
//! `L286 15 05 00 01`, …), and on NHL 10+ the `size_le` word excludes a 16-byte tail. Rebuilding a
//! package through the builder therefore changes header words, member offsets and the trailer
//! even when nothing else changed — bytes the loader has never been shown to tolerate.
//!
//! [`replan_member`] instead keeps every byte of the source it can: everything before the
//! replaced member is copied verbatim (header, table, trailer, earlier members and their gaps);
//! the replaced member keeps its offset and takes its new length; every later member keeps its
//! *relative* placement (`align64(previous end) + delta`, `delta ∈ {0, 64}` read from the source —
//! the stride after a zero-length marker) and its own bytes and inter-member gap bytes; the tail
//! after the last member is carried as is and `size_le` keeps its relation to it. Only the table
//! words of the replaced member (size) and of the members that moved (offset), the `size_le`
//! word, the replaced payload and the alignment gap right after it change. With an identical
//! member the output is the input byte for byte.
//!
//! Anything the measured layouts do not exhibit is refused rather than guessed: an entry pad
//! variant (`BIG5`+), overlapping members, a `size_le` that is not the end of the last member,
//! a follower whose delta is neither 0 nor 64, or a non-zero gap right after the replaced member
//! that a length change would have to overwrite (research 02 F2: gap bytes are never erased).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReplanError {
    #[error("package is shorter than the 16-byte BIG header ({0} bytes)")]
    Truncated(usize),
    #[error("unrecognised package magic {0:?}")]
    BadMagic([u8; 4]),
    #[error("package magic {0:?} carries an entry pad byte; that variant's placement rules are not measured")]
    UnmeasuredVariant([u8; 4]),
    #[error("package table is truncated at entry {entry} (offset {offset})")]
    TruncatedTable { entry: usize, offset: usize },
    #[error("package data_offset {data_offset} lies before the end of the table ({table_end})")]
    DataOffsetBeforeTable { data_offset: u32, table_end: usize },
    #[error("member {index} `{name}` span {offset}+{size} lies outside {data_offset}..{len}")]
    MemberOutOfRange { index: usize, name: String, offset: u32, size: u32, data_offset: u32, len: usize },
    #[error("members `{a}` and `{b}` overlap")]
    Overlap { a: String, b: String },
    #[error("size_le {size_le} is neither the end of the last member ({last_end}) nor the package length ({len}); the header word's meaning on this package is not established")]
    SizeWordUnexplained { size_le: u32, last_end: u32, len: usize },
    #[error("member `{0}` is not in the package")]
    NoSuchMember(String),
    #[error("member `{0}` is listed more than once")]
    DuplicateMember(String),
    #[error("member `{name}` at {offset} follows `{previous}` with a residual of {delta} bytes after 64-byte alignment; only 0 and 64 are measured placements")]
    UnmeasuredPlacement { name: String, offset: u32, previous: String, delta: i64 },
    #[error("the {len} gap bytes at {start} after member `{member}` are not all zero; a length change would overwrite them, so the package is left as it is (research 02 F2)")]
    NonZeroGap { member: String, start: u32, len: usize },
    #[error("the re-planned package would be {0} bytes, beyond the 32-bit offsets of the format")]
    TooLarge(u64),
    #[error("internal verification failed after re-plan: {0}")]
    Verification(String),
}

/// One table row of a package, with where its offset/size words live in the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub index: usize,
    pub name: String,
    pub offset: u32,
    pub size: u32,
    /// Byte position of the row's 4-byte big-endian offset word (size follows at +4).
    pub row_pos: usize,
}

/// The parsed shape of a package: what [`replan_member`] preserves.
#[derive(Debug, Clone)]
pub struct PackageLayout {
    pub magic: [u8; 4],
    pub size_le: u32,
    pub data_offset: u32,
    /// End of the name table (the opaque trailer, if any, is `table_end..data_offset`).
    pub table_end: usize,
    pub members: Vec<Member>,
    /// End of the last member (`max(offset + size)`); `size_le == last_end` on every measured package.
    pub last_end: u32,
    /// Bytes after `last_end` (0 on NHL 07–09 / PS3 08, 16 zero bytes on NHL 10+).
    pub tail_len: usize,
}

impl PackageLayout {
    /// Parse the header and table; validate the span facts the re-plan relies on.
    pub fn parse(bytes: &[u8]) -> Result<Self, ReplanError> {
        if bytes.len() < 16 {
            return Err(ReplanError::Truncated(bytes.len()));
        }
        let magic: [u8; 4] = bytes[0..4].try_into().expect("len checked");
        if !crate::KNOWN_MAGICS.iter().any(|m| **m == magic) {
            return Err(ReplanError::BadMagic(magic));
        }
        if magic != *b"BIGF" && magic != *b"BIG4" {
            return Err(ReplanError::UnmeasuredVariant(magic));
        }
        let size_le = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let count = u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let data_offset = u32::from_be_bytes(bytes[12..16].try_into().unwrap());
        // Bounded before allocation: every row needs 8 bytes plus a name terminator.
        if count.saturating_mul(9) > bytes.len().saturating_sub(16) {
            return Err(ReplanError::TruncatedTable { entry: 0, offset: 16 });
        }
        let mut cursor = 16usize;
        let mut members = Vec::with_capacity(count);
        for index in 0..count {
            if cursor + 8 > bytes.len() {
                return Err(ReplanError::TruncatedTable { entry: index, offset: cursor });
            }
            let row_pos = cursor;
            let offset = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
            let size = u32::from_be_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap());
            cursor += 8;
            let name_start = cursor;
            let Some(rel) = bytes[cursor..].iter().position(|b| *b == 0) else {
                return Err(ReplanError::TruncatedTable { entry: index, offset: cursor });
            };
            let name = String::from_utf8_lossy(&bytes[name_start..name_start + rel]).into_owned();
            cursor = name_start + rel + 1;
            members.push(Member { index, name, offset, size, row_pos });
        }
        let table_end = cursor;
        if (data_offset as usize) < table_end {
            return Err(ReplanError::DataOffsetBeforeTable { data_offset, table_end });
        }
        let len = bytes.len();
        for m in &members {
            let end = m.offset as u64 + m.size as u64;
            if (m.offset as u64) < data_offset as u64 || end > len as u64 {
                return Err(ReplanError::MemberOutOfRange { index: m.index, name: m.name.clone(), offset: m.offset, size: m.size, data_offset, len });
            }
        }
        // Overlap check on the offset order (zero-length members may share an offset).
        let order = offset_order(&members);
        for w in order.windows(2) {
            let (a, b) = (&members[w[0]], &members[w[1]]);
            if a.size > 0 && b.offset < a.offset + a.size {
                return Err(ReplanError::Overlap { a: a.name.clone(), b: b.name.clone() });
            }
        }
        let last_end = members.iter().map(|m| m.offset + m.size).max().unwrap_or(data_offset);
        if size_le != last_end {
            return Err(ReplanError::SizeWordUnexplained { size_le, last_end, len });
        }
        let tail_len = len - last_end as usize;
        Ok(Self { magic, size_le, data_offset, table_end, members, last_end, tail_len })
    }

    fn member(&self, name: &str) -> Result<usize, ReplanError> {
        let mut it = self.members.iter().filter(|m| m.name == name);
        let Some(m) = it.next() else { return Err(ReplanError::NoSuchMember(name.to_string())) };
        if it.next().is_some() {
            return Err(ReplanError::DuplicateMember(name.to_string()));
        }
        Ok(m.index)
    }
}

/// Indices of `members` sorted by (offset, index) — zero-length markers precede the member that
/// starts at the same offset when they are listed first, as every measured package does.
fn offset_order(members: &[Member]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..members.len()).collect();
    order.sort_by_key(|i| (members[*i].offset, *i));
    order
}

fn align64(x: u64) -> u64 {
    x.div_ceil(64) * 64
}

/// One member whose table offset the re-plan changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    pub name: String,
    pub old_offset: u32,
    pub new_offset: u32,
}

/// The re-planned package.
#[derive(Debug, Clone)]
pub struct Replan {
    pub bytes: Vec<u8>,
    pub member: String,
    pub old_len: usize,
    pub new_len: usize,
    pub old_member_len: u32,
    pub new_member_len: u32,
    /// Members after the replaced one whose offset changed (empty when the length did not).
    pub moved: Vec<Moved>,
}

impl Replan {
    /// `true` when nothing changed (identical member bytes).
    pub fn is_identity(&self) -> bool {
        self.old_member_len == self.new_member_len && self.moved.is_empty()
    }
}

/// Replace `member` inside `package` with `new_member`, preserving the package's shape (module
/// doc). Refuses rather than guesses; the output is re-parsed and every other member's bytes are
/// checked equal before it is returned.
pub fn replan_member(package: &[u8], member: &str, new_member: &[u8]) -> Result<Replan, ReplanError> {
    let layout = PackageLayout::parse(package)?;
    let target = layout.member(member)?;
    let order = offset_order(&layout.members);
    let pos = order.iter().position(|i| *i == target).expect("member is in the order");
    let t = &layout.members[target];
    let (off_n, old_s) = (t.offset as u64, t.size as u64);
    let new_s = new_member.len() as u64;
    if new_s > u32::MAX as u64 {
        return Err(ReplanError::TooLarge(new_s));
    }

    // Placement of the followers: relative residual after 64-byte alignment, from the source.
    struct Follower {
        idx: usize,
        new_offset: u64,
    }
    let mut followers: Vec<Follower> = Vec::new();
    let mut prev_old_end = off_n + old_s;
    let mut prev_new_end = off_n + new_s;
    let mut prev_name = t.name.clone();
    for &i in &order[pos + 1..] {
        let m = &layout.members[i];
        let delta = m.offset as i64 - align64(prev_old_end) as i64;
        if delta != 0 && delta != 64 {
            return Err(ReplanError::UnmeasuredPlacement { name: m.name.clone(), offset: m.offset, previous: prev_name.clone(), delta });
        }
        let new_offset = align64(prev_new_end) + delta as u64;
        followers.push(Follower { idx: i, new_offset });
        prev_old_end = m.offset as u64 + m.size as u64;
        prev_new_end = new_offset + m.size as u64;
        prev_name = m.name.clone();
    }
    let old_last_end = layout.last_end as u64;
    let new_last_end = if followers.is_empty() { off_n + new_s } else { prev_new_end };
    let new_len = new_last_end + layout.tail_len as u64;
    if new_len > u32::MAX as u64 {
        return Err(ReplanError::TooLarge(new_len));
    }

    // The gap right after the replaced member: carried when the length is unchanged, else it
    // must be zero (it is rewritten as a fresh alignment gap).
    let old_gap_start = off_n + old_s;
    let old_gap_end = followers.first().map(|f| layout.members[f.idx].offset as u64).unwrap_or(old_last_end);
    let length_changes = new_s != old_s;
    if length_changes && package[old_gap_start as usize..old_gap_end as usize].iter().any(|b| *b != 0) {
        return Err(ReplanError::NonZeroGap { member: t.name.clone(), start: old_gap_start as u32, len: (old_gap_end - old_gap_start) as usize });
    }

    // Assemble.
    let mut out = Vec::with_capacity(new_len as usize);
    out.extend_from_slice(&package[..off_n as usize]);
    out.extend_from_slice(new_member);
    if length_changes {
        let first_follower = followers.first().map(|f| f.new_offset).unwrap_or(new_last_end);
        out.resize(first_follower as usize, 0);
    } else {
        out.extend_from_slice(&package[old_gap_start as usize..old_gap_end as usize]);
    }
    // Followers with their own bytes and the gap bytes between them (lengths preserved by
    // construction: each follower keeps its size and lands on the same 64-byte residual).
    let mut prev_old_end = old_gap_end; // = first follower's old offset
    for (k, f) in followers.iter().enumerate() {
        let m = &layout.members[f.idx];
        if k > 0 {
            let gap = &package[prev_old_end as usize..m.offset as usize];
            if out.len() + gap.len() != f.new_offset as usize {
                return Err(ReplanError::Verification(format!("gap before `{}` changed length ({} vs {})", m.name, f.new_offset as usize - out.len(), gap.len())));
            }
            out.extend_from_slice(gap);
        } else if out.len() != f.new_offset as usize {
            return Err(ReplanError::Verification(format!("`{}` would land at {} not {}", m.name, out.len(), f.new_offset)));
        }
        out.extend_from_slice(&package[m.offset as usize..(m.offset + m.size) as usize]);
        prev_old_end = m.offset as u64 + m.size as u64;
    }
    if !followers.is_empty() {
        // Between the last follower's end and the tail there is nothing on the measured
        // packages (last_end is the max end); carry whatever is there anyway.
        out.extend_from_slice(&package[prev_old_end as usize..old_last_end as usize]);
    }
    out.extend_from_slice(&package[old_last_end as usize..]);
    if out.len() as u64 != new_len {
        return Err(ReplanError::Verification(format!("assembled {} bytes, planned {new_len}", out.len())));
    }

    // Table words.
    out[4..8].copy_from_slice(&(new_last_end as u32).to_le_bytes());
    out[t.row_pos + 4..t.row_pos + 8].copy_from_slice(&(new_s as u32).to_be_bytes());
    let mut moved = Vec::new();
    for f in &followers {
        let m = &layout.members[f.idx];
        if f.new_offset != m.offset as u64 {
            out[m.row_pos..m.row_pos + 4].copy_from_slice(&(f.new_offset as u32).to_be_bytes());
            moved.push(Moved { name: m.name.clone(), old_offset: m.offset, new_offset: f.new_offset as u32 });
        }
    }

    // Verify: re-parse, the target holds the new bytes, every other member its old bytes.
    let back = PackageLayout::parse(&out).map_err(|e| ReplanError::Verification(format!("output does not parse: {e}")))?;
    if back.members.len() != layout.members.len() || back.data_offset != layout.data_offset || back.tail_len != layout.tail_len {
        return Err(ReplanError::Verification("member count, data_offset or tail changed".into()));
    }
    for (a, b) in layout.members.iter().zip(&back.members) {
        if a.name != b.name {
            return Err(ReplanError::Verification(format!("member order changed at `{}`", a.name)));
        }
        let want: &[u8] = if a.index == target { new_member } else { &package[a.offset as usize..(a.offset + a.size) as usize] };
        if &out[b.offset as usize..(b.offset + b.size) as usize] != want {
            return Err(ReplanError::Verification(format!("member `{}` bytes differ after re-plan", a.name)));
        }
    }
    if out[..layout.table_end] != package[..layout.table_end] {
        // Only the patched words may differ inside the table; check the rest byte-wise.
        let mut patched: Vec<std::ops::Range<usize>> = vec![4..8, t.row_pos + 4..t.row_pos + 8];
        for f in &followers {
            let m = &layout.members[f.idx];
            patched.push(m.row_pos..m.row_pos + 4);
        }
        for (i, (x, y)) in out[..layout.table_end].iter().zip(&package[..layout.table_end]).enumerate() {
            if x != y && !patched.iter().any(|r| r.contains(&i)) {
                return Err(ReplanError::Verification(format!("table byte {i} changed outside the patched words")));
            }
        }
    }
    if out[layout.table_end..layout.data_offset as usize] != package[layout.table_end..layout.data_offset as usize] {
        return Err(ReplanError::Verification("the table trailer changed".into()));
    }
    Ok(Replan {
        bytes: out,
        member: t.name.clone(),
        old_len: package.len(),
        new_len: new_len as usize,
        old_member_len: old_s as u32,
        new_member_len: new_s as u32,
        moved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A package in the X-C shape (`0`, `<n+1>`, `sg1`, `<n>`, `sg2`, `1`; stride 0; 16-byte tail).
    fn xc_package(tex: &[u8], stride: u64, tail: usize) -> Vec<u8> {
        let names = ["0", "3", "sg1", "2", "sg2", "1"];
        let m0 = vec![0x11u8; 240];
        let m3 = vec![0x33u8; 288];
        let m1 = vec![0x44u8; 32];
        let table_end = 16 + names.iter().map(|n| 8 + n.len() + 1).sum::<usize>();
        let data_offset = table_end + 8;
        let off0 = align64(data_offset as u64);
        let off3 = align64(off0 + 240);
        let sg1 = align64(off3 + 288);
        let off2 = sg1 + stride;
        let sg2 = align64(off2 + tex.len() as u64);
        let off1 = sg2 + stride;
        let last_end = off1 + 32;
        let sizes = [240u32, 288, 0, tex.len() as u32, 0, 32];
        let offs = [off0, off3, sg1, off2, sg2, off1];
        let mut b = vec![0u8; last_end as usize + tail];
        for (k, t) in b[last_end as usize..].iter_mut().enumerate() {
            *t = 0xC0 | k as u8; // the opaque tail is not zero on the shipped packages
        }
        b[..4].copy_from_slice(b"BIGF");
        b[4..8].copy_from_slice(&(last_end as u32).to_le_bytes());
        b[8..12].copy_from_slice(&6u32.to_be_bytes());
        b[12..16].copy_from_slice(&(data_offset as u32).to_be_bytes());
        let mut c = 16;
        for (k, n) in names.iter().enumerate() {
            b[c..c + 4].copy_from_slice(&(offs[k] as u32).to_be_bytes());
            b[c + 4..c + 8].copy_from_slice(&sizes[k].to_be_bytes());
            c += 8;
            b[c..c + n.len()].copy_from_slice(n.as_bytes());
            c += n.len() + 1;
        }
        b[table_end..table_end + 8].copy_from_slice(b"L286\x15\x05\x00\x01");
        b[off0 as usize..off0 as usize + 240].copy_from_slice(&m0);
        b[off3 as usize..off3 as usize + 288].copy_from_slice(&m3);
        b[off2 as usize..off2 as usize + tex.len()].copy_from_slice(tex);
        b[off1 as usize..off1 as usize + 32].copy_from_slice(&m1);
        b
    }

    fn member_bytes(pkg: &[u8], name: &str) -> Vec<u8> {
        let l = PackageLayout::parse(pkg).unwrap();
        let m = l.members.iter().find(|m| m.name == name).unwrap();
        pkg[m.offset as usize..(m.offset + m.size) as usize].to_vec()
    }

    #[test]
    fn identical_member_reproduces_the_package_byte_for_byte() {
        for stride in [0u64, 64] {
            for tail in [0usize, 16] {
                let tex = vec![0xAAu8; 7415];
                let pkg = xc_package(&tex, stride, tail);
                let r = replan_member(&pkg, "2", &tex).unwrap();
                assert!(r.is_identity());
                assert_eq!(r.bytes, pkg, "stride {stride} tail {tail}");
            }
        }
    }

    #[test]
    fn a_longer_member_shifts_only_the_followers_and_keeps_the_stride_and_tail() {
        for stride in [0u64, 64] {
            let tex = vec![0xAAu8; 7415];
            let pkg = xc_package(&tex, stride, 16);
            let l0 = PackageLayout::parse(&pkg).unwrap();
            for grow in [1i64, 63, 64, 65, 1000, -1, -63, -64, -1000] {
                let new_tex = vec![0xBBu8; (7415 + grow) as usize];
                let r = replan_member(&pkg, "2", &new_tex).unwrap();
                let l1 = PackageLayout::parse(&r.bytes).unwrap();
                assert_eq!(l1.tail_len, 16);
                assert_eq!(&r.bytes[r.bytes.len() - 16..], &pkg[pkg.len() - 16..], "opaque tail carried");
                assert_eq!(l1.data_offset, l0.data_offset);
                assert_eq!(&r.bytes[..4], &pkg[..4]);
                assert_eq!(&r.bytes[8..l0.members[3].row_pos], &pkg[8..l0.members[3].row_pos], "header + earlier rows verbatim (size_le aside)");
                assert_eq!(&r.bytes[l0.table_end..l0.data_offset as usize], b"L286\x15\x05\x00\x01");
                assert_eq!(member_bytes(&r.bytes, "2"), new_tex);
                for n in ["0", "3", "1"] {
                    assert_eq!(member_bytes(&r.bytes, n), member_bytes(&pkg, n), "grow {grow} member {n}");
                }
                // sg2 = align64(end of 2); 1 = sg2 + stride; size_le = end of 1.
                let m2 = l1.members.iter().find(|m| m.name == "2").unwrap();
                let sg2 = l1.members.iter().find(|m| m.name == "sg2").unwrap();
                let m1 = l1.members.iter().find(|m| m.name == "1").unwrap();
                assert_eq!(sg2.offset as u64, align64(m2.offset as u64 + m2.size as u64));
                assert_eq!(m1.offset as u64, sg2.offset as u64 + stride);
                assert_eq!(l1.size_le, m1.offset + 32);
                assert_eq!(r.bytes.len() as u32, l1.size_le + 16);
                // The gap after the texture is zero.
                assert!(r.bytes[(m2.offset + m2.size) as usize..sg2.offset as usize].iter().all(|b| *b == 0));
                let expected_moved = if align64(m2.offset as u64 + new_tex.len() as u64) != align64(m2.offset as u64 + 7415) { 2 } else { 0 };
                assert_eq!(r.moved.len(), expected_moved, "grow {grow}");
            }
        }
    }

    #[test]
    fn a_non_zero_gap_after_the_member_refuses_a_length_change_but_not_an_identity() {
        let tex = vec![0xAAu8; 7415];
        let mut pkg = xc_package(&tex, 0, 16);
        let l = PackageLayout::parse(&pkg).unwrap();
        let m2 = l.members.iter().find(|m| m.name == "2").unwrap();
        pkg[(m2.offset + m2.size) as usize + 3] = 0x5A;
        assert!(replan_member(&pkg, "2", &tex).unwrap().is_identity());
        let e = replan_member(&pkg, "2", &vec![0u8; 7416]).unwrap_err();
        assert!(matches!(e, ReplanError::NonZeroGap { .. }), "{e}");
        assert!(replan_member(&pkg, "0", &vec![0u8; 240]).is_ok());
    }

    #[test]
    fn unexplained_shapes_are_refused() {
        let tex = vec![0xAAu8; 100];
        let mut pkg = xc_package(&tex, 0, 16);
        // size_le not the last end.
        let mut bad = pkg.clone();
        let bad_len = bad.len() as u32;
        bad[4..8].copy_from_slice(&bad_len.to_le_bytes());
        assert!(matches!(PackageLayout::parse(&bad).unwrap_err(), ReplanError::SizeWordUnexplained { .. }));
        // A follower on an unmeasured residual (32).
        let l = PackageLayout::parse(&pkg).unwrap();
        let m1 = l.members.iter().find(|m| m.name == "1").unwrap();
        let sg2 = l.members.iter().find(|m| m.name == "sg2").unwrap();
        let shifted = sg2.offset + 32;
        pkg[m1.row_pos..m1.row_pos + 4].copy_from_slice(&shifted.to_be_bytes());
        // keep last_end/size_le consistent
        pkg.truncate(shifted as usize + 32);
        pkg.extend_from_slice(&[0u8; 16]);
        pkg[4..8].copy_from_slice(&(shifted + 32).to_le_bytes());
        let e = replan_member(&pkg, "2", &vec![0u8; 101]).unwrap_err();
        assert!(matches!(e, ReplanError::UnmeasuredPlacement { delta: 32, .. }), "{e}");
        // Unknown member.
        assert!(matches!(replan_member(&pkg, "9", &[]).unwrap_err(), ReplanError::NoSuchMember(_)));
        // Entry-pad variants are not measured.
        let mut b5 = xc_package(&tex, 0, 16);
        b5[..4].copy_from_slice(b"BIG5");
        assert!(matches!(PackageLayout::parse(&b5).unwrap_err(), ReplanError::UnmeasuredVariant(_)));
    }

    #[test]
    fn replacing_the_last_member_keeps_the_tail_and_needs_no_followers() {
        let tex = vec![0xAAu8; 200];
        let pkg = xc_package(&tex, 64, 16);
        let r = replan_member(&pkg, "1", &vec![0x77u8; 40]).unwrap();
        let l = PackageLayout::parse(&r.bytes).unwrap();
        assert_eq!(member_bytes(&r.bytes, "1"), vec![0x77u8; 40]);
        assert_eq!(l.tail_len, 16);
        assert!(r.moved.is_empty());
        assert_eq!(r.new_len, r.old_len + 8);
    }

    // ---------------------------------------------------------------- P1 audit (21 Sep 2026)

    /// Malformed packages refuse without panicking: a count word the table cannot hold (no
    /// capacity overflow), a member span past the end, two overlapping members, a data_offset
    /// inside the table, a truncated file.
    #[test]
    fn malformed_packages_are_refused_not_panicked() {
        let tex = vec![0xAAu8; 100];
        let pkg = xc_package(&tex, 0, 16);
        let mut bad = pkg.clone();
        bad[8..12].copy_from_slice(&0xFFFF_FFF0u32.to_be_bytes());
        assert!(matches!(PackageLayout::parse(&bad).unwrap_err(), ReplanError::TruncatedTable { .. }));
        let l = PackageLayout::parse(&pkg).unwrap();
        let m1 = l.members.iter().find(|m| m.name == "1").unwrap();
        let mut bad = pkg.clone();
        bad[m1.row_pos + 4..m1.row_pos + 8].copy_from_slice(&0x7FFF_FFFFu32.to_be_bytes());
        assert!(matches!(PackageLayout::parse(&bad).unwrap_err(), ReplanError::MemberOutOfRange { .. }));
        let m3 = l.members.iter().find(|m| m.name == "3").unwrap();
        let m0 = l.members.iter().find(|m| m.name == "0").unwrap();
        let mut bad = pkg.clone();
        bad[m3.row_pos..m3.row_pos + 4].copy_from_slice(&m0.offset.to_be_bytes());
        assert!(matches!(PackageLayout::parse(&bad).unwrap_err(), ReplanError::Overlap { .. }));
        let mut bad = pkg.clone();
        bad[12..16].copy_from_slice(&8u32.to_be_bytes());
        assert!(matches!(PackageLayout::parse(&bad).unwrap_err(), ReplanError::DataOffsetBeforeTable { .. }));
        assert!(matches!(PackageLayout::parse(&pkg[..40]).unwrap_err(), ReplanError::TruncatedTable { .. } | ReplanError::MemberOutOfRange { .. }));
        assert!(matches!(PackageLayout::parse(&pkg[..10]).unwrap_err(), ReplanError::Truncated(10)));
        let mut bad = pkg.clone();
        bad[..4].copy_from_slice(b"XXXX");
        assert!(matches!(PackageLayout::parse(&bad).unwrap_err(), ReplanError::BadMagic(_)));
    }

    /// The first member (index 0, lowest offset) grows and shrinks: every later member, both
    /// zero-length markers and the tail follow with their stride; different bytes, not identity.
    #[test]
    fn replacing_the_first_member_moves_every_follower_consistently() {
        for stride in [0u64, 64] {
            let tex = vec![0xAAu8; 500];
            let pkg = xc_package(&tex, stride, 16);
            for new_len in [1usize, 240, 241, 304, 305, 2000] {
                let new0: Vec<u8> = (0..new_len).map(|i| (i % 251) as u8).collect();
                let r = replan_member(&pkg, "0", &new0).unwrap();
                let l1 = PackageLayout::parse(&r.bytes).unwrap();
                assert_eq!(member_bytes(&r.bytes, "0"), new0);
                for n in ["3", "2", "1"] {
                    assert_eq!(member_bytes(&r.bytes, n), member_bytes(&pkg, n), "stride {stride} len {new_len} {n}");
                }
                let by = |n: &str| l1.members.iter().find(|m| m.name == n).unwrap();
                assert_eq!(by("3").offset as u64, align64(by("0").offset as u64 + new_len as u64));
                assert_eq!(by("sg1").offset as u64, align64(by("3").offset as u64 + 288));
                assert_eq!(by("2").offset as u64, by("sg1").offset as u64 + stride);
                assert_eq!(by("sg2").offset as u64, align64(by("2").offset as u64 + 500));
                assert_eq!(by("1").offset as u64, by("sg2").offset as u64 + stride);
                assert_eq!(l1.size_le, by("1").offset + 32);
                assert_eq!(&r.bytes[r.bytes.len() - 16..], &pkg[pkg.len() - 16..]);
                assert_eq!(r.moved.len(), if align64(by("0").offset as u64 + new_len as u64) == align64(by("0").offset as u64 + 240) { 0 } else { 5 });
            }
        }
    }
}
