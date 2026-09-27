# Native source capacity review

Scope: source changes based on inventory delivery `34afe52`. This is a source
review; executable acceptance, final revision and resource measurements belong
to the validation receipt. It does not establish complete self-compilation.

- Determinism: source byte admission uses a per-file ceiling and the explicit
  JOB1 total. Discovery still opens each reached module once, preserves original
  package indices and records each direct-use occurrence. Sorting and name
  equality inspect complete spans; their ordering and declaration identities
  are unchanged. Source bytes, comments and offsets remain intact.
- Types: `SOURCE_CAP` bounds byte offsets; `TABLE_CAP` bounds internal IDs,
  descriptors and append-only tables. Absent declaration IDs retain their
  existing sentinel. Parameter/function/module IDs are never derived from byte
  offsets. U32 increments require room for the next U32 value; table and source
  owners independently enforce their narrower bounds.
- Errors: selected and newly reached files reject per-file excess before
  guest payload validation. Remaining total capacity is computed before each source
  is opened. The entry is already bounded by its admitted JOB allowance.
  Encoding and binding errors retain the original source and span. Arena,
  reduction and frame exhaustion remain execution failures outside RES1.
- Complete scans: identifier, decimal, raw numeric-bound, name-copy, name-order,
  name-equality, same-line and UTF-8 scans cover the per-file ceiling. Decimal
  overflow consumes the complete literal. Bounded parser and call-graph drivers
  retain separate work limits and return capacity errors on exhaustion.
  No oversized token is accepted by returning a matching prefix.
- Tables: parser and graph capacities still clamp the requested sequence cap
  to the internal ID bound. Intrinsic/export trees, type-registry absence and
  function/parameter descriptors continue using that bound. Module/use counts
  independently bound graph traversal; call-graph work no longer assumes that
  the entire package fits the old source byte limit.
- Validation scope: component tests may supply execution limits above Joy's
  supported job tier to test full scans. Complete JOB1 tests and installed
  acceptance record their own limits. Component success does not establish
  that every admitted source fits the current worker tier. The Rust test host
  accepts Joy's explicit maximum validation allowance while existing requests
  keep their original allowance.
- Readability: capacity names now describe the resource they bound. Literal
  loop ceilings remain explicit because the native compiler currently requires
  literal loop ranges. The canonical job/reference contracts distinguish source
  admission, internal storage, per-pass work and execution limits.
- Existing quotas: whitespace classification uses the exact ASCII range with
  vertical tab excluded. Its byte truth table and wide U32 rejection are checked
  independently. This reduces the compiler footprint enough to retain the
  original nested-group CLI job's fixed arena allowance; that job's metadata is
  also exercised by the Rust regression. No requested capacity is increased.
