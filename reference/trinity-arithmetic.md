# Trinity arithmetic demonstration

`std.trinity.inference.trinity` retains its 29-argument interface and composes
ciphertext-shaped linear arithmetic, witness-assisted decryption, a plaintext
dense layer and lookup activation, two custom scalar hashes, PBS witness
transport, and an unnormalized two-qubit field-matrix calculation. A passing
execution demonstrates these equations. It does not establish secure TFHE,
private inference, or a quantum commitment.

The independent fixtures in Trisha's `baselines/triton/fixtures/trinity-*`
exercise two inputs, two neurons, a four-coefficient ring, all 29 arguments,
86 custom-hash constants and 112 LUT-sponge constants. Ascending and descending
plaintexts exercise both classification results. Expected ciphertext/plaintext
outputs, both hashes and the private witness stream come from integer matrix
arithmetic and independent modular Python oracles in
`scripts/generate_trinity_baseline.py`. Incorrect decryption witnesses are
rejected by both source and hand execution. The hand implementation uses
scratch `[6000000,6011000)` and links independent tensor/custom-hash libraries.

## Unclosed cryptographic requirements

- `lwe.decrypt` uses a high-word-only noise comparison. It does not establish a
  full integer noise bound, secure parameters, key generation or input encoding.
- `lut_sponge` constrains `x = q*d + r` in the field with `r < d`; without an
  integer quotient bound this does not uniquely determine the integer remainder.
- `pbs.bootstrap` does not bind rotation to the input ciphertext. Monomial source
  indices/signs and key-switched ciphertext words are supplied without the
  claimed rotation/key-switch equations. A final decryption equality cannot
  supply those missing equations. The input ciphertext parameter is not read.
- Caller-supplied `weights_digest` and `key_digest` are not recomputed from the
  corresponding arrays. The legacy custom permutation and lookup S-box do not
  carry a reviewed binding guarantee. Both outputs are only one field element.
- The final Boolean is a deterministic field-matrix simulation. It identifies
  zero versus nonzero class here; it is not a binding commitment or a quantum
  hardware measurement.

These gaps remain release gates for any private-inference feature. Correcting
comments and adding arithmetic fixtures does not close them. A production path
must first choose a reviewed encryption/bootstrapping protocol and parameters,
then constrain canonical encoding/noise, rotation, extraction and key switching,
bind the actual input/model/key/table identities, and use a reviewed digest
construction. Adversarial tests must mutate each of those links independently.

## Tensor comparison correction

The dense classifier now compares both canonical 32-bit limbs. Earlier `argmax`
incorrectly treated values such as 1 and 2 as equal because it compared only the
high limb. The same correction fixes ReLU at the field sign threshold. Source
and independent hand tests cover reversed inputs, values crossing 2^32 and the
threshold `(p-1)/2` under both source optimization profiles. Negative values use
this documented field encoding convention; this is not floating-point inference.
