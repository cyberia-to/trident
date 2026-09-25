# Guest constant imports through C1

Compiler source `17685e1353e01a2200f28cfc993a4d006d0c0412`; acceptance harness `655ac691837c529abc10c4b9283d5d2338526df5`.
The [receipt](guest-constant-linking-validation.json)
pins sibling revisions, commands and local macOS ARM64 evidence.

C1 resolves the exact JOB1 package on nox: validate reached UTF-8/headers, discover
and order direct uses, validate all dependency constants, publish final public
bindings, then compile the entry. Full/short aliases retain per-symbol source
order; private replacements withdraw exports without erasing another imported
owner's earlier public symbol. Literal provenance stays with its original source.
Runtime constant nodes use normalized values and caller spans. Discovery, checking and emission run entirely on nox.

```sh
python3 audit/self-hosting/check-guest-constant-linking.py --joy ../install/bin/joy --output /tmp/native-constant-linking-imports-cli.json
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --time-ms 60000 --output /tmp/native-constant-linking-full-cli.json
python3 audit/self-hosting/check-guest-module-graph.py --joy ../install/bin/joy --output /tmp/native-constant-linking-graph-cli.json
```

Installed acceptance: 135 commands / 31 import observations compare complete
execution outputs with the seed, check dependency diagnostics and preserve prior
output files on rejection. The 1195-command / 401-observation corpus
preserves all 234 successful compilation observations, representing 202 distinct
ART1 programs. The current corpus has 235 successful compilations and 203
distinct ART1 programs. The graph corpus preserves all 12 complete outputs/admission
allowances across 39 commands. All binaries remain fixed during these runs. The host deadline is explicitly
60000ms; the public 30000ms default remains unchanged.

All 1143 Trident, 122 Joy and 380 Trisha CPU tests pass
without warnings; four existing Trisha tests remain ignored. All 133 baseline
rows / 43 manual programs are unchanged. All 106 formal audits report
UNKNOWN. These audits do not establish a native proof relation.

The original 61–64-bit record-write programs still return 3199 under the same
786432-node ceiling. The 64-bit case uses 767225 nodes,
36667976 reductions and 1346 peak frames. The 65-bit case still exhausts the arena. The unchanged 2158-byte combined
long-name source now compiles and returns 79 using 776801 nodes at the same arena ceiling; the old
expected-Unavailable assertion was corrected after observing this improvement. The final two tree levels are
read without allocating a continuation; Bytes privately caches its word count
while keeping BYT1 and admission charges unchanged. A no-import entry bypasses
graph storage after the shared UTF-8/header validation.

This slice admits constant/import dependencies. Dependency functions, structs
and attributes, qualified calls/constructors/types and legacy remaps remain
unsupported. Generated compiler profiles, complete compiler-scale closure,
C2/C3, six platforms and native Zheng gates remain open. Noun stays 128K.
