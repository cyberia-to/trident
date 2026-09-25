use crate::parse_source_silent;

#[test]
fn every_consumed_path_dot_requires_an_identifier_at_the_original_position() {
    for (prefix, following, suffix) in [
        ("program entry use a.", "", ""),
        ("program entry use a.", "fn", " main()->Field{7}"),
        ("program entry use a.", "pub", " fn main()->Field{7}"),
        ("program entry use a.", "use", " b fn main()->Field{7}"),
        ("program entry use a. ", ".", "b fn main()->Field{7}"),
        ("module a.", "", ""),
        ("program entry fn main()->Field{a.X.", "}", ""),
        ("program entry fn main()->Field{a.value.", "(", ")}"),
        (
            "program entry fn main()->Field{let b:Box.",
            "=",
            "Box{x:7} b.x}",
        ),
        ("program entry fn main(x:lib.Box.", ")", "->Field{7}"),
        ("program entry fn main()->lib.Box.", "{", "lib.Box{x:7}}"),
    ] {
        let source = format!("{prefix}{following}{suffix}");
        let errors = parse_source_silent(&source, "path.tri").unwrap_err();
        assert!(
            errors.iter().any(|error| {
                error.message.starts_with("expected identifier, found")
                    && error.span.start as usize == prefix.len()
                    && error.span.end as usize == prefix.len() + following.len()
            }),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn dotted_paths_keep_comments_whitespace_and_original_import_spans() {
    let import = "use lib // first part\n . values";
    let source = format!("module alpha // owner\n . beta {import} pub fn value()->Field{{7}}");
    let file = parse_source_silent(&source, "path.tri").unwrap();
    assert_eq!(file.name.node, "alpha.beta");
    assert_eq!(file.uses.len(), 1);
    let path = &file.uses[0];
    assert_eq!(path.node.as_dotted(), "lib.values");
    assert_eq!(
        &source[path.span.start as usize..path.span.end as usize],
        import
    );
    for source in [
        "program entry fn main()->Field{a // owner\n . X}",
        "program entry fn main()->Field{a . value()}",
        "program entry fn main(x:lib // owner\n . Box)->lib . Box{x}",
        "program entry fn main()->Field{let b:lib . Box=lib . Box{x:7} b . x}",
    ] {
        parse_source_silent(source, "path.tri").unwrap();
    }
}
