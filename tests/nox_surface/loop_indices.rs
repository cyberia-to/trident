use super::{nox_options, run_profile};

#[test]
fn unrolled_indices_read_and_write_their_own_iteration() {
    let source = "program indices
const i:U32=2
fn main()->Field {
 let mut words:[Field;3]=[1,2,3]
 for i in 0..2 {
   for i in 0..2 {words[i]=words[i]+1}
   words[i]=words[i]+10
 }
 words[0]*100+words[1]*10+words[i]
}";
    for profile in ["debug", "release"] {
        assert_eq!(run_profile(source, &[], profile), 1443);
    }
}

#[test]
fn bounded_indexed_loop_returns_the_selected_element() {
    let source = "program returning_index
fn pick(words:[Field;3],n:Field)->Field {
 for i in 0..n bounded 3 {if as_field(i)==1 {return words[i]}}
 words[2]
}
fn main(n:Field)->Field {pick([7,19,31],n)+100}";
    for profile in ["debug", "release"] {
        for (input, expected) in [(0, 131), (1, 131), (2, 119), (3, 119)] {
            assert_eq!(run_profile(source, &[input], profile), expected);
        }
    }
}

#[test]
fn runtime_shadow_never_inherits_unrolled_constant_value() {
    let source = "program shadow
fn main(n:Field)->Field {
 let words=[7,19]
 let mut result:Field=0
 for i in 0..2 {let i=as_u32(n)\nresult=words[i]}
 result
}";
    let error = trident::compile_with_options(source, "shadow.tri", &nox_options()).unwrap_err();
    assert!(error.iter().any(|d| d
        .message
        .contains("array index must be a compile-time constant")));
}

#[test]
fn unrolled_index_respects_its_declared_u32_type() {
    let source = "program largest
fn main()->Field {for i in 4294967295..4294967296 {return as_field(i)}\n0}";
    for profile in ["debug", "release"] {
        assert_eq!(run_profile(source, &[], profile), u32::MAX as u64);
    }
    let overflow = source.replace("4294967295..4294967296", "4294967296..4294967297");
    assert!(trident::compile_with_options(&overflow, "overflow.tri", &nox_options()).is_err());
}
