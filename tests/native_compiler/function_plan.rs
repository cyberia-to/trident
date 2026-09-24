use super::{signatures::run_probe, support};

fn plan(source: &str, declarations: u32) -> Result<Vec<u64>, u64> {
    static PROBE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let code = PROBE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_function_plan.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes
    });
    let (mut arena, result) = run_probe(source, 4096, code);
    let error = arena
        .atom_value(arena.head(result).unwrap())
        .unwrap()
        .as_u64();
    if error != 0 {
        return Err(error);
    }
    let payload = arena.tail(result).unwrap();
    let order = arena.head(payload).unwrap();
    let mut slots = arena.tail(payload).unwrap();
    let order = support::data::Seq::decode(&mut arena, order, 4096, 1_000_000).unwrap();
    let ids: Vec<_> = (0..order.len())
        .map(|i| {
            arena
                .atom_value(order.get(&arena, i).unwrap())
                .unwrap()
                .as_u64()
        })
        .collect();
    let root = slots;
    let height = u32::BITS - declarations.saturating_sub(1).leading_zeros();
    for id in 0..declarations {
        slots = root;
        for bit in (0..height).rev() {
            slots = if (id >> bit) & 1 == 0 {
                arena.head(slots).unwrap()
            } else {
                arena.tail(slots).unwrap()
            };
        }
        let expected = ids
            .iter()
            .position(|v| *v == u64::from(id))
            .map_or(0, |i| i as u64 + 1);
        assert_eq!(
            arena.atom_value(slots).unwrap().as_u64(),
            expected,
            "slot of {id}"
        );
    }
    Ok(ids)
}

#[test]
fn final_callable_graph_rejects_cycles_even_in_unused_and_unselected_code() {
    support::worker(|| {
        for (source, count) in [
            ("program sample fn main()->Field{main()}", 1),
            (
                "program sample fn f()->Field{g()} fn g()->Field{f()} fn main()->Field{7}",
                3,
            ),
            (
                "program sample fn f()->Field{if false{return f()} 7} fn main()->Field{7}",
                2,
            ),
            ("program sample fn f()->Field{f()} fn main()->Field{7}", 2),
        ] {
            assert_eq!(plan(source, count), Err(5), "{source}");
        }
        assert_eq!(
            plan(
                "program sample fn f()->Field{f()} fn f()->Field{7} fn main()->Field{f()}",
                3
            ),
            Ok(vec![1, 2])
        );
    });
}

#[test]
fn table_order_uses_full_names_and_only_reachable_final_definitions() {
    support::worker(|| {
        for (source,count,expected) in [
            ("program sample fn z()->Field{7} fn a()->Field{z()} fn main()->Field{a()}",3,vec![1,2,0]),
            ("program sample fn z()->Field{7} fn main()->Field{7}",2,vec![1]),
            ("program sample fn common_prefix_z()->Field{7} fn common_prefix_a()->Field{common_prefix_z()} fn main()->Field{common_prefix_a()}",3,vec![1,0,2]),
            ("program sample fn aa()->Field{7} fn a()->Field{aa()} fn main()->Field{a()}",3,vec![1,0,2]),
        ] { assert_eq!(plan(source,count),Ok(expected),"{source}"); }
    });
}
