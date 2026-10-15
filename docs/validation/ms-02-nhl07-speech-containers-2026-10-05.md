# NHL 07 Xbox 360 speech-container extraction

## Diagnosis and scope

The two failures were `audio/speech/pbp_eng.big` and `audio/speech/rink.big`. Despite the extension, neither has a BIGF/BIG4 directory header: they contain EA-XMA block streams, while the matching `.idx` contains the offsets and SNR headers. Treating them as ordinary EA BIG archives caused the failures.

The new reader is connected to the existing onboarding extraction service, including cancellation, safe output paths, existing-file preservation, progress and completion markers. Encoded samples are exported as `.snr`/`.sns` pairs under `pbp_eng_samples/` and `rink_samples/`, with group, record, ID and take in the paths. These are generated export names, not original loose-file paths used by the game. The original BIG/IDX/CSI/EVT files stay intact. No loose-file override, bank rebuilding, audio playback or editing support is implied.

## Measured layout

Measured on the user's NHL 07 USA Xbox 360 installation, previously verified against its original ISO:

- IDX header: six big-endian u32s: group count, record-table start, sample-table start, metadata-table start, group-flags start, group-counts start.
- Group flags are zero in this admitted variant. Group counts sum to the number of 24-byte records.
- Records contain ID, sample-table relative offset/size, metadata-table relative offset/size, and BIG base offset. Sample and metadata ranges tile their respective tables.
- Each 12-byte sample entry contains a relative BIG offset and the original eight-byte SNR header. Admitted headers are version 0, codec 3 (EA-XMA), streamed and non-looping. Sample starts are increasing and 64-byte aligned.
- SNS blocks carry an eight-byte prefix (flag/u24 length, u32 sample count). Flags are 0x00 and terminal 0x80. All block sample counts sum to the SNR declared total. Export ends after the terminal block; up to 64 bytes of zero inter-record padding are omitted.
- `pbp_eng`: 5,914 records, 12,064 samples, 391,596 blocks; rates 44,100 and 48,000 Hz, including one stereo sample.
- `rink`: 3,078 records, 7,595 samples, 54,636 blocks; 22,050 Hz mono.

The table interpretation and counts were measured directly. The established [vgmstream EA AudioCore reader](https://github.com/vgmstream/vgmstream/blob/master/src/meta/ea_eaac.c) corroborates the SNR bit fields and version-0 stream block flags. No third-party source code was copied. Other games and format variants are not qualified by this test.

## Protection and completion

Malformed/truncated tables, unsupported SNR variants, non-increasing/out-of-bank ranges, invalid stream blocks, sample-count mismatches and unexplained trailing data are refused. IDX reads are limited to 32 MiB and individual samples to 64 MiB. Completion markers now include the complete companion IDX SHA-256; changes invalidate prior completion. A change during extraction prevents recording completion. Existing differing outputs remain untouched.

## Verification

Read-only preflight of the user's full installation parsed all 11 original containers with no parser failures. Synthetic tests cover valid paired exports, truncated/invalid tables, malformed streams, source preservation, repeat extraction, changed/missing indexes, existing output conflicts and index changes during extraction.

Real-file extraction is exercised through `crates/studio-domain/tests/nhl07_speech_extract.rs`, using copies under `target/nhl07-speech-qa`. It verifies every SNR/SNS output against the source, completion on rescan, no writes on a repeat run, and unchanged hashes for both original BIG/IDX pairs. Run explicitly with `NHL07_SPEECH_FIXTURE` set to the original `audio/speech` directory and `cargo test -p studio-domain --test nhl07_speech_extract --offline -- --ignored --nocapture`.

Application review of the new extraction route and native audio behavior remain separate from these automated checks.

## Vanilla boot files after extraction

The user's subsequent installation extraction left `boot.big` marked partial because 17 members also ship loose with different bytes. All 17 loose files match the original ISO manifest. Extraction now preserves these as resolved only when the full boot archive and each loose file match their reference hashes. Its other 10 loading-screen textures still require extraction. Existing conflict markers resolve on read-only rescan; modified loose files, changed archives and missing outputs remain unresolved. `options.viv` is not an extraction input; the inspected copy contains only a placeholder `options.txt`.

Validation: all 13 extraction-service tests passed, covering fresh extraction and migration from old conflict markers, same-size modifications, missing outputs and changed archives. The opt-in `extracted_install_preserves_vanilla_boot_files` test passed against the user's extracted installation: all 11 containers complete, zero members to write, 17 verified loose boot files preserved. This test made no installation writes. Run with `NHL07_EXTRACTED_INSTALL` set to the installation root and `cargo test -p studio-domain --test nhl07_speech_extract extracted_install_preserves_vanilla_boot_files --offline -- --ignored --nocapture`. UI review remains separate from this backend status check.

Executed results: 12 extraction-service tests passed, including the two new speech integration/identity tests; both new speech parser tests passed. The explicit real-bank integration test passed in 166.38 seconds: 19,659 SNR/SNS pairs (39,318 files), zero failed archives, zero unsupported outputs, zero output conflicts. Every exported pair was compared with the source; rescan reported complete and the repeat run wrote zero files. Hashes of all four original BIG/IDX inputs were unchanged. Owned output evidence remains under `target/nhl07-speech-qa/1791251831468134900`. The original game installation was not extracted into or modified by this test.
