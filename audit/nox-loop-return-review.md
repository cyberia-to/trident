# Independent Nox loop-return review

Reviewed `src/ir/tree/lower/nox/loops.rs`, statement dispatch and scope restoration
in `nox.rs`, aggregate layout/type lookup, and the new `tests/nox_surface.rs`
regressions. This was source review; the separately reported lower/surface and
Triton parity runs belong to the implementation owner, not this review.

One actionable correctness issue was found in `Scope::seal_frames_above`:
absorbed branch fallthrough erased local names while retaining their parallel
`tys` entries. An outer aggregate value could consequently be read using a
shadowing inner aggregate's layout. For example, outer `Pair { a:11, b:22 }`
and inner `Reverse { b:99, a:88 }` named `x` could make a subsequent outer
`x.a` select field1 instead of field0. This affects continuations inside a
return-aware loop as well as existing absorbed branches. The implementation
owner was asked to seal type frames too and add an execution regression with
different aggregate field order. The owner reproduced the bug in actual Nox execution inside a returning loop:
expected11, actual999 after the wrong `x.a == 22` branch
(`/tmp/nox-loop-type-shadow-before.log`). The fix now clears sealed type frames,
stops type lookup at the same nearest lexical binding as value lookup, and
clears stale same-frame types on bind. The finding is now **resolved**: the named actual regression
`returning_loop_seals_shadowed_aggregate_type_with_its_name ... ok` appears in
`/tmp/trident-nox-return-workspace-neural-final.log`, inspected independently.
It exercises debug/release and return/fallthrough inputs0,1,9. The owner reports
858 workspace tests (including39 Nox surface tests) passing and two genuine
Joy public/private proof tests passing; those runs were not executed by this
reviewer. Earlier scalar-shadow tests did not cover the defect.

The new loop continuation design otherwise preserves the reviewed semantics:

- `return` compiles directly to the function result. Only fallthrough calls the
  continuation that drops iteration-local subject bindings and starts later
  iterations/the post-loop continuation. Nested loops share that function result
  continuation; no synthetic scalar flag truncates aggregate returns.
- The post-loop continuation is compiled against the outer scope; that scope is
  restored before compiling iteration bodies. The analogous non-returning loop
  restoration fixes following bindings shifting body variable axes.
- Dynamic end guards are evaluated lazily against the current outer subject.
  Inactive bodies do not evaluate assertions, secret calls or state lookups.
  Static empty/reversed ranges and zero dynamic bounds preserve fallthrough.
- The actual shared iteration bound is4096, not256. Each returning-loop index
  addition is checked; emitted continuation trees are additionally limited to
  2,000,000 nodes. Subject depth checks remain active. These are bounds, not a
  claim that every syntactically bounded nested loop fits the formula budget.
- Struct/tuple results retain the ordinary aggregate noun representation.
  Existing new tests cover nested helper returns, unit returns, shadowing,
  post-loop locals and unreachable secret/state effects in debug and release.
- `match` is still explicitly unsupported by Nox lowering; recursive return
  discovery does not falsely make match lowering available.

No additional actionable issue was identified in this bounded review. This is
not a proof of all compiler behavior or a new heavy proof/test-suite execution.
