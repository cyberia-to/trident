use super::*;
use schema::*;

fn config() -> (Vec<Module>, Options) {
    (
        vec![Module {
            path: "demo".into(),
            origin: "fixture".into(),
            version: "1".into(),
            source: b"raw\0\r\n\xff".to_vec(),
        }],
        Options {
            input: 0,
            output: 0,
            optimization: 0,
            cfg: vec!["release".into()],
        },
    )
}

#[test]
fn containers_roundtrip_and_extracted_program_executes_on_nox() {
    let mut ar = Reduction::<4096>::new();
    let (compiler, job, success, failure) = fixture(&mut ar).unwrap();
    let mut dest = Reduction::<4096>::new();
    let mut roots = Vec::new();
    for root in [compiler, job, success, failure] {
        let bytes = nox::artifact::encode(&ar, root, TRANSPORT).unwrap();
        let copy = nox::artifact::decode(&mut dest, &bytes, TRANSPORT).unwrap();
        assert_eq!(ar.digest(root), dest.digest(copy));
        assert_eq!(
            nox::artifact::encode(&dest, copy, TRANSPORT).unwrap(),
            bytes
        );
        roots.push(copy);
    }
    let job = validate::job(&mut dest, roots[1], roots[0], FIXTURE_CAPS).unwrap();
    assert_eq!(job.entry(), "demo");
    assert_eq!(job.function(), "main");
    let formula = match validate::result(&mut dest, roots[2], roots[1], &job).unwrap() {
        validate::ResultValue::Success { formula, .. } => formula,
        _ => panic!("expected success"),
    };
    let input = atom(&mut dest, 0).unwrap();
    match nox::reduce(
        &mut dest,
        input,
        formula,
        10,
        &nox::NullCalls,
        &mut nox::NoTrace,
    ) {
        nox::Outcome::Ok(n, 9) => assert_eq!(data::value(&dest, n), Ok(14)),
        other => panic!("{other:?}"),
    }
    match validate::result(&mut dest, roots[3], roots[1], &job).unwrap() {
        validate::ResultValue::Failure(ds) => assert_eq!(ds[0].code, 6),
        _ => panic!("expected diagnostics"),
    }
}

#[test]
fn package_admission_preserves_raw_bytes_without_parsing_source() {
    let mut ar = Reduction::<4096>::new();
    let (compiler, _, _, _) = fixture(&mut ar).unwrap();
    let (modules, options) = config();
    let root = job(
        &mut ar,
        compiler,
        &modules,
        "demo",
        "main",
        &options,
        &FIXTURE_CAPS,
    )
    .unwrap();
    let admitted = validate::job(&mut ar, root, compiler, FIXTURE_CAPS).unwrap();
    assert_eq!(admitted.modules(), modules);
    assert_eq!(admitted.options(), &options);
}

#[test]
fn producer_binding_changes_job_identity_without_changing_executable_bytes() {
    let mut ar = Reduction::<4096>::new();
    let (compiler, _, _, _) = fixture(&mut ar).unwrap();
    let (modules, options) = config();
    let q = atom(&mut ar, 1).unwrap();
    let v = atom(&mut ar, 99).unwrap();
    let f = pair(&mut ar, q, v).unwrap();
    let compiler2 = artifact(&mut ar, f, 1, 1).unwrap();
    let program = artifact(&mut ar, f, 0, 0).unwrap();
    let expected = nox::artifact::encode(&ar, program, TRANSPORT).unwrap();
    let mut jobs = Vec::new();
    let mut results = Vec::new();
    for compiler in [compiler, compiler2] {
        let root = job(
            &mut ar,
            compiler,
            &modules,
            "demo",
            "main",
            &options,
            &FIXTURE_CAPS,
        )
        .unwrap();
        let admitted = validate::job(&mut ar, root, compiler, FIXTURE_CAPS).unwrap();
        let result = success(&mut ar, root, program).unwrap();
        assert!(matches!(
            validate::result(&mut ar, result, root, &admitted).unwrap(),
            validate::ResultValue::Success { artifact, .. } if artifact == program
        ));
        jobs.push(root);
        results.push(result);
        assert_eq!(
            nox::artifact::encode(&ar, program, TRANSPORT).unwrap(),
            expected
        );
    }
    assert_ne!(ar.digest(jobs[0]), ar.digest(jobs[1]));
    assert_ne!(ar.digest(results[0]), ar.digest(results[1]));
    assert!(validate::job(&mut ar, jobs[0], compiler2, FIXTURE_CAPS).is_err());
    let job1 = validate::job(&mut ar, jobs[0], compiler, FIXTURE_CAPS).unwrap();
    let job2 = validate::job(&mut ar, jobs[1], compiler2, FIXTURE_CAPS).unwrap();
    assert!(validate::result(&mut ar, results[1], jobs[0], &job1).is_err());
    assert!(validate::result(&mut ar, results[0], jobs[0], &job2).is_err());
}

