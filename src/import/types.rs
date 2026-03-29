// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! MIR type → Goldilocks field element width mapping.
//!
//! Every Rust type that passes Rs edition restrictions maps to a fixed
//! number of Goldilocks field elements (each 64 bits).

use std::collections::BTreeMap;

use mir_format::{MirStruct, MirType};

/// Compute the field-element width of a MIR type.
///
/// Returns the number of Goldilocks field elements needed to represent this type.
pub fn field_width(ty: &MirType, structs: &BTreeMap<String, &MirStruct>) -> u32 {
    match ty {
        // Single field element types (all fit in 64-bit Goldilocks).
        MirType::Bool
        | MirType::U8
        | MirType::U16
        | MirType::U32
        | MirType::U64
        | MirType::I8
        | MirType::I16
        | MirType::I32
        | MirType::I64 => 1,

        // 128-bit types need two field elements.
        MirType::U128 | MirType::I128 => 2,

        // Unit has no representation.
        MirType::Unit => 0,

        // Tuples: sum of field widths.
        MirType::Tuple(fields) => fields.iter().map(|f| field_width(f, structs)).sum(),

        // Arrays: element width * length.
        MirType::Array(elem, len) => field_width(elem, structs) * (*len as u32),

        // Structs: sum of field widths from struct definition.
        MirType::Struct(name) => {
            if let Some(def) = structs.get(name.as_str()) {
                def.fields.iter().map(|f| field_width(&f.ty, structs)).sum()
            } else {
                1 // fallback for unknown structs
            }
        }

        // Rs edition refs are stack values — same width as the referent.
        MirType::Ref(inner) => field_width(inner, structs),
    }
}

/// Build a struct lookup map from a slice of MirStruct definitions.
pub fn struct_map(structs: &[MirStruct]) -> BTreeMap<String, &MirStruct> {
    structs.iter().map(|s| (s.name.clone(), s)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir_format::{MirStructField, MirType};

    fn empty_structs() -> BTreeMap<String, &'static MirStruct> {
        BTreeMap::new()
    }

    #[test]
    fn scalar_widths() {
        let s = empty_structs();
        assert_eq!(field_width(&MirType::Bool, &s), 1);
        assert_eq!(field_width(&MirType::U8, &s), 1);
        assert_eq!(field_width(&MirType::U32, &s), 1);
        assert_eq!(field_width(&MirType::U64, &s), 1);
        assert_eq!(field_width(&MirType::U128, &s), 2);
        assert_eq!(field_width(&MirType::I128, &s), 2);
        assert_eq!(field_width(&MirType::Unit, &s), 0);
    }

    #[test]
    fn composite_widths() {
        let s = empty_structs();
        let tuple = MirType::Tuple(vec![MirType::U32, MirType::U64, MirType::Bool]);
        assert_eq!(field_width(&tuple, &s), 3);

        let array = MirType::Array(Box::new(MirType::U32), 10);
        assert_eq!(field_width(&array, &s), 10);

        let array128 = MirType::Array(Box::new(MirType::U128), 5);
        assert_eq!(field_width(&array128, &s), 10);
    }

    #[test]
    fn struct_width() {
        let point = MirStruct {
            name: "Point".into(),
            fields: vec![
                MirStructField { name: "x".into(), ty: MirType::U32 },
                MirStructField { name: "y".into(), ty: MirType::U32 },
            ],
            variants: None,
        };
        let structs = vec![point];
        let map = struct_map(&structs);
        assert_eq!(field_width(&MirType::Struct("Point".into()), &map), 2);
    }
}
