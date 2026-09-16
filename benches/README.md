# Compiler reference fixtures

Trident owns portable `.tri` benchmark inputs in `harnesses/` and independent
Rust reference implementations in `references/`. These describe computations
and expected results shared by targets.

Trisha owns [hand Triton baselines](../../trisha/baselines/triton/) and the
`trisha bench` command. There is no `trident bench` command or baseline tree in
this repository.

From the Trisha checkout:

```sh
trisha bench baselines/triton
trisha bench baselines/triton --full
```

Each `.bench.toml` fixture specifies source, hand assembly, input and expected
output. Both programs must execute and match that output before their cycle
counts are compared. `--full` additionally proves and verifies both programs.
Missing or failing fixtures make coverage incomplete and the command fails.
The historical baseline gate remains incomplete; see
[release validation](../audit/warrior-release-validation.md).

Target-neutral references remain here. Triton ABI wrappers, instruction-level
expectations and Neptune transaction fixtures belong in Trisha. The detailed
ownership review is in [target ownership](../audit/target-ownership.md).