#[test]
fn ordering_names_profiles_and_versions_are_explicit() {
    let mut ar = Reduction::<4096>::new();
    let (compiler, _, _, _) = fixture(&mut ar).unwrap();
    let (modules, options) = config();
    let mut cases = Vec::new();
    let mut wrong = modules.clone();
    wrong[0].path = "../escape".into();
    cases.push((wrong, options.clone(), "demo"));
    let mut wrong = modules.clone();
    wrong.push(wrong[0].clone());
    cases.push((wrong, options.clone(), "demo"));
    let mut wrong = modules.clone();
    wrong[0].version.clear();
    cases.push((wrong, options.clone(), "demo"));
    let mut wrong = modules.clone();
    wrong[0].path = "a.demo".into();
    cases.push((wrong, options.clone(), "a.demo"));
    let mut wrong = options.clone();
    wrong.cfg = vec!["release".into(), "debug".into()];
    cases.push((modules.clone(), wrong, "demo"));
    let mut wrong = options.clone();
    wrong.cfg = vec!["release".into(), "release".into()];
    cases.push((modules.clone(), wrong, "demo"));
    let mut wrong = options.clone();
    wrong.input = 1;
    cases.push((modules.clone(), wrong, "demo"));
    let mut wrong = options.clone();
    wrong.optimization = 1;
    cases.push((modules.clone(), wrong, "demo"));
    cases.push((modules.clone(), options.clone(), "absent"));
    for (modules, options, entry) in cases {
        let root = job(
            &mut ar,
            compiler,
            &modules,
            entry,
            "main",
            &options,
            &FIXTURE_CAPS,
        )
        .unwrap();
        assert!(validate::job(&mut ar, root, compiler, FIXTURE_CAPS).is_err());
    }
    let first = job(
        &mut ar,
        compiler,
        &modules,
        "demo",
        "main",
        &options,
        &FIXTURE_CAPS,
    )
    .unwrap();
    let mut second = modules;
    second[0].source.push(b' ');
    let second = job(
        &mut ar,
        compiler,
        &second,
        "demo",
        "main",
        &options,
        &FIXTURE_CAPS,
    )
    .unwrap();
    assert_ne!(ar.digest(first), ar.digest(second));
}

#[test]
fn limits_cannot_raise_host_caps_or_reset_shared_validation_work() {
    let mut ar = Reduction::<4096>::new();
    let (compiler, _, _, _) = fixture(&mut ar).unwrap();
    let (modules, options) = config();
    for (i, v) in [
        (0, 1),
        (1, 0),
        (4, 1),
        (4, 150),
        (5, 100),
        (6, 1),
        (7, 1),
        (8, 0),
        (9, 3001),
        (10, 0),
    ] {
        let mut limits = FIXTURE_CAPS;
        limits[i] = v;
        let root = job(
            &mut ar, compiler, &modules, "demo", "main", &options, &limits,
        )
        .unwrap();
        assert!(
            validate::job(&mut ar, root, compiler, FIXTURE_CAPS).is_err(),
            "limit {i}={v}"
        );
    }
    let mut more = modules.clone();
    let mut extra = modules[0].clone();
    extra.path = "second".into();
    more.push(extra);
    let mut limits = FIXTURE_CAPS;
    limits[0] = modules[0].source.len() as u64;
    let root = job(&mut ar, compiler, &more, "demo", "main", &options, &limits).unwrap();
    assert!(validate::job(&mut ar, root, compiler, FIXTURE_CAPS).is_err());
}

