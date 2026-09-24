use nox::{artifact, Reduction};
use trident::{CompileOptions, NativeArtifactProfile as Profile, NATIVE_ARTIFACT_LIMITS as LIMITS};

const SOURCE: &str = "program compiler\nfn main(input: Noun) -> Noun { input }";

#[test]
fn explicit_profiles_change_only_the_art1_envelope_and_preserve_raw_api_bytes() {
    let options = CompileOptions::default();
    let raw = trident::compile_raw_artifact(SOURCE, "compiler.tri", &options, LIMITS).unwrap();
    let explicit = trident::compile_native_artifact(
        SOURCE,
        "compiler.tri",
        &options,
        Profile::RawNoun,
        LIMITS,
    )
    .unwrap();
    let compiler = trident::compile_native_artifact(
        SOURCE,
        "compiler.tri",
        &options,
        Profile::CompilerJob,
        LIMITS,
    )
    .unwrap();
    assert_eq!(raw.bytes, explicit.bytes);
    assert_eq!(raw.particle, explicit.particle);
    assert_ne!(raw.particle, compiler.particle);
    assert_eq!(raw.profile, Profile::RawNoun);
    assert_eq!(compiler.profile, Profile::CompilerJob);
    let mut ar = Reduction::<4096>::new();
    let mut formulas = Vec::new();
    for emitted in [raw, compiler] {
        let root = artifact::decode(&mut ar, &emitted.bytes, LIMITS).unwrap();
        assert_eq!(
            nox::data::digest_bytes(ar.digest(root).unwrap()),
            emitted.particle
        );
        assert_eq!(
            ar.atom_value(ar.head(root).unwrap()).unwrap().as_u64(),
            0x41525431
        );
        let mut body = ar.tail(root).unwrap();
        for expected in [0, emitted.profile.value(), emitted.profile.value()] {
            assert_eq!(
                ar.atom_value(ar.head(body).unwrap()).unwrap().as_u64(),
                expected
            );
            body = ar.tail(body).unwrap();
        }
        formulas.push(*ar.digest(ar.head(body).unwrap()).unwrap());
        assert_eq!(ar.atom_value(ar.tail(body).unwrap()).unwrap().as_u64(), 0);
        assert_eq!(artifact::encode(&ar, root, LIMITS).unwrap(), emitted.bytes);
    }
    assert_eq!(formulas[0], formulas[1]);
}

#[test]
fn compiler_profile_retains_pure_noun_entry_and_seed_emission_limits() {
    for source in [
        "program p\nfn main(x: Field) -> Field { x }",
        "program p\nfn main(x: Noun) -> Noun { nox_noun_atom(divine()) }",
        "program p\nfn main(x: Noun) -> Noun { write_io(7) x }",
    ] {
        assert!(trident::compile_native_artifact(
            source,
            "p.tri",
            &CompileOptions::default(),
            Profile::CompilerJob,
            LIMITS
        )
        .is_err());
    }
    assert!(trident::compile_native_artifact(
        SOURCE,
        "compiler.tri",
        &CompileOptions::default(),
        Profile::CompilerJob,
        artifact::Limits {
            max_bytes: 1,
            ..LIMITS
        }
    )
    .is_err());
}
