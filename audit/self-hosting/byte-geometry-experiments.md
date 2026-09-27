# Byte geometry experiments

Both proposed changes were measured and kept out of the implementation. The
complete literal compiler source still exceeds the original arena. A cached
capacity reduces some work but expands several workloads; common-length
thresholds offer small allocation changes and do not remove the scale barrier.
The working source returns to `533c4a31e47580aabfd2f09bdadd69784cc31260`.

The [receipt](byte-geometry-experiments.json) contains every command, complete
sources, binary/C1 identities and source hashes. Reproduce each experiment from
that revision with its recorded patch. These are local raw0/0 measurements,
with explicit786432 arena nodes,100000000 reductions and60000ms; they do not
establish generated compiler profiles or self-compilation.

| Workload | Original nodes / reductions | Cached capacity | Thresholds |
|---|---|---|---|
| literal-compiler-raw | Unavailable | Unavailable | Unavailable |
| scan-only | 687086 / 62459242 | 693474 / 48607839 | 687146 / 55878574 |
| record-write-wide-64-arena | 771885 / 36976878 | 777865 / 32375459 | 771348 / 35214478 |
| record-long-arena | 781469 / 74158171 | 779992 / 61438954 | 781220 / 68882901 |
| sequence-wrapper | 27575 / 1484361 | 27824 / 1582153 | 27716 / 1484361 |

Both modified variants pass18 collection tests, including all U32 tree-height
boundaries. Each original record program keeps its complete output (3199 and79);
the Seq probe keeps7936. The original source/protocol/private layouts and
validation charges are retained in the implementation. Follow-up runtime work
uses an explicit larger arena and independent budget/deadline allowances;
old fixed-limit outcomes remain part of acceptance.
