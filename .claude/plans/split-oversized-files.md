# Split Oversized Files

## Context

9 files in `src/` exceed the 500-line quality limit (`reference/quality.md`).
Clean architecture is a prerequisite for forward progress. This plan
splits 8 of them into focused submodules following the established
`cost/mod.rs` pattern (minimal coordinator + explicit re-exports).

`syntax/grammar/trident.rs` (594 LOC) is skipped — it's pure
declarative grammar data. Splitting it would hurt readability.

## Convention

Follow `src/cost/mod.rs` as the gold standard:
- `mod.rs`: submodule declarations + re-exports only
- Three visibility levels: `pub`, `mod`, `pub(crate)`
- Re-export only high-value consumer-facing types
- Each new file owns one logical concern, targets ≤300 LOC

## Deduplication First

`bench.rs` and `audit.rs` share 3 identical functions:
`find_baseline_files()`, `find_project_root()`, `resolve_bench_dir()`.
Extract to `src/cli/baseline.rs` (~65 LOC), import from both.

## Splits (ordered by LOC, biggest first)

### 1. `src/cost/stack_verifier.rs` → `src/cost/stack_verifier/` (1142 LOC)

| New file | Contents | ~LOC |
|----------|----------|------|
| `mod.rs` | Re-exports | 15 |
| `executor.rs` | `StackState` struct + `execute()` + `execute_line()` | 420 |
| `equivalence.rs` | `verify_equivalent()` + `diagnose_failure()` + `generate_test_stack()` | 220 |
| `scoring.rs` | `score_candidate()` + `score_neural_output()` + `score_neural_improvement()` | 60 |
| `tests.rs` | All 37 unit tests | 340 |

### 2. `src/cli/train.rs` → `src/cli/train/` (1015 LOC)

| New file | Contents | ~LOC |
|----------|----------|------|
| `mod.rs` | `TrainArgs`, `TrainAction`, `cmd_train()`, `run_training_loop()` + re-exports | 260 |
| `stage1.rs` | `run_stage1()` + `eval_holdout_validity()` | 200 |
| `stage2.rs` | `run_stage2()` | 150 |
| `eval.rs` | `CompiledFile`, `FileEval`, `eval_files()` | 180 |
| `display.rs` | `display_epoch_table()` + `display_file_table()` | 110 |
| `corpus.rs` | `cmd_train_reset()`, `walkdir*()`, `compile_corpus()`, `discover_corpus()`, `find_repo_root()`, `short_path()` | 130 |

### 3. `src/cli/bench.rs` → `src/cli/bench/` (977 LOC)

| New file | Contents | ~LOC |
|----------|----------|------|
| `mod.rs` | `BenchArgs`, `DimTiming`, `ModuleBench`, `cmd_bench()` + re-exports | 300 |
| `display.rs` | `render_insn_table()`, `render_full_table()`, `fmt_ms()`, `fmt_verify_row()`, `fmt_rust()` | 180 |
| `execution.rs` | `run_trisha_timed()`, `run_dimension()`, `verify_dimension()`, `run_rust_reference()` | 130 |
| `neural.rs` | `compile_neural_tasm_inline()`, `derive_neural_tasm_path()` | 140 |
| `inputs.rs` | `LiveInputs`, `parse_inputs_file()` | 50 |

File discovery moves to shared `baseline.rs` (see dedup above).

### 4. `src/cli/audit.rs` → `src/cli/audit/` (645 LOC)

| New file | Contents | ~LOC |
|----------|----------|------|
| `mod.rs` | `AuditArgs`, `cmd_audit()` dispatcher + re-exports | 40 |
| `exec.rs` | `DimAudit`, `ModuleAudit`, `AuditStatus`, `cmd_audit_exec()`, `audit_run_pipeline()`, `print_dim_failures()` | 320 |
| `symbolic.rs` | `cmd_audit_symbolic()`, `run_z3_analysis()` | 160 |
| `equiv.rs` | `EquivArgs`, `cmd_equiv()` | 60 |

File discovery moves to shared `baseline.rs`.

### 5. `src/neural/data/tir_graph.rs` → `src/neural/data/tir_graph/` (619 LOC)

| New file | Contents | ~LOC |
|----------|----------|------|
| `mod.rs` | `TirGraph` struct + public API + re-exports | 80 |
| `types.rs` | `EdgeKind`, `FieldType`, `OpKind`, `NUM_OP_KINDS`, `OpKind::from_tir_op()` | 160 |
| `node.rs` | `TirNode`, `NODE_FEATURE_DIM`, `feature_vector()`, `output_field_type()` | 80 |
| `builder.rs` | `flatten_ops()`, `extract_data_deps()`, `stack_effect_from_kind()`, `extract_mem_order()`, `StackEntry` | 200 |
| `tests.rs` | All 11 unit tests | 130 |

### 6. `src/neural/training/augment.rs` → `src/neural/training/augment/` (568 LOC)

| New file | Contents | ~LOC |
|----------|----------|------|
| `mod.rs` | `AugmentConfig`, `augment_pairs()`, `Xorshift64` + re-exports | 140 |
| `tasm.rs` | `random_walk_tasm()`, `instructions_are_independent()`, `equivalent_substitutions()`, `verify_substitution()` | 170 |
| `tir.rs` | `insert_dead_code()` | 130 |
| `tests.rs` | All 7 unit tests | 110 |

### 7. `src/api/mod.rs` → split into peer files (559 LOC)

| New file | Contents | ~LOC |
|----------|----------|------|
| `mod.rs` | Imports, `CompileOptions`, submodule decls, re-exports | 100 |
| `compile.rs` | `compile()`, `compile_with_options()`, `compile_project*()`, `compile_module()` | 120 |
| `check.rs` | `check()`, `check_project()` | 25 |
| `testing.rs` | `discover_tests()`, `TestResult`, `run_tests()` | 165 |
| `tir.rs` | `build_tir()`, `build_tir_project()` | 75 |
| `bundle.rs` | `compile_to_bundle()` | 100 |

### 8. `src/cli/mod.rs` → extract helpers (514 LOC)

| New file | Contents | ~LOC |
|----------|----------|------|
| `mod.rs` | Submodule declarations + re-exports only | 50 |
| `resolve.rs` | `BattlefieldSelection`, `resolve_battlefield*()`, `ResolvedInput`, `resolve_input()`, `load_project()`, `resolve_options()` | 170 |
| `artifact.rs` | `PreparedArtifact`, `prepare_artifact()`, `audit_or_exit()` | 120 |
| `warrior.rs` | `find_warrior()`, `which_on_path()`, `delegate_to_warrior()` | 80 |
| `files.rs` | `try_load_and_parse()`, `load_and_parse()`, `find_program_source()`, `short_hash()`, `resolve_tri_files()`, `collect_tri_files*()`, `MAX_DIR_DEPTH` | 80 |
| `clients.rs` | `open_codebase()`, `registry_client()`, `registry_url()`, `load_dep_dirs()` | 50 |

## Execution Order

Partitioned by directory for parallel agents (per CLAUDE.md rule):

1. **cli/** — baseline.rs dedup, then mod.rs, then train/, bench/, audit/
2. **cost/** — stack_verifier/
3. **neural/** — tir_graph/, augment/
4. **api/** — mod.rs split

Each split is one atomic commit. All imports updated in the same commit.

## Verification

After each split:
- `cargo check` — zero warnings
- `cargo test` — all tests pass
- `wc -l` on every new file — none exceeds 500

After all splits:
- `find src/ -name '*.rs' | xargs wc -l | awk '$1 > 500'` — only
  `syntax/grammar/trident.rs` (594, grammar exemption) should remain
