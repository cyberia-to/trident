use super::support::{self, Result};

#[test]
fn guest_admission_charges_repeated_entry_bytes_to_exact_shared_allowance() {
    support::worker(|| support::admission_boundary(&support::source("2+3*4")));
}

#[test]
fn operator_stack_capacity_is_checked_before_push() {
    support::worker(|| {
        let exact = format!("{}1{}", "(".repeat(64), ")".repeat(64));
        // Keep both the original model allowance and the installed CLI JOB1
        // metadata: distinct cap atoms also consume the fixed lifetime arena.
        for limit in [4096, 8192] {
            let mut caps = support::CAPS;
            caps[0] = limit;
            caps[3] = limit;
            assert_eq!(
                support::value(support::compile_package(
                    &[support::module(&support::source(&exact))],
                    "sample",
                    "main",
                    support::options(),
                    caps,
                )),
                1
            );
        }
        let excess = format!("{}1{}", "(".repeat(65), ")".repeat(65));
        let source = support::source(&excess);
        let error = support::error(&source, 7);
        assert_eq!(&source[error.start as usize..error.end as usize], b"(");
    });
}

#[test]
fn irrelevant_package_metadata_and_cfg_never_enter_program_identity() {
    support::worker(|| {
        let source = support::source("2+3*4");
        let original = match support::compile(&source) {
            Result::Program { bytes, .. } => bytes,
            other => panic!("{other:?}"),
        };
        let mut changed = support::module(&source);
        changed.origin = "another origin".into();
        changed.version = "42".into();
        let mut options = support::options();
        options.cfg = vec!["release".into()];
        let mut caps = support::CAPS;
        caps[0] = 3000;
        caps[8] = 50_000_000;
        caps[10] = 16384;
        match support::compile_package(&[changed], "sample", "main", options, caps) {
            Result::Program { bytes, .. } => assert_eq!(bytes, original),
            other => panic!("{other:?}"),
        }
    });
}

#[test]
fn requested_sequence_cap_restricts_live_parser_stacks() {
    support::worker(|| {
        let mut caps = support::CAPS;
        caps[2] = 1;
        caps[3] = 1;
        assert_eq!(
            support::value(support::compile_package(
                &[support::module(&support::source("7"))],
                "sample",
                "main",
                support::options(),
                caps
            )),
            7
        );
        match support::compile_package(
            &[support::module(&support::source("2+3"))],
            "sample",
            "main",
            support::options(),
            caps,
        ) {
            Result::Errors(errors) => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].code, 7);
            }
            other => panic!("{other:?}"),
        }
    });
}
