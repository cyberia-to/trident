use super::{
    bad, context, create,
    frames::{Frame, Input, Output, CHUNK},
    index::{self, Index},
    input, noun, RESULT,
};
use std::{
    io::{self, Read, Write},
    path::Path,
};

fn original_result(path: &Path, index: &Index) -> io::Result<Vec<u8>> {
    let mut file = input(path)?;
    let length = file.metadata()?.len();
    let expected = u32::from_le_bytes(
        index.terminal[40..44]
            .try_into()
            .map_err(|_| bad("result size"))?,
    ) as u64;
    if length == 0 || length > RESULT as u64 || length != expected {
        return Err(bad("original result cap/length"));
    }
    let mut bytes = vec![0; length as usize];
    file.read_exact(&mut bytes)?;
    noun::check(&bytes)?;
    if bytes[8..40] != index.terminal[..32] {
        return Err(bad("indexed result root"));
    }
    Ok(bytes)
}

fn terminal(index: &Index, original: &[u8], output: &Path, mode: &str) -> io::Result<Vec<u8>> {
    if mode == "omit-terminal" {
        return Ok(Vec::new());
    }
    let mut header = index.terminal.clone();
    let result = if mode.starts_with("valid-output-") {
        let result = noun::regenerate(original, mode.contains("topology"))?;
        if mode.ends_with("rebound") {
            header[..32].copy_from_slice(&result[8..40]);
        }
        header[40..44].copy_from_slice(&(result.len() as u32).to_le_bytes());
        let mut sidecar = create(&output.with_extension("dag"))?;
        sidecar.write_all(&result)?;
        sidecar.sync_all()?;
        result
    } else if mode == "cost" {
        let cost = u64::from_le_bytes(header[32..40].try_into().map_err(|_| bad("cost"))?);
        header[32..40].copy_from_slice(
            &cost
                .checked_add(1)
                .ok_or_else(|| bad("cost overflow"))?
                .to_le_bytes(),
        );
        original.to_vec()
    } else {
        return Err(bad("terminal mutation mode"));
    };
    let mut bytes = Vec::with_capacity(45 + result.len());
    bytes.push(6);
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(&result);
    Ok(bytes)
}

fn original_totals<R: Read>(reader: &Input<R>, index: &Index) -> io::Result<()> {
    if reader.wire != index.source_bytes
        || reader.decoded != index.decoded_bytes
        || reader.frames != index.frames
    {
        return Err(bad("source totals differ from index"));
    }
    Ok(())
}

fn malformed<R: Read>(
    mut reader: Input<R>,
    output: &Path,
    index: &Index,
    mode: &str,
) -> io::Result<()> {
    let mut writer = create(output)?;
    writer.write_all(b"JOYSC001")?;
    writer.write_all(&reader.context)?;
    let mut first = None;
    let mut changed = false;
    while let Some(frame) = reader.next()? {
        let sequence = reader.frames - 1;
        match mode {
            "drop-first" if sequence == 0 && !frame.terminal() => changed = true,
            "swap-first-two" if sequence == 0 && !frame.terminal() => first = Some(frame),
            "swap-first-two" if sequence == 1 && !frame.terminal() => {
                frame.raw(&mut writer)?;
                first
                    .take()
                    .ok_or_else(|| bad("first frame absent"))?
                    .raw(&mut writer)?;
                changed = true;
            }
            "omit-completion" if frame.terminal() => changed = true,
            "truncate-last-byte" if frame.terminal() => {
                writer.write_all(&frame.header)?;
                writer.write_all(&frame.digest[..31])?;
                changed = true;
            }
            _ => frame.raw(&mut writer)?,
        }
    }
    original_totals(&reader, index)?;
    if mode == "trailing-byte" {
        writer.write_all(&[0])?;
        changed = true;
    }
    if !changed || first.is_some() {
        return Err(bad("framing mutation absent"));
    }
    writer.sync_all()?;
    println!(
        "{}",
        serde_json::json!({"mode":mode, "source_records":index.records,
        "wire_bytes":writer.metadata()?.len(), "source_decoded_bytes":reader.decoded})
    );
    Ok(())
}

