# Full-package identity and relocation audit

Package identity passed: relocating all 94 source files and reversing their
manifest entries produced exactly the JOB1 bytes from the successful C1(S) run.
Changing one dependency comment changed its source, module, package and JOB1
identities while preserving all other admitted modules. This closes the
full-package identity slice of SH4; it performs no second self-compilation and
does not establish C2 usability, a fixed point or platform reproduction.

The method binds the completed receipt to actual compiler, source and artifact
bytes with the fixed-point checker's single-step validation, then invokes Joy
`pack-job` in an independent temporary directory. The retained commands, hashes
and exact source archive support each comparison. Integrity guards reject a
tampered original receipt, compiler or Joy binary before tool invocation.

## Measured result

[The receipt](receipt.json) passed against the actual successful full-source
[C1-to-C2 execution](../body-scale/c1-to-c2.json), frozen Trident source
`b991d901e6585a40bedd0e0a3d4382c2ad3d89c1`. C1 SHA256 was
`4b8276068704063eb13e5555bca872372d2975b4ac71cf93edc03273222327b6`;
Joy was `506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8`.
The 94 relocated sources retained all 369,820 bytes. Reversing every package
entry and using the independent directory as cwd produced the identical
6,754,045-byte JOB1, SHA256
`262fdfaa44f91d0bca113addf9711e71949dd5c63fb888a747543107ee6cdb0e`.
The entire admission package metadata also remained equal. This is exact input
equality tied to the recorded successful computation, not a second execution.

Changing comment byte 254 of `std.nox.bytes` from `T` to `t` retained its
12,478-byte length. Source and MOD1 identities changed; all other 93 modules
remained identical. The package particle changed from
`4aca1b9df73e965da4479d1e435f75076acdf84a7b95eb250084f93446fc4f79` to
`e0b93bee81f2e900530abbf1b78968e7a480afe76f14b2a35823a8135af21154`.
Changed JOB1 SHA256 is
`a97c5866602e6834e38d48c63d04d791c693e913db1c9921c72580ee14a322e7`.
Compiler, options, origins and every limit stayed fixed. No output-equivalence
claim is made for the changed source because it was packed but not executed.

The [deterministic archive](receipt.inputs.tar.gz) retains all exact moved
sources, the single changed dependency and both manifests (83,412 bytes,
SHA256 `4049adc3cc99c32fca8bde0122452d7cf6f11322d7e4d40ce65326fcd695806b`).
Compiler and duplicate JOB1 bytes remain in the original full-source evidence.
The receipt stores complete argv, cwd, exit codes, tool hashes and admission
output. Every command exited zero. Reproduce with fresh output paths:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -W error audit/self-hosting/full-package-determinism/check.py --receipt ../measurements/body-scale-closure-20b.json --receipt-sha256 354d7be504a511dfaf11303e218487d3dcbb8d11d85948af506f5fb6795e6c2f --joy ../install-compiler-work-budget/bin/joy --inventory-checker ../target-root/release/examples/selfhost_inventory --output /tmp/full-package-determinism-new.json
PYTHONDONTWRITEBYTECODE=1 python3 -W error audit/self-hosting/full-package-determinism/test_check.py
```

Six [integrity tests](tests-final.log) pass with warnings treated as errors.
The measured source is retained as [measured-check.py](measured-check.py), SHA256
`ac5273e881dddc5da268e79839f922e5052b7893b513a10d02da5436657a8b99`.
The final runner only removes an unused `shutil` import (SHA256
`4101fd63355bae9ee35bf48ca7eace747fc5cc2856ec15394124eacfe029dc1b`);
its guards were rechecked without repeating the successful pack measurements.

The separate [source-scale audit](../source-scale-compacting/README.md) retains
a valid 64 KiB frame failure. SH4 remains open until that case and any outstanding
resource-bound acceptance complete. This identity audit closes only the
full-package changed-dependency, relocation and ordering evidence slice.
