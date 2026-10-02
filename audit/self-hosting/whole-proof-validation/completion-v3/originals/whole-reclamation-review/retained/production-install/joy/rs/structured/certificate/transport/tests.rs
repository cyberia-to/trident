use super::*;

fn limits() -> Limits {
    Limits {
        wire_bytes: 1 << 22,
        decoded_bytes: 1 << 21,
        frames: 128,
    }
}

fn encode(value: &[u8]) -> (Vec<u8>, Stats) {
    let mut writer = Writer::new(Vec::new(), [7; 32], limits()).unwrap();
    writer.write_all(value).unwrap();
    writer.finish().unwrap()
}

fn decode(bytes: &[u8], cap: Limits) -> io::Result<(Vec<u8>, Stats)> {
    let mut reader = Reader::new(bytes, [7; 32], cap)?;
    let mut output = Vec::new();
    reader.read_to_end(&mut output)?;
    Ok((output, reader.finish()?))
}

#[test]
fn complete_empty_raw_compressed_and_multiframe_streams() {
    let mut seed = 17u64;
    let random: Vec<_> = (0..CHUNK * 2 + 7)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u8
        })
        .collect();
    for value in [vec![], vec![0], vec![19; CHUNK * 2 + 1], random] {
        let (bytes, written) = encode(&value);
        let (decoded, read) = decode(&bytes, limits()).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(read, written);
        assert_eq!(read.wire_bytes, bytes.len() as u64);
        assert_eq!(read.decoded_bytes, value.len() as u64);
        assert_eq!(read.frames, value.len().div_ceil(CHUNK) as u64 + 1);
        let exact = Limits {
            wire_bytes: written.wire_bytes,
            decoded_bytes: written.decoded_bytes.max(1),
            frames: written.frames,
        };
        assert!(decode(&bytes, exact).is_ok());
        assert!(decode(
            &bytes,
            Limits {
                wire_bytes: exact.wire_bytes - 1,
                ..exact
            }
        )
        .is_err());
        assert!(decode(
            &bytes,
            Limits {
                frames: exact.frames - 1,
                ..exact
            }
        )
        .is_err());
        if value.len() > 1 {
            assert!(decode(
                &bytes,
                Limits {
                    decoded_bytes: exact.decoded_bytes - 1,
                    ..exact
                }
            )
            .is_err());
        }
    }
    assert_eq!(encode(&[0]).0[49], 0);
    assert_eq!(encode(&[0; 1024]).0[49], 1);
}

#[test]
fn every_truncated_prefix_and_changed_byte_fails() {
    let (bytes, _) = encode(&[9; 128]);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end], limits()).is_err(), "prefix {end}");
    }
    for offset in 0..bytes.len() {
        let mut changed = bytes.clone();
        changed[offset] ^= 1;
        assert!(decode(&changed, limits()).is_err(), "byte {offset}");
    }
    assert!(Reader::new(bytes.as_slice(), [8; 32], limits()).is_err());
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode(&extra, limits()).is_err());
    extra = bytes.clone();
    extra.extend_from_slice(&bytes);
    assert!(decode(&extra, limits()).is_err());
}

fn custom_frame(codec: u8, decoded: u32, payload: &[u8]) -> Vec<u8> {
    let state = State::new(limits(), [7; 32]).unwrap();
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&[7; 32]);
    let mut header = [0; HEADER];
    header[9] = codec;
    header[10..14].copy_from_slice(&decoded.to_le_bytes());
    header[14..18].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    header[18..].copy_from_slice(&state.previous);
    let digest = frame_digest(&header, payload);
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(&digest);
    let mut end = [0; HEADER];
    end[..8].copy_from_slice(&1u64.to_le_bytes());
    end[8] = 1;
    end[18..].copy_from_slice(&digest);
    bytes.extend_from_slice(&end);
    bytes.extend_from_slice(&frame_digest(&end, &[]));
    bytes
}