pub fn run(
    source: &Path,
    index_path: &Path,
    original_path: &Path,
    output: &Path,
    mode: &str,
    args: &[String],
) -> io::Result<()> {
    let index = index::load(index_path)?;
    let file = input(source)?;
    if file.metadata()?.len() != index.source_bytes {
        return Err(bad("indexed source length"));
    }
    let mut reader = Input::new(file)?;
    if reader.context != index.context {
        return Err(bad("indexed context"));
    }
    if [
        "drop-first",
        "swap-first-two",
        "omit-completion",
        "truncate-last-byte",
        "trailing-byte",
    ]
    .contains(&mode)
    {
        return malformed(reader, output, &index, mode);
    }
    let new_context = if mode == "rebind" {
        context(args)?
    } else {
        reader.context
    };
    let tail = [
        "cost",
        "valid-output-payload",
        "valid-output-topology",
        "valid-output-payload-rebound",
        "valid-output-topology-rebound",
        "omit-terminal",
    ]
    .contains(&mode);
    let patch = match mode {
        "continuation" => Some(&index.continuation),
        "generation" => Some(&index.generation),
        _ => None,
    };
    if !tail && patch.is_none() && mode != "rebind" && mode != "rechain" {
        return Err(bad("mutation mode"));
    }
    let original = if tail {
        original_result(original_path, &index)?
    } else {
        Vec::new()
    };
    let replacement = if tail {
        terminal(&index, &original, output, mode)?
    } else {
        Vec::new()
    };
    let mut writer = Output::new(create(output)?, new_context)?;
    let (mut offset, mut changed) = (0u64, 0usize);
    let mut suffix = Vec::new();
    let mut suffix_start = None;
    while let Some(frame) = reader.next()? {
        let end = offset + frame.decoded_len() as u64;
        if tail && (suffix_start.is_some() || end > index.terminal_offset) {
            if !frame.terminal() {
                suffix_start.get_or_insert(offset);
                if suffix.len() + frame.decoded_len() > RESULT + CHUNK + 45 {
                    return Err(bad("terminal suffix cap"));
                }
                suffix.extend_from_slice(&frame.decoded()?);
            }
        } else if let Some(patch) = patch {
            let patch_end = patch.offset + patch.before.len() as u64;
            if end > patch.offset && offset < patch_end {
                let mut bytes = frame.decoded()?;
                let start = offset.max(patch.offset);
                let finish = end.min(patch_end);
                for position in start..finish {
                    let a = (position - offset) as usize;
                    let b = (position - patch.offset) as usize;
                    if bytes[a] != patch.before[b] {
                        return Err(bad("indexed patch before bytes differ"));
                    }
                    bytes[a] = patch.after[b];
                    changed += 1;
                }
                writer.emit(&Frame::from_decoded(&bytes)?)?;
            } else {
                writer.emit(&frame)?;
            }
        } else {
            writer.emit(&frame)?;
        }
        offset = end;
    }
    original_totals(&reader, &index)?;
    if let Some(patch) = patch {
        if changed != patch.before.len() {
            return Err(bad("fixed mutation incomplete"));
        }
    }
    if tail {
        let start = suffix_start.ok_or_else(|| bad("terminal suffix absent"))?;
        let prefix = (index.terminal_offset - start) as usize;
        if suffix.len() != prefix + 45 + original.len()
            || suffix[prefix] != 6
            || suffix[prefix + 1..prefix + 45] != index.terminal
            || suffix[prefix + 45..] != original
        {
            return Err(bad("indexed terminal suffix differs"));
        }
        suffix.truncate(prefix);
        suffix.extend_from_slice(&replacement);
        for bytes in suffix.chunks(CHUNK) {
            writer.emit(&Frame::from_decoded(bytes)?)?;
        }
        writer.emit(&Frame::end())?;
    }
    writer.writer.sync_all()?;
    println!(
        "{}",
        serde_json::json!({"mode":mode, "source_records":index.records,
        "wire_bytes":writer.wire, "decoded_bytes":writer.decoded, "frames":writer.frames,
        "source_sha256":index.source_sha256, "fixed_changed_bytes":changed, "terminal_suffix_rebuilt":tail})
    );
    Ok(())
}
