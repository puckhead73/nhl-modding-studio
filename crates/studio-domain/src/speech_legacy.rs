//! NHL 07/08 Xbox 360 and NHL 08 PS3 IDX/BIG speech banks.
//! The BIG contains version-0 EA-XMA or EALayer3 streams;
//! the separate big-endian IDX supplies sample offsets and eight-byte SNR headers.
//! Exports retain encoded SNR/SNS bytes; this is not a decoder or bank rebuilder.
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};

#[derive(Clone, Debug, Serialize)]
pub struct Sample {
    pub group: u32,
    pub record: u32,
    pub id: u32,
    pub take: u32,
    pub offset: u64,
    pub limit: u64,
    pub snr: [u8; 8],
}
impl Sample {
    pub fn name(&self, bank: &str) -> String {
        format!(
            "{bank}_samples/group_{:03}/record_{:06}_id_{:08x}_take_{:03}",
            self.group, self.record, self.id, self.take
        )
    }
}
fn word(b: &[u8], p: usize) -> Result<usize, String> {
    let bytes: [u8; 4] = b
        .get(p..p + 4)
        .ok_or("truncated speech index")?
        .try_into()
        .unwrap();
    Ok(u32::from_be_bytes(bytes) as usize)
}
/// Validate the complete measured table layout before admitting any output.
pub fn parse(idx: &[u8], bank_len: u64) -> Result<Vec<Sample>, String> {
    let groups = word(idx, 0)?;
    let records = word(idx, 4)?;
    let headers = word(idx, 8)?;
    let metadata = word(idx, 12)?;
    if groups == 0
        || groups > 4096
        || word(idx, 16)? != 24
        || word(idx, 20)? != 24 + groups * 4
        || records != 24 + groups * 8
        || records > headers
        || headers > metadata
        || metadata > idx.len()
        || (headers - records) % 24 != 0
    {
        return Err("unrecognized IDX speech table layout".into());
    }
    let counts = (0..groups)
        .map(|g| word(idx, 24 + groups * 4 + g * 4))
        .collect::<Result<Vec<_>, _>>()?;
    if (0..groups).any(|g| word(idx, 24 + g * 4).ok() != Some(0)) {
        return Err("unsupported IDX group flags".into());
    }
    let count = (headers - records) / 24;
    if count == 0 || counts.iter().sum::<usize>() != count {
        return Err("IDX group counts do not match records".into());
    }
    let mut samples: Vec<Sample> = vec![];
    let mut record = 0;
    let mut next_header = 0;
    let mut next_meta = 0;
    let mut previous_base = None;
    for (group, n) in counts.into_iter().enumerate() {
        for _ in 0..n {
            let p = records + record * 24;
            let id = word(idx, p)?;
            let off = word(idx, p + 4)?;
            let size = word(idx, p + 8)?;
            let meta_off = word(idx, p + 12)?;
            let meta_size = word(idx, p + 16)?;
            let base = word(idx, p + 20)? as u64;
            if off != next_header
                || size == 0
                || size % 12 != 0
                || headers + off + size > metadata
                || meta_off != next_meta
                || metadata + meta_off + meta_size > idx.len()
                || base % 64 != 0
                || previous_base.is_some_and(|b| base <= b)
            {
                return Err(format!("invalid IDX record {record}"));
            }
            previous_base = Some(base);
            for take in 0..size / 12 {
                let h = headers + off + take * 12;
                let relative = word(idx, h)? as u64;
                let offset = base + relative;
                let snr: [u8; 8] = idx[h + 4..h + 12].try_into().unwrap();
                let header = tdb_snd::SnrHeader::parse(&snr).unwrap();
                if (take == 0 && relative != 0)
                    || offset % 64 != 0
                    || offset >= bank_len
                    || header.version != 0
                    || !matches!(header.codec, 3 | 5)
                    || header.stream_type != 1
                    || header.looped
                    || header.sample_rate == 0
                    || header.sample_rate > 200000
                    || header.num_samples == 0
                {
                    return Err(format!(
                        "unsupported speech sample in record {record}, take {take}: codec {}, version {}, {} Hz, {} channels, stream type {}, looped {}",
                        header.codec_label(), header.version, header.sample_rate, header.channels(), header.stream_type, header.looped
                    ));
                }
                if let Some(prev) = samples.last_mut() {
                    if offset <= prev.offset {
                        return Err("speech sample ranges are not increasing".into());
                    }
                    prev.limit = offset;
                } else if offset != 0 {
                    return Err("speech bank does not start at offset zero".into());
                }
                samples.push(Sample {
                    group: group as u32,
                    record: record as u32,
                    id: id as u32,
                    take: take as u32,
                    offset,
                    limit: bank_len,
                    snr,
                });
            }
            next_header = off + size;
            next_meta = meta_off + meta_size;
            record += 1;
        }
    }
    if headers + next_header != metadata || metadata + next_meta != idx.len() {
        return Err("IDX tables do not cover the index".into());
    }
    Ok(samples)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn fixture() -> (Vec<u8>, Vec<u8>) {
        let mut idx = vec![0u8; 72];
        for (at, n) in [
            (0, 1u32),
            (4, 32),
            (8, 56),
            (12, 68),
            (16, 24),
            (20, 28),
            (28, 1),
            (32, 7),
            (36, 0),
            (40, 12),
            (44, 0),
            (48, 4),
            (52, 0),
            (56, 0),
            (60, 0x03005622),
            (64, 0x40000020),
        ] {
            idx[at..at + 4].copy_from_slice(&n.to_be_bytes());
        }
        let mut bank = vec![0u8; 64];
        bank[..8].copy_from_slice(&[0x80, 0, 0, 8, 0, 0, 0, 32]);
        (idx, bank)
    }
    #[test]
    fn indexes_and_extracts_encoded_audio_without_alignment_padding() {
        let (idx, bank) = fixture();
        let samples = parse(&idx, bank.len() as u64).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].id, 7);
        assert_eq!(
            read_sample(&mut std::io::Cursor::new(bank), &samples[0]).unwrap(),
            vec![0x80, 0, 0, 8, 0, 0, 0, 32]
        );
    }
    #[test]
    fn ps3_layer3_headers_preserve_all_measured_rate_and_channel_classes() {
        for (rate, channels) in [(44100u32, 0u32), (48000, 0), (22050, 0), (22050, 1)] {
            let (mut idx, bank) = fixture();
            let word = (5u32 << 24) | (channels << 18) | rate;
            idx[60..64].copy_from_slice(&word.to_be_bytes());
            let samples = parse(&idx, bank.len() as u64).unwrap();
            assert_eq!(samples[0].snr, idx[60..68]);
            // Extraction preserves encoded payloads; it does not transcode audio.
            assert_eq!(read_sample(&mut std::io::Cursor::new(&bank), &samples[0]).unwrap(), bank[..8]);
        }
    }

    #[test]
    fn rejects_bad_tables_ranges_headers_and_streams() {
        let (idx, bank) = fixture();
        for end in 0..idx.len() {
            assert!(parse(&idx[..end], 64).is_err());
        }
        for (at, value) in [(28, 2u32), (36, 12), (40, 13), (52, 64), (60, 0x09005622)] {
            let mut b = idx.clone();
            b[at..at + 4].copy_from_slice(&value.to_be_bytes());
            assert!(parse(&b, 64).is_err());
        }
        let sample = &parse(&idx, 64).unwrap()[0];
        for (at, value) in [(0, 0u8), (3, 100), (7, 31), (63, 1)] {
            let mut b = bank.clone();
            b[at] = value;
            assert!(read_sample(&mut std::io::Cursor::new(b), sample).is_err());
        }
    }
}
/// Preserve complete version-0 SNS blocks through the terminal block, excluding
/// inter-record alignment. Block bounds and summed sample counts must agree with IDX.
pub fn read_sample(reader: &mut (impl Read + Seek), sample: &Sample) -> Result<Vec<u8>, String> {
    let len = sample.limit - sample.offset;
    if len > 64 * 1024 * 1024 {
        return Err("speech sample exceeds the 64 MiB extraction limit".into());
    }
    let mut data = vec![0; len as usize];
    reader
        .seek(SeekFrom::Start(sample.offset))
        .map_err(|e| e.to_string())?;
    reader.read_exact(&mut data).map_err(|e| e.to_string())?;
    let mut p = 0;
    let mut samples = 0u64;
    loop {
        let h = data
            .get(p..p + 8)
            .ok_or("speech stream has no terminal block")?;
        let flag = h[0];
        let size = u32::from_be_bytes([0, h[1], h[2], h[3]]) as usize;
        if !matches!(flag, 0 | 128) || size < 8 || p + size > data.len() {
            return Err("invalid speech stream block".into());
        }
        samples += u32::from_be_bytes(h[4..8].try_into().unwrap()) as u64;
        p += size;
        if flag == 128 {
            break;
        }
    }
    if samples != tdb_snd::SnrHeader::parse(&sample.snr).unwrap().num_samples as u64 {
        return Err("speech block sample count differs from IDX".into());
    }
    // Measured banks align records to 128 bytes and samples to 64 bytes.
    // Do not include the next record's padding or silently discard unexplained data.
    if data.len() - p > 64 || data[p..].iter().any(|b| *b != 0) {
        return Err("unexpected data after speech terminal block".into());
    }
    data.truncate(p);
    Ok(data)
}