#[test]
fn self_consistent_bad_lengths_deflate_end_and_bombs_fail() {
    let compressed = miniz_oxide::deflate::compress_to_vec(&[3; 1024], 1);
    let valid = custom_frame(1, 1024, &compressed);
    assert_eq!(decode(&valid, limits()).unwrap().0, [3; 1024]);
    for length in [0, 1, 1023, 1025, CHUNK as u32 + 1, u32::MAX] {
        assert!(decode(&custom_frame(1, length, &compressed), limits()).is_err());
    }
    assert!(decode(&custom_frame(2, 1024, &compressed), limits()).is_err());
    assert!(decode(&custom_frame(0, 1024, &compressed), limits()).is_err());
    for end in 0..compressed.len() {
        assert!(decode(&custom_frame(1, 1024, &compressed[..end]), limits()).is_err());
    }
    let mut extra = compressed.clone();
    extra.push(0);
    assert!(decode(&custom_frame(1, 1024, &extra), limits()).is_err());
    extra.extend_from_slice(&compressed);
    assert!(decode(&custom_frame(1, 1024, &extra), limits()).is_err());
    let bomb = miniz_oxide::deflate::compress_to_vec(&vec![0; CHUNK * 10], 1);
    assert!(decode(&custom_frame(1, CHUNK as u32, &bomb), limits()).is_err());
}

#[test]
fn reordered_cross_context_and_duplicate_frames_fail() {
    let mut writer = Writer::new(Vec::new(), [7; 32], limits()).unwrap();
    writer.write_all(&[1, 2, 3]).unwrap();
    writer.flush().unwrap();
    writer.write_all(&[4, 5, 6]).unwrap();
    let (bytes, _) = writer.finish().unwrap();
    let first = 40..40 + HEADER + 3 + 32;
    let second = first.end..first.end + HEADER + 3 + 32;
    let mut swapped = bytes.clone();
    swapped[first.clone()].copy_from_slice(&bytes[second.clone()]);
    swapped[second.clone()].copy_from_slice(&bytes[first.clone()]);
    assert!(decode(&swapped, limits()).is_err());
    let mut duplicate = bytes.clone();
    duplicate[second.clone()].copy_from_slice(&bytes[first.clone()]);
    assert!(decode(&duplicate, limits()).is_err());
    let mut foreign = Writer::new(Vec::new(), [8; 32], limits()).unwrap();
    foreign.write_all(&[1, 2, 3]).unwrap();
    let foreign = foreign.finish().unwrap().0;
    duplicate[first.clone()].copy_from_slice(&foreign[first]);
    assert!(decode(&duplicate, limits()).is_err());
}

struct ShortRead<'a>(&'a [u8]);
impl Read for ShortRead<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let len = output.len().min(3);
        self.0.read(&mut output[..len])
    }
}

#[test]
fn small_reads_and_explicit_semantic_finish_are_strict() {
    let (bytes, _) = encode(&[4; 128]);
    let mut reader = Reader::new(ShortRead(&bytes), [7; 32], limits()).unwrap();
    let mut actual = [0; 128];
    reader.read_exact(&mut actual).unwrap();
    assert_eq!(actual, [4; 128]);
    assert!(reader.finish().is_ok());
    let reader = Reader::new(bytes.as_slice(), [7; 32], limits()).unwrap();
    assert!(reader.finish().is_err());
}

#[test]
fn failure_poisons_reader_and_writer() {
    let (mut bytes, _) = encode(&[4; 8]);
    bytes[HEADER + 40] ^= 1;
    let mut reader = Reader::new(bytes.as_slice(), [7; 32], limits()).unwrap();
    let mut out = [0; 8];
    assert!(reader.read(&mut out).is_err());
    assert!(reader.read(&mut out).is_err());
    assert!(reader.finish().is_err());
    let mut writer = Writer::new(
        Vec::new(),
        [7; 32],
        Limits {
            decoded_bytes: 1,
            ..limits()
        },
    )
    .unwrap();
    assert!(writer.write_all(&[0, 1]).is_err());
    assert!(writer.write_all(&[0]).is_err());
    assert!(writer.finish().is_err());
}

struct BrokenWrite {
    remaining: usize,
    flush_fails: bool,
}
impl Write for BrokenWrite {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Err(io::Error::other("injected I/O failure"));
        }
        let len = bytes.len().min(self.remaining);
        self.remaining -= len;
        Ok(len)
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.flush_fails {
            Err(io::Error::other("injected flush failure"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn partial_destinations_and_flush_failure_cannot_finish() {
    for remaining in [0, 7, 39, 40, 41, 96, 122] {
        let writer = Writer::new(
            BrokenWrite {
                remaining,
                flush_fails: false,
            },
            [7; 32],
            limits(),
        );
        if let Ok(mut writer) = writer {
            writer.write_all(&[1, 2, 3]).unwrap();
            assert!(writer.finish().is_err());
        }
    }
    let mut writer = Writer::new(
        BrokenWrite {
            remaining: usize::MAX,
            flush_fails: true,
        },
        [7; 32],
        limits(),
    )
    .unwrap();
    assert!(writer.flush().is_err());
    assert!(writer.finish().is_err());
}
