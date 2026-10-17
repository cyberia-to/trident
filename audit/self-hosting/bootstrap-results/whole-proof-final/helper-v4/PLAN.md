# Reuse authenticated unchanged diagnostic frames

The v3 C1 cost-mutation helper reached its unchanged 1800-second deadline.
The original coordinator remains failed, its cleanup exception remains visible,
and the final acceptance checker did not launch. This change concerns the
negative-test file constructor; Joy, Zheng, source inputs and proof limits stay
unchanged.

Input::next already authenticates each frame once. Seal frame header/payload
and keep an optional digest that only this checked reader can supply. Output
may reuse it only when the complete emitted header is byte-identical; new
frames and rewritten headers always require a fresh digest. This removes a
second identical full-prefix hash while preserving every output byte and every
input validation. Verify exact output equality against the original helper on
all existing bounded fixture mutation modes and unchanged/rebound chain tests;
exercise altered frame/context/order and untrusted newly encoded frames.

Build and validate in the isolated helper directory with actual Rust 1.89.
Retain source and command identities. Independent source review precedes any
new complete-proof negative schedule. Do not alter or retry the failed v3 run,
raise caps, claim incomplete cases passed, or reclaim old files here. A new
completion schedule and any proven duplicate-prefix reclamation need separate
explicit source-bound review. The current PR117 matrix is untouched.
