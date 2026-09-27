# Native compiler source inventory

Tool source `aa43e4f860771771dbfbabb3ae23c9a147671051`; validated integration `a3cf879931d22b29c42c1444e7dc8db41c4cd164`.
The [receipt](native-source-inventory-validation.json) pins every command, source
identity and sibling revision. [Native closure](native-compiler-closure.json)
records all declarations in `compiler/nox/main.tri` and its canonical imports:
94 modules, 455 functions and 345639 source bytes. These are local macOS ARM64 results.

`selfhost_inventory --entry compiler/nox/main.tri` resolves the program's declared
name and follows parser-owned imports, deduplicating diamonds. It rejects missing,
malformed or mismatched modules, imported programs and program/module ambiguity.
Paths are relative; relocation preserves output. Without `--entry`, the original
RAM compiler inventory remains byte-identical. `--check` rejects stale evidence
without replacing it. Nine focused tests and actual Cargo CLI checks pass.

All seven owner gates pass: 1197 Trident, 123 Joy and 380 Trisha tests, zero Rust
warnings and four existing ignored Trisha tests. All 133 baseline rows / 43 manual
programs remain unchanged. All 119 installed formal verdicts remain UNKNOWN.
The audit-driver PATH failure and its correction remain recorded.

Source and integrated committed installs reproduce all three binaries and the
C1/graph artifacts. C1 remains `1acb0acf2950528c366db85e30e33ac0cc4450ba7d538d517664492bd153be1c`,
with 95315 DAG entries. The preceding intrinsic delivery's
1199-command execution corpus is explicitly reused: guest/compiler/runtime code
and sibling revisions are unchanged; only the inventory example and documents change.

This closes reproducible enumeration of the complete native source closure.
Source admission and compiler-scale execution still require SH4 work; C2/C3,
six CPU platforms and native Zheng proofs remain open. Noun stays 128K.
