use super::{bad, create, exact, input, limits, noun, RECORDS, RESULT};
use joy_rs::structured::certificate::transport::Reader;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

#[derive(Serialize, Deserialize)]
pub struct Patch {
    pub offset: u64,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct Index {
    pub schema: String,
    pub source_sha256: String,
    pub source_bytes: u64,
    pub context: [u8; 32],
    pub records: u64,
    pub frames: u64,
    pub decoded_bytes: u64,
    pub continuation: Patch,
    pub generation: Patch,
    pub terminal_offset: u64,
    pub terminal: Vec<u8>,
}

pub fn load(path: &Path) -> io::Result<Index> {
    let file = super::input(path)?;
    if file.metadata()?.len() > 65536 {
        return Err(bad("index byte cap"));
    }
    let index: Index = serde_json::from_reader(file).map_err(|_| bad("index JSON"))?;
    if index.schema != "trident/whole-proof-mutation-index/v1"
        || index.records > RECORDS
        || index.decoded_bytes > super::DECODED
        || index.source_bytes > super::WIRE
        || index.terminal.len() != 44
        || index.terminal_offset + 45 > index.decoded_bytes
    {
        return Err(bad("index bounds"));
    }
    for patch in [&index.continuation, &index.generation] {
        if patch.before.is_empty()
            || patch.before.len() != patch.after.len()
            || patch.before.len() > 8
            || patch
                .offset
                .checked_add(patch.before.len() as u64)
                .is_none_or(|n| n >= index.terminal_offset)
        {
            return Err(bad("patch bounds"));
        }
    }
    Ok(index)
}

pub fn build(source: &Path, target: &Path, result_path: &Path, sha: &str) -> io::Result<()> {
    super::hex::<32>(sha)?;
    let mut file = input(source)?;
    let source_bytes = file.metadata()?.len();
    let preamble = exact::<40>(&mut file)?;
    let context = preamble[8..].try_into().map_err(|_| bad("context"))?;
    file.seek(SeekFrom::Start(0))?;
    let mut reader = Reader::new(file, context, limits())?;
    let (mut offset, mut count, mut enters) = (0u64, 0u64, 0u64);
    let (mut continuation, mut generation) = (None, None);
    let (terminal_offset, terminal, result) = loop {
        count += 1;
        if count > RECORDS {
            return Err(bad("record cap"));
        }
        let start = offset;
        let tag = exact::<1>(&mut reader)?[0];
        offset += 1;
        if tag == 6 {
            let terminal = exact::<44>(&mut reader)?;
            let length = u32::from_le_bytes(
                terminal[40..44]
                    .try_into()
                    .map_err(|_| bad("result size"))?,
            ) as usize;
            if length == 0 || length > RESULT {
                return Err(bad("result cap"));
            }
            let mut result = vec![0; length];
            reader.read_exact(&mut result)?;
            offset += 44 + length as u64;
            break (start, terminal.to_vec(), result);
        }
        let length = match tag {
            0 | 1 | 3 | 4 => 8,
            2 | 5 => 12,
            _ => return Err(bad("semantic tag")),
        };
        let mut record = [0; 12];
        reader.read_exact(&mut record[..length])?;
        offset += length as u64;
        if tag == 3 {
            enters += 1;
            if continuation.is_none() && enters > 1 && record[..4] != record[4..8] {
                continuation = Some(Patch {
                    offset: start + 5,
                    before: record[4..8].to_vec(),
                    after: record[..4].to_vec(),
                });
            }
        }
        if tag == 5 && generation.is_none() {
            generation = Some(Patch {
                offset: start + 5,
                before: vec![record[4]],
                after: vec![record[4] ^ 1],
            });
        }
    };
    let stats = reader.finish()?;
    if stats.decoded_bytes != offset || stats.wire_bytes != source_bytes {
        return Err(bad("index totals"));
    }
    noun::check(&result)?;
    if result[8..40] != terminal[..32] {
        return Err(bad("original terminal/output mismatch"));
    }
    let index = Index {
        schema: "trident/whole-proof-mutation-index/v1".into(),
        source_sha256: sha.into(),
        source_bytes,
        context,
        records: count,
        frames: stats.frames,
        decoded_bytes: stats.decoded_bytes,
        continuation: continuation.ok_or_else(|| bad("continuation absent"))?,
        generation: generation.ok_or_else(|| bad("reuse absent"))?,
        terminal_offset,
        terminal,
    };
    let mut output = create(result_path)?;
    output.write_all(&result)?;
    output.sync_all()?;
    let mut encoded = create(target)?;
    serde_json::to_writer_pretty(&mut encoded, &index).map_err(|_| bad("index serialization"))?;
    encoded.write_all(b"\n")?;
    encoded.sync_all()?;
    println!(
        "{}",
        serde_json::json!({"status":"indexed", "records":count, "wire_bytes":source_bytes,
        "decoded_bytes":stats.decoded_bytes, "frames":stats.frames, "terminal_offset":terminal_offset, "output_bytes":result.len()})
    );
    Ok(())
}
