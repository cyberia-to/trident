# Direct self-built compiler CLI rehearsal

The exact three commands in `receipt.json` packaged `sample.tri`, executed
actual C2 to produce `sample.dag`, and executed that program to `answer.dag`.
Every command exited successfully with empty stderr; the complete output is
canonical atom 13. Inputs and outputs are retained byte for byte here.

C2 came from run `36359020560`, artifact `10948746458`, macOS ARM repetition 1,
`repeat-1/c2.dag`; its SHA-256 is
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
The complete original C2 and producer chain are retained in the
[platform store](../run-36359020560/README.md).
The rehearsal uses local installed Joy `ec83bd8d85b20a8bd20d2d14b0f25aab0f75e9fe`,
binary SHA-256 `1e2efc36fb8900c53965a1c31c176c99462dad94576396f5dd3c55ff871cbee5`.
Its original installation receipt is retained losslessly. Local sibling working
trees do not supply clean release provenance; the native CI gate remains separate.

The source and manifest explicitly declare all job limits. The commands use
ordinary Joy host defaults. `zero.dag` is the existing independent compiler
vector's canonical atom-zero input. The final 85-byte output was read independently:
NOXDAG01, one atom entry, equal header/root particle, tag 8, payload 13.
This supplements the full semantic corpora; it establishes no compilation proof.

[The usage guide](../../../../docs/guides/self-hosted-compilation.md) explains
these same commands without depending on a benchmark driver.