#[test]
fn compiler_container_obeys_host_and_requested_limits_independently_of_job_size() {
    let mut ar = Reduction::<4096>::new();
    let (modules, options) = config();
    let mut tree = atom(&mut ar, 42).unwrap();
    for _ in 0..70 {
        tree = pair(&mut ar, tree, tree).unwrap();
    }
    let compiler = artifact(&mut ar, tree, 1, 1).unwrap();
    let mut limits = FIXTURE_CAPS;
    limits[7] = 50;
    let root = job(
        &mut ar, compiler, &modules, "demo", "main", &options, &limits,
    )
    .unwrap();
    assert!(nox::artifact::encode(
        &ar,
        root,
        nox::artifact::Limits {
            max_depth: 50,
            ..TRANSPORT
        }
    )
    .is_ok());
    assert!(validate::job(&mut ar, root, compiler, FIXTURE_CAPS).is_err());
    let mut caps = FIXTURE_CAPS;
    caps[7] = 60;
    assert!(validate::job(&mut ar, root, compiler, caps).is_err());
}

#[test]
fn result_diagnostics_require_valid_job_spans_order_and_nonempty_payload() {
    let mut ar = Reduction::<4096>::new();
    let (compiler, job_root, _, _) = fixture(&mut ar).unwrap();
    let admitted = validate::job(&mut ar, job_root, compiler, FIXTURE_CAPS).unwrap();
    let d = Diagnostic {
        module: 0,
        start: 0,
        end: 1,
        code: 2,
        message: "syntax".into(),
    };
    let mut wrong = d.clone();
    wrong.end = 1000;
    let mut module = d.clone();
    module.module = 1;
    let mut code = d.clone();
    code.code = 99;
    let mut limit_marker = d.clone();
    limit_marker.code = 8;
    let mut reversed = d.clone();
    reversed.start = 2;
    reversed.end = 3;
    for ds in [
        vec![],
        vec![wrong],
        vec![module],
        vec![code],
        vec![limit_marker],
        vec![reversed, d.clone()],
    ] {
        let root = failure(&mut ar, job_root, &ds).unwrap();
        assert!(validate::result(&mut ar, root, job_root, &admitted).is_err());
    }
    let bad = success(&mut ar, job_root, compiler).unwrap();
    assert!(validate::result(&mut ar, bad, job_root, &admitted).is_err());
    let id = identity(&mut ar, job_root).unwrap();
    let two = atom(&mut ar, 2).unwrap();
    let bad = record(&mut ar, RESULT, &[id, two, compiler]).unwrap();
    assert!(validate::result(&mut ar, bad, job_root, &admitted).is_err());
}

#[test]
fn record_shape_and_arity_fail_before_success() {
    let mut ar = Reduction::<4096>::new();
    let (compiler, _, _, _) = fixture(&mut ar).unwrap();
    let zero = atom(&mut ar, 0).unwrap();
    let short = record(&mut ar, JOB, &[zero]).unwrap();
    for root in [zero, short, compiler] {
        assert!(validate::job(&mut ar, root, compiler, FIXTURE_CAPS).is_err());
    }
}

#[test]
fn protocol_vectors_are_reproducible() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("audit/self-hosting/job-vectors.json");
    assert_eq!(vectors().unwrap(), std::fs::read_to_string(path).unwrap());
}
