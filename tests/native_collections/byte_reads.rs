use super::{atom, compile, equal, pair, run, support, Arena, Bytes};

#[test]
fn every_byte_value_survives_each_packed_lane_without_neighbor_contamination() {
    support::worker(|| {
        let program = compile(
            "let b=bytes.from_noun(input,as_u32(4),as_u32(100))
             noun.pair(noun.atom(as_field(bytes.get(b,as_u32(0)))),
                 noun.pair(noun.atom(as_field(bytes.get(b,as_u32(1)))),
                     noun.pair(noun.atom(as_field(bytes.get(b,as_u32(2)))),
                         noun.atom(as_field(bytes.get(b,as_u32(3)))))))",
        );
        for value in 0..=255u8 {
            let content = [value, value ^ 0x55, value ^ 0xaa, 255 - value];
            let mut arena = Arena::new();
            let input = Bytes::from_slice(&mut arena, &content, 4)
                .unwrap()
                .encode(&mut arena)
                .unwrap();
            let mut expected = atom(&mut arena, u64::from(content[3])).unwrap();
            for byte in content[..3].iter().rev() {
                let head = atom(&mut arena, u64::from(*byte)).unwrap();
                expected = pair(&mut arena, head, expected).unwrap();
            }
            let actual = run(&mut arena, &program, input)
                .unwrap_or_else(|error| panic!("{content:?}: {error}"));
            equal(&arena, actual, expected);
        }
    });
}
