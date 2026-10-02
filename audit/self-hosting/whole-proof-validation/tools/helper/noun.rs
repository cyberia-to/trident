//! Regenerate a complete canonical noun DAG after altering one reachable node.
//! Construction and codec validation only; no formula evaluation.
use super::bad;
use nox::{artifact, encode, Reduction};
use std::{collections::BTreeMap, io};

const LIMITS: artifact::Limits = artifact::Limits {
    max_bytes: 16_777_216,
    max_nodes: 196_608,
    max_depth: 4096,
};

pub fn check(bytes: &[u8]) -> io::Result<()> {
    let mut arena = Reduction::<262144>::try_new_boxed().map_err(|_| bad("codec arena"))?;
    let root = artifact::decode(&mut arena, bytes, LIMITS).map_err(|_| bad("codec decode"))?;
    let canonical = artifact::encode(&arena, root, LIMITS).map_err(|_| bad("codec encode"))?;
    if canonical != bytes {
        return Err(bad("canonical roundtrip differs"));
    }
    Ok(())
}

pub fn regenerate(bytes: &[u8], topology: bool) -> io::Result<Vec<u8>> {
    check(bytes)?;
    let mut arena = Reduction::<262144>::try_new_boxed().map_err(|_| bad("rebuild arena"))?;
    let mut map = BTreeMap::new();
    let count = u32::from_le_bytes(bytes[40..44].try_into().map_err(|_| bad("count"))?);
    let mut cursor = 44;
    let mut changed = false;
    for _ in 0..count {
        let original: [u8; 32] = bytes[cursor..cursor + 32]
            .try_into()
            .map_err(|_| bad("particle"))?;
        let length = bytes[cursor + 32] as usize;
        let payload = &bytes[cursor + 33..cursor + 33 + length];
        let node = match encode::decode(payload).map_err(|_| bad("payload decode"))? {
            encode::DecodedData::Atom(mut value) => {
                if !topology && !changed {
                    // Toggle a low bit while preserving canonical field range.
                    let altered = (value.as_u64() ^ 1) % 0xffff_ffff_0000_0001;
                    let encode::DecodedData::Atom(next) =
                        encode::decode(&altered.to_le_bytes()).map_err(|_| bad("altered atom"))?
                    else {
                        return Err(bad("altered atom kind"));
                    };
                    value = next;
                    changed = true;
                }
                arena.atom(value)
            }
            encode::DecodedData::Pair { left, right } => {
                let mut l = *map.get(&left).ok_or_else(|| bad("prior left"))?;
                let mut r = *map.get(&right).ok_or_else(|| bad("prior right"))?;
                if topology && !changed && left != right {
                    std::mem::swap(&mut l, &mut r);
                    changed = true;
                }
                arena.pair(l, r)
            }
        }
        .ok_or_else(|| bad("rebuild allocation"))?;
        map.insert(original, node);
        cursor += 33 + length;
    }
    if !changed || cursor != bytes.len() {
        return Err(bad("mutation absent or framing"));
    }
    let original_root: [u8; 32] = bytes[8..40].try_into().map_err(|_| bad("root"))?;
    let root = *map.get(&original_root).ok_or_else(|| bad("rebuilt root"))?;
    let result = artifact::encode(&arena, root, LIMITS).map_err(|_| bad("rebuilt encode"))?;
    if result[8..40] == bytes[8..40] {
        return Err(bad("root identity unchanged"));
    }
    drop(arena);
    check(&result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regenerated_payload_and_topology_are_canonical_new_nouns() {
        let mut arena = Reduction::<16>::new();
        let encode::DecodedData::Atom(a) = encode::decode(&7u64.to_le_bytes()).unwrap() else {
            panic!("atom");
        };
        let encode::DecodedData::Atom(b) = encode::decode(&9u64.to_le_bytes()).unwrap() else {
            panic!("atom");
        };
        let l = arena.atom(a).unwrap();
        let r = arena.atom(b).unwrap();
        let root = arena.pair(l, r).unwrap();
        let original = artifact::encode(&arena, root, LIMITS).unwrap();
        for topology in [false, true] {
            let altered = regenerate(&original, topology).unwrap();
            assert_ne!(original[8..40], altered[8..40]);
            check(&altered).unwrap();
        }
    }
}
